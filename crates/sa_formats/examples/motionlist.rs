//! List animations in SA2 *mtn.prs files: id, node count, frames, track types.
use sa_formats::{bytes::Endian, ninja::motion, prs};
fn main() {
    for path in std::env::args().skip(1) {
        let d = prs::decompress(&std::fs::read(&path).unwrap()).unwrap();
        let ms = motion::read_motion_file(&d, Endian::Big).unwrap();
        println!("{path}: {} motions", ms.len());
        for m in &ms {
            let keys: usize = m.motion.nodes.iter().map(|n| n.rotation.len() + n.position.len()).sum();
            println!("  id {:3} nodes {:3} frames {:4} type {:#06x} keys {keys}", m.id, m.motion.nodes.len(), m.motion.frame_count, m.motion.type_flags);
        }
    }
}
