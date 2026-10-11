use sa_formats::{bytes::Endian, ninja::{self, chunk::PolyChunk, object::{Attach, Node}}, prs};
use std::collections::HashSet;
// Mode "post": vertices on entry, polys after children. Mode "pre": polys right after own vertices.
fn visit(n: &Node, post: bool, written: &mut HashSet<usize>, missing: &mut usize, total: &mut usize) {
    let polys = |n: &Node, written: &HashSet<usize>, missing: &mut usize, total: &mut usize| {
        if let Some(Attach::Chunk(m)) = &n.attach { for pc in &m.poly_chunks { if let PolyChunk::Strip { strips, .. } = pc { for s in strips { for v in &s.vertices { *total += 1; if !written.contains(&(v.index as usize)) { *missing += 1; } } } } } }
    };
    if let Some(Attach::Chunk(m)) = &n.attach {
        for vc in &m.vertex_chunks { for (k, v) in vc.vertices.iter().enumerate() {
            let slot = match v.ninja_flags { Some(f) if vc.weight_status().is_some() => (f & 0xFFFF) as usize + vc.index_offset as usize, _ => vc.index_offset as usize + k };
            written.insert(slot);
        } }
    }
    if !post { polys(n, written, missing, total); }
    for c in &n.children { visit(c, post, written, missing, total); }
    if post { polys(n, written, missing, total); }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let models = ninja::read_model_file(&prs::decompress(&std::fs::read(&a[1]).unwrap()).unwrap(), Endian::Big).unwrap();
    for m in &models {
        let r: Vec<_> = [false, true].iter().map(|&post| { let (mut w, mut miss, mut tot) = (HashSet::new(), 0, 0); visit(&m.root, post, &mut w, &mut miss, &mut tot); (miss, tot) }).collect();
        if r[0].1 > 0 { println!("model {:3}: pre-order missing {:5}/{:5}   post-order missing {:5}/{:5}", m.id, r[0].0, r[0].1, r[1].0, r[1].1); }
    }
}
