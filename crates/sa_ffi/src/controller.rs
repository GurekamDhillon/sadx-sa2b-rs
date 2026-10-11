//! C ABI for the character controller.

use std::ffi::c_void;

use sa_controller::world::Ground;
use sa_controller::{Action, Input, MeshWorld, Player, Vec3, World};

use crate::{free, null, put, write, SaCollision, SaPhysics, SaStatus};

pub const SA_ACTION_STAND: u32 = 0;
pub const SA_ACTION_RUN: u32 = 1;
pub const SA_ACTION_JUMP: u32 = 2;
pub const SA_ACTION_FALL: u32 = 3;
pub const SA_ACTION_HURT: u32 = 4;

/// Ground query result written by an `SaGroundFn` callback.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SaGroundHit {
    pub point: [f32; 3],
    pub normal: [f32; 3],
    pub flags: u32,
}

/// Highest walkable surface on the vertical segment `top` -> `bottom`
/// (3 floats each). Return 1 and fill `out` on a hit, 0 otherwise.
pub type SaGroundFn = Option<unsafe extern "C" fn(user: *mut c_void, top: *const f32, bottom: *const f32, out: *mut SaGroundHit) -> i32>;
/// Wall contact for a sphere: return 1 and write the push-out displacement
/// (3 floats) to `out`, or 0 for no contact.
pub type SaPushOutFn = Option<unsafe extern "C" fn(user: *mut c_void, center: *const f32, radius: f32, out: *mut f32) -> i32>;

enum WorldKind {
    Mesh(MeshWorld),
    Callbacks { ground: SaGroundFn, push_out: SaPushOutFn, user: *mut c_void },
}

/// Opaque collision world.
pub struct SaWorld(WorldKind);

impl World for SaWorld {
    fn ground(&self, top: Vec3, bottom: Vec3) -> Option<Ground> {
        match &self.0 {
            WorldKind::Mesh(m) => m.ground(top, bottom),
            WorldKind::Callbacks { ground, user, .. } => {
                let f = (*ground)?;
                let (t, b): ([f32; 3], [f32; 3]) = (top.into(), bottom.into());
                let mut hit = SaGroundHit::default();
                // SAFETY: the caller guarantees the callback is valid while the world exists.
                let found = unsafe { f(*user, t.as_ptr(), b.as_ptr(), &mut hit) } != 0;
                found.then(|| Ground { point: hit.point.into(), normal: Vec3::from(hit.normal).normalized(), flags: hit.flags })
            }
        }
    }

    fn push_out(&self, center: Vec3, radius: f32) -> Option<Vec3> {
        match &self.0 {
            WorldKind::Mesh(m) => m.push_out(center, radius),
            WorldKind::Callbacks { push_out, user, .. } => {
                let f = (*push_out)?;
                let c: [f32; 3] = center.into();
                let mut out = [0f32; 3];
                // SAFETY: as above.
                let hit = unsafe { f(*user, c.as_ptr(), radius, out.as_mut_ptr()) } != 0;
                hit.then(|| out.into())
            }
        }
    }
}

/// One frame of controller input.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SaInput {
    /// World-space stick yaw, BAMS (0x10000 = full turn), camera already applied.
    pub stick_angle: i32,
    /// 0 = released .. 1 = full tilt.
    pub stick_magnitude: f32,
    pub jump_held: u8,
    pub jump_pressed: u8,
}

/// Snapshot of a player after a step.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SaPlayerState {
    pub position: [f32; 3],
    /// BAMS x, y (yaw), z.
    pub rotation: [i32; 3],
    /// Local-frame speed (x forward, y up, z side).
    pub speed: [f32; 3],
    /// World-space velocity.
    pub velocity: [f32; 3],
    /// `SA_ACTION_*`.
    pub action: u32,
    pub animation: u32,
    pub on_ground: u8,
    pub invulnerable_frames: u16,
}

/// Opaque controlled character.
pub struct SaPlayer(Player);

/// Collision world over a landtable's collision triangles (copied).
#[no_mangle]
pub unsafe extern "C" fn sa_world_from_collision(c: *const SaCollision, out: *mut *mut SaWorld) -> SaStatus {
    crate::guard(|| put(out, SaWorld(WorldKind::Mesh(MeshWorld::new(&c.as_ref().ok_or_else(null)?.0)))))
}

/// Collision world backed by your engine. `ground` is required; `push_out`
/// may be NULL (no walls). Callbacks run on the thread calling `sa_player_step`.
#[no_mangle]
pub unsafe extern "C" fn sa_world_from_callbacks(ground: SaGroundFn, push_out: SaPushOutFn, user: *mut c_void, out: *mut *mut SaWorld) -> SaStatus {
    crate::guard(|| {
        if ground.is_none() {
            return Err(null());
        }
        put(out, SaWorld(WorldKind::Callbacks { ground, push_out, user }))
    })
}

#[no_mangle]
pub unsafe extern "C" fn sa_world_free(w: *mut SaWorld) {
    free(w)
}

