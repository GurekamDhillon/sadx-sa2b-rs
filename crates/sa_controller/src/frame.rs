//! Conversions between world space and the player's local frame
//! (x = forward, y = surface up, z = side).
//!
//! From the SADX 2011 exe: `PConvertVector_P2G` (0x4EBF90) builds
//! `RotZ(z) * RotX(x) * RotY(-y)`, and `PConvertVector_G2P` (0x4EBEB0) the
//! inverse `RotY(y) * RotX(-x) * RotZ(-z)`, using the same Ninja rotation
//! matrices as model nodes (`sa_formats::ninja::math`).

use sa_formats::ninja::math::Mat4;

use crate::angle::to_radians;
use crate::vec::Vec3;

fn mat(rot: [i32; 3], inverse: bool) -> Mat4 {
    let [x, y, z] = rot.map(to_radians);
    if inverse {
        Mat4::rot_y(y).mul(&Mat4::rot_x(-x)).mul(&Mat4::rot_z(-z))
    } else {
        Mat4::rot_z(z).mul(&Mat4::rot_x(x)).mul(&Mat4::rot_y(-y))
    }
}

/// Player-local to world.
pub fn p2g(rot: [i32; 3], v: Vec3) -> Vec3 {
    Vec3::from(mat(rot, false).point(v.into()))
}

/// World to player-local.
pub fn g2p(rot: [i32; 3], v: Vec3) -> Vec3 {
    Vec3::from(mat(rot, true).point(v.into()))
}

/// Rotation x/z (BAMS) that make the local up axis equal `normal`
/// (`p2g(rot, (0,1,0)) == normal`), keeping yaw.
pub fn tilt_for_normal(normal: Vec3) -> (i32, i32) {
    let n = normal.normalized();
    let x = (n.z.clamp(-1.0, 1.0) as f64).asin();
    let z = (-n.x as f64).atan2(n.y as f64);
    ((x * 65536.0 / std::f64::consts::TAU) as i32, (z * 65536.0 / std::f64::consts::TAU) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn round_trip() {
        let rot = [0x1234, 0x4567, -0x0800];
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert!(close(g2p(rot, p2g(rot, v)), v));
    }

    #[test]
    fn yaw_only_rotates_forward_in_xz_plane() {
        let f = p2g([0, 0x4000, 0], Vec3::new(1.0, 0.0, 0.0));
        assert!(f.y.abs() < 1e-5);
        assert!((f.x.abs() + f.z.abs() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn tilt_maps_up_to_normal() {
        let n = Vec3::new(0.3, 0.8, -0.2).normalized();
        let (x, z) = tilt_for_normal(n);
        for yaw in [0, 0x3000, 0x9000] {
            assert!(close(p2g([x, yaw, z], Vec3::new(0.0, 1.0, 0.0)), n), "yaw {yaw:#x}");
        }
    }
}
