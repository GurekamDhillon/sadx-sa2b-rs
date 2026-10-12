//! Centroids per drawing node: armcheck <mdl> <model id> <mtn> <motion id> <frame>
use sa_formats::{bytes::Endian, ninja::{self, pose}, prs};
use std::collections::BTreeMap;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let models = ninja::read_model_file(&prs::decompress(&std::fs::read(&a[1]).unwrap()).unwrap(), Endian::Big).unwrap();
    let m = models.iter().find(|m| m.id == a[2].parse::<i32>().unwrap()).unwrap();
    let motions = ninja::motion::read_motion_file(&prs::decompress(&std::fs::read(&a[3]).unwrap()).unwrap(), Endian::Big).unwrap();
    let mo = motions.iter().find(|x| x.id == a[4].parse::<u16>().unwrap()).unwrap();
    let mesh = ninja::flatten(&pose::posed(&m.root, &mo.motion, a[5].parse().unwrap()));
    let mut by: BTreeMap<u32, (usize, [f32; 3])> = BTreeMap::new();
    for t in mesh.indices.chunks(3) { for &i in t {
        let e = by.entry(mesh.src_node[i as usize]).or_insert((0, [0.0; 3]));
        e.0 += 1; for k in 0..3 { e.1[k] += mesh.positions[i as usize][k]; }
    } }
    println!("(model space: forward = -x, up = y, lateral = z)");
    for (n, (c, s)) in by { println!("node {:2} (index {:2}): {:4} verts  centroid x {:6.2} y {:6.2} z {:6.2}", n, n - 1, c, s[0] / c as f32, s[1] / c as f32, s[2] / c as f32); }
}
