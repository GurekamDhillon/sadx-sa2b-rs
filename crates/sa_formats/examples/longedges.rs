use sa_formats::{bytes::Endian, ninja, prs};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let models = ninja::read_model_file(&prs::decompress(&std::fs::read(&a[1]).unwrap()).unwrap(), Endian::Big).unwrap();
    let m = models.iter().find(|m| m.id == a[2].parse::<i32>().unwrap()).unwrap();
    let mesh = ninja::flatten(&m.root);
    let mut tris: Vec<(f32, usize)> = mesh.indices.chunks(3).enumerate().map(|(ti, t)| {
        let d = |i: usize, j: usize| { let (p, q) = (mesh.positions[t[i] as usize], mesh.positions[t[j] as usize]); ((p[0]-q[0]).powi(2)+(p[1]-q[1]).powi(2)+(p[2]-q[2]).powi(2)).sqrt() };
        (d(0, 1).max(d(1, 2)).max(d(2, 0)), ti)
    }).collect();
    tris.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    let sub = |ti: usize| mesh.submeshes.iter().position(|s| (ti as u32 * 3) >= s.first_index && (ti as u32 * 3) < s.first_index + s.index_count).unwrap();
    // How many long triangles per submesh, and the slots they use.
    let mut per_sub = std::collections::BTreeMap::new();
    for (len, ti) in tris.iter().take_while(|t| t.0 > 2.5) { *per_sub.entry(sub(*ti)).or_insert(0) += 1; let _ = len; }
    println!("triangles with an edge > 2.5: {} by submesh {:?}", tris.iter().filter(|t| t.0 > 2.5).count(), per_sub);
    for (len, ti) in tris.iter().take(8) {
        let t = &mesh.indices[ti * 3..ti * 3 + 3];
        println!("len {:.2} submesh {} slots {:?} tex {:?}", len, sub(*ti), t.iter().map(|&i| mesh.src_slot[i as usize]).collect::<Vec<_>>(), mesh.submeshes[sub(*ti)].texture_id);
    }
}
