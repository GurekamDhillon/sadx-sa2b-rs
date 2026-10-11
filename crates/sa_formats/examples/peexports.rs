//! List exports of a PE file matching a substring.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let pe = sa_formats::pe::Pe::parse(&std::fs::read(&a[1]).unwrap()).unwrap();
    println!("image base {:#x}, {} sections, {} exports", pe.image_base, pe.sections.len(), pe.exports.len());
    for s in &pe.sections { println!("  {:8} va {:#010x} vsize {:#x} raw {:#x} rawsize {:#x}", s.name, s.virtual_address, s.virtual_size, s.raw_offset, s.raw_size); }
    for (n, addr) in &pe.exports { if n.contains(a.get(2).map(|s| s.as_str()).unwrap_or("")) { println!("{addr:#010x} {n}"); } }
}
