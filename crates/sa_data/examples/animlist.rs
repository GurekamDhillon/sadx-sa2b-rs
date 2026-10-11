//! Print a character's animation table entries: `animlist <SA2 dir> [states...]`
use sa_data::{anim, enums::Character, Game};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let g = Game::open(&a[1]).unwrap();
    let (addr, n) = anim::sa2_table(Character::Sonic).unwrap();
    let t = anim::read_table(&std::fs::read(g.root.join("sonic2app.exe")).unwrap(), addr, n).unwrap();
    let want: Vec<usize> = if a.len() > 2 { a[2..].iter().map(|s| s.parse().unwrap()).collect() } else { (0..n).collect() };
    for i in want { let e = t[i]; println!("state {i:3}: motion {:3} model {:2} mode {} next {:3} transition {:.3} speed {:.3}", e.anim_num, e.model_num, e.mtn_mode, e.next, e.transition_speed, e.anim_speed); }
}
