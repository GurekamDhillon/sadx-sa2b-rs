//! For each polygon-only node, find the preorder time at which drawing its strips
//! from the replayed vertex cache gives the least stretched triangles.
use sa_formats::{bytes::Endian, ninja::{self, chunk::{PolyChunk, WeightStatus}, math::Mat4, object::{Attach, Node}}, prs};
struct Ev<'a> { node: &'a Node, world: Mat4, depth: usize }
fn collect<'a>(n: &'a Node, parent: &Mat4, depth: usize, out: &mut Vec<Ev<'a>>) {
    let world = parent.mul(&n.local_matrix());
    out.push(Ev { node: n, world, depth });
    for c in &n.children { collect(c, &world, depth + 1, out); }
}
fn write(cache: &mut Vec<[f32; 4]>, n: &Node, world: &Mat4) {
    if let Some(Attach::Chunk(m)) = &n.attach {
        for vc in &m.vertex_chunks {
            let ws = vc.weight_status();
            for (k, v) in vc.vertices.iter().enumerate() {
                let (slot, w) = match (ws, v.ninja_flags) { (Some(_), Some(f)) => ((f & 0xFFFF) as usize + vc.index_offset as usize, (f >> 16) as f32 / 255.0), _ => (vc.index_offset as usize + k, 1.0) };
                if slot >= cache.len() { cache.resize(slot + 1, [0.0; 4]); }
                let p = world.point(v.position);
                let e = &mut cache[slot];
                if matches!(ws, None | Some(WeightStatus::Start)) { *e = [p[0] * w, p[1] * w, p[2] * w, 1.0]; } else { for i in 0..3 { e[i] += p[i] * w; } e[3] = 1.0; }
            }
        }
    }
}
fn max_edge(cache: &[[f32; 4]], n: &Node) -> f32 {
    let mut worst = 0.0f32;
    if let Some(Attach::Chunk(m)) = &n.attach { for pc in &m.poly_chunks { if let PolyChunk::Strip { strips, .. } = pc { for s in strips { for w in s.vertices.windows(2) {
        let (p, q) = (cache.get(w[0].index as usize), cache.get(w[1].index as usize));
        match (p, q) {
            (Some(p), Some(q)) if p[3] > 0.5 && q[3] > 0.5 => worst = worst.max(((p[0]-q[0]).powi(2)+(p[1]-q[1]).powi(2)+(p[2]-q[2]).powi(2)).sqrt()),
            _ => worst = worst.max(1000.0),
        }
    } } } } }
    worst
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let rigid_writes = a.get(3).map(|s| s == "rigid-writes").unwrap_or(true);
    let models = ninja::read_model_file(&prs::decompress(&std::fs::read(&a[1]).unwrap()).unwrap(), Endian::Big).unwrap();
    let m = models.iter().find(|m| m.id == a[2].parse::<i32>().unwrap()).unwrap();
    let mut evs = vec![]; collect(&m.root, &Mat4::IDENTITY, 0, &mut evs);
    let has_poly = |n: &Node| matches!(&n.attach, Some(Attach::Chunk(c)) if c.poly_chunks.iter().any(|p| matches!(p, PolyChunk::Strip { .. })));
    let has_v = |n: &Node| matches!(&n.attach, Some(Attach::Chunk(c)) if !c.vertex_chunks.is_empty());
    // Snapshots after processing events 0..t (t = 0..=len).
    let mut snaps: Vec<Vec<[f32; 4]>> = vec![vec![]];
    let mut cache: Vec<[f32; 4]> = vec![];
    for e in &evs {
        if rigid_writes || !(has_poly(e.node) && has_v(e.node)) { write(&mut cache, e.node, &e.world); }
        snaps.push(cache.clone());
    }
    let subtree_end = |i: usize| { let d = evs[i].depth; let mut j = i + 1; while j < evs.len() && evs[j].depth > d { j += 1; } j };
    for (i, e) in evs.iter().enumerate() {
        if !has_poly(e.node) || has_v(e.node) { continue; }
        let scores: Vec<f32> = snaps.iter().map(|s| max_edge(s, e.node)).collect();
        let (best_t, best) = scores.iter().enumerate().fold((0, f32::MAX), |b, (t, &s)| if s < b.1 - 1e-4 { (t, s) } else { b });
        let ok: Vec<usize> = scores.iter().enumerate().filter(|(_, &s)| s < best + 0.35).map(|(t, _)| t).collect();
        println!("skin node {i:2} (depth {}, subtree ends {}): pre {:.2} post {:.2} best t={} {:.2}; good times {:?}..{:?}", e.depth, subtree_end(i), scores[i + 1], scores[subtree_end(i)], best_t, best, ok.first(), ok.last());
    }
}
