//! Research tool: x86 disassembly straight from a PE file (no analysis DB).
//!   disasm <exe> refs <hexaddr>...      code locations whose instructions reference an address
//!   disasm <exe> fn <hexaddr> [max]     disassemble from an address until ret+padding (or max instrs)
//!   disasm <exe> k <hexaddr>...         print f32/f64/u32 at data addresses
use iced_x86::{Decoder, DecoderOptions, Formatter, Instruction, IntelFormatter, Mnemonic, OpKind};
use sa_formats::{bytes::AddressMap, pe::Pe};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let data = std::fs::read(&a[1]).unwrap();
    let pe = Pe::parse(&data).unwrap();
    let text = pe.sections.iter().find(|s| s.name == ".text").unwrap();
    let base = (pe.image_base + text.virtual_address) as u64;
    let code = &data[text.raw_offset as usize..(text.raw_offset + text.raw_size) as usize];
    let hex = |s: &str| u64::from_str_radix(s.trim_start_matches("0x"), 16).unwrap();
    match a[2].as_str() {
        "refs" => {
            let targets: Vec<u64> = a[3..].iter().map(|s| hex(s)).collect();
            // Linear sweep is unreliable on data; resync by scanning for the address bytes.
            for t in &targets {
                let needle = (*t as u32).to_le_bytes();
                for (i, w) in code.windows(4).enumerate() {
                    if w != needle { continue; }
                    // Try decoding an instruction starting 2..=3 bytes earlier that contains this operand.
                    for back in 1..=4usize {
                        if i < back { continue; }
                        let ip = base + (i - back) as u64;
                        let mut d = Decoder::with_ip(32, &code[i - back..], ip, DecoderOptions::NONE);
                        let ins = d.decode();
                        if !ins.is_invalid() && ins.len() >= back + 4 && ins.memory_displacement64() == *t {
                            let mut s = String::new();
                            IntelFormatter::new().format(&ins, &mut s);
                            println!("{t:#x} <- {:#x}: {s}", ins.ip());
                            break;
                        }
                    }
                }
            }
        }
        "fn" => {
            let start = hex(&a[3]);
            let max: usize = a.get(4).map(|s| s.parse().unwrap()).unwrap_or(2000);
            let off = (start - base) as usize;
            let mut d = Decoder::with_ip(32, &code[off..], start, DecoderOptions::NONE);
            let mut f = IntelFormatter::new();
            let mut ins = Instruction::default();
            let mut n = 0;
            let mut furthest = start;
            while d.can_decode() && n < max {
                d.decode_out(&mut ins);
                let mut s = String::new();
                f.format(&ins, &mut s);
                // Show float constants loaded from memory.
                let mut note = String::new();
                if ins.memory_displacement64() > base && ins.memory_base() == iced_x86::Register::None {
                    let addr = ins.memory_displacement64() as u32;
                    if let Some(o) = pe.offset(addr) {
                        if let Some(b) = data.get(o..o + 8) {
                            let f32v = f32::from_le_bytes(b[..4].try_into().unwrap());
                            let f64v = f64::from_le_bytes(b.try_into().unwrap());
                            note = format!("   ; [f32 {f32v:e} | f64 {f64v:e}]");
                        }
                    }
                }
                println!("{:#x}: {s}{note}", ins.ip());
                for i in 0..ins.op_count() {
                    if matches!(ins.op_kind(i), OpKind::NearBranch32) && ins.mnemonic() != Mnemonic::Call {
                        furthest = furthest.max(ins.near_branch_target());
                    }
                }
                n += 1;
                if ins.mnemonic() == Mnemonic::Ret && ins.ip() >= furthest { break; }
            }
        }
        "calls" => {
            // Direct E8 rel32 calls to any of the given targets.
            let targets: Vec<u64> = a[3..].iter().map(|s| hex(s)).collect();
            for i in 0..code.len().saturating_sub(5) {
                if code[i] != 0xE8 { continue; }
                let rel = i32::from_le_bytes(code[i + 1..i + 5].try_into().unwrap()) as i64;
                let target = (base as i64 + i as i64 + 5 + rel) as u64;
                if targets.contains(&target) { println!("{target:#x} <- {:#x}", base + i as u64); }
            }
        }
        "disp" => {
            // Linear sweep: FPU instructions using [reg+disp] with the given displacement.
            let want: Vec<u64> = a[3..].iter().map(|s| hex(s)).collect();
            let mut d = Decoder::with_ip(32, code, base, DecoderOptions::NONE);
            let mut f = IntelFormatter::new();
            let mut ins = Instruction::default();
            while d.can_decode() {
                d.decode_out(&mut ins);
                if ins.memory_base() != iced_x86::Register::None && ins.memory_base() != iced_x86::Register::EBP && ins.memory_base() != iced_x86::Register::ESP
                    && want.contains(&ins.memory_displacement64()) && format!("{:?}", ins.mnemonic()).starts_with('F') {
                    let mut s = String::new();
                    f.format(&ins, &mut s);
                    println!("{:#x}: {s}", ins.ip());
                }
            }
        }
        "seq" => {
            // seq <lo> <hi> <dispA> <dispB>: FPU load from [reg+dispA] followed within 4
            // instructions by an FPU store to [reg+dispB], inside [lo, hi).
            let (lo, hi, da, db) = (hex(&a[3]), hex(&a[4]), hex(&a[5]), hex(&a[6]));
            let start = (lo - base) as usize;
            let end = ((hi - base) as usize).min(code.len());
            let mut d = Decoder::with_ip(32, &code[start..end], lo, DecoderOptions::NONE);
            let mut win: Vec<Instruction> = Vec::new();
            let is_mem = |i: &Instruction, disp: u64| i.memory_base() != iced_x86::Register::None && i.memory_base() != iced_x86::Register::EBP && i.memory_base() != iced_x86::Register::ESP && i.memory_displacement64() == disp;
            while d.can_decode() {
                let ins = d.decode();
                if matches!(ins.mnemonic(), Mnemonic::Fstp | Mnemonic::Fst | Mnemonic::Mov) && ins.op0_kind() == OpKind::Memory && is_mem(&ins, db) {
                    if let Some(src) = win.iter().find(|w| matches!(w.mnemonic(), Mnemonic::Fld | Mnemonic::Mov) && w.op_count() == 2 && w.op1_kind() == OpKind::Memory && is_mem(w, da)) {
                        println!("{:#x} -> {:#x}", src.ip(), ins.ip());
                    }
                }
                win.push(ins);
                if win.len() > 10 { win.remove(0); }
            }
        }
        "imm" => {
            // imm <lo> <hi> <mnemonic> <value>: instructions with that mnemonic and immediate.
            let (lo, hi) = (hex(&a[3]), hex(&a[4]));
            let want = a[5].to_lowercase();
            let val = hex(&a[6]);
            let start = (lo - base) as usize;
            let end = ((hi - base) as usize).min(code.len());
            let mut d = Decoder::with_ip(32, &code[start..end], lo, DecoderOptions::NONE);
            let mut f = IntelFormatter::new();
            while d.can_decode() {
                let ins = d.decode();
                if format!("{:?}", ins.mnemonic()).to_lowercase() == want && (0..ins.op_count()).any(|i| matches!(ins.op_kind(i), OpKind::Immediate8 | OpKind::Immediate16 | OpKind::Immediate32 | OpKind::Immediate8to32 | OpKind::Immediate8to16) && ins.immediate(i) == val) {
                    let mut s = String::new();
                    f.format(&ins, &mut s);
                    println!("{:#x}: {s}", ins.ip());
                }
            }
        }
        "inc0" => {
            // inc0 <lo> <hi>: fld [r]; fadd/fsub <const>; fstp [r] (in-place update of offset 0).
            let (lo, hi) = (hex(&a[3]), hex(&a[4]));
            let start = (lo - base) as usize;
            let end = ((hi - base) as usize).min(code.len());
            let mut d = Decoder::with_ip(32, &code[start..end], lo, DecoderOptions::NONE);
            let r0 = |i: &Instruction| i.memory_base() != iced_x86::Register::None && i.memory_base() != iced_x86::Register::EBP && i.memory_base() != iced_x86::Register::ESP && i.memory_displacement64() == 0 && i.memory_index() == iced_x86::Register::None;
            let mut w: Vec<Instruction> = Vec::new();
            while d.can_decode() {
                let ins = d.decode();
                w.push(ins);
                if w.len() > 6 { w.remove(0); }
                if ins.mnemonic() == Mnemonic::Fstp && r0(&ins) {
                    let n = w.len();
                    for k in 0..n.saturating_sub(1) {
                        if w[k].mnemonic() == Mnemonic::Fld && r0(&w[k]) {
                            if let Some(op) = w[k + 1..n - 1].iter().find(|x| matches!(x.mnemonic(), Mnemonic::Fadd | Mnemonic::Fsub | Mnemonic::Fmul) && x.memory_base() == iced_x86::Register::None && x.memory_displacement64() > 0x900000) {
                                let c = op.memory_displacement64() as u32;
                                let o = pe.offset(c).unwrap();
                                let v = if op.memory_size() == iced_x86::MemorySize::Float64 { f64::from_le_bytes(data[o..o + 8].try_into().unwrap()) } else { f32::from_le_bytes(data[o..o + 4].try_into().unwrap()) as f64 };
                                println!("{:#x}: {:?} {c:#x} = {v}", w[k].ip(), op.mnemonic());
                            }
                        }
                    }
                }
            }
        }
        "cst" => {
            // cst <lo> <hi> <disp>: fld <absolute const> followed by fstp [reg+disp].
            let (lo, hi, db) = (hex(&a[3]), hex(&a[4]), hex(&a[5]));
            let start = (lo - base) as usize;
            let end = ((hi - base) as usize).min(code.len());
            let mut d = Decoder::with_ip(32, &code[start..end], lo, DecoderOptions::NONE);
            let mut prev: Option<Instruction> = None;
            while d.can_decode() {
                let ins = d.decode();
                if matches!(ins.mnemonic(), Mnemonic::Fstp) && ins.memory_base() != iced_x86::Register::None && ins.memory_base() != iced_x86::Register::EBP && ins.memory_displacement64() == db {
                    if let Some(p) = prev {
                        if p.mnemonic() == Mnemonic::Fld && p.memory_base() == iced_x86::Register::None && p.memory_displacement64() > 0x900000 {
                            let c = p.memory_displacement64() as u32;
                            let o = pe.offset(c).unwrap();
                            let v = if p.memory_size() == iced_x86::MemorySize::Float64 { f64::from_le_bytes(data[o..o + 8].try_into().unwrap()) } else { f32::from_le_bytes(data[o..o + 4].try_into().unwrap()) as f64 };
                            println!("{:#x}: [{c:#x}] = {v}", p.ip());
                        }
                    }
                }
                prev = Some(ins);
            }
        }
        "near" => {
            // near <lo> <hi> <mnemA> <immA> <mnemB> <immB> <window>: both within window instrs.
            let (lo, hi) = (hex(&a[3]), hex(&a[4]));
            let (ma, ia, mb, ib) = (a[5].to_lowercase(), hex(&a[6]), a[7].to_lowercase(), hex(&a[8]));
            let win: usize = a[9].parse().unwrap();
            let start = (lo - base) as usize;
            let end = ((hi - base) as usize).min(code.len());
            let mut d = Decoder::with_ip(32, &code[start..end], lo, DecoderOptions::NONE);
            let has = |ins: &Instruction, m: &str, v: u64| format!("{:?}", ins.mnemonic()).to_lowercase() == m && (0..ins.op_count()).any(|i| matches!(ins.op_kind(i), OpKind::Immediate8 | OpKind::Immediate16 | OpKind::Immediate32 | OpKind::Immediate8to32 | OpKind::Immediate8to16) && ins.immediate(i) == v);
            let mut w: Vec<Instruction> = Vec::new();
            while d.can_decode() {
                let ins = d.decode();
                if has(&ins, &mb, ib) {
                    if let Some(x) = w.iter().find(|x| has(x, &ma, ia)) { println!("{:#x} .. {:#x}", x.ip(), ins.ip()); }
                }
                w.push(ins);
                if w.len() > win { w.remove(0); }
            }
        }
        "jt" => {
            // jt <lo> <hi>: jmp [index*4 + table] dispatches in a range.
            let (lo, hi) = (hex(&a[3]), hex(&a[4]));
            let start = (lo - base) as usize;
            let end = ((hi - base) as usize).min(code.len());
            let mut d = Decoder::with_ip(32, &code[start..end], lo, DecoderOptions::NONE);
            while d.can_decode() {
                let ins = d.decode();
                if ins.mnemonic() == Mnemonic::Jmp && ins.op0_kind() == OpKind::Memory && ins.memory_index_scale() == 4 && ins.memory_displacement64() > 0x400000 {
                    println!("{:#x}: jmp table at {:#x}", ins.ip(), ins.memory_displacement64());
                }
            }
        }
        "k" => {
            for s in &a[3..] {
                let addr = hex(s) as u32;
                let o = pe.offset(addr).expect("unmapped");
                let b = &data[o..o + 8];
                println!("{addr:#x}: f32 {:?}  f64 {:?}  u32 {:#x}", f32::from_le_bytes(b[..4].try_into().unwrap()), f64::from_le_bytes(b.try_into().unwrap()), u32::from_le_bytes(b[..4].try_into().unwrap()));
            }
        }
        _ => panic!("unknown mode"),
    }
}
