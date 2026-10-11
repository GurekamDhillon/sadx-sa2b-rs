//! Find little-endian u32/u64 hex values in a PE file and print their virtual addresses.
use sa_formats::pe::Pe;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let data = std::fs::read(&a[1]).unwrap();
    let pe = Pe::parse(&data).unwrap();
    for v in &a[2..] {
        let hex = v.trim_start_matches("0x");
        let n = u64::from_str_radix(hex, 16).unwrap();
        let needle: Vec<u8> = if hex.len() > 8 { n.to_le_bytes().to_vec() } else { (n as u32).to_le_bytes().to_vec() };
        let hits: Vec<String> = data.windows(needle.len()).enumerate().filter(|(_, w)| *w == &needle[..]).filter_map(|(off, _)| {
            pe.sections.iter().find(|s| off >= s.raw_offset as usize && off < (s.raw_offset + s.raw_size) as usize)
                .map(|s| format!("{}:{:#x}", s.name, pe.image_base + s.virtual_address + (off - s.raw_offset as usize) as u32))
        }).take(12).collect();
        println!("{v}: {}", hits.join(" "));
    }
}
