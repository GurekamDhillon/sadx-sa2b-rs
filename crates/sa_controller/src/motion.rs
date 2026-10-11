//! The engine's per-frame player physics: acceleration, turning and speed
//! integration.
//!
//! Sources:
//! - `ground_acceleration` is a structured port of `PGetAcceleration` from
//!   michael-fadely/sa-movement-rnd (MIT License, Copyright (c) 2020
//!   Michael Fadely), itself a cleaned decompilation of SADX.
//! - Everything else was decoded from the SADX 2011 Steam exe (SHA-256
//!   e4330c00…7fd6); the address of each function is in its doc comment.
//!
//! SA2 uses the same engine and `PhysicsData` layout; its own code could not
//! be read (the Steam `sonic2app.exe` code section is DRM-encrypted), so
//! SA2-specific differences are not reflected here.
//!
//! Local frame: x = forward, y = surface up, z = side. Units per frame (60 Hz).

use sa_data::physics::PhysicsData;

use crate::angle::{adjust_angle, atan2_bams, diff_angle};
use crate::frame::{g2p, p2g};
use crate::vec::Vec3;

/// World gravity direction (the game's `Gravity` global defaults to this).
pub const GRAVITY: Vec3 = Vec3::new(0.0, -1.0, 0.0);

/// Surface flag: friction applies along the slope (`SurfaceFlags & 4`).
pub const SURFACE_FRICTION: u32 = 0x4;
/// Surface flag tested before keeping a high-speed heading (`0x800000`).
pub const SURFACE_HIGH_SPEED_TURN: u32 = 0x800000;

/// Analog stick: world-space yaw (BAMS, camera already applied) and 0..=1 magnitude.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stick {
    pub angle: i32,
    pub magnitude: f32,
}

/// The state the physics functions read and write. Field comments give the
/// game's names (`taskwk` / `motionwk` / `playerwk`).
#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    /// `twp->ang` (BAMS x, y, z).
    pub rotation: [i32; 3],
    /// `pwp->spd`: local-frame speed.
    pub speed: Vec3,
    /// `pwp->acc`: this frame's acceleration (local frame).
    pub acceleration: Vec3,
    /// `mwp->spd`: world-space velocity after `get_speed`.
    pub velocity: Vec3,
    /// `mwp->acc`: extra world-space force (wind, springs).
    pub external_force: Vec3,
    /// `pwp->eff`: extra world-space velocity added after integration (conveyors).
    pub push_velocity: Vec3,
    /// `mwp->ang_aim.y`: heading the player is turning toward.
    pub target_yaw: i32,
    /// `mwp->force`: last stick drive amount.
    pub force: f32,
    /// `mwp->rad`... ground grip multiplier (1 = normal).
    pub grip: f32,
    /// Excess friction stored by the slope code (`mwp+0x38`).
    pub friction_excess: f32,
    /// `pwp->up`: surface normal Y (1 = flat ground, negative = ceiling).
    pub up: f32,
    /// World-space normal of the surface underfoot.
    pub ground_normal: Vec3,
    /// `pwp->attr`: surface flags of the ground.
    pub surface_flags: u32,
    /// On the ground or standing on an object (`Status & 3`).
    pub on_ground: bool,
    /// Rolling (`Status & 0x100`).
    pub rolling: bool,
    pub physics: PhysicsData,
}

impl Body {
    /// Rotation (BAMS x, y, z) to draw the model with. SA2's `Sonic_Display`
    /// (0x720090) rotates Z, then X, then Y by `0x8000 - heading`: the model
    /// faces the opposite way to the physics forward axis, so the heading gets
    /// a half turn when drawing.
    pub fn display_rotation(&self) -> [i32; 3] {
        [self.rotation[0], 0x8000 - self.rotation[1], self.rotation[2]]
    }

