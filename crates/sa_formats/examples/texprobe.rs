//! Decode every texture in a GVM/PVM (optionally PRS-compressed) or PAK and print sizes.
use sa_formats::{pak, prs, texture::{archive, dds}};

fn main() {
    for path in std::env::args().skip(1) {
        let mut data = std::fs::read(&path).expect("read");
        if path.to_lowercase().ends_with(".prs") {
            data = prs::decompress(&data).expect("prs");
        }
        if data.starts_with(pak::MAGIC) {
            for e in pak::parse(&data).expect("pak") {
                let r = if e.name.to_lowercase().ends_with(".dds") { dds::decode(&e.data).map(|i| format!("{}x{}", i.width, i.height)) } else { Ok("-".into()) };
                println!("{path}: {} {:?}", e.name, r);
            }
        } else {
            let a = archive::parse(&data).expect("archive");
            for t in &a.textures {
                println!("{path}: {} {:?}", t.name, t.decode(a.kind).map(|i| (i.width, i.height, i.get(i.width / 2, i.height / 2))));
            }
        }
    }
}
