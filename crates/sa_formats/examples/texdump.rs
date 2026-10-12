//! Dump one texture of a GVM as colour and alpha PNGs: texdump <x.prs> <index> <out prefix>
mod common;
use sa_formats::{prs, texture::archive};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let data = prs::decompress(&std::fs::read(&a[1]).unwrap()).unwrap();
    let arc = archive::parse(&data).unwrap();
    let t = &arc.textures[a[2].parse::<usize>().unwrap()];
    let img = t.decode(arc.kind).unwrap();
    let s = 6; // upscale for viewing
    let (w, h) = (img.width as usize * s, img.height as usize * s);
    let (mut col, mut alp) = (vec![0u8; w * h * 3], vec![0u8; w * h * 3]);
    let mut hist = [0usize; 4];
    for y in 0..h { for x in 0..w {
        let p = img.get((x / s) as u32, (y / s) as u32);
        col[(y * w + x) * 3..][..3].copy_from_slice(&p[..3]);
        alp[(y * w + x) * 3..][..3].copy_from_slice(&[p[3]; 3]);
    } }
    for y in 0..img.height { for x in 0..img.width { hist[(img.get(x, y)[3] / 64).min(3) as usize] += 1; } }
    println!("{} {}x{} alpha histogram (0-63, 64-127, 128-191, 192-255): {hist:?}", t.name, img.width, img.height);
    std::fs::write(format!("{}_color.png", a[3]), common::png(w as u32, h as u32, &col)).unwrap();
    std::fs::write(format!("{}_alpha.png", a[3]), common::png(w as u32, h as u32, &alp)).unwrap();
}
