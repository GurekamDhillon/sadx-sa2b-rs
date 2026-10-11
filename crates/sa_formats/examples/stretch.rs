//! Score a model by edge-length stretch: stretch <mdl.prs> <model id>
use sa_formats::{bytes::Endian, ninja, prs};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let models = ninja::read_model_file(&prs::decompress(&std::fs::read(&a[1]).unwrap()).unwrap(), Endian::Big).unwrap();
    let m = models.iter().find(|m| m.id == a[2].parse::<i32>().unwrap()).unwrap();
    let mesh = ninja::flatten(&m.root);
    let mut edges: Vec<f32> = Vec::new();
    for t in mesh.indices.chunks(3) {
        for (i, j) in [(0, 1), (1, 2), (2, 0)] {
            let (p, q) = (mesh.positions[t[i] as usize], mesh.positions[t[j] as usize]);
            edges.push(((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt());
        }
    }
    edges.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pct = |p: f32| edges[((edges.len() - 1) as f32 * p) as usize];
    println!("edges {}: median {:.3} p99 {:.3} max {:.3}  bounds {:?}", edges.len(), pct(0.5), pct(0.99), pct(1.0), mesh.bounds().map(|(l, h)| (h[1] - l[1])));
}
