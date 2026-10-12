//! `NJS_OBJECT` node trees.
//!
//! Layout (0x34 bytes): u32 eval flags, attach ptr, position[3] f32,
//! rotation[3] i32 BAMS, scale[3] f32, child ptr, sibling ptr.

use super::basic::{self, BasicModel, BasicVariant};
use super::chunk::{self, ChunkModel};
use super::gc::{self, GcModel};
use super::math::{bams_to_radians, Mat4};
use crate::bytes::{AddressMap, Bytes};
use crate::error::{invalid, Result};

pub mod eval {
    pub const UNIT_POS: u32 = 0x1;
    pub const UNIT_ANG: u32 = 0x2;
    pub const UNIT_SCL: u32 = 0x4;
    pub const HIDE: u32 = 0x8;
    pub const BREAK: u32 = 0x10;
    pub const ZXY_ANG: u32 = 0x20;
    pub const SKIP: u32 = 0x40;
    pub const SHAPE_SKIP: u32 = 0x80;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelKind {
    Chunk,
    Gc,
    Basic(BasicVariant),
}

#[derive(Debug, Clone)]
pub enum Attach {
    Chunk(ChunkModel),
    Gc(GcModel),
    Basic(BasicModel),
}

#[derive(Debug, Clone)]
pub struct Node {
    pub flags: u32,
    pub position: [f32; 3],
    /// BAMS angles (0x10000 = full turn).
    pub rotation: [i32; 3],
    pub scale: [f32; 3],
    pub attach: Option<Attach>,
    pub children: Vec<Node>,
}

const MAX_NODES: usize = 4096;

impl Node {
    /// Reads the node at file offset `off` and its whole child/sibling tree.
    pub fn read(b: &Bytes, map: &dyn AddressMap, off: usize, kind: ModelKind) -> Result<Node> {
        let mut budget = MAX_NODES;
        let mut roots = read_siblings(b, map, off, kind, &mut budget, 0)?;
        if roots.len() == 1 {
            Ok(roots.pop().unwrap())
        } else {
            // A sibling chain at the root: wrap in an identity node.
            Ok(Node { flags: eval::UNIT_POS | eval::UNIT_ANG | eval::UNIT_SCL, position: [0.; 3], rotation: [0; 3], scale: [1.; 3], attach: None, children: roots })
        }
    }

    /// Local transform: T * R * S with Ninja's rotation order.
    pub fn local_matrix(&self) -> Mat4 {
        let mut m = Mat4::IDENTITY;
        if self.flags & eval::UNIT_POS == 0 {
            m = m.mul(&Mat4::translation(self.position));
        }
        if self.flags & eval::UNIT_ANG == 0 {
            let [x, y, z] = self.rotation.map(bams_to_radians);
            // Ninja's njRotateX/Y/Z each PRE-multiply, and `RotateXYZ` calls Z, then Y,
            // then X, so a point is rotated by X first, then Y, then Z (the ZXY flag
            // calls Y, X, Z: Z first, then X, then Y). Angles are plain right-handed.
            // (Checked against SA Tools' MatrixFunctions; an earlier "mirrored X/Y"
            // guess only compensated for getting this order wrong.)
            m = if self.flags & eval::ZXY_ANG != 0 {
                m.mul(&Mat4::rot_y(y)).mul(&Mat4::rot_x(x)).mul(&Mat4::rot_z(z))
            } else {
                m.mul(&Mat4::rot_z(z)).mul(&Mat4::rot_y(y)).mul(&Mat4::rot_x(x))
            };
        }
        if self.flags & eval::UNIT_SCL == 0 {
            m = m.mul(&Mat4::scale(self.scale));
        }
        m
    }

    /// Total node count, depth-first.
    pub fn count(&self) -> usize {
        1 + self.children.iter().map(Node::count).sum::<usize>()
    }

    /// Visits nodes depth-first in file order with their world matrices.
    pub fn walk<'a>(&'a self, parent: &Mat4, f: &mut dyn FnMut(&'a Node, &Mat4)) {
        let m = parent.mul(&self.local_matrix());
        f(self, &m);
        if self.flags & eval::BREAK == 0 {
            for c in &self.children {
                c.walk(&m, f);
            }
        }
    }
}

fn read_siblings(b: &Bytes, map: &dyn AddressMap, mut off: usize, kind: ModelKind, budget: &mut usize, depth: usize) -> Result<Vec<Node>> {
    if depth > 256 {
        return Err(invalid(off, "node tree too deep"));
    }
    let mut out = Vec::new();
    loop {
        if *budget == 0 {
            return Err(invalid(off, "too many nodes (cycle?)"));
        }
        *budget -= 1;
        let attach_ptr = b.u32(off + 4)?;
        let attach = match map.offset(attach_ptr) {
            None => None,
            Some(a) => Some(match kind {
                ModelKind::Chunk => Attach::Chunk(chunk::read_model(b, map, a)?),
                ModelKind::Gc => Attach::Gc(gc::read_model(b, map, a)?),
                ModelKind::Basic(v) => Attach::Basic(basic::read_model(b, map, a, v)?),
            }),
        };
        let child_ptr = b.u32(off + 0x2C)?;
        let children = match map.offset(child_ptr) {
            Some(c) => read_siblings(b, map, c, kind, budget, depth + 1)?,
            None => Vec::new(),
        };
        out.push(Node {
            flags: b.u32(off)?,
            position: b.vec3(off + 8)?,
            rotation: [b.i32(off + 0x14)?, b.i32(off + 0x18)?, b.i32(off + 0x1C)?],
            scale: b.vec3(off + 0x20)?,
            attach,
            children,
        });
        match map.offset(b.u32(off + 0x30)?) {
            Some(s) => off = s,
            None => return Ok(out),
        }
    }
}
