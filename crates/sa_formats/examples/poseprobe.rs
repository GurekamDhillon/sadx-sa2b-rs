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
    // Bodies with fewer nodes than the motion sit below its extra root nodes.
    let missing = mo.motion.nodes.len().saturating_sub(m.root.count());
    let rooted = if missing > 0 {
        let skel = models.iter().find(|s| s.root.count() == mo.motion.nodes.len()).expect("skeleton model");
        pose::with_prefix(&m.root, &skel.root, missing)
    } else {
        m.root.clone()
    };
    println!("prefix nodes added: {missing}, total {}", rooted.count());
    let mesh = ninja::flatten(&pose::posed(&rooted, &mo.motion, frame));
    println!("tris {} bounds {:?}", mesh.triangle_count(), mesh.bounds());
    let (mut degenerate, mut at_origin) = (0, 0);
    for t in mesh.indices.chunks(3) {
        let [p, q, r] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize]);
        let e1 = [q[0] - p[0], q[1] - p[1], q[2] - p[2]];
        let e2 = [r[0] - p[0], r[1] - p[1], r[2] - p[2]];
        let c = [e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2], e1[0] * e2[1] - e1[1] * e2[0]];
        if c.iter().map(|v| v * v).sum::<f32>() < 1e-10 { degenerate += 1; }
        if p.iter().all(|v| v.abs() < 1e-4) { at_origin += 1; }
    }
    println!("degenerate triangles {degenerate}, starting at the origin {at_origin}, of {}", mesh.triangle_count());
    for sm in &mesh.submeshes {
        let idx = &mesh.indices[sm.first_index as usize..(sm.first_index + sm.index_count) as usize];
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for &i in idx { let p = mesh.positions[i as usize]; for k in 0..3 { lo[k] = lo[k].min(p[k]); hi[k] = hi[k].max(p[k]); } }
        println!("  texture {:?}: {:4} tris  x {:5.1}..{:5.1}  y {:5.1}..{:5.1}  z {:5.1}..{:5.1}", sm.texture_id, idx.len() / 3, lo[0], hi[0], lo[1], hi[1], lo[2], hi[2]);
    }
    common::render(&mesh, &a[6], false);
}
