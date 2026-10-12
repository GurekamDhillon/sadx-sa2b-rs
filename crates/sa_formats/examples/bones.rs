//! World joint positions: bones <mdl> <model id> <mtn> <motion id> <frame> <first idx> <last idx>
use sa_formats::{bytes::Endian, ninja::{self, math::Mat4, object::Node, pose}, prs};
fn visit(n: &Node, parent: &Mat4, idx: &mut usize, lo: usize, hi: usize, depth: usize) {
    let world = parent.mul(&n.local_matrix());
    let me = *idx; *idx += 1;
    if me >= lo && me <= hi {
        let o = world.point([0.0; 3]); let tip = world.point([1.0, 0.0, 0.0]);
        println!("{}{me:2} flags {:#04x} rot {:04x},{:04x},{:04x} origin ({:5.2},{:5.2},{:5.2})  +x axis points ({:5.2},{:5.2},{:5.2})", "  ".repeat(depth), n.flags, n.rotation[0] & 0xFFFF, n.rotation[1] & 0xFFFF, n.rotation[2] & 0xFFFF, o[0], o[1], o[2], tip[0] - o[0], tip[1] - o[1], tip[2] - o[2]);
    }
    for c in &n.children { visit(c, &world, idx, lo, hi, depth + 1); }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let models = ninja::read_model_file(&prs::decompress(&std::fs::read(&a[1]).unwrap()).unwrap(), Endian::Big).unwrap();
    let m = models.iter().find(|m| m.id == a[2].parse::<i32>().unwrap()).unwrap();
    let motions = ninja::motion::read_motion_file(&prs::decompress(&std::fs::read(&a[3]).unwrap()).unwrap(), Endian::Big).unwrap();
    let mo = motions.iter().find(|x| x.id == a[4].parse::<u16>().unwrap()).unwrap();
    let root = pose::posed(&m.root, &mo.motion, a[5].parse().unwrap());
    visit(&root, &Mat4::IDENTITY, &mut 0, a[6].parse().unwrap(), a[7].parse().unwrap(), 0);
}
