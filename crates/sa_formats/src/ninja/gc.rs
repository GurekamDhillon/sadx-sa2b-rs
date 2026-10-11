//! GameCube "Ginja" models, used by SA2B/SA2 PC for visible level geometry.
//!
//! Attach: vertex set ptr, i32, opaque meshes ptr, translucent meshes ptr,
//! u16 opaque count, u16 translucent count, center[3], radius.
//! Vertex set entry (16 bytes, ends with attribute 0xFF): u8 attribute,
//! u8 element size, u16 count, u32 format, data ptr, u32 data size.
//! Mesh (16 bytes): params ptr, u32 param count, primitives ptr, u32 size.
//! Primitives are GX display lists: u8 type, u16 big-endian vertex count,
//! then per vertex one index per enabled attribute (u8 or u16 big-endian).
//! Index flags and texture state carry over from one mesh to the next.

use crate::bytes::{AddressMap, Bytes, Endian};
use crate::error::{invalid, Result};

pub mod attr {
    pub const POSITION: u8 = 1;
    pub const NORMAL: u8 = 2;
    pub const COLOR0: u8 = 3;
    pub const TEX0: u8 = 5;
    pub const END: u8 = 0xFF;
}

pub mod index_flags {
    pub const POSITION_16: u32 = 0x4;
    pub const HAS_POSITION: u32 = 0x8;
    pub const NORMAL_16: u32 = 0x10;
    pub const HAS_NORMAL: u32 = 0x20;
    pub const COLOR_16: u32 = 0x40;
    pub const HAS_COLOR: u32 = 0x80;
    pub const UV_16: u32 = 0x400;
    pub const HAS_UV: u32 = 0x800;
}

