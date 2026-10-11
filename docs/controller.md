# Character controller (`sa_controller`)

## Where the code comes from

SA2's Steam `sonic2app.exe` is wrapped in Steam DRM (SteamStub): its
`.text` section is encrypted on disk (a `.bind` section is present and the
bytes at known function addresses are not x86). This project does not remove
DRM, so SA2's own player code was **not** read.

SADX and SA2 share the same engine: the same player functions
(`PGetAcceleration`, `PGetSpeed`, ...) and the same 0x84-byte `PhysicsData`.
Your SADX install (2011 Steam port, SHA-256 `e4330c00…7fd6`) has no DRM
wrapper, so the movement code was decoded from it with a small disassembler
(`sa_formats/examples/disasm.rs`). The controller runs that logic with SA2's
physics values, read from `sonic2app.exe`'s data section.

`PGetAcceleration` itself is a structured port of the cleaned decompilation
in [michael-fadely/sa-movement-rnd](https://github.com/michael-fadely/sa-movement-rnd)
(MIT). Its call order matches the 2011 exe's function at 0x4DBE50.

## Function map (SADX 2011 Steam exe)

| Address | Function | In `sa_controller` |
| --- | --- | --- |
| 0x4DBE50 | `PGetAcceleration` (ground) | `Body::ground_acceleration` |
| 0x4DDEC0 | `PGetAccelerationAir` | `Body::air_acceleration` |
| 0x4E2950 | `PGetSpeed` | `Body::integrate_speed` |
| 0x4E2CB0 | velocity box clamp (±`speed_cap_h` on x/z, ±`speed_cap_v` on y) | in `integrate_speed` |
| 0x4E2EC0 | dead band: zero world velocity components within ±0.02 without input | in `integrate_speed` |
| 0x4E2480 | `PAdjustAngleY` | `Body::adjust_angle_y` |
| 0x4E2800 | `PAdjustAngleYS` | `Body::adjust_angle_ys` |
| 0x4E2770 | `PAdjustAngleYQ` | `Body::adjust_angle_yq` |
| 0x4E1E40 | `PRotatedByGravity` | `Body::rotated_by_gravity` |
| 0x4DB790 | `GetAnalog` (stick angle + magnitude per player) | `Stick` input |
| 0x4CBA60 | `DiffAngle` | `angle::diff_angle` |
| 0x4CB9C0 | `AdjustAngle` | `angle::adjust_angle` |
| 0x431900 | signed square root | `motion::signed_sqrt` |
| 0x4EBEB0 / 0x4EBF90 | `PConvertVector_G2P` / `_P2G` | `frame::g2p` / `frame::p2g` |
| 0x4DBA90 | jump-hold check (`JumpTime < hang_time` while held) | `Player::jump_hold` |
| ~0x509810 | Sonic jump start (`spd.y = jump_speed`, anim 15) | `Player::jump` |
| 0x505ABC | jump-hold lift (`spd.y += jump_add * 0.8`) | `Player::step` |
| 0x513840 | damage reaction (`spd = (-run_speed, jump_speed)`, 120 frames invulnerable, anim 13) | `Player::damage` |

## Behaviour this produces (tested)

- Sonic (SA2 values) reaches a flat-ground top speed of 3 + 0.02/0.0136 ≈
  4.47 units/frame: past `speed_max_h` the drive drops to `run_accel × 0.4`
  and drag is `(speed − 3) × air_resist × 1.7`.
- Releasing the stick brakes with `run_decel` until speed falls under
  `jog_speed × 0.87`, then stops dead.
- From a standstill, a stick more than 22.5° away turns the player in place
  (45° per frame) before moving.
- Holding jump gives extra lift for up to `hang_time` (60) frames; a held
  jump peaks well over 1.5× a tapped one.
- On the real Neutral Chao Garden collision, Sonic lands, runs a circle,
  jumps and stays inside the walls.

## Not decoded (approximations or missing)

- **Collision response.** The game's floor/wall/ceiling solver was not
  decoded. `Player` snaps to the ground via `World::ground`, pushes spheres out
  of steep triangles via `World::push_out`, and keeps the velocity along the
  surface. This is a standard approach, not the game's.
- **Special surfaces.** Tubes (`0x20000`), stairs (`0x4000`), path following,
  `PRotatedByGravityS` (`0x800000`) and the `pwp+0x1E4` brake flags are not
  implemented.
- **Character moves.** Spin dash, homing attack, light dash, rolling,
  bouncing, gliding/climbing, mech hover and the SA2 upgrade checks are not
  implemented yet. `Body::rolling` exists for the slope code only.
- **SA2-specific tuning.** Any change SA2 made to these functions is unknown
  (its code is encrypted). The `jump_add × 0.8` lift factor is SADX Sonic's.