    pub fn new(physics: PhysicsData) -> Body {
        Body {
            rotation: [0; 3],
            speed: Vec3::ZERO,
            acceleration: Vec3::ZERO,
            velocity: Vec3::ZERO,
            external_force: Vec3::ZERO,
            push_velocity: Vec3::ZERO,
            target_yaw: 0,
            force: 0.0,
            grip: 1.0,
            friction_excess: 0.0,
            up: 1.0,
            ground_normal: Vec3::UP,
            surface_flags: 0,
            on_ground: true,
            rolling: false,
            physics,
        }
    }

    fn gravity_force(&self) -> Vec3 {
        GRAVITY * self.physics.weight + self.external_force
    }

    /// `PAdjustAngleYQ` (0x4E2770): quick turn up to 45° that keeps world velocity.
    pub fn adjust_angle_yq(&mut self, target: i32) {
        let v = p2g(self.rotation, self.speed);
        self.rotation[1] = adjust_angle(self.rotation[1], target, 0x2000);
        self.speed = g2p(self.rotation, v);
    }

    /// `PAdjustAngleYS` (0x4E2800): high-speed turn; the step shrinks above
    /// dash speed and speed blends between the old and new frames.
    pub fn adjust_angle_ys(&mut self, target: i32) {
        let v = p2g(self.rotation, self.speed);
        let mut step = 0x100;
        if self.physics.dash_speed < self.speed.x {
            let x = (self.speed.x - self.physics.dash_speed) / 16.0;
            step -= (signed_sqrt(x) as f64 * 256.0) as i32;
        }
        self.rotation[1] = adjust_angle(self.rotation[1], target, step);
        let v = g2p(self.rotation, v);
        let k = if self.up > 0.4 { 0.99 } else { 0.5 };
        self.speed = self.speed * k + v * (1.0 - k);
    }

    /// `PAdjustAngleY` (0x4E2480): normal turn, rate by angle gap.
    pub fn adjust_angle_y(&mut self, target: i32, have_stick: bool) {
        let v = p2g(self.rotation, self.speed);
        let d = diff_angle(self.rotation[1], target);
        let step = if d > 0x2000 { 0x800 } else if d > 0x1000 { d >> 2 } else { d >> 3 };
        self.rotation[1] = adjust_angle(self.rotation[1], target, step);
        let v = g2p(self.rotation, v);
        if !self.on_ground {
            self.speed = self.speed * 0.1 + v * 0.9;
        } else {
            let mut k = if !have_stick {
                0.05
            } else if self.up > 0.4 {
                0.99
            } else {
                0.5
            };
            if self.grip < 1.0 {
                k *= self.grip;
            }
            self.speed = self.speed * k + v * (1.0 - k);
        }
    }

    /// `PRotatedByGravity` (0x4E1E40): with no stick on a slope, turn downhill.
    /// The game skips this while a lock field (`pwp+0x0A`) is set; not modelled.
    pub fn rotated_by_gravity(&mut self) {
        let len = self.velocity.length();
        if !(self.physics.jog_speed < len) {
            return;
        }
        let dir = self.velocity.normalized();
        if GRAVITY.dot(dir) < -0.86 {
            return;
        }
        let g = g2p(self.rotation, GRAVITY);
        if g.y > -0.87 {
            let angle = atan2_bams(g.z, g.x.abs()) + self.rotation[1];
            let gz = g.z.abs();
            let step = if self.rolling {
                (gz as f64 * 4096.0 * self.speed.x as f64 / self.physics.jog_speed as f64) as i32
            } else {
                (gz as f64 * 2048.0) as i32
            };
            self.target_yaw = adjust_angle(self.target_yaw, angle, step);
        } else {
            self.target_yaw = self.rotation[1];
        }
        self.adjust_angle_yq(self.target_yaw);
    }

