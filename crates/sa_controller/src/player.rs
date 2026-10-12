//! A playable character: action state machine on top of [`Body`] physics.
//!
//! Decoded from SADX Sonic's action code (2011 Steam exe):
//! - jump start ~0x509810, hold check 0x4DBA90, hold lift 0x505ABC,
//!   damage reaction 0x513840;
//! - spin dash: charge/release 0x54C720, start and roll handling in the
//!   action switch at 0x541E6F / 0x5422EE (base speed 5.0, +0.4 per frame up
//!   to 10, released into forward speed);
//! - homing attack: trigger 0x4DBA30 (jump pressed while airborne in a jump),
//!   start 0x548B90, per-frame homing 0x548C90.
//!
//! Collision response is NOT decompiled: [`Player::step`] uses ground
//! snapping and wall push-out through the [`World`] trait.

use sa_data::physics::PhysicsData;

use crate::angle::{adjust_angle, atan2_bams, diff_angle, to_radians};
use crate::frame::{g2p, tilt_for_normal};
use crate::motion::{signed_sqrt, Body, Stick};
use crate::vec::Vec3;
use crate::world::World;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Stand = 0,
    Run = 1,
    Jump = 2,
    Fall = 3,
    Hurt = 4,
    /// Revving a spin dash in place.
    SpindashCharge = 5,
    /// Rolling on the ground (after a spin dash release).
    Roll = 6,
    /// Homing attack or, with no target, air dash.
    Homing = 7,
}

/// One frame of input.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Input {
    /// World-space stick yaw in BAMS (apply the camera before calling).
    pub stick_angle: i32,
    /// 0 = released, 1 = full tilt.
    pub stick_magnitude: f32,
    pub jump_held: bool,
    pub jump_pressed: bool,
    /// The spin dash / action button (B/X in SADX, mask 0x402).
    pub action_held: bool,
    pub action_pressed: bool,
}

/// Animation ids the SADX code sets for these states (Sonic).
pub mod anim {
    pub const JUMP: u16 = 15;
    pub const FALL: u16 = 14;
    pub const HURT: u16 = 13;
    pub const SPIN: u16 = 14;
    pub const SPINDASH_RELEASE: u16 = 15;
    pub const STOP_SPIN: u16 = 0x1D;
}

/// Spin dash constants (SADX 0x95B2EC, 0x54C954, 0x95C670, 0x988938).
pub mod spindash {
    pub const BASE_SPEED: f32 = 5.0;
    pub const MAX_SPEED: f32 = 10.0;
    pub const CHARGE_STEP: f32 = 0.4;
    /// Below this forward speed a roll turns into a charge (or ends).
    pub const ROLL_MIN_SPEED: f32 = 1.85;
    /// Charging ends on its own after this many frames (0x12C).
    pub const MAX_CHARGE_FRAMES: i32 = 300;
}

/// Homing attack constants (SADX 0x95B2EC, 0x95A948, 0x95CC60).
pub mod homing {
    pub const SPEED: f32 = 5.0;
    pub const SUPER_SPEED: f32 = 10.0;
    /// Maximum squared distance to a target (100 units).
    pub const MAX_DIST2: f32 = 10000.0;
    /// Targets more than this far off Sonic's facing are ignored (0x5000 = 112.5°).
    pub const MAX_ANGLE: i32 = 0x5000;
    pub const TURN: i32 = 0x800;
    pub const SUPER_TURN: i32 = 0x1000;
    /// After this many frames of homing the speed is scaled by 0.7..0.8.
    pub const SLOWDOWN_FRAMES: i32 = 180;
    /// Air dash (no target) speed decay per frame.
    pub const DASH_DECAY: f32 = 0.9;
}

#[derive(Debug, Clone)]
pub struct Player {
    pub body: Body,
    pub position: Vec3,
    pub action: Action,
    /// Frames the jump button has been held in this jump (`pwp->jumptime`).
    pub jump_time: i32,
    /// Invulnerability frames left after taking damage.
    pub invulnerable: u16,
    pub animation: u16,
    /// `pwp->spd_spin` (offset 0): stored spin dash speed.
    pub spindash_speed: f32,
    /// Frames spent charging (`pwp+0x7E`).
    pub spin_timer: i32,
    /// Homing state (`pwp+0x80`): <0 just started, 0 homing on a target,
    /// >0 air dash / gave up (counts frames).
    pub homing_timer: i32,
    /// Frames since the homing attack started (`pwp+0x82`).
    pub homing_time: i32,
    /// Super Sonic (`Upgrades & 0x8000`): faster homing, no facing limit.
    pub super_form: bool,
}

