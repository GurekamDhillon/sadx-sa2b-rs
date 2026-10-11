//! What the controller needs from the host's collision system.

use sa_formats::landtable::CollisionMesh;

use crate::vec::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ground {
    /// Point on the surface directly below/above the query position.
    pub point: Vec3,
    /// Surface normal (unit length, facing the player).
    pub normal: Vec3,
    /// Surface flags (`SurfaceFlags`) of the hit.
    pub flags: u32,
}

/// Collision queries. Implement this over your engine's physics, or use
/// [`MeshWorld`] with a landtable collision mesh.
pub trait World {
    /// The highest walkable surface crossed by the vertical segment from
    /// `top` down to `bottom` (`top.y > bottom.y`, same x/z).
    fn ground(&self, top: Vec3, bottom: Vec3) -> Option<Ground>;

    /// Wall contact for a sphere: the displacement that moves it out of
    /// steep surfaces (normal.y below `WALL_MAX_UP`), or `None`.
    fn push_out(&self, _center: Vec3, _radius: f32) -> Option<Vec3> {
        None
    }
}

/// Surfaces whose normal Y is below this count as walls for `push_out`.
pub const WALL_MAX_UP: f32 = 0.5;

/// An infinite flat floor at `height`.
#[derive(Debug, Clone, Copy)]
pub struct FlatWorld {
    pub height: f32,
}

impl World for FlatWorld {
    fn ground(&self, top: Vec3, bottom: Vec3) -> Option<Ground> {
        (top.y >= self.height && bottom.y <= self.height).then(|| Ground { point: Vec3::new(top.x, self.height, top.z), normal: Vec3::UP, flags: 1 })
    }
}

/// Brute-force world over a triangle soup (e.g. `LandTable::collision_mesh`),
/// with a uniform XZ grid to keep queries cheap.
pub struct MeshWorld {
    positions: Vec<Vec3>,
    triangles: Vec<([u32; 3], u32)>,
    cell: f32,
    origin: (f32, f32),
    dims: (usize, usize),
    grid: Vec<Vec<u32>>,
}

impl MeshWorld {
    pub fn new(mesh: &CollisionMesh) -> MeshWorld {
        let positions: Vec<Vec3> = mesh.positions.iter().map(|p| Vec3::from(*p)).collect();
        let triangles: Vec<([u32; 3], u32)> = mesh.indices.chunks_exact(3).zip(&mesh.flags).map(|(t, f)| ([t[0], t[1], t[2]], *f)).collect();
        let (mut lo, mut hi) = ((f32::MAX, f32::MAX), (f32::MIN, f32::MIN));
        for p in &positions {
            lo = (lo.0.min(p.x), lo.1.min(p.z));
            hi = (hi.0.max(p.x), hi.1.max(p.z));
        }
        if positions.is_empty() {
            lo = (0.0, 0.0);
            hi = (1.0, 1.0);
        }
        let cell = 32.0;
        let dims = (((hi.0 - lo.0) / cell) as usize + 1, ((hi.1 - lo.1) / cell) as usize + 1);
        let mut grid = vec![Vec::new(); dims.0 * dims.1];
        for (i, (t, _)) in triangles.iter().enumerate() {
            let vs = t.map(|k| positions[k as usize]);
            let x0 = ((vs.iter().map(|v| v.x).fold(f32::MAX, f32::min) - lo.0) / cell) as usize;
            let x1 = ((vs.iter().map(|v| v.x).fold(f32::MIN, f32::max) - lo.0) / cell) as usize;
            let z0 = ((vs.iter().map(|v| v.z).fold(f32::MAX, f32::min) - lo.1) / cell) as usize;
            let z1 = ((vs.iter().map(|v| v.z).fold(f32::MIN, f32::max) - lo.1) / cell) as usize;
            for x in x0..=x1.min(dims.0 - 1) {
                for z in z0..=z1.min(dims.1 - 1) {
                    grid[z * dims.0 + x].push(i as u32);
                }
            }
        }
        MeshWorld { positions, triangles, cell, origin: lo, dims, grid }
    }

    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    fn cells_around(&self, p: Vec3, r: f32) -> Vec<usize> {
        let x0 = ((p.x - r - self.origin.0) / self.cell).floor().max(0.0) as usize;
        let x1 = ((p.x + r - self.origin.0) / self.cell).floor().max(0.0) as usize;
        let z0 = ((p.z - r - self.origin.1) / self.cell).floor().max(0.0) as usize;
        let z1 = ((p.z + r - self.origin.1) / self.cell).floor().max(0.0) as usize;
        let mut out = Vec::new();
        for z in z0..=z1.min(self.dims.1 - 1) {
            for x in x0..=x1.min(self.dims.0 - 1) {
                out.push(z * self.dims.0 + x);
            }
        }
        out
    }
}

