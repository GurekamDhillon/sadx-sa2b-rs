//! Read every objLandTable export from Data_DLL.dll and print stats.
mod common;
use sa_formats::{bytes::AddressMap, landtable::LandTable, pe::Pe};
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let data = std::fs::read(&path).unwrap();
    let pe = Pe::parse(&data).unwrap();
    let (mut ok, mut bad) = (0, 0);
    for (name, addr) in pe.exports.iter().filter(|(n, _)| n.starts_with("objLandTable")) {
        match LandTable::read_sa2(&data, &pe, pe.offset(*addr).unwrap()) {
            Ok(lt) => { ok += 1; let c = lt.collision_mesh(); println!("{name}: cols {:4} visible tris {:6} collision tris {:6} tex {:?}", lt.cols.len(), lt.visual_mesh().triangle_count(), c.indices.len() / 3, lt.texture_name); }
            Err(e) => { bad += 1; println!("{name}: ERROR {e}"); }
        }
    }
    println!("ok {ok} errors {bad}");
    let a: Vec<String> = std::env::args().collect();
    for pair in a[2..].chunks(2) {
        let lt = LandTable::read_sa2(&data, &pe, pe.offset(pe.export(&pair[0]).unwrap()).unwrap()).unwrap();
        common::render(&lt.visual_mesh(), &pair[1], true);
    }
}