    /// `PGetAccelerationAir` (0x4DDEC0).
    pub fn air_acceleration(&mut self, stick: Option<Stick>) {
        let p = self.physics;
        let mut acc = g2p(self.rotation, self.gravity_force());
        acc.x += p.air_decel * self.speed.x;
        acc.y += p.air_resist_v * self.speed.y;
        acc.z += p.air_resist_h * self.speed.z;
        let mut drive = 0.0;
        match stick {
            None => self.target_yaw = self.rotation[1],
            Some(s) => {
                let d = diff_angle(self.rotation[1], s.angle);
                drive = s.magnitude;
                if p.run_speed < self.speed.x && d > 0x6000 {
                    drive *= p.air_brake;
                } else if d > 0x1000 {
                    self.target_yaw = s.angle;
                    drive = 0.0;
                } else {
                    self.target_yaw = s.angle;
                    drive = if self.velocity.y < 0.0 { p.air_accel * drive * 2.0 } else { p.air_accel * drive };
                }
                self.adjust_angle_y(self.target_yaw, true);
            }
        }
        acc.x += drive;
        self.acceleration = acc;
    }

    /// `PGetAcceleration` (SADX 0x4DBE50; port of sa-movement-rnd). Tube,
    /// stair, path-following and the `PRotatedByGravityS` variant are not
    /// implemented and fall through to the normal ground code.
    pub fn ground_acceleration(&mut self, stick: Option<Stick>) {
        let p = self.physics;
        let mut acc = g2p(self.rotation, self.gravity_force());
        let have = stick.is_some();
        let cross = self.ground_normal.cross(self.velocity);
        let sx = self.speed.x;

        // Slope handling: unstick from walls/ceilings, extra pull on gentle slopes when slow.
        let acc_z;
        if self.up < 0.1 && cross.y.abs() > 0.6 && sx > 1.16 {
            acc_z = 0.0;
            acc.x = 0.0;
            acc.y = -p.weight;
        } else if self.up < -0.4 && sx > 1.16 {
            acc.y -= p.weight * 5.0;
            acc_z = acc.z;
        } else if self.up < -0.3 && sx > 1.16 {
            acc.y -= p.weight * 0.8;
            acc_z = acc.z;
        } else if self.up < -0.1 && sx > 1.16 {
            acc.y -= p.weight * 0.4;
            acc_z = acc.z;
        } else if self.up >= 0.5 || sx.abs() >= p.run_speed {
            if self.up >= 0.7 || sx.abs() >= p.run_speed {
                acc_z = if self.up >= 0.87 || sx.abs() >= p.jog_speed { acc.z } else { acc.z * 1.4 };
            } else {
                acc_z = acc.z * 2.0;
            }
        } else {
            acc.x *= 4.225;
            acc_z = acc.z * 4.225;
        }

        // Drag above top speed.
        let over_max = || if sx <= p.speed_max_h { if sx >= 0.0 { None } else { Some(p.air_resist * sx) } } else { Some((sx - p.speed_max_h) * p.air_resist) };
        let drag = if have {
            if sx > p.speed_max_h && self.up > 0.96 { Some((sx - p.speed_max_h) * p.air_resist * 1.7) } else { over_max() }
        } else if sx > p.run_speed {
            Some(p.air_resist * sx)
        } else {
            over_max()
        };
        if let Some(d) = drag {
            acc.x += d;
        }
        acc.y += p.air_resist_v * self.speed.y;
        acc.z = p.air_resist_h * self.speed.z + acc_z;

        // Stick drive and turning.
        let mut drive = Vec3::ZERO;
        if let Some(s) = stick {
            let target = s.angle;
            let mut mag = s.magnitude;
            if sx < p.speed_max_h {
                if sx < p.jog_speed {
                    mag = if mag > 0.5 || p.jog_speed * 0.4 > sx { mag * p.run_accel } else { 0.0 };
                } else if sx < p.run_speed {
                    mag = if mag > 0.7 || p.run_speed > sx { mag * p.run_accel } else { 0.0 };
                } else if sx < p.rush_speed {
                    let v = mag * p.run_accel;
                    mag = if mag <= 0.9 { v * 0.3 } else { v };
                } else {
                    mag *= p.run_accel;
                }
            } else if sx < p.speed_max_h || self.up >= 0.0 {
                mag = mag * p.run_accel * 0.4;
            } else {
                mag *= p.run_accel;
            }

            let d = diff_angle(self.rotation[1], target);
            if self.speed.x == 0.0 && d > 0x1000 {
                // Standing still and the stick points away: just turn.
                self.target_yaw = target;
                mag = 0.0;
                self.adjust_angle_yq(target);
            } else if (p.run_speed + p.jog_speed) * 0.5 > self.speed.x || d <= 0x1000 {
                if self.speed.x >= p.jog_speed && d < 0x1000 {
                    self.target_yaw = target;
                    self.adjust_angle_ys(target);
                } else if self.speed.x >= p.dash_speed && self.surface_flags & SURFACE_HIGH_SPEED_TURN != 0 {
                    self.target_yaw = target;
                    self.adjust_angle_ys(target);
                } else {
                    if self.speed.x >= p.jog_speed && self.speed.x <= p.rush_speed && d > 0x2000 {
                        mag *= 0.8;
                    }
                    self.target_yaw = target;
                    self.adjust_angle_y(target, true);
                }
            } else {
                // Sharp turn at speed: brake instead of driving.
                self.target_yaw = target;
                mag = p.run_decel;
                self.adjust_angle_y(target, true);
            }
            self.force = mag;
            drive.x = mag;
        } else {
            if self.up >= 0.71 {
                if self.speed.x > 0.0 {
                    drive.x = p.run_decel;
                } else if self.speed.x < 0.0 {
                    drive.x = -p.run_decel;
                }
            } else {
                self.rotated_by_gravity();
            }
            self.force = 0.0;
        }

        // Forward friction and stopping.
        let sx = self.speed.x;
        let near_zero = |v: f32| v < 0.051 && v > -0.051;
        'x: {
            if self.up < 0.71 && sx.abs() < p.jog_speed && !have {
                acc.x *= 10.0;
                acc.z *= 10.0;
                break 'x;
            }
            if sx == 0.0 {
                let limit = p.friction_cap * self.grip * acc.y;
                acc.x += drive.x;
                if !have && ((acc.x < limit && -limit < acc.x) || acc.x.abs() < 0.051) {
                    acc.x = 0.0;
                }
                break 'x;
            }
            let clamp_to = |v: f32, limit: f32, excess: &mut f32| {
                if v > limit {
                    *excess = v - limit;
                    limit
                } else {
                    v
                }
            };
            if drive.x < 0.0 {
                if self.surface_flags & SURFACE_FRICTION == 0 {
                    acc.x = if sx <= 0.0 {
                        let early = !have && sx <= p.jog_speed && acc.x.abs() < 0.051;
                        let v = p.friction_cap * self.grip * acc.y + drive.x;
                        if early || v >= 0.0 { 0.0 } else { v }
                    } else {
                        let v = acc.x + drive.x;
                        if drive.x * v < 0.0 { 0.0 } else { v }
                    };
                    break 'x;
                }
                if acc.y < 0.0 {
                    let limit = p.ground_friction * self.grip * acc.y;
                    if limit > 0.0 && -drive.x > limit {
                        let v = acc.x + drive.x;
                        if v < 0.0 && v < -limit {
                            self.friction_excess = v - limit;
                            acc.x = -limit;
                        } else if v <= 0.0 {
                            acc.x = v;
                        } else {
                            acc.x = clamp_to(v, limit, &mut self.friction_excess);
                        }
                        break 'x;
                    }
                }
            } else {
                let limit = p.friction_cap * self.grip * acc.y;
                if !(sx >= 0.0 || !near_zero(acc.x)) {
                    let v = acc.x + drive.x;
                    acc.x = if v * drive.x < 0.0 { 0.0 } else { v };
                    break 'x;
                }
                if limit > 0.0 && limit < drive.x {
                    acc.x = clamp_to(acc.x + drive.x, limit, &mut self.friction_excess);
                    break 'x;
                }
            }
            acc.x = if !have && sx <= p.jog_speed && near_zero(acc.x) { 0.0 } else { acc.x + drive.x };
        }