mod param {
    pub const INDEX_FLAGS: u32 = 1;
    pub const TEXTURE: u32 = 8;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveKind {
    Triangles,
    TriangleStrip,
    TriangleFan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Corner {
    pub position: u16,
    pub normal: Option<u16>,
    pub color: Option<u16>,
    pub uv: Option<u16>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Primitive {
    pub kind: PrimitiveKind,
    pub corners: Vec<Corner>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GcMesh {
    pub texture_id: Option<u16>,
    pub translucent: bool,
    pub primitives: Vec<Primitive>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GcModel {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub meshes: Vec<GcMesh>,
    pub center: [f32; 3],
    pub radius: f32,
}

pub fn read_model(b: &Bytes, map: &dyn AddressMap, off: usize) -> Result<GcModel> {
    let mut m = GcModel { positions: Vec::new(), normals: Vec::new(), uvs: Vec::new(), meshes: Vec::new(), center: b.vec3(off + 0x14)?, radius: b.f32(off + 0x20)? };
    if let Some(mut v) = map.offset(b.u32(off)?) {
        for _ in 0..32 {
            let a = b.u8(v)?;
            if a == attr::END {
                break;
            }
            let count = b.u16(v + 2)? as usize;
            if let Some(data) = map.offset(b.u32(v + 8)?) {
                match a {
                    attr::POSITION => m.positions = (0..count).map(|i| b.vec3(data + i * 12)).collect::<Result<_>>()?,
                    attr::NORMAL => m.normals = (0..count).map(|i| b.vec3(data + i * 12)).collect::<Result<_>>()?,
                    attr::TEX0 => m.uvs = (0..count).map(|i| Ok([b.i16(data + i * 4)? as f32 / 256.0, b.i16(data + i * 4 + 2)? as f32 / 256.0])).collect::<Result<_>>()?,
                    _ => {}
                }
            }
            v += 16;
        }
    }
    let mut state = (0u32, None);
    for (list, count, translucent) in [(b.u32(off + 8)?, b.u16(off + 0x10)?, false), (b.u32(off + 12)?, b.u16(off + 0x12)?, true)] {
        let Some(ms) = map.offset(list) else { continue };
        for i in 0..count as usize {
            m.meshes.push(read_mesh(b, map, ms + i * 16, translucent, &mut state)?);
        }
    }
    Ok(m)
}

fn read_mesh(b: &Bytes, map: &dyn AddressMap, off: usize, translucent: bool, state: &mut (u32, Option<u16>)) -> Result<GcMesh> {
    if let Some(p) = map.offset(b.u32(off)?) {
        let n = b.u32(off + 4)? as usize;
        if n > 64 {
            return Err(invalid(off, format!("implausible GC parameter count {n}")));
        }
        for k in 0..n {
            let (ty, data) = (b.u32(p + k * 8)?, b.u32(p + k * 8 + 4)?);
            match ty {
                param::INDEX_FLAGS => state.0 = data,
                param::TEXTURE => state.1 = Some(data as u16),
                _ => {}
            }
        }
    }
    let flags = state.0;
    let mut primitives = Vec::new();
    if let Some(start) = map.offset(b.u32(off + 8)?) {
        let size = b.u32(off + 12)? as usize;
        let dl = Bytes::new(b.slice(start, size)?, Endian::Big);
        let mut pos = 0;
        while pos < size {
            let kind = match dl.u8(pos)? {
                0x90 => PrimitiveKind::Triangles,
                0x98 => PrimitiveKind::TriangleStrip,
                0xA0 => PrimitiveKind::TriangleFan,
                0 => break, // padding
                t => return Err(invalid(start + pos, format!("unknown GX primitive {t:#x}"))),
            };
            let count = dl.u16(pos + 1)? as usize;
            pos += 3;
            let mut corners = Vec::with_capacity(count);
            for _ in 0..count {
                let mut c = Corner::default();
                let mut idx = |has: u32, wide: u32| -> Result<Option<u16>> {
                    if flags & has == 0 {
                        return Ok(None);
                    }
                    let v = if flags & wide != 0 { dl.u16(pos)? } else { dl.u8(pos)? as u16 };
                    pos += if flags & wide != 0 { 2 } else { 1 };
                    Ok(Some(v))
                };
                c.position = idx(index_flags::HAS_POSITION, index_flags::POSITION_16)?.unwrap_or(0);
                c.normal = idx(index_flags::HAS_NORMAL, index_flags::NORMAL_16)?;
                c.color = idx(index_flags::HAS_COLOR, index_flags::COLOR_16)?;
                c.uv = idx(index_flags::HAS_UV, index_flags::UV_16)?;
                corners.push(c);
            }
            primitives.push(Primitive { kind, corners });
        }
    }
    Ok(GcMesh { texture_id: state.1, translucent, primitives })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::bytes::FileOffsets;

    /// Little-endian GC attach at 0x10: 3 positions, one mesh with index
    /// flags (pos+uv, u8), texture 7, and one triangle.
    pub fn triangle_gc_le() -> Vec<u8> {
        let mut d = vec![0u8; 0x10];
        let at = d.len();
        d.extend_from_slice(&[0; 0x24]);
        let pos = d.len();
        for p in [[0f32, 0., 0.], [1., 0., 0.], [0., 0., 1.]] {
            for f in p {
                d.extend_from_slice(&f.to_le_bytes());
            }
        }
        let uv = d.len();
        for t in [[0i16, 0], [256, 0], [0, 256]] {
            for v in t {
                d.extend_from_slice(&v.to_le_bytes());
            }
        }
        let vset = d.len();
        let entry = |d: &mut Vec<u8>, a: u8, size: u8, ptr: usize, bytes: u32| {
            d.extend_from_slice(&[a, size]);
            d.extend_from_slice(&3u16.to_le_bytes());
            d.extend_from_slice(&0u32.to_le_bytes());
            d.extend_from_slice(&(ptr as u32).to_le_bytes());
            d.extend_from_slice(&bytes.to_le_bytes());
        };
        entry(&mut d, attr::POSITION, 12, pos, 36);
        entry(&mut d, attr::TEX0, 4, uv, 12);
        d.extend_from_slice(&[0xFF; 1]);
        d.extend_from_slice(&[0; 15]);
        let params = d.len();
        for (t, v) in [(1u32, index_flags::HAS_POSITION | index_flags::HAS_UV), (8, 7)] {
            d.extend_from_slice(&t.to_le_bytes());
            d.extend_from_slice(&v.to_le_bytes());
        }
        let prims = d.len();
        d.extend_from_slice(&[0x90, 0x00, 0x03, 0, 0, 1, 1, 2, 2, 0, 0]);
        let mesh = d.len();
        for v in [params as u32, 2, prims as u32, 11] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d[at..at + 4].copy_from_slice(&(vset as u32).to_le_bytes());
        d[at + 8..at + 12].copy_from_slice(&(mesh as u32).to_le_bytes());
        d[at + 0x10..at + 0x12].copy_from_slice(&1u16.to_le_bytes());
        d
    }

    #[test]
    fn reads_triangle_mesh() {
        let d = triangle_gc_le();
        let m = read_model(&Bytes::new(&d, Endian::Little), &FileOffsets, 0x10).unwrap();
        assert_eq!(m.positions.len(), 3);
        assert_eq!(m.uvs[1], [1.0, 0.0]);
        assert_eq!(m.meshes.len(), 1);
        assert_eq!(m.meshes[0].texture_id, Some(7));
        let p = &m.meshes[0].primitives[0];
        assert_eq!(p.kind, PrimitiveKind::Triangles);
        assert_eq!(p.corners[2], Corner { position: 2, normal: None, color: None, uv: Some(2) });
    }

    #[test]
    fn unknown_primitive_errors() {
        let mut d = triangle_gc_le();
        let i = d.windows(3).position(|w| w == [0x90, 0x00, 0x03]).unwrap();
        d[i] = 0x55;
        assert!(read_model(&Bytes::new(&d, Endian::Little), &FileOffsets, 0x10).is_err());
    }
}
