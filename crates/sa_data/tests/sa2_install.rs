//! Tests against a real SA2 Steam install. Run with
//! `SA2_DIR=".../Sonic Adventure 2" cargo test -p sa_data --features game-tests`.
#![cfg(feature = "game-tests")]

use sa_data::catalog::{self, Costume};
use sa_data::enums::Character;
use sa_data::Game;

fn game() -> Game {
    let dir = std::env::var("SA2_DIR").expect("set SA2_DIR to the Sonic Adventure 2 folder");
    Game::open(dir).expect("open SA2 install")
}

#[test]
fn every_prs_decompresses() {
    let g = game();
    let mut n = 0;
    for e in std::fs::read_dir(g.data_dir()).unwrap() {
        let name = e.unwrap().file_name().into_string().unwrap();
        if name.to_ascii_lowercase().ends_with(".prs") {
            g.read_data(&name).unwrap_or_else(|err| panic!("{name}: {err}"));
            n += 1;
        }
    }
    assert!(n > 700, "only {n} PRS files");
}

#[test]
fn physics_rows_are_sane() {
    let table = game().physics_table().unwrap();
    assert_eq!(table.len(), 19);
    for (i, p) in table.iter().enumerate().take(17) {
        let c = Character::from_index(i as u32).unwrap();
        assert!(p.floats().iter().all(|f| f.is_finite()), "{c:?} has non-finite values");
        assert!(p.hang_time >= 0 && p.hang_time < 1000, "{c:?} hang_time {}", p.hang_time);
        if c != Character::B {
            assert!(p.radius > 0.0 && p.radius < 100.0, "{c:?} radius {}", p.radius);
            assert!(p.height > 0.0 && p.height < 100.0, "{c:?} height {}", p.height);
        }
    }
    let sonic = table[Character::Sonic as usize];
    assert!(sonic.jump_speed > 0.0);
    assert!(sonic.run_speed < sonic.rush_speed, "run {} rush {}", sonic.run_speed, sonic.rush_speed);
}

#[test]
fn every_catalog_character_loads() {
    let g = game();
    for c in Character::ALL {
        for costume in [Costume::Default, Costume::Alternate] {
            let Some(files) = catalog::sa2_character(c, costume) else { continue };
            let mesh = g.character_mesh(c, costume).unwrap_or_else(|e| panic!("{c:?} {costume:?} model: {e}"));
            assert!(mesh.triangle_count() > 50, "{c:?} {costume:?}: {} triangles", mesh.triangle_count());
            let tex = g.character_textures(c, costume).unwrap_or_else(|e| panic!("{c:?} {costume:?} textures ({}): {e}", files.texture_file()));
            assert!(!tex.is_empty());
            let max_tex = mesh.submeshes.iter().filter_map(|s| s.texture_id).max().unwrap_or(0) as usize;
            assert!(max_tex < tex.len(), "{c:?} {costume:?}: texture id {max_tex} but {} textures", tex.len());
            if costume == Costume::Default {
                let m = g.character_motions(c).unwrap_or_else(|e| panic!("{c:?} motions: {e}"));
                assert!(!m.is_empty());
            }
        }
    }
}

#[test]
fn chao_gardens_and_city_escape_load() {
    let g = game();
    for name in catalog::SA2_CHAO_LANDTABLES {
        let lt = g.landtable(name).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(lt.visual_mesh().triangle_count() > 100, "{name}");
    }
    let city = g.landtable(&catalog::sa2_stage_landtable(13)).unwrap();
    assert!(city.collision_mesh().indices.len() / 3 > 1000);
}

fn longest_edge(m: &sa_formats::ninja::Mesh) -> f32 {
    m.indices
        .chunks(3)
        .flat_map(|t| [(0, 1), (1, 2), (2, 0)].map(|(a, b)| (t[a] as usize, t[b] as usize)))
        .map(|(a, b)| {
            let (p, q) = (m.positions[a], m.positions[b]);
            ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()
        })
        .fold(0.0, f32::max)
}

/// Regression: skinned limbs must stay attached. A wrong vertex-cache draw
/// order once stretched triangles across the whole body (edges over 6 units
/// on a 10-unit character); correct skinning keeps every edge under 2.5.
#[test]
fn sonic_skin_stays_attached_in_every_pose() {
    let g = game();
    let rig = g.character_rig(Character::Sonic, Costume::Default).unwrap();
    for (state, frames) in [(sa_data::anim::state::STAND, [0.0, 4.0]), (sa_data::anim::state::RUN, [0.0, 7.0]), (sa_data::anim::state::JOG, [3.0, 15.0]), (sa_data::anim::state::JUMP, [0.0, 2.0])] {
        for f in frames {
            let mesh = rig.mesh(state, f).unwrap_or_else(|| panic!("no mesh for state {state}"));
            assert!(mesh.triangle_count() > 500, "state {state}: {} triangles", mesh.triangle_count());
            let e = longest_edge(&mesh);
            assert!(e < 2.5, "state {state} frame {f}: longest edge {e}");
        }
    }
}
