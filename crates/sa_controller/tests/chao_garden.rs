//! Sonic on the real Neutral Chao Garden collision. Run with
//! `SA2_DIR=... cargo test -p sa_controller --features game-tests`.
#![cfg(feature = "game-tests")]

use sa_controller::{Input, MeshWorld, Player, Vec3, World};
use sa_data::{enums::Character, Game};

#[test]
fn sonic_lands_runs_and_jumps_in_the_neutral_garden() {
    let game = Game::open(std::env::var("SA2_DIR").expect("set SA2_DIR")).unwrap();
    let physics = game.physics(Character::Sonic).unwrap();
    let col = game.landtable("objLandTableNeut").unwrap().collision_mesh();
    let world = MeshWorld::new(&col);
    assert!(world.triangle_count() > 100);

    // Find solid ground near the garden centre and drop Sonic from above it.
    let (lo, hi) = col.positions.iter().fold(([f32::MAX; 3], [f32::MIN; 3]), |(mut lo, mut hi), p| {
        for k in 0..3 { lo[k] = lo[k].min(p[k]); hi[k] = hi[k].max(p[k]); }
        (lo, hi)
    });
    let centre = Vec3::new((lo[0] + hi[0]) / 2.0, hi[1] + 50.0, (lo[2] + hi[2]) / 2.0);
    let start = (0..200)
        .map(|i| centre + Vec3::new((i % 20) as f32 * 10.0 - 100.0, 0.0, (i / 20) as f32 * 10.0 - 50.0))
        .find(|p| world.ground(*p, *p - Vec3::new(0.0, 1000.0, 0.0)).is_some())
        .expect("no ground under the garden centre");
    let floor = world.ground(start, start - Vec3::new(0.0, 1000.0, 0.0)).unwrap().point.y;

    let mut sonic = Player::new(physics, start);
    for _ in 0..600 {
        sonic.step(&Input::default(), &world);
        if sonic.body.on_ground { break; }
    }
    assert!(sonic.body.on_ground, "never landed (at {:?})", sonic.position);
    assert!(sonic.position.y >= floor - 1.0, "fell through: {} < floor {}", sonic.position.y, floor);

    // Run in a tight circle (~60 units radius at top speed; walls are not
    // modelled yet) for 4 seconds, jumping once.
    let mut min_y = f32::MAX;
    let mut grounded_frames = 0;
    for f in 0..240 {
        let input = Input { stick_angle: f * 0x300, stick_magnitude: 1.0, jump_pressed: f == 60, jump_held: (60..90).contains(&f), ..Default::default() };
        sonic.step(&input, &world);
        min_y = min_y.min(sonic.position.y);
        if std::env::var("TRACE").is_ok() && f % 5 == 0 {
            println!("f{f:3} pos ({:7.1},{:7.1},{:7.1}) ground {} rot.y {:#06x} spd ({:5.2},{:5.2},{:5.2}) vel.y {:5.2}", sonic.position.x, sonic.position.y, sonic.position.z, sonic.body.on_ground, sonic.body.rotation[1] & 0xFFFF, sonic.body.speed.x, sonic.body.speed.y, sonic.body.speed.z, sonic.body.velocity.y);
        }
        if !sonic.body.on_ground && grounded_frames as i32 == f && !(60..140).contains(&f) {
            println!("left ground at frame {f}: pos {:?} speed {:?} below? {:?}", sonic.position, sonic.body.speed, world.ground(sonic.position + Vec3::new(0.0, 5.0, 0.0), sonic.position - Vec3::new(0.0, 500.0, 0.0)).map(|g| g.point.y));
        }
        grounded_frames += sonic.body.on_ground as u32;
        assert!(sonic.position.x.is_finite() && sonic.position.y.is_finite());
    }
    let moved = (sonic.position - start).length();
    println!("ended at {:?}, moved {moved:.1}, grounded {grounded_frames}/240 frames, lowest y {min_y:.1} (floor near start {floor:.1})", sonic.position);
    println!("level bounds {lo:?} .. {hi:?}");
    assert!(min_y > lo[1] - 1.0, "fell out of the level");
    assert!(grounded_frames > 120);

    // Spin dash on the real floor: charge for 30 frames, release, roll.
    while !sonic.body.on_ground {
        sonic.step(&Input::default(), &world);
    }
    for _ in 0..120 {
        sonic.step(&Input::default(), &world);
    }
    // Face back toward open floor (the circle ends next to a wall).
    let to_start = start - sonic.position;
    sonic.body.rotation[1] = sa_controller::angle::atan2_bams(to_start.z, to_start.x);
    sonic.step(&Input { action_pressed: true, action_held: true, ..Default::default() }, &world);
    assert_eq!(sonic.action, sa_controller::Action::SpindashCharge);
    for _ in 0..30 {
        sonic.step(&Input { action_held: true, ..Default::default() }, &world);
    }
    let before = sonic.position;
    sonic.step(&Input::default(), &world);
    assert_eq!(sonic.action, sa_controller::Action::Roll);
    for _ in 0..30 {
        sonic.step(&Input::default(), &world);
    }
    let rolled = (sonic.position - before).length();
    println!("spin dash rolled {rolled:.1} units in 30 frames, now {:?} at {:?}", sonic.action, sonic.position);
    assert!(rolled > 60.0 && sonic.position.y > lo[1] - 1.0);
}
