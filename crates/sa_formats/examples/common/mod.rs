pub fn render(mesh: &sa_formats::ninja::Mesh, path: &str, top: bool) {
    let (w, h) = (400usize, 400usize);
    let rgb = render_rgb(mesh, w, h, top, None);
    std::fs::write(path, png(w as u32, h as u32, &rgb)).unwrap();
}

/// Software-render `mesh` to an RGB buffer. `camera` = (centre x, centre y, span) in
/// the projected plane; `None` fits the mesh.
pub fn render_rgb(mesh: &sa_formats::ninja::Mesh, w: usize, h: usize, top: bool, camera: Option<(f32, f32, f32)>) -> Vec<u8> {
    let (lo, hi) = mesh.bounds().unwrap();
    let mut img = vec![30u8; w * h * 3];
    let mut zb = vec![f32::MIN; w * h];
    let side = std::env::var("VIEW").map(|v| v == "side").unwrap_or(false);
    let top = top || std::env::var("VIEW").map(|v| v == "top").unwrap_or(false);
    let pick = |p: [f32; 3]| if top { [p[0], -p[2], p[1]] } else if side { [-p[2], p[1], p[0]] } else { p };
    let mut tri_color = vec![[200u8, 200, 190]; mesh.indices.len() / 3];
    let palette = [[230u8, 90, 80], [90, 200, 110], [90, 140, 240], [240, 210, 80], [200, 110, 230], [90, 220, 220], [250, 150, 60], [180, 180, 180]];
    for (si, sm) in mesh.submeshes.iter().enumerate() {
        for t in (sm.first_index / 3)..((sm.first_index + sm.index_count) / 3) {
            tri_color[t as usize] = palette[si % palette.len()];
        }
    }
    let (lo, hi) = (pick(lo), pick(hi));
    let (lo, hi) = ([lo[0].min(hi[0]), lo[1].min(hi[1]), 0.0], [lo[0].max(hi[0]), lo[1].max(hi[1]), 0.0]);
    let (cx, cy, span) = camera.unwrap_or(((hi[0] + lo[0]) / 2.0, (hi[1] + lo[1]) / 2.0, (hi[0] - lo[0]).max(hi[1] - lo[1]) * 1.05));
    let proj = |p: [f32; 3]| { let p = pick(p); (((p[0] - cx) / span + 0.5) * w as f32, ((cy - p[1]) / span + 0.5) * h as f32, p[2]) };
    for (ti, t) in mesh.indices.chunks(3).enumerate() {
        let v: Vec<_> = t.iter().map(|&i| proj(mesh.positions[i as usize])).collect();
        let n = mesh.normals[t[0] as usize];
        let shade = (60.0 + 190.0 * (0.3 * n[0] + 0.5 * n[1] + 0.8 * n[2]).abs().min(1.0)) as u8;
        let (minx, maxx) = (v.iter().map(|p| p.0).fold(f32::MAX, f32::min).max(0.0) as usize, v.iter().map(|p| p.0).fold(f32::MIN, f32::max).min(w as f32 - 1.0) as usize);
        let (miny, maxy) = (v.iter().map(|p| p.1).fold(f32::MAX, f32::min).max(0.0) as usize, v.iter().map(|p| p.1).fold(f32::MIN, f32::max).min(h as f32 - 1.0) as usize);
        let area = (v[1].0 - v[0].0) * (v[2].1 - v[0].1) - (v[2].0 - v[0].0) * (v[1].1 - v[0].1);
        if area.abs() < 1e-6 { continue; }
        for y in miny..=maxy { for x in minx..=maxx {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let e = |a: (f32, f32, f32), b: (f32, f32, f32)| ((b.0 - a.0) * (py - a.1) - (b.1 - a.1) * (px - a.0)) / area;
            let (w0, w1, w2) = (e(v[1], v[2]), e(v[2], v[0]), e(v[0], v[1]));
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 { continue; }
            let z = w0 * v[0].2 + w1 * v[1].2 + w2 * v[2].2;
            if z > zb[y * w + x] { zb[y * w + x] = z; let c = tri_color[ti];
            let f = |v: u8| (v as u16 * shade as u16 / 255) as u8;
            img[(y * w + x) * 3..][..3].copy_from_slice(&[f(c[0]), f(c[1]), f(c[2])]); }
        }}
    }
    img
}

pub fn png(w: u32, h: u32, rgb: &[u8]) -> Vec<u8> {
    fn crc(d: &[u8]) -> u32 { let mut c = !0u32; for &b in d { c ^= b as u32; for _ in 0..8 { c = if c & 1 != 0 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 }; } } !c }
    let mut raw = Vec::new();
    for y in 0..h as usize { raw.push(0); raw.extend_from_slice(&rgb[y * w as usize * 3..(y + 1) * w as usize * 3]); }
    let mut z = vec![0x78, 0x01];
    for (i, blk) in raw.chunks(65535).enumerate() { let last = (i + 1) * 65535 >= raw.len(); z.push(last as u8); z.extend_from_slice(&(blk.len() as u16).to_le_bytes()); z.extend_from_slice(&(!(blk.len() as u16)).to_le_bytes()); z.extend_from_slice(blk); }
    let (mut a, mut b) = (1u32, 0u32); for &x in &raw { a = (a + x as u32) % 65521; b = (b + a) % 65521; }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |t: &[u8], d: &[u8]| { out.extend_from_slice(&(d.len() as u32).to_be_bytes()); let mut c = t.to_vec(); c.extend_from_slice(d); out.extend_from_slice(&c); out.extend_from_slice(&crc(&c).to_be_bytes()); };
    let mut ihdr = w.to_be_bytes().to_vec(); ihdr.extend_from_slice(&h.to_be_bytes()); ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(b"IHDR", &ihdr); chunk(b"IDAT", &z); chunk(b"IEND", &[]);
    out
}
