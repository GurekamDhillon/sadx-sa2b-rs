use sa_formats::{bytes::Endian, ninja::{self, chunk::PolyChunk, object::{Attach, Node}}, prs};
use std::collections::BTreeMap;
fn walk(n: &Node, out: &mut Vec<usize>, vk: &mut BTreeMap<u8, usize>, pk: &mut BTreeMap<String, usize>, written: &mut std::collections::HashSet<usize>, missing: &mut usize, i: &mut usize) {
    if let Some(Attach::Chunk(m)) = &n.attach {
        let mut node_missing = 0;
        for vc in &m.vertex_chunks {
            *vk.entry(vc.kind).or_default() += 1;
            for (k, v) in vc.vertices.iter().enumerate() {
                let slot = match v.ninja_flags { Some(f) if vc.weight_status().is_some() => (f & 0xFFFF) as usize + vc.index_offset as usize, _ => vc.index_offset as usize + k };
                written.insert(slot);
            }
        }
        for pc in &m.poly_chunks {
            match pc {
                PolyChunk::Strip { kind, strips, .. } => {
                    *pk.entry(format!("strip{kind}")).or_default() += 1;
                    for s in strips { for v in &s.vertices { if !written.contains(&(v.index as usize)) { node_missing += 1; } } }
                }
                PolyChunk::Texture { .. } => *pk.entry("tex".into()).or_default() += 1,
                PolyChunk::Material { kind, .. } => *pk.entry(format!("mat{kind}")).or_default() += 1,
                PolyChunk::Bits { kind, .. } => *pk.entry(format!("bits{kind}")).or_default() += 1,
                PolyChunk::Other { kind, .. } => *pk.entry(format!("other{kind}")).or_default() += 1,
            }
        }
        let mut slots: Vec<usize> = m.poly_chunks.iter().filter_map(|p| if let PolyChunk::Strip { strips, .. } = p { Some(strips) } else { None }).flatten().flat_map(|s| s.vertices.iter().map(|v| v.index as usize)).collect();
        slots.sort(); slots.dedup();
        let w: Vec<usize> = { let mut w: Vec<usize> = written.iter().copied().collect(); w.sort(); w };
        println!("node {:2}: vchunks {} {:?} | strip slots {}..{} ({} distinct) | missing {node_missing} | written so far {}..{} ({})", *i, m.vertex_chunks.len(), m.vertex_chunks.iter().map(|v| (v.kind, v.index_offset, v.vertices.len(), v.flags)).collect::<Vec<_>>(), slots.first().unwrap_or(&0), slots.last().unwrap_or(&0), slots.len(), w.first().unwrap_or(&0), w.last().unwrap_or(&0), w.len());
        *missing += node_missing;
        out.push(*i);
    }
    *i += 1;
    for c in &n.children { walk(c, out, vk, pk, written, missing, i); }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let models = ninja::read_model_file(&prs::decompress(&std::fs::read(&a[1]).unwrap()).unwrap(), Endian::Big).unwrap();
    let m = models.iter().find(|m| m.id == a[2].parse::<i32>().unwrap()).unwrap();
    let (mut vk, mut pk, mut written, mut missing, mut i, mut out) = (BTreeMap::new(), BTreeMap::new(), Default::default(), 0, 0, vec![]);
    walk(&m.root, &mut out, &mut vk, &mut pk, &mut written, &mut missing, &mut i);
    println!("nodes with chunk attach: {}", out.len());
    println!("vertex chunk kinds: {vk:?}");
    println!("poly chunk kinds: {pk:?}");
    println!("unwritten refs total: {missing}");
}
