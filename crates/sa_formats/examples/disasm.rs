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