impl World for MeshWorld {
    fn push_out(&self, center: Vec3, radius: f32) -> Option<Vec3> {
        let mut total = Vec3::ZERO;
        let mut c = center;
        let mut seen = std::collections::HashSet::new();
        for cell in self.cells_around(c, radius) {
            for &ti in &self.grid[cell] {
                if !seen.insert(ti) {
                    continue;
                }
                let (t, _) = self.triangles[ti as usize];
                let [a, b, cc] = t.map(|k| self.positions[k as usize]);
                let n = (b - a).cross(cc - a).normalized();
                if n.y.abs() >= WALL_MAX_UP {
                    continue;
                }
                let q = closest_point_on_triangle(c, a, b, cc);
                let d = c - q;
                let dist = d.length();
                if dist < radius && dist > 1e-6 {
                    let mut push = d * ((radius - dist) / dist);
                    push.y = 0.0;
                    c += push;
                    total += push;
                }
            }
        }
        (total.length() > 0.0).then_some(total)
    }

    fn ground(&self, top: Vec3, bottom: Vec3) -> Option<Ground> {
        let cx = ((top.x - self.origin.0) / self.cell).floor();
        let cz = ((top.z - self.origin.1) / self.cell).floor();
        if cx < 0.0 || cz < 0.0 || cx as usize >= self.dims.0 || cz as usize >= self.dims.1 {
            return None;
        }
        let mut best: Option<Ground> = None;
        for &ti in &self.grid[cz as usize * self.dims.0 + cx as usize] {
            let (t, flags) = self.triangles[ti as usize];
            let [a, b, c] = t.map(|k| self.positions[k as usize]);
            let mut n = (b - a).cross(c - a);
            if n.y.abs() < 1e-6 {
                continue; // vertical wall
            }
            if n.y < 0.0 {
                n = -n;
            }
            let n = n.normalized();
            // Barycentric test in XZ.
            let d = (b.x - a.x) * (c.z - a.z) - (c.x - a.x) * (b.z - a.z);
            let u = ((b.x - top.x) * (c.z - top.z) - (c.x - top.x) * (b.z - top.z)) / d;
            let v = ((c.x - top.x) * (a.z - top.z) - (a.x - top.x) * (c.z - top.z)) / d;
            let w = 1.0 - u - v;
            if u < 0.0 || v < 0.0 || w < 0.0 {
                continue;
            }
            let y = a.y * u + b.y * v + c.y * w;
            if y > top.y || y < bottom.y {
                continue;
            }
            if best.map_or(true, |g| y > g.point.y) {
                best = Some(Ground { point: Vec3::new(top.x, y, top.z), normal: n, flags });
            }
        }
        best
    }
}

/// Closest point to `p` on triangle `abc` (Ericson, Real-Time Collision Detection 5.1.5).
pub fn closest_point_on_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denom = 1.0 / (va + vb + vc);
    a + ab * (vb * denom) + ac * (vc * denom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wall_pushes_sphere_out() {
        // Vertical wall in the x = 0 plane.
        let mesh = CollisionMesh {
            positions: vec![[0.0, -10.0, -10.0], [0.0, -10.0, 10.0], [0.0, 10.0, 0.0]],
            indices: vec![0, 1, 2],
            flags: vec![1],
        };
        let w = MeshWorld::new(&mesh);
        let push = w.push_out(Vec3::new(1.0, 0.0, 0.0), 3.0).unwrap();
        assert!((push.x - 2.0).abs() < 1e-4 && push.z.abs() < 1e-4, "{push:?}");
        assert!(w.push_out(Vec3::new(5.0, 0.0, 0.0), 3.0).is_none());
    }

    #[test]
    fn mesh_ground_hit_and_miss() {
        let mesh = CollisionMesh {
            positions: vec![[-10.0, 0.0, -10.0], [10.0, 0.0, -10.0], [0.0, 2.0, 10.0]],
            indices: vec![0, 1, 2],
            flags: vec![1],
        };
        let w = MeshWorld::new(&mesh);
        let g = w.ground(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, -5.0, 0.0)).unwrap();
        assert!((g.point.y - 1.0).abs() < 1e-4);
        assert!(g.normal.y > 0.9);
        assert!(w.ground(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, 3.0, 0.0)).is_none());
        assert!(w.ground(Vec3::new(50.0, 5.0, 0.0), Vec3::new(50.0, -5.0, 0.0)).is_none());
    }
}
