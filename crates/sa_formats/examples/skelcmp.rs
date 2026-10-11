use sa_formats::{bytes::Endian, ninja::{self, object::Node}, prs};
fn collect<'a>(n: &'a Node, d: usize, out: &mut Vec<(usize, &'a Node)>) { out.push((d, n)); for c in &n.children { collect(c, d + 1, out); } }
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let models = ninja::read_model_file(&prs::decompress(&std::fs::read(&a[1]).unwrap()).unwrap(), Endian::Big).unwrap();
    let get = |id: i32| { let m = models.iter().find(|m| m.id == id).unwrap(); let mut v = vec![]; collect(&m.root, 0, &mut v); v };
    let (x, y) = (get(a[2].parse().unwrap()), get(a[3].parse().unwrap()));
    let s = |n: &Node| format!("{:#04x} pos {:5.2},{:5.2},{:5.2} rot {:04x},{:04x},{:04x} {}", n.flags, n.position[0], n.position[1], n.position[2], n.rotation[0] & 0xFFFF, n.rotation[1] & 0xFFFF, n.rotation[2] & 0xFFFF, if n.attach.is_some() { "A" } else { "-" });
    for i in 0..x.len().max(y.len()) {
        println!("{i:2} | {:30} | {}", x.get(i).map(|(d, n)| format!("d{d} {}", s(n))).unwrap_or_default(), y.get(i).map(|(d, n)| format!("d{d} {}", s(n))).unwrap_or_default());
    }
}