/// How far below the feet the ground is still "stuck to" while grounded.
const GROUND_SNAP: f32 = 2.0;

impl Player {
    pub fn new(physics: PhysicsData, position: Vec3) -> Player {
        Player {
            body: Body::new(physics),
            position,
            action: Action::Stand,
            jump_time: 0,
            invulnerable: 0,
            animation: 0,
            spindash_speed: 0.0,
            spin_timer: 0,
            homing_timer: 0,
            homing_time: 0,
            super_form: false,
        }
    }

    fn stick(input: &Input) -> Option<Stick> {
        (input.stick_magnitude > 0.0).then(|| Stick { angle: input.stick_angle, magnitude: input.stick_magnitude.min(1.0) })
    }

    /// Starts a jump (SADX Sonic: `spd.y = jump_speed`, jump time reset).
    pub fn jump(&mut self) {
        self.body.speed.y = self.body.physics.jump_speed;
        self.body.on_ground = false;
        self.body.rolling = true;
        self.jump_time = 0;
        self.action = Action::Jump;
        self.animation = anim::JUMP;
    }

    /// Damage reaction (SADX 0x513840): knocked up and back, 120 frames of
    /// invulnerability. Returns false while still invulnerable.
    pub fn damage(&mut self) -> bool {
        if self.invulnerable > 0 {
            return false;
        }
        let p = self.body.physics;
        self.body.speed.y = p.jump_speed;
        self.body.speed.x = -p.run_speed;
        self.body.on_ground = false;
        self.body.rolling = false;
        self.invulnerable = 120;
        self.action = Action::Hurt;
        self.animation = anim::HURT;
        true
    }

    /// The host calls this when a homing attack (or jump) hits an enemy.
    /// Not decoded: bounces back up like a fresh jump with no forward speed.
    pub fn bounce_off_target(&mut self) {
        self.body.speed = Vec3::new(0.0, self.body.physics.jump_speed, 0.0);
        self.body.on_ground = false;
        self.action = Action::Jump;
        self.jump_time = self.body.physics.hang_time; // no extra lift
        self.homing_timer = 0;
        self.animation = anim::JUMP;
    }

    /// 0x4DBA90: while held, counts frames and allows lift for `hang_time` frames.
    fn jump_hold(&mut self, held: bool) -> bool {
        if !held {
            return false;
        }
        let t = self.jump_time;
        self.jump_time += 1;
        t < self.body.physics.hang_time
    }

    fn start_spindash(&mut self) {
        self.action = Action::SpindashCharge;
        self.spindash_speed = spindash::BASE_SPEED;
        self.spin_timer = 0;
        self.body.rolling = true;
        self.body.rotation[0] = 0;
        self.body.rotation[2] = 0;
        self.animation = anim::SPIN;
    }

    /// 0x54C720: charge while held, release into forward speed. Returns true
    /// if the action changed.
    fn spindash_charge(&mut self, input: &Input) -> bool {
        self.spin_timer += 1;
        if self.spin_timer >= spindash::MAX_CHARGE_FRAMES {
            self.spin_timer = 0;
            self.action = if self.body.speed.x > 0.0 { Action::Run } else { Action::Stand };
            self.body.rolling = false;
            self.animation = anim::STOP_SPIN;
            return true;
        }
        if input.jump_pressed {
            // SADX cancel: jump out of the charge, no dash.
            self.spindash_speed = 0.0;
            self.spin_timer = 0;
            self.jump();
            return true;
        }
        if !input.action_held {
            self.action = Action::Roll;
            self.animation = anim::SPINDASH_RELEASE;
            self.body.speed.x = self.spindash_speed;
            self.spindash_speed = 0.0;
            self.spin_timer = 0;
            return true;
        }
        if self.spindash_speed < spindash::MAX_SPEED {
            self.spindash_speed += spindash::CHARGE_STEP;
        }
        false
    }

    /// 0x548B90: start a homing attack / air dash.
    fn start_homing(&mut self, world: &dyn World) {
        self.action = Action::Homing;
        self.homing_timer = -1;
        self.homing_time = 0;
        let speed = if self.super_form { homing::SUPER_SPEED } else { homing::SPEED };
        match self.pick_target(world) {
            None => {
                self.body.speed.x = speed;
                self.homing_timer = 1;
            }
            Some(_) => {
                self.homing_step(world);
                if self.homing_timer != 0 {
                    self.body.speed.x = speed;
                }
            }
        }
    }