        // Side friction.
        if self.speed.z == 0.0 {
            let limit = p.friction_cap * self.grip * acc.y;
            if acc.z < limit && -limit < acc.z {
                acc.z = 0.0;
            }
        } else {
            let fric = if acc.y >= 0.0 { 0.0 } else { p.ground_friction_h * self.grip * acc.y };
            if self.surface_flags & SURFACE_FRICTION == 0 {
                if acc.z > 0.0 {
                    drive.z = -fric;
                } else if acc.z < 0.0 {
                    drive.z = fric;
                }
                let v = acc.z + drive.z;
                acc.z = if acc.z != 0.0 && drive.z != 0.0 && acc.z * v < 0.0 { 0.0 } else { v };
            } else if acc.z > 0.0 {
                let v = acc.z - fric;
                acc.z = if v >= 0.0 { v } else { 0.0 };
            } else if acc.z < 0.0 {
                acc.z = (acc.z + fric).min(0.0);
            }
        }
        acc.y += drive.y;
        self.acceleration = acc;
    }

    /// `PGetSpeed` (0x4E2950) + velocity clamp (0x4E2CB0) + dead band
    /// (0x4E2EC0): integrates acceleration into speed and produces the world
    /// velocity. `brake_flags` is `pwp+0x1E4` (meaning unknown; pass 0).
    pub fn integrate_speed(&mut self, have_stick: bool, brake_flags: u32) {
        let p = self.physics;
        let thr = p.jog_speed * 0.87;
        let s0 = self.speed.x;
        self.speed.x += self.acceleration.x;
        if s0 > 0.0 {
            if self.speed.x < 0.0 || (!have_stick && self.speed.x < thr && self.up > 0.7) {
                self.speed.x = 0.0;
            }
        } else if s0 < 0.0 && (self.speed.x > 0.0 || (!have_stick && self.speed.x > -thr && self.up > 0.7)) {
            self.speed.x = 0.0;
        }
        self.speed.y += self.acceleration.y;
        let z0 = self.speed.z;
        self.speed.z += self.acceleration.z;
        if z0 > 0.0 {
            if (z0 < thr && self.speed.z == z0) || (z0 >= thr && self.speed.z < thr) {
                self.speed.z = 0.0;
            }
        } else if z0 < 0.0 && ((z0 > -thr && self.speed.z == z0) || (z0 <= -thr && self.speed.z > -thr)) {
            self.speed.z = 0.0;
        }
        if brake_flags & 0x400000 != 0 {
            if self.speed.x >= p.jog_speed * 0.8 {
                self.speed.x += p.run_brake;
            }
        } else if brake_flags & 0x400002 != 0 && self.speed.x >= (p.run_speed + p.jog_speed) * 0.5 {
            self.speed.x += p.run_brake;
        }

        let mut v = p2g(self.rotation, self.speed) + self.push_velocity;
        v.x = v.x.clamp(-p.speed_cap_h, p.speed_cap_h);
        v.z = v.z.clamp(-p.speed_cap_h, p.speed_cap_h);
        v.y = v.y.clamp(-p.speed_cap_v, p.speed_cap_v);
        if !have_stick {
            for c in [&mut v.x, &mut v.y, &mut v.z] {
                if *c < 0.02 && *c > -0.02 {
                    *c = 0.0;
                }
            }
        }
        self.velocity = v;
    }
}

