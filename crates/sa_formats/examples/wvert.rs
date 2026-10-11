use sa_formats::{bytes::Endian, ninja::{self, object::{Attach, Node}}, prs};
fn find<'a>(n: &'a Node, i: &mut usize, want: &[usize], out: &mut Vec<(usize, &'a Node)>) { if want.contains(i) { out.push((*i, n)); } *i += 1; for c in &n.children { find(c, i, want, out); } }
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let models = ninja::read_model_file(&prs::decompress(&std::fs::read(&a[1]).unwrap()).unwrap(), Endian::Big).unwrap();
    let m = models.iter().find(|m| m.id == a[2].parse::<i32>().unwrap()).unwrap();
    let want: Vec<usize> = a[3..].iter().map(|s| s.parse().unwrap()).collect();
    let mut out = vec![]; find(&m.root, &mut 0, &want, &mut out);
    for (i, n) in out {
        if let Some(Attach::Chunk(c)) = &n.attach {
            for vc in &c.vertex_chunks {
                println!("node {i} chunk kind {} flags {} offset {} count {}", vc.kind, vc.flags, vc.index_offset, vc.vertices.len());
                for v in vc.vertices.iter().take(8) { println!("   pos {:?} nf {:?}", v.position.map(|x| (x * 100.0).round() / 100.0), v.ninja_flags.map(|f| (f & 0xFFFF, f >> 16))); }
            }
        }
    }
}
