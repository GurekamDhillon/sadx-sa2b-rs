use sa_formats::{bytes::Endian, ninja::{self, object::Node}, prs};
fn collect<'a>(n: &'a Node, out: &mut Vec<&'a Node>) { out.push(n); for c in &n.children { collect(c, out); } }
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let models = ninja::read_model_file(&prs::decompress(&std::fs::read(&a[1]).unwrap()).unwrap(), Endian::Big).unwrap();
    let m = models.iter().find(|m| m.id == a[2].parse::<i32>().unwrap()).unwrap();
    let motions = ninja::motion::read_motion_file(&prs::decompress(&std::fs::read(&a[3]).unwrap()).unwrap(), Endian::Big).unwrap();
    let mo = motions.iter().find(|x| x.id == a[4].parse::<u16>().unwrap()).unwrap();
    let mut nodes = vec![]; collect(&m.root, &mut nodes);
    for (i, n) in nodes.iter().enumerate().take(14) {
        let t = &mo.motion.nodes[i];
        println!("node {i:2} flags {:#04x} children {} attach {} | own pos {:?} rot {:x?} | track pos keys {} {:?} rot keys {} {:x?}",
            n.flags, n.children.len(), n.attach.is_some(), n.position.map(|v| (v * 100.0).round() / 100.0), n.rotation.map(|r| r & 0xFFFF),
            t.position.len(), t.position.first().map(|k| k.value.map(|v| (v * 100.0).round() / 100.0)),
            t.rotation.len(), t.rotation.first().map(|k| k.value.map(|r| r & 0xFFFF)));
    }
}