    /// Closest candidate within range and (unless Super) within ±112.5° of facing.
    fn pick_target(&self, world: &dyn World) -> Option<Vec3> {
        let mut best: Option<(Vec3, f32)> = None;
        for t in world.homing_targets(self.position) {
            let d = t - self.position;
            let dist2 = d.dot(d);
            if dist2 >= best.map_or(homing::MAX_DIST2, |b| b.1) {
                continue;
            }
            let ang = atan2_bams(d.z, d.x);
            if !self.super_form && diff_angle(self.body.rotation[1], ang) > homing::MAX_ANGLE {
                continue;
            }
            best = Some((t, dist2));
        }
        best.map(|b| b.0)
    }

    /// 0x548C90: one frame of homing. Returns true if it set the world
    /// velocity directly (no speed integration this frame).
    fn homing_step(&mut self, world: &dyn World) -> bool {
        self.homing_time += 1;
        let target = if self.homing_timer > 0 { None } else { self.pick_target(world) };
        let Some(target) = target else {
            // Air dash / lost target: decays through the ground physics, as the game does.
            if self.homing_timer < 1 {
                self.homing_timer = 1;
            } else {
                self.homing_timer += 1;
                self.body.speed.x *= homing::DASH_DECAY;
            }
            return false;
        };
        let mut d = target - self.position;
        let ang = atan2_bams(d.z, d.x) & 0xFFFF;
        let turn = if self.super_form { homing::SUPER_TURN } else { homing::TURN };
        self.body.rotation[1] = adjust_angle(self.body.rotation[1], ang, turn);
        if d.y > 0.0 && self.homing_timer < 0 {
            d.y = 0.0;
            self.homing_timer = 1;
        }
        let d = d.normalized();
        let h = signed_sqrt(1.0 - d.y * d.y);
        let yaw = to_radians(self.body.rotation[1]);
        let mut speed = if self.super_form { homing::SUPER_SPEED } else { homing::SPEED };
        if self.homing_time > homing::SLOWDOWN_FRAMES {
            // The game picks a random factor in 0.7..0.8; use the midpoint.
            speed *= 0.75;
        }
        let v = Vec3::new(h * yaw.cos(), d.y, h * yaw.sin()) * speed;
        self.body.velocity = v;
        self.body.speed = g2p(self.body.rotation, v);
        if self.homing_timer < 0 {
            self.homing_timer = 0;
        }
        true
    }

    /// Advances one frame (1/60 s).
    pub fn step(&mut self, input: &Input, world: &dyn World) {
        let stick = Self::stick(input);
        let mut velocity_set = false;
        match self.action {
            Action::SpindashCharge => {
                if !self.body.on_ground {
                    self.spindash_speed = spindash::BASE_SPEED;
                    self.action = Action::Jump;
                    self.jump_time = self.body.physics.hang_time;
                } else {
                    self.spindash_charge(input);
                }
            }
            Action::Roll => {
                if self.body.on_ground && self.body.speed.x < spindash::ROLL_MIN_SPEED {
                    if input.action_held {
                        self.start_spindash();
                    } else {
                        self.action = Action::Run;
                        self.body.rolling = false;
                    }
                } else if self.body.on_ground && input.jump_pressed {
                    self.jump();
                }
            }
            Action::Jump if input.jump_pressed => self.start_homing(world),
            Action::Homing => velocity_set = self.homing_timer <= 0 && self.homing_step(world),
            _ => {
                if self.body.on_ground && input.jump_pressed {
                    self.jump();
                } else if self.body.on_ground && input.action_pressed {
                    if self.body.speed.x < spindash::ROLL_MIN_SPEED {
                        self.start_spindash();
                    } else {
                        self.action = Action::Roll;
                        self.body.rolling = true;
                    }
                }
            }
        }
        if !velocity_set {
            match self.action {
                // Charging brakes in place; rolling coasts and only steers.
                Action::SpindashCharge => self.body.ground_acceleration(None),
                Action::Roll if self.body.on_ground => {
                    if let Some(s) = stick {
                        self.body.adjust_angle_y(s.angle, true);
                    }
                    self.body.ground_acceleration(None);
                }
                Action::Homing => self.body.ground_acceleration(None),
                _ if self.body.on_ground => self.body.ground_acceleration(stick),
                _ => {
                    if self.action == Action::Jump && self.jump_hold(input.jump_held) {
                        self.body.speed.y += self.body.physics.jump_add * 0.8;
                    }
                    if self.body.velocity.y <= 0.0 && self.action == Action::Jump {
                        self.animation = anim::FALL;
                    }
                    self.body.air_acceleration(stick);
                }
            }
            let have = stick.is_some() && !matches!(self.action, Action::SpindashCharge | Action::Roll | Action::Homing);
            self.body.integrate_speed(have, 0);
        }
        self.move_and_collide(world);
        if self.invulnerable > 0 {
            self.invulnerable -= 1;
        }
        if self.body.on_ground {
            match self.action {
                Action::SpindashCharge | Action::Roll => {}
                _ => {
                    self.body.rolling = false;
                    self.homing_timer = 0;
                    self.action = if self.body.speed.x.abs() > 0.0 || stick.is_some() { Action::Run } else { Action::Stand };
                }
            }
        } else if matches!(self.action, Action::Stand | Action::Run | Action::Roll) {
            self.action = Action::Fall;
        }
    }

