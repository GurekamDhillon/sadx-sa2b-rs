//! Contact sheet of a motion: sheet <mdl.prs> <model id> <mtn.prs> <motion id> <frames...> <out.png>
mod common;
use sa_formats::{bytes::Endian, ninja::{self, pose}, prs};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let models = ninja::read_model_file(&prs::decompress(&std::fs::read(&a[1]).unwrap()).unwrap(), Endian::Big).unwrap();
    let m = models.iter().find(|m| m.id == a[2].parse::<i32>().unwrap()).unwrap();
    let motions = ninja::motion::read_motion_file(&prs::decompress(&std::fs::read(&a[3]).unwrap()).unwrap(), Endian::Big).unwrap();
    let mo = motions.iter().find(|x| x.id == a[4].parse::<u16>().unwrap()).unwrap();
    let frames: Vec<f32> = a[5..a.len() - 1].iter().map(|s| s.parse().unwrap()).collect();
    let (w, h) = (300usize, 300usize);
    let mut sheet = vec![0u8; w * frames.len() * h * 3];
    for (i, f) in frames.iter().enumerate() {
        let rooted = if mo.motion.nodes.len() > m.root.count() {
            let sk = models.iter().find(|s| s.root.count() == mo.motion.nodes.len()).unwrap();
            pose::with_prefix(&m.root, &sk.root, mo.motion.nodes.len() - m.root.count())
        } else { m.root.clone() };
        let mesh = ninja::flatten(&pose::posed(&rooted, &mo.motion, *f));
        let rgb = common::render_rgb(&mesh, w, h, false, Some((1.5, 5.5, 14.0)));
        for y in 0..h { sheet[(y * w * frames.len() + i * w) * 3..][..w * 3].copy_from_slice(&rgb[y * w * 3..(y + 1) * w * 3]); }
    }
    std::fs::write(&a[a.len() - 1], common::png((w * frames.len()) as u32, h as u32, &sheet)).unwrap();
}