/// 0x431900: `sqrt(x)` for x >= 0, `-sqrt(-x)` otherwise.
pub fn signed_sqrt(x: f32) -> f32 {
    if x < 0.0 { -(-x).sqrt() } else { x.sqrt() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sonic_like() -> PhysicsData {
        // SA2 Sonic row as read from sonic2app.exe (sa_data physics example).
        PhysicsData {
            hang_time: 60,
            floor_grip: 2.0,
            speed_cap_h: 16.0,
            speed_cap_v: 16.0,
            speed_max_h: 3.0,
            push_speed_max: 0.6,
            jump_speed: 1.66,
            no_control_speed: 3.0,
            slide_speed: 0.23,
            jog_speed: 0.46,
            run_speed: 1.39,
            rush_speed: 2.3,
            knockback_speed: 3.7,
            dash_speed: 5.09,
            jump_add: 0.076,
            run_accel: 0.05,
            air_accel: 0.031,
            run_decel: -0.06,
            run_brake: -0.18,
            air_brake: -0.17,
            air_decel: -0.028,
            air_resist: -0.008,
            air_resist_v: -0.01,
            air_resist_h: -0.4,
            ground_friction: -0.1,
            ground_friction_h: -0.6,
            friction_cap: -0.2825,
            rat_bound: 0.3,
            radius: 4.0,
            height: 10.0,
            weight: 0.08,
            camera_y: 7.0,
            center_height: 5.4,
        }
    }

    fn run_frames(b: &mut Body, stick: Option<Stick>, n: usize) {
        for _ in 0..n {
            b.ground_acceleration(stick);
            b.integrate_speed(stick.is_some(), 0);
            b.speed.y = 0.0; // flat ground holds the player
        }
    }

    #[test]
    fn full_stick_accelerates_then_levels_off() {
        let mut b = Body::new(sonic_like());
        let stick = Some(Stick { angle: 0, magnitude: 1.0 });
        run_frames(&mut b, stick, 30);
        let early = b.speed.x;
        assert!(early > 0.5, "speed after 30 frames {early}");
        run_frames(&mut b, stick, 600);
        let late = b.speed.x;
        // Above speed_max_h drive is run_accel*0.4 and drag (sx-3)*air_resist*1.7:
        // they balance at 3 + 0.02/0.0136 = 4.47.
        assert!((late - 4.4706).abs() < 0.01, "top speed {late}");
        assert!(b.velocity.x > 0.0 && b.velocity.z.abs() < 1e-3);
    }

    #[test]
    fn releasing_stick_stops() {
        let mut b = Body::new(sonic_like());
        run_frames(&mut b, Some(Stick { angle: 0, magnitude: 1.0 }), 120);
        run_frames(&mut b, None, 600);
        assert_eq!(b.speed.x, 0.0);
        // velocity.y still holds this frame's gravity; ground collision removes it in Player.
        assert_eq!((b.velocity.x, b.velocity.z), (0.0, 0.0));
    }

    #[test]
    fn standing_turn_rotates_without_moving() {
        let mut b = Body::new(sonic_like());
        b.ground_acceleration(Some(Stick { angle: 0x8000, magnitude: 1.0 }));
        // A half-turn difference is -0x8000, so the game steps the negative way.
        assert_eq!(b.rotation[1], 0xE000);
        assert_eq!(b.force, 0.0);
    }

    #[test]
    fn air_drag_and_gravity() {
        let mut b = Body::new(sonic_like());
        b.on_ground = false;
        b.speed = Vec3::new(1.0, 1.66, 0.0);
        b.air_acceleration(None);
        assert!((b.acceleration.y - (-0.08 - 0.0166)).abs() < 1e-5, "{:?}", b.acceleration);
        assert!((b.acceleration.x - (-0.028)).abs() < 1e-5);
    }

    #[test]
    fn velocity_box_clamp() {
        let mut b = Body::new(sonic_like());
        b.speed = Vec3::new(100.0, 0.0, 0.0);
        b.rotation[1] = 0x2000;
        b.integrate_speed(true, 0);
        assert!(b.velocity.x.abs() <= 16.0 && b.velocity.z.abs() <= 16.0);
        assert!(b.velocity.x.abs() > 15.9 && b.velocity.z.abs() > 15.9, "box clamp keeps both axes at the cap: {:?}", b.velocity);
    }

    #[test]
    fn display_rotation_adds_the_half_turn() {
        let mut b = Body::new(sonic_like());
        b.rotation = [0x100, 0x2000, -0x80];
        assert_eq!(b.display_rotation(), [0x100, 0x6000, -0x80]);
    }

    #[test]
    fn signed_sqrt_matches_game() {
        assert_eq!(signed_sqrt(4.0), 2.0);
        assert_eq!(signed_sqrt(-4.0), -2.0);
    }
}
