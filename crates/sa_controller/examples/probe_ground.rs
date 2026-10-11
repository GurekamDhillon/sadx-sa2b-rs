//! List every collision triangle crossing a vertical line in a landtable.
//! `probe_ground <SA2 dir> <export> <x> <z>`
use sa_data::Game;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let g = Game::open(&a[1]).unwrap();
    let lt = g.landtable(&a[2]).unwrap();
    let (x, z): (f32, f32) = (a[3].parse().unwrap(), a[4].parse().unwrap());
    for (ci, c) in lt.cols.iter().enumerate() {
        let m = sa_formats::ninja::flatten(&c.model);
        for t in m.indices.chunks(3) {
            let [p, q, r] = [t[0], t[1], t[2]].map(|i| m.positions[i as usize]);
            let d = (q[0] - p[0]) * (r[2] - p[2]) - (r[0] - p[0]) * (q[2] - p[2]);
            if d.abs() < 1e-6 { continue; }
            let u = ((q[0] - x) * (r[2] - z) - (r[0] - x) * (q[2] - z)) / d;
            let v = ((r[0] - x) * (p[2] - z) - (p[0] - x) * (r[2] - z)) / d;
            if u < 0.0 || v < 0.0 || u + v > 1.0 { continue; }
            let y = p[1] * u + q[1] * v + r[1] * (1.0 - u - v);
            println!("col {ci:3} flags {:#010x} y {y:8.2}", c.flags);
        }
    }
}
