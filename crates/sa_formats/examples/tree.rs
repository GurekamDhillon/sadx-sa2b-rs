use sa_formats::{bytes::Endian, ninja::{self, chunk::PolyChunk, object::{Attach, Node}}, prs};
fn visit(n: &Node, depth: usize, idx: &mut usize) {
    let me = *idx; *idx += 1;
    let (v, p, f): (String, usize, String) = match &n.attach {
        Some(Attach::Chunk(m)) => (m.vertex_chunks.iter().map(|c| format!("{}@{}x{}", if c.weight_status().is_some() { ["S","M","E"][(c.flags & 3).min(2) as usize] } else { "v" }, c.index_offset, c.vertices.len())).collect::<Vec<_>>().join(","),
            m.poly_chunks.iter().filter(|p| matches!(p, PolyChunk::Strip { .. })).count(),
            { let mut s: Vec<usize> = m.poly_chunks.iter().filter_map(|p| if let PolyChunk::Strip { strips, .. } = p { Some(strips) } else { None }).flatten().flat_map(|s| s.vertices.iter().map(|v| v.index as usize)).collect(); s.sort(); s.dedup(); if s.is_empty() { String::new() } else { format!("uses {}..{} ({})", s[0], s[s.len()-1], s.len()) } }),
        _ => (String::new(), 0, String::new()),
    };
    println!("{}{me:2} flags {:#04x} {} {} {}", "  ".repeat(depth), n.flags, if v.is_empty() { "-".to_string() } else { format!("V[{v}]") }, if p > 0 { format!("P{p}") } else { String::new() }, f);
    for c in &n.children { visit(c, depth + 1, idx); }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let models = ninja::read_model_file(&prs::decompress(&std::fs::read(&a[1]).unwrap()).unwrap(), Endian::Big).unwrap();
    visit(&models.iter().find(|m| m.id == a[2].parse::<i32>().unwrap()).unwrap().root, 0, &mut 0);
}
