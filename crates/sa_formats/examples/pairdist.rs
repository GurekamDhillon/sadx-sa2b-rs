//! For weighted chunk vertices, distance between the Start-copy and End-copy world positions.
use sa_formats::{bytes::Endian, ninja::{self, chunk::WeightStatus, math::Mat4, object::{Attach, Node}}, prs};
use std::collections::HashMap;
fn visit(n: &Node, parent: &Mat4, slots: &mut HashMap<usize, Vec<([f32; 3], f32)>>, dists: &mut Vec<f32>) {
    let world = parent.mul(&n.local_matrix());
    if let Some(Attach::Chunk(m)) = &n.attach {
        for vc in &m.vertex_chunks {
            let Some(ws) = vc.weight_status() else { continue };
            for v in &vc.vertices {
                let f = v.ninja_flags.unwrap();
                let slot = (f & 0xFFFF) as usize + vc.index_offset as usize;
                let w = (f >> 16) as f32 / 255.0;
                let p = world.point(v.position);
                let e = slots.entry(slot).or_default();
                if ws == WeightStatus::Start { e.clear(); }
                e.push((p, w));
                if ws == WeightStatus::End && e.len() >= 2 {
                    let (a, b) = (e[0].0, e[e.len() - 1].0);
                    dists.push(((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt());
                }
            }
        }
    }
    for c in &n.children { visit(c, &world, slots, dists); }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let models = ninja::read_model_file(&prs::decompress(&std::fs::read(&a[1]).unwrap()).unwrap(), Endian::Big).unwrap();
    let m = models.iter().find(|m| m.id == a[2].parse::<i32>().unwrap()).unwrap();
    let (mut slots, mut d) = (HashMap::new(), Vec::new());
    visit(&m.root, &Mat4::IDENTITY, &mut slots, &mut d);
    d.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = d.len().max(1);
    println!("pairs {}: median {:.3} p90 {:.3} max {:.3}", d.len(), d.get(n / 2).unwrap_or(&0.0), d.get(n * 9 / 10).unwrap_or(&0.0), d.last().unwrap_or(&0.0));
}
