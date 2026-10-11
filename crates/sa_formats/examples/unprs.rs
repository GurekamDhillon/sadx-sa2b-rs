//! Decompress a PRS file: `cargo run --example unprs -- in.prs out.bin`
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let data = std::fs::read(&args[1]).expect("read input");
    let out = sa_formats::prs::decompress(&data).expect("decompress");
    std::fs::write(&args[2], out).expect("write output");
}
