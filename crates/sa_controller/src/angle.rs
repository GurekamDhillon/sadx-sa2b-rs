//! BAMS angle helpers (0x10000 = full turn), matching the game's integer math.
//!
//! Decoded from the SADX 2011 Steam exe: `DiffAngle` at 0x4CBA60,
//! `AdjustAngle` at 0x4CB9C0.

pub const BAMS_PER_TURN: f32 = 65536.0;

/// Absolute shortest difference between two angles (0..=0x8000).
pub fn diff_angle(a: i32, b: i32) -> i32 {
    let d = (b & 0xFFFF).wrapping_sub(a & 0xFFFF) as i16;
    (d.wrapping_abs() as i32) & 0xFFFF
}

/// Moves `cur` toward `target` by at most `step`; returns the new angle
/// (0..=0xFFFF). Reaches `target` exactly when within `step`.
pub fn adjust_angle(cur: i32, target: i32, step: i32) -> i32 {
    let cur = cur & 0xFFFF;
    let target = target & 0xFFFF;
    let d = target.wrapping_sub(cur) as i16 as i32;
    if d <= step && d >= -step {
        target
    } else if d < 0 {
        (cur - step) & 0xFFFF
    } else {
        (cur + step) & 0xFFFF
    }
}

pub fn to_radians(a: i32) -> f32 {
    a as f32 * (std::f32::consts::TAU / BAMS_PER_TURN)
}

/// `(int)(atan2(y, x) * 65536 / 2π)` as the game computes it.
pub fn atan2_bams(y: f32, x: f32) -> i32 {
    ((y as f64).atan2(x as f64) * 65536.0 / 6.283184) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_wraps_and_is_absolute() {
        assert_eq!(diff_angle(0, 0x1000), 0x1000);
        assert_eq!(diff_angle(0x1000, 0), 0x1000);
        assert_eq!(diff_angle(0xF000, 0x1000), 0x2000);
        assert_eq!(diff_angle(0, 0x8000), 0x8000);
    }

    #[test]
    fn adjust_steps_and_snaps() {
        assert_eq!(adjust_angle(0, 0x100, 0x800), 0x100);
        assert_eq!(adjust_angle(0, 0x4000, 0x800), 0x800);
        assert_eq!(adjust_angle(0, 0xC000, 0x800), 0xF800);
        assert_eq!(adjust_angle(0xFF00, 0x0100, 0x800), 0x0100);
    }

    #[test]
    fn atan2_quarter_turn() {
        assert_eq!(atan2_bams(1.0, 0.0), 0x4000);
    }
}
