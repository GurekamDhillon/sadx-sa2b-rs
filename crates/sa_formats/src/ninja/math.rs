//! Minimal column-major 4x4 matrix math for Ninja node transforms.

/// Ninja angles are BAMS: 0x10000 units per full turn.
pub fn bams_to_radians(a: i32) -> f32 {
    a as f32 * (std::f32::consts::TAU / 65536.0)
}

/// Column-major 4x4 matrix: element (row r, column c) is `m[c * 4 + r]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4(pub [f32; 16]);

impl Mat4 {
    pub const IDENTITY: Mat4 = Mat4([1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.]);

    pub fn translation(t: [f32; 3]) -> Mat4 {
        let mut m = Self::IDENTITY;
        m.0[12] = t[0];
        m.0[13] = t[1];
        m.0[14] = t[2];
        m
    }

    pub fn scale(s: [f32; 3]) -> Mat4 {
        let mut m = Self::IDENTITY;
        m.0[0] = s[0];
        m.0[5] = s[1];
        m.0[10] = s[2];
        m
    }

    pub fn rot_x(a: f32) -> Mat4 {
        let (s, c) = a.sin_cos();
        Mat4([1., 0., 0., 0., 0., c, s, 0., 0., -s, c, 0., 0., 0., 0., 1.])
    }

    pub fn rot_y(a: f32) -> Mat4 {
        let (s, c) = a.sin_cos();
        Mat4([c, 0., -s, 0., 0., 1., 0., 0., s, 0., c, 0., 0., 0., 0., 1.])
    }

    pub fn rot_z(a: f32) -> Mat4 {
        let (s, c) = a.sin_cos();
        Mat4([c, s, 0., 0., -s, c, 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.])
    }

    pub fn mul(&self, o: &Mat4) -> Mat4 {
        let mut r = [0f32; 16];
        for c in 0..4 {
            for row in 0..4 {
                r[c * 4 + row] = (0..4).map(|k| self.0[k * 4 + row] * o.0[c * 4 + k]).sum();
            }
        }
        Mat4(r)
    }

    pub fn point(&self, p: [f32; 3]) -> [f32; 3] {
        let m = &self.0;
        [
            m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12],
            m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13],
            m[2] * p[0] + m[6] * p[1] + m[10] * p[2] + m[14],
        ]
    }

    /// Transforms a direction (ignores translation) and renormalises it.
    pub fn normal(&self, n: [f32; 3]) -> [f32; 3] {
        let m = &self.0;
        let v = [
            m[0] * n[0] + m[4] * n[1] + m[8] * n[2],
            m[1] * n[0] + m[5] * n[1] + m[9] * n[2],
            m[2] * n[0] + m[6] * n[1] + m[10] * n[2],
        ];
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if len > 0.0 { [v[0] / len, v[1] / len, v[2] / len] } else { v }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-4)
    }

    #[test]
    fn bams() {
        assert!((bams_to_radians(0x4000) - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    }

    #[test]
    fn rotations_are_right_handed() {
        let q = std::f32::consts::FRAC_PI_2;
        assert!(close(Mat4::rot_z(q).point([1., 0., 0.]), [0., 1., 0.]));
        assert!(close(Mat4::rot_x(q).point([0., 1., 0.]), [0., 0., 1.]));
        assert!(close(Mat4::rot_y(q).point([0., 0., 1.]), [1., 0., 0.]));
    }

    #[test]
    fn compose_translate_after_scale() {
        let m = Mat4::translation([1., 2., 3.]).mul(&Mat4::scale([2., 2., 2.]));
        assert!(close(m.point([1., 1., 1.]), [3., 4., 5.]));
    }
}
