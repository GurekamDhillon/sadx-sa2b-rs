//! Print the physics row for each character: `cargo run --example physics -- <SA2 dir>`
use sa_data::{enums::Character, physics::PhysicsData, Game};
fn main() {
    let g = Game::open(std::env::args().nth(1).expect("SA2 dir")).expect("open");
    let t = g.physics_table().expect("physics");
    for c in [Character::Sonic, Character::Shadow, Character::Knuckles, Character::MechTails] {
        let p = t[c as usize];
        println!("{c:?}: hang_time {} {}", p.hang_time, PhysicsData::FLOAT_FIELDS.iter().zip(p.floats()).map(|(n, v)| format!("{n}={v}")).collect::<Vec<_>>().join(" "));
    }
    let raw = t[0].encode();
    println!("sonic raw[0..16] = {:02x?}", &raw[..16]);
}
