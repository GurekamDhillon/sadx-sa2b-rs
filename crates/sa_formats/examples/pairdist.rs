//! Weighted-vertex consistency: distance between the Start-copy and End-copy world
//! positions of each weighted vertex, listed worst first with the nodes involved.
//! pairdist <mdl> <model id> [<mtn> <motion id> <frame>]
use sa_formats::{bytes::Endian, ninja::{self, chunk::WeightStatus, math::Mat4, object::{Attach, Node}}, prs};
use std::collections::HashMap;
struct St { slots: HashMap<usize, Vec<([f32; 3], usize)>>, out: Vec<(f32, usize, usize, usize)>, idx: usize }
fn visit(n: &Node, parent: &Mat4, st: &mut St) {
    let world = parent.mul(&n.local_matrix());
    let me = st.idx; st.idx += 1;
    if let Some(Attach::Chunk(m)) = &n.attach {
        for vc in &m.vertex_chunks {
            let Some(ws) = vc.weight_status() else { continue };
            for v in &vc.vertices {
                let f = v.ninja_flags.unwrap();
                let slot = (f & 0xFFFF) as usize + vc.index_offset as usize;
                let p = world.point(v.position);
                let e = st.slots.entry(slot).or_default();
                if ws == WeightStatus::Start { e.clear(); }
                e.push((p, me));
                if ws == WeightStatus::End && e.len() >= 2 {
                    let (a, b) = (e[0], e[e.len() - 1]);
                    let d = ((a.0[0] - b.0[0]).powi(2) + (a.0[1] - b.0[1]).powi(2) + (a.0[2] - b.0[2]).powi(2)).sqrt();
                    st.out.push((d, slot, a.1, b.1));
                }
            }
        }
    }
    for c in &n.children { visit(c, &world, st); }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let models = ninja::read_model_file(&prs::decompress(&std::fs::read(&a[1]).unwrap()).unwrap(), Endian::Big).unwrap();
    let m = models.iter().find(|m| m.id == a[2].parse::<i32>().unwrap()).unwrap();
    let root = if a.len() > 5 {
        let motions = ninja::motion::read_motion_file(&prs::decompress(&std::fs::read(&a[3]).unwrap()).unwrap(), Endian::Big).unwrap();
        let mo = motions.iter().find(|x| x.id == a[4].parse::<u16>().unwrap()).unwrap();
        ninja::pose::posed(&m.root, &mo.motion, a[5].parse().unwrap())
    } else { m.root.clone() };
    let mut st = St { slots: HashMap::new(), out: vec![], idx: 0 };
    visit(&root, &Mat4::IDENTITY, &mut st);
    let mut d: Vec<f32> = st.out.iter().map(|o| o.0).collect(); d.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = d.len().max(1);
    println!("pairs {}: median {:.3} p90 {:.3} max {:.3}", d.len(), d.get(n / 2).unwrap_or(&0.0), d.get(n * 9 / 10).unwrap_or(&0.0), d.last().unwrap_or(&0.0));
    let mut by: std::collections::BTreeMap<(usize, usize), (usize, f32)> = Default::default();
    for (dist, _, s, e) in &st.out { let x = by.entry((*s, *e)).or_default(); x.0 += 1; x.1 = x.1.max(*dist); }
    let mut v: Vec<_> = by.into_iter().collect(); v.sort_by(|a, b| b.1.1.partial_cmp(&a.1.1).unwrap());
    for ((s, e), (c, mx)) in v.iter().take(8) { println!("  start node {s:2} -> end node {e:2}: {c:3} pairs, worst {mx:.2}"); }
}