    /// Pushes a stack of spheres (feet, middle, head) out of walls and
    /// removes the velocity component going into them.
    fn collide_walls(&mut self, world: &dyn World) {
        let p = self.body.physics;
        let r = p.radius;
        let heights = [r, p.height * 0.5, (p.height - r).max(r)];
        for h in heights {
            if let Some(push) = world.push_out(self.position + Vec3::new(0.0, h, 0.0), r) {
                self.position += push;
                let n = push.normalized();
                let into = self.body.velocity.dot(n);
                if into < 0.0 {
                    self.body.velocity = self.body.velocity - n * into;
                    let keep_y = self.body.speed.y;
                    self.body.speed = g2p(self.body.rotation, self.body.velocity);
                    if self.body.on_ground {
                        self.body.speed.y = 0.0;
                    } else {
                        self.body.speed.y = keep_y;
                    }
                }
            }
        }
    }

    fn move_and_collide(&mut self, world: &dyn World) {
        // Move in sub-steps no longer than half the collision radius and
        // resolve walls after each, so fast rolls cannot tunnel through.
        let start = self.position;
        let v0 = self.body.velocity;
        let steps = ((Vec3::new(v0.x, 0.0, v0.z).length() / (self.body.physics.radius * 0.5).max(0.5)).ceil() as usize).clamp(1, 16);
        for _ in 0..steps {
            self.position += self.body.velocity * (1.0 / steps as f32);
            self.collide_walls(world);
        }
        let next = self.position;
        self.position = start;
        let v = next - start;
        let reach_up = self.body.physics.height * 0.5;
        let hit = if self.body.on_ground {
            world.ground(next + Vec3::new(0.0, reach_up, 0.0), next - Vec3::new(0.0, GROUND_SNAP + v.length(), 0.0))
        } else if v.y <= 0.0 {
            // Only land while moving down; sweep the vertical extent of the move.
            world.ground(Vec3::new(next.x, self.position.y.max(next.y) + reach_up * 0.5, next.z), next)
        } else {
            None
        };
        match hit {
            Some(g) if g.normal.y > 0.0 || self.body.on_ground => {
                self.position = g.point;
                let was_airborne = !self.body.on_ground;
                self.body.on_ground = true;
                self.body.ground_normal = g.normal;
                self.body.up = g.normal.y;
                self.body.surface_flags = g.flags;
                let (rx, rz) = tilt_for_normal(g.normal);
                self.body.rotation[0] = rx;
                self.body.rotation[2] = rz;
                // Keep momentum along the surface; drop the into-surface part.
                let vel = self.body.velocity;
                let along = vel - g.normal * vel.dot(g.normal);
                self.body.velocity = along;
                self.body.speed = g2p(self.body.rotation, along);
                self.body.speed.y = 0.0;
                if was_airborne {
                    self.jump_time = 0;
                }
            }
            _ => {
                self.position = next;
                if self.body.on_ground {
                    // Walked off a ledge: keep world velocity, return upright.
                    self.body.on_ground = false;
                    self.body.rotation[0] = 0;
                    self.body.rotation[2] = 0;
                    self.body.speed = g2p(self.body.rotation, self.body.velocity);
                    self.body.up = 1.0;
                    self.body.ground_normal = Vec3::UP;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::FlatWorld;

    fn sonic() -> PhysicsData {
        PhysicsData {
            hang_time: 60, floor_grip: 2.0, speed_cap_h: 16.0, speed_cap_v: 16.0, speed_max_h: 3.0, push_speed_max: 0.6,
            jump_speed: 1.66, no_control_speed: 3.0, slide_speed: 0.23, jog_speed: 0.46, run_speed: 1.39, rush_speed: 2.3,
            knockback_speed: 3.7, dash_speed: 5.09, jump_add: 0.076, run_accel: 0.05, air_accel: 0.031, run_decel: -0.06,
            run_brake: -0.18, air_brake: -0.17, air_decel: -0.028, air_resist: -0.008, air_resist_v: -0.01, air_resist_h: -0.4,
            ground_friction: -0.1, ground_friction_h: -0.6, friction_cap: -0.2825, rat_bound: 0.3, radius: 4.0, height: 10.0,
            weight: 0.08, camera_y: 7.0, center_height: 5.4,
        }
    }

    fn jump_height(hold_frames: usize) -> f32 {
        let world = FlatWorld { height: 0.0 };
        let mut p = Player::new(sonic(), Vec3::ZERO);
        p.step(&Input { jump_pressed: true, jump_held: true, ..Default::default() }, &world);
        let mut peak = 0.0f32;
        for f in 0..400 {
            p.step(&Input { jump_held: f < hold_frames, ..Default::default() }, &world);
            peak = peak.max(p.position.y);
            if p.body.on_ground {
                break;
            }
        }
        assert!(p.body.on_ground, "never landed");
        peak
    }

    #[test]
    fn holding_jump_goes_higher() {
        let tap = jump_height(0);
        let held = jump_height(200);
        assert!(tap > 5.0, "tap jump peak {tap}");
        assert!(held > tap * 1.5, "held {held} vs tap {tap}");
    }

    #[test]
    fn runs_on_flat_ground_and_stays_grounded() {
        let world = FlatWorld { height: 0.0 };
        let mut p = Player::new(sonic(), Vec3::ZERO);
        for _ in 0..120 {
            p.step(&Input { stick_angle: 0, stick_magnitude: 1.0, ..Default::default() }, &world);
        }
        assert!(p.body.on_ground);
        assert_eq!(p.action, Action::Run);
        assert!(p.position.y.abs() < 1e-4);
        assert!(Vec3::new(p.position.x, 0.0, p.position.z).length() > 50.0, "moved only to {:?}", p.position);
    }

    #[test]
    fn damage_knocks_back_and_grants_invulnerability() {
        let world = FlatWorld { height: 0.0 };
        let mut p = Player::new(sonic(), Vec3::ZERO);
        assert!(p.damage());
        assert!(!p.damage());
        p.step(&Input::default(), &world);
        assert_eq!(p.action, Action::Hurt);
        assert!(p.position.y > 0.0);
        assert_eq!(p.invulnerable, 119);
    }

    fn press(action: bool, jump: bool) -> Input {
        Input { action_pressed: action, action_held: action, jump_pressed: jump, jump_held: jump, ..Default::default() }
    }

    #[test]
    fn spindash_charges_caps_and_releases() {
        let world = FlatWorld { height: 0.0 };
        let mut p = Player::new(sonic(), Vec3::ZERO);
        p.step(&press(true, false), &world);
        assert_eq!(p.action, Action::SpindashCharge);
        assert_eq!(p.spindash_speed, 5.0);
        let held = Input { action_held: true, ..Default::default() };
        for _ in 0..5 {
            p.step(&held, &world);
        }
        assert!((p.spindash_speed - 7.0).abs() < 1e-4, "5 frames of +0.4: {}", p.spindash_speed);
        for _ in 0..30 {
            p.step(&held, &world);
        }
        assert!(p.spindash_speed >= 10.0 && p.spindash_speed < 10.4, "capped near 10: {}", p.spindash_speed);
        let cap = p.spindash_speed;
        p.step(&Input::default(), &world);
        assert_eq!(p.action, Action::Roll);
        assert_eq!(p.spindash_speed, 0.0);
        assert!(p.body.speed.x > cap - 0.5, "released at {}", p.body.speed.x);
        // Rolling coasts and eventually unrolls.
        for _ in 0..2000 {
            p.step(&Input::default(), &world);
        }
        assert!(matches!(p.action, Action::Run | Action::Stand), "{:?}", p.action);
    }

    #[test]
    fn jumping_out_of_a_charge_is_a_jump() {
        let world = FlatWorld { height: 0.0 };
        let mut p = Player::new(sonic(), Vec3::ZERO);
        p.step(&press(true, false), &world);
        // Knocked off the ground mid-charge (e.g. by a spring).
        p.position.y = 50.0;
        p.body.on_ground = false;
        p.step(&Input { action_held: true, ..Default::default() }, &world);
        assert_eq!(p.action, Action::Jump);
        assert_eq!(p.spindash_speed, 5.0);
    }

    #[test]
    fn jump_cancels_a_charge_and_a_roll() {
        let world = FlatWorld { height: 0.0 };
        // SADX: jump during the charge leaves with no dash.
        let mut p = Player::new(sonic(), Vec3::ZERO);
        p.step(&press(true, false), &world);
        assert_eq!(p.action, Action::SpindashCharge);
        p.step(&Input { action_held: true, jump_pressed: true, ..Default::default() }, &world);
        assert_eq!(p.action, Action::Jump);
        assert_eq!(p.body.speed.x, 0.0);
        // SA2: jump out of the roll keeps the speed.
        let mut p = Player::new(sonic(), Vec3::ZERO);
        p.step(&press(true, false), &world);
        p.step(&Input::default(), &world);
        assert_eq!(p.action, Action::Roll);
        p.step(&Input { jump_pressed: true, ..Default::default() }, &world);
        assert_eq!(p.action, Action::Jump);
        assert!(p.body.speed.x > 4.0);
    }

    struct Targets(Vec<Vec3>);
    impl World for Targets {
        fn ground(&self, top: Vec3, bottom: Vec3) -> Option<crate::world::Ground> {
            FlatWorld { height: 0.0 }.ground(top, bottom)
        }
        fn homing_targets(&self, _from: Vec3) -> Vec<Vec3> {
            self.0.clone()
        }
    }

    #[test]
    fn homing_attack_reaches_target() {
        let target = Vec3::new(40.0, 10.0, 20.0);
        let world = Targets(vec![target]);
        let mut p = Player::new(sonic(), Vec3::ZERO);
        p.body.rotation[1] = atan2_bams(target.z, target.x);
        p.step(&press(false, true), &world);
        for _ in 0..10 {
            p.step(&Input { jump_held: true, ..Default::default() }, &world);
        }
        p.step(&press(false, true), &world);
        assert_eq!(p.action, Action::Homing);
        let mut closest = f32::MAX;
        for _ in 0..30 {
            p.step(&Input::default(), &world);
            closest = closest.min((p.position - target).length());
        }
        assert!(closest < 5.0, "homing got within {closest}");
    }

    #[test]
    fn homing_without_target_air_dashes_forward() {
        let world = FlatWorld { height: 0.0 };
        let mut p = Player::new(sonic(), Vec3::ZERO);
        p.step(&press(false, true), &world);
        p.step(&Input { jump_held: true, ..Default::default() }, &world);
        p.step(&press(false, true), &world);
        assert_eq!(p.action, Action::Homing);
        assert!(p.body.speed.x > 3.0, "dash speed {}", p.body.speed.x);
        assert!(p.homing_timer > 0);
    }

    #[test]
    fn homing_ignores_targets_behind() {
        let world = Targets(vec![Vec3::new(-30.0, 5.0, 0.0)]);
        let mut p = Player::new(sonic(), Vec3::ZERO);
        p.step(&press(false, true), &world);
        p.step(&press(false, true), &world);
        assert!(p.homing_timer > 0, "should air-dash, not home backwards");
        p.super_form = true;
        p.homing_timer = -1;
        assert!(p.pick_target(&world).is_some(), "Super Sonic has no facing limit");
    }

    #[test]
    fn walks_off_a_ledge_and_falls() {
        struct Ledge;
        impl World for Ledge {
            fn ground(&self, top: Vec3, bottom: Vec3) -> Option<crate::world::Ground> {
                if top.x < 20.0 { FlatWorld { height: 0.0 }.ground(top, bottom) } else { FlatWorld { height: -100.0 }.ground(top, bottom) }
            }
        }
        let mut p = Player::new(sonic(), Vec3::ZERO);
        for _ in 0..60 {
            p.step(&Input { stick_magnitude: 1.0, ..Default::default() }, &Ledge);
        }
        let mut fell = false;
        for _ in 0..300 {
            p.step(&Input::default(), &Ledge);
            fell |= !p.body.on_ground;
        }
        assert!(fell);
        assert!(p.body.on_ground && (p.position.y + 100.0).abs() < 1e-3, "{:?}", p.position);
    }
}
