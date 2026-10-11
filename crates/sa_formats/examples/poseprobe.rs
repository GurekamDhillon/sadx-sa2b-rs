//! Render a model posed by an animation: poseprobe <mdl.prs> <model id> <mtn.prs> <motion id> <frame> <out.png>
mod common;
use sa_formats::{bytes::Endian, ninja::{self, pose}, prs};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let models = ninja::read_model_file(&prs::decompress(&std::fs::read(&a[1]).unwrap()).unwrap(), Endian::Big).unwrap();
    let id: i32 = a[2].parse().unwrap();
    let m = models.iter().find(|m| m.id == id).expect("model id");
    let motions = ninja::motion::read_motion_file(&prs::decompress(&std::fs::read(&a[3]).unwrap()).unwrap(), Endian::Big).unwrap();
    let mid: u16 = a[4].parse().unwrap();
    let mo = motions.iter().find(|m| m.id == mid).expect("motion id");
    let frame: f32 = a[5].parse().unwrap();
    println!("model {id}: {} nodes, motion {mid}: {} node tracks", m.root.count(), mo.motion.nodes.len());
    let mesh = ninja::flatten(&pose::posed(&m.root, &mo.motion, frame));
    println!("tris {} bounds {:?}", mesh.triangle_count(), mesh.bounds());
    common::render(&mesh, &a[6], false);
}
