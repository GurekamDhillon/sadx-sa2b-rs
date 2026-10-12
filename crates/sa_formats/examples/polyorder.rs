use sa_formats::{bytes::Endian, ninja::{self, chunk::PolyChunk, object::{Attach, Node}}, prs};
fn visit(n: &Node, idx: &mut usize) {
    let me = *idx; *idx += 1;
    if let Some(Attach::Chunk(m)) = &n.attach {
        if !m.poly_chunks.is_empty() {
            let s: Vec<String> = m.poly_chunks.iter().map(|p| match p {
                PolyChunk::Bits { kind, flags } => format!("B{kind}({flags})"),
                PolyChunk::Texture { texture_id, .. } => format!("T{texture_id}"),
                PolyChunk::Material { kind, .. } => format!("M{kind}"),
                PolyChunk::Strip { kind, strips, .. } => format!("S{kind}x{}", strips.len()),
                PolyChunk::Other { kind, .. } => format!("O{kind}"),
            }).collect();
            println!("node {me:2} verts {}: {}", m.vertex_chunks.len(), s.join(" "));
        }
    }
    for c in &n.children { visit(c, idx); }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let models = ninja::read_model_file(&prs::decompress(&std::fs::read(&a[1]).unwrap()).unwrap(), Endian::Big).unwrap();
    visit(&models.iter().find(|m| m.id == a[2].parse::<i32>().unwrap()).unwrap().root, &mut 0);
}
