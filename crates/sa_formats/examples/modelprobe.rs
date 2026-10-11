//! Parse an SA2 *mdl.prs, flatten one model and render a front-view PNG.
//! `cargo run --example modelprobe -- sonicmdl.prs <model index> out.png [mtn.prs]`
mod common;
use sa_formats::{bytes::Endian, ninja, prs};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let data = prs::decompress(&std::fs::read(&a[1]).unwrap()).unwrap();
    let models = ninja::read_model_file(&data, Endian::Big).expect("models");
    for m in &models {
        let mesh = ninja::flatten(&m.root);
        println!("model id {:3}: nodes {:3} tris {:5} bounds {:?}", m.id, m.root.count(), mesh.triangle_count(), mesh.bounds());
    }
    let idx: usize = a[2].parse().unwrap();
    let mesh = ninja::flatten(&models[idx].root);
    common::render(&mesh, &a[3], false);
    if let Some(mtn) = a.get(4) {
        let d = prs::decompress(&std::fs::read(mtn).unwrap()).unwrap();
        let ms = ninja::motion::read_motion_file(&d, Endian::Big).expect("motions");
        let max_ang = ms.iter().flat_map(|m| m.motion.nodes.iter()).flat_map(|n| n.rotation.iter()).flat_map(|k| k.value).map(|v| v.unsigned_abs()).max();
        println!("motions: {} (first id {}, frames {}), max |angle| {:?}", ms.len(), ms[0].id, ms[0].motion.frame_count, max_ang);
    }
}