/// Creates a player with the given physics (e.g. from `sa_physics`) at
/// `position` (3 floats).
#[no_mangle]
pub unsafe extern "C" fn sa_player_create(physics: *const SaPhysics, position: *const f32, out: *mut *mut SaPlayer) -> SaStatus {
    crate::guard(|| {
        let p = physics.as_ref().ok_or_else(null)?;
        if position.is_null() {
            return Err(null());
        }
        let pos = Vec3::new(*position, *position.add(1), *position.add(2));
        // SAFETY: identical repr(C) layouts (checked by the physics_layout_matches test).
        let physics = std::mem::transmute::<SaPhysics, sa_data::physics::PhysicsData>(*p);
        put(out, SaPlayer(Player::new(physics, pos)))
    })
}

/// Advances the player one frame (1/60 s).
#[no_mangle]
pub unsafe extern "C" fn sa_player_step(p: *mut SaPlayer, input: *const SaInput, world: *const SaWorld) -> SaStatus {
    crate::guard(|| {
        let p = &mut p.as_mut().ok_or_else(null)?.0;
        let i = input.as_ref().ok_or_else(null)?;
        let w = world.as_ref().ok_or_else(null)?;
        let input = Input { stick_angle: i.stick_angle, stick_magnitude: i.stick_magnitude, jump_held: i.jump_held != 0, jump_pressed: i.jump_pressed != 0 };
        p.step(&input, w);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn sa_player_state(p: *const SaPlayer, out: *mut SaPlayerState) -> SaStatus {
    crate::guard(|| {
        let p = &p.as_ref().ok_or_else(null)?.0;
        let action = match p.action {
            Action::Stand => SA_ACTION_STAND,
            Action::Run => SA_ACTION_RUN,
            Action::Jump => SA_ACTION_JUMP,
            Action::Fall => SA_ACTION_FALL,
            Action::Hurt => SA_ACTION_HURT,
        };
        write(
            out,
            SaPlayerState {
                position: p.position.into(),
                rotation: p.body.rotation,
                speed: p.body.speed.into(),
                velocity: p.body.velocity.into(),
                action,
                animation: p.animation as u32,
                on_ground: p.body.on_ground as u8,
                invulnerable_frames: p.invulnerable,
            },
        )
    })
}

/// Applies damage (knockback + 120 invulnerable frames). Writes 1 to
/// `applied` if it took effect, 0 while invulnerable. `applied` may be NULL.
#[no_mangle]
pub unsafe extern "C" fn sa_player_damage(p: *mut SaPlayer, applied: *mut u8) -> SaStatus {
    crate::guard(|| {
        let took = p.as_mut().ok_or_else(null)?.0.damage();
        if let Some(a) = applied.as_mut() {
            *a = took as u8;
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn sa_player_free(p: *mut SaPlayer) {
    free(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    unsafe extern "C" fn flat(_: *mut c_void, top: *const f32, bottom: *const f32, out: *mut SaGroundHit) -> i32 {
        if *top.add(1) >= 0.0 && *bottom.add(1) <= 0.0 {
            *out = SaGroundHit { point: [*top, 0.0, *top.add(2)], normal: [0.0, 1.0, 0.0], flags: 1 };
            1
        } else {
            0
        }
    }

    #[test]
    fn callback_world_drives_player() {
        unsafe {
            let mut w = ptr::null_mut();
            assert_eq!(sa_world_from_callbacks(None, None, ptr::null_mut(), &mut w), SaStatus::NullArgument);
            assert_eq!(sa_world_from_callbacks(Some(flat), None, ptr::null_mut(), &mut w), SaStatus::Ok);
            let physics = SaPhysics {
                hang_time: 60,
                jump_speed: 1.66,
                jump_add: 0.076,
                weight: 0.08,
                speed_cap_h: 16.0,
                speed_cap_v: 16.0,
                speed_max_h: 3.0,
                run_accel: 0.05,
                run_decel: -0.06,
                jog_speed: 0.46,
                run_speed: 1.39,
                rush_speed: 2.3,
                dash_speed: 5.09,
                height: 10.0,
                radius: 4.0,
                ..Default::default()
            };
            let mut p = ptr::null_mut();
            assert_eq!(sa_player_create(&physics, [0.0f32, 0.0, 0.0].as_ptr(), &mut p), SaStatus::Ok);
            let run = SaInput { stick_angle: 0, stick_magnitude: 1.0, jump_held: 0, jump_pressed: 0 };
            for _ in 0..60 {
                assert_eq!(sa_player_step(p, &run, w), SaStatus::Ok);
            }
            let mut s = SaPlayerState::default();
            assert_eq!(sa_player_state(p, &mut s), SaStatus::Ok);
            assert_eq!((s.on_ground, s.action), (1, SA_ACTION_RUN));
            assert!(s.position[0].abs() + s.position[2].abs() > 10.0, "{:?}", s.position);
            let jump = SaInput { jump_pressed: 1, jump_held: 1, ..run };
            assert_eq!(sa_player_step(p, &jump, w), SaStatus::Ok);
            assert_eq!(sa_player_state(p, &mut s), SaStatus::Ok);
            assert_eq!(s.action, SA_ACTION_JUMP);
            let mut applied = 0u8;
            assert_eq!(sa_player_damage(p, &mut applied), SaStatus::Ok);
            assert_eq!(applied, 1);
            assert_eq!(sa_player_step(p, ptr::null(), w), SaStatus::NullArgument);
            sa_player_free(p);
            sa_world_free(w);
        }
    }
}
