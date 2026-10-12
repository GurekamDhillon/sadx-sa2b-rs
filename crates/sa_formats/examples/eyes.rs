use sa_formats::{bytes::Endian, ninja::{self, chunk::PolyChunk, object::{Attach, Node}}, prs};
fn visit(n: &Node, idx: &mut usize, want: &[usize]) {
    let me = *idx; *idx += 1;
    if let (true, Some(Attach::Chunk(m))) = (want.contains(&me), &n.attach) {
        for p in &m.poly_chunks {
            match p {
                PolyChunk::Texture { kind, flags, texture_id } => println!("node {me}: Texture kind {kind} flags {flags:#04x} id {texture_id}"),
                PolyChunk::Material { kind, flags, diffuse } => println!("node {me}: Material kind {kind} flags {flags:#04x} diffuse {diffuse:08x?}"),
                PolyChunk::Strip { kind, flags, strips } => {
                    let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
                    for s in strips { for v in &s.vertices { if let Some(uv) = v.uv { for k in 0..2 { lo[k] = lo[k].min(uv[k]); hi[k] = hi[k].max(uv[k]); } } } }
                    println!("node {me}: Strip kind {kind} flags {flags:#04x} strips {} uv {lo:?}..{hi:?}", strips.len());
                }
                other => println!("node {me}: {other:?}"),
            }
        }
    }
    for c in &n.children { visit(c, idx, want); }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let models = ninja::read_model_file(&prs::decompress(&std::fs::read(&a[1]).unwrap()).unwrap(), Endian::Big).unwrap();
    let want: Vec<usize> = a[3..].iter().map(|s| s.parse().unwrap()).collect();
    visit(&models.iter().find(|m| m.id == a[2].parse::<i32>().unwrap()).unwrap().root, &mut 0, &want);
}
