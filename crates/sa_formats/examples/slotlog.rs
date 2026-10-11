use sa_formats::{bytes::Endian, ninja::{self, math::Mat4, object::{Attach, Node}}, prs};
fn visit(n: &Node, parent: &Mat4, idx: &mut usize, want: &[usize]) {
    let world = parent.mul(&n.local_matrix());
    let me = *idx; *idx += 1;
    if let Some(Attach::Chunk(m)) = &n.attach {
        for (ci, vc) in m.vertex_chunks.iter().enumerate() {
            for (k, v) in vc.vertices.iter().enumerate() {
                let (slot, w) = match v.ninja_flags { Some(f) if vc.weight_status().is_some() => ((f & 0xFFFF) as usize + vc.index_offset as usize, (f >> 16) as f32 / 255.0), _ => (vc.index_offset as usize + k, 1.0) };
                if want.contains(&slot) { let p = world.point(v.position); println!("slot {slot}: node {me} chunk {ci} kind {} flags {} w {:.2} world ({:.2},{:.2},{:.2})", vc.kind, vc.flags, w, p[0], p[1], p[2]); }
            }
        }
        for pc in &m.poly_chunks { if let ninja::chunk::PolyChunk::Strip { strips, .. } = pc {
            let used: Vec<usize> = strips.iter().flat_map(|s| s.vertices.iter().map(|v| v.index as usize)).filter(|i| want.contains(i)).collect();
            if !used.is_empty() { println!("  node {me} POLYS use {:?}", { let mut u = used; u.sort(); u.dedup(); u }); }
        } }
    }
    for c in &n.children { visit(c, &world, idx, want); }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let models = ninja::read_model_file(&prs::decompress(&std::fs::read(&a[1]).unwrap()).unwrap(), Endian::Big).unwrap();
    let m = models.iter().find(|m| m.id == a[2].parse::<i32>().unwrap()).unwrap();
    let want: Vec<usize> = a[3..].iter().map(|s| s.parse().unwrap()).collect();
    visit(&m.root, &Mat4::IDENTITY, &mut 0, &want);
}
