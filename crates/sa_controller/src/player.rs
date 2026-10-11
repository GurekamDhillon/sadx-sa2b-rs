//! A playable character: action state machine on top of [`Body`] physics.
//!
//! Jump, jump-hold and damage reactions follow SADX Sonic's action code
//! (2011 Steam exe): jump start near 0x509810, hold check 0x4DBA90,
//! hold boost 0x505ABC, damage reaction 0x513840.
//!
//! Collision response is NOT decompiled: [`Player::step`] uses a simple
//! ground-snapping scheme through the [`World`] trait (no walls yet).

use sa_data::physics::PhysicsData;

use crate::frame::{g2p, tilt_for_normal};
use crate::motion::{Body, Stick};
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
}

/// Animation ids the SADX code sets for these states (Sonic).
pub mod anim {
    pub const JUMP: u16 = 15;
    pub const FALL: u16 = 14;
    pub const HURT: u16 = 13;
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
}

/// How far below the feet the ground is still "stuck to" while grounded.
const GROUND_SNAP: f32 = 2.0;

impl Player {
    pub fn new(physics: PhysicsData, position: Vec3) -> Player {
        Player { body: Body::new(physics), position, action: Action::Stand, jump_time: 0, invulnerable: 0, animation: 0 }
    }

    fn stick(input: &Input) -> Option<Stick> {
        (input.stick_magnitude > 0.0).then(|| Stick { angle: input.stick_angle, magnitude: input.stick_magnitude.min(1.0) })
    }

    /// Starts a jump (SADX Sonic: `spd.y = jump_speed`, jump time reset).
    pub fn jump(&mut self) {
        self.body.speed.y = self.body.physics.jump_speed;
        self.body.on_ground = false;
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
        self.invulnerable = 120;
        self.action = Action::Hurt;
        self.animation = anim::HURT;
        true
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

    /// Advances one frame (1/60 s).
    pub fn step(&mut self, input: &Input, world: &dyn World) {
        let stick = Self::stick(input);
        if self.body.on_ground && input.jump_pressed {
            self.jump();
        }
        if self.body.on_ground {
            self.body.ground_acceleration(stick);
        } else {
            if self.action == Action::Jump && self.jump_hold(input.jump_held) {
                self.body.speed.y += self.body.physics.jump_add * 0.8;
            }
            if self.body.velocity.y <= 0.0 && self.action == Action::Jump {
                self.animation = anim::FALL;
            }
            self.body.air_acceleration(stick);
        }
        self.body.integrate_speed(stick.is_some(), 0);
        self.move_and_collide(world);
        if self.invulnerable > 0 {
            self.invulnerable -= 1;
        }
        if self.body.on_ground {
            self.action = if self.body.speed.x.abs() > 0.0 || stick.is_some() { Action::Run } else { Action::Stand };
        } else if self.action == Action::Stand || self.action == Action::Run {
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
        let v = self.body.velocity;
        let next = self.position + v;
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
                let along = v - g.normal * v.dot(g.normal);
                self.body.velocity = along;
                self.body.speed = g2p(self.body.rotation, along);
                self.body.speed.y = 0.0;
                if was_airborne {
                    self.jump_time = 0;
                }
                self.collide_walls(world);
            }
            _ => {
                self.position = next;
                self.collide_walls(world);
                if self.body.on_ground {
                    // Walked off a ledge: keep world velocity, return upright.
                    self.body.on_ground = false;
                    self.body.rotation[0] = 0;
                    self.body.rotation[2] = 0;
                    self.body.speed = g2p(self.body.rotation, v);
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
