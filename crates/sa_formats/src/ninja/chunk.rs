//! Ninja chunk models (`NJS_CNK_MODEL`), used by SA2 for characters/objects.
//!
//! Model: vertex list ptr, polygon list ptr, center[3], radius.
//! The vertex list is a stream of 32-bit words; the polygon list a stream of
//! 16-bit words. Both use the file's byte order, so the same code reads
//! little-endian (SADX/DC) and big-endian (SA2 PC/GC) data.

use crate::bytes::{AddressMap, Bytes};
use crate::error::{invalid, Result};

pub mod kind {
    pub const NULL: u8 = 0;
    pub const BITS_FIRST: u8 = 1;
    pub const BITS_LAST: u8 = 7;
    pub const TINY_FIRST: u8 = 8;
    pub const TINY_LAST: u8 = 9;
    pub const MATERIAL_FIRST: u8 = 16;
    pub const MATERIAL_LAST: u8 = 31;
    pub const VERTEX_FIRST: u8 = 32;
    pub const VERTEX_LAST: u8 = 50;
    pub const VOLUME_FIRST: u8 = 56;
    pub const VOLUME_LAST: u8 = 58;
    pub const STRIP_FIRST: u8 = 64;
    pub const STRIP_LAST: u8 = 75;
    pub const END: u8 = 255;

    pub const V_SH: u8 = 32;
    pub const VN_SH: u8 = 33;
    pub const V_NF: u8 = 37;
    pub const VN: u8 = 41;
    pub const VN_NF: u8 = 44;
    pub const VN_VN32_FIRST: u8 = 48;
}

/// How a weighted vertex chunk combines into the vertex cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeightStatus {
    Start,
    Middle,
    End,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: Option<[f32; 3]>,
    /// For `*_NF` chunks: low 16 bits = cache index offset, high 16 = weight (0..=255).
    pub ninja_flags: Option<u32>,
    /// D8 colour (ARGB) when present.
    pub color: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VertexChunk {
    pub kind: u8,
    pub flags: u8,
    pub index_offset: u16,
    pub vertices: Vec<Vertex>,
}

impl VertexChunk {
    pub fn weight_status(&self) -> Option<WeightStatus> {
        if !matches!(self.kind, kind::V_NF | kind::VN_NF) {
            return None;
        }
        Some(match self.flags & 3 {
            0 => WeightStatus::Start,
            1 => WeightStatus::Middle,
            _ => WeightStatus::End,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StripVertex {
    pub index: u16,
    pub uv: Option<[f32; 2]>,
    pub normal: Option<[f32; 3]>,
    pub color: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Strip {
    /// Winding starts reversed (negative length in the file).
    pub reversed: bool,
    pub vertices: Vec<StripVertex>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PolyChunk {
    Bits { kind: u8, flags: u8 },
    /// Selects texture `texture_id` of the model's texture list.
    Texture { kind: u8, flags: u8, texture_id: u16 },
    Material { kind: u8, flags: u8, diffuse: Option<u32> },
    Strip { kind: u8, flags: u8, strips: Vec<Strip> },
    /// Volume or unknown chunk, skipped by size.
    Other { kind: u8, flags: u8 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChunkModel {
    pub vertex_chunks: Vec<VertexChunk>,
    pub poly_chunks: Vec<PolyChunk>,
    pub center: [f32; 3],
    pub radius: f32,
}

const MAX_CHUNKS: usize = 1 << 16;

pub fn read_model(b: &Bytes, map: &dyn AddressMap, off: usize) -> Result<ChunkModel> {
    let vertex_chunks = match map.offset(b.u32(off)?) {
        Some(v) => read_vertices(b, v)?,
        None => Vec::new(),
    };
    let poly_chunks = match map.offset(b.u32(off + 4)?) {
        Some(p) => read_polys(b, p)?,
        None => Vec::new(),
    };
    Ok(ChunkModel { vertex_chunks, poly_chunks, center: b.vec3(off + 8)?, radius: b.f32(off + 20)? })
}

fn read_vertices(b: &Bytes, mut off: usize) -> Result<Vec<VertexChunk>> {
    let mut out = Vec::new();
    for _ in 0..MAX_CHUNKS {
        let head = b.u32(off)?;
        let ty = head as u8;
        if ty == kind::END {
            return Ok(out);
        }
        if !(kind::VERTEX_FIRST..=kind::VERTEX_LAST).contains(&ty) {
            return Err(invalid(off, format!("expected vertex chunk, found type {ty}")));
        }
        let flags = (head >> 8) as u8;
        let size_words = (head >> 16) as usize;
        let second = b.u32(off + 4)?;
        let index_offset = second as u16;
        let count = (second >> 16) as usize;
        let data = off + 8;
        let data_len = (size_words * 4).checked_sub(4).ok_or_else(|| invalid(off, "vertex chunk too small"))?;
        let stride = if count == 0 { 0 } else { data_len / count };
        let mut vertices = Vec::with_capacity(count);
        for i in 0..count {
            let v = data + i * stride;
            let position = b.vec3(v)?;
            let (normal, extra) = match ty {
                kind::V_SH => (None, None),
                kind::VN_SH => (Some(b.vec3(v + 16)?), None),
                34 => (None, None),
                35..=40 => (None, Some(b.u32(v + 12)?)),
                41 => (Some(b.vec3(v + 12)?), None),
                42..=47 => (Some(b.vec3(v + 12)?), Some(b.u32(v + 24)?)),
                48..=50 => (Some(unpack_vn32(b.u32(v + 12)?)), if ty > 48 { Some(b.u32(v + 16)?) } else { None }),
                _ => (None, None),
            };
            let is_nf = matches!(ty, kind::V_NF | kind::VN_NF);
            let is_color = matches!(ty, 35 | 42 | 49);
            vertices.push(Vertex {
                position,
                normal,
                ninja_flags: if is_nf { extra } else { None },
                color: if is_color { extra } else { None },
            });
        }
        out.push(VertexChunk { kind: ty, flags, index_offset, vertices });
        off = data + data_len;
    }
    Err(invalid(off, "vertex list has no end chunk"))
}

/// 32-bit packed normal: 10 bits per axis (x high), signed.
fn unpack_vn32(v: u32) -> [f32; 3] {
    let f = |shift: u32| {
        let raw = ((v >> shift) & 0x3FF) as i32;
        let signed = if raw & 0x200 != 0 { raw - 0x400 } else { raw };
        signed as f32 / 511.0
    };
    [f(20), f(10), f(0)]
}

fn read_polys(b: &Bytes, mut off: usize) -> Result<Vec<PolyChunk>> {
    let mut out = Vec::new();
    for _ in 0..MAX_CHUNKS {
        let head = b.u16(off)?;
        let ty = head as u8;
        let flags = (head >> 8) as u8;
        match ty {
            kind::END => return Ok(out),
            kind::NULL => off += 2,
            kind::BITS_FIRST..=kind::BITS_LAST => {
                out.push(PolyChunk::Bits { kind: ty, flags });
                off += 2;
            }
            kind::TINY_FIRST..=kind::TINY_LAST => {
                let data = b.u16(off + 2)?;
                out.push(PolyChunk::Texture { kind: ty, flags, texture_id: data & 0x1FFF });
                off += 4;
            }
            kind::MATERIAL_FIRST..=kind::MATERIAL_LAST => {
                let size = b.u16(off + 2)? as usize;
                // Diffuse colour is stored as two 16-bit words (BG then AR in LE).
                let diffuse = if ty & 1 != 0 {
                    let lo = b.u16(off + 4)? as u32;
                    let hi = b.u16(off + 6)? as u32;
                    Some(hi << 16 | lo)
                } else {
                    None
                };
                out.push(PolyChunk::Material { kind: ty, flags, diffuse });
                off += 4 + size * 2;
            }
            kind::STRIP_FIRST..=kind::STRIP_LAST => {
                let size = b.u16(off + 2)? as usize;
                let strips = read_strips(b, ty, off + 4)?;
                out.push(PolyChunk::Strip { kind: ty, flags, strips });
                off += 4 + size * 2;
            }
            kind::VOLUME_FIRST..=kind::VOLUME_LAST => {
                let size = b.u16(off + 2)? as usize;
                out.push(PolyChunk::Other { kind: ty, flags });
                off += 4 + size * 2;
            }
            _ => return Err(invalid(off, format!("unknown polygon chunk type {ty}"))),
        }
    }
    Err(invalid(off, "polygon list has no end chunk"))
}

fn read_strips(b: &Bytes, ty: u8, off: usize) -> Result<Vec<Strip>> {
    let header = b.u16(off)?;
    let nstrips = (header & 0x3FFF) as usize;
    let user_words = (header >> 14) as usize;
    let rel = ty - kind::STRIP_FIRST;
    // Strip, UVN, UVH, VN, UVN_VN, UVH_VN, D8, UVN_D8, UVH_D8, 2, UVN2, UVH2
    let uv_scale = match rel {
        1 | 4 | 7 | 10 => Some(1.0 / 256.0),
        2 | 5 | 8 | 11 => Some(1.0 / 1024.0),
        _ => None,
    };
    let uv_sets = if rel >= 10 { 2 } else if uv_scale.is_some() { 1 } else { 0 };
    let has_normal = matches!(rel, 3..=5);
    let has_color = matches!(rel, 6..=8);
    let mut pos = off + 2;
    let mut strips = Vec::with_capacity(nstrips);
    for _ in 0..nstrips {
        let len = b.i16(pos)?;
        pos += 2;
        let n = len.unsigned_abs() as usize;
        let mut vertices = Vec::with_capacity(n);
        for i in 0..n {
            let index = b.u16(pos)?;
            pos += 2;
            let mut uv = None;
            for set in 0..uv_sets {
                let u = b.i16(pos)? as f32;
                let v = b.i16(pos + 2)? as f32;
                if set == 0 {
                    uv = uv_scale.map(|s| [u * s, v * s]);
                }
                pos += 4;
            }
            let normal = if has_normal {
                let n = [b.i16(pos)?, b.i16(pos + 2)?, b.i16(pos + 4)?].map(|c| c as f32 / 32767.0);
                pos += 6;
                Some(n)
            } else {
                None
            };
            let color = if has_color {
                let lo = b.u16(pos)? as u32;
                let hi = b.u16(pos + 2)? as u32;
                pos += 4;
                Some(hi << 16 | lo)
            } else {
                None
            };
            if i >= 2 {
                pos += 2 * user_words;
            }
            vertices.push(StripVertex { index, uv, normal, color });
        }
        strips.push(Strip { reversed: len < 0, vertices });
    }
    Ok(strips)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::bytes::{Endian, FileOffsets};

    /// Builds a big-endian chunk model at offset 0x10 with 3 VN vertices and
    /// one textured UVN strip triangle. Returns the bytes.
    pub fn triangle_model_be() -> Vec<u8> {
        let mut d = vec![0u8; 0x10];
        let model = d.len();
        d.extend_from_slice(&[0; 24]);
        // Vertex list.
        let vlist = d.len();
        let verts: [[f32; 6]; 3] = [[0., 0., 0., 0., 1., 0.], [1., 0., 0., 0., 1., 0.], [0., 0., 1., 0., 1., 0.]];
        let size_words = 1 + 3 * 6;
        d.extend_from_slice(&(((size_words as u32) << 16) | kind::VN as u32).to_be_bytes());
        d.extend_from_slice(&((3u32 << 16) | 0).to_be_bytes());
        for v in verts {
            for f in v {
                d.extend_from_slice(&f.to_be_bytes());
            }
        }
        d.extend_from_slice(&0xFFu32.to_be_bytes());
        // Polygon list: texture 5, then UVN strip of 3.
        let plist = d.len();
        for w in [0x0008u16, 5] {
            d.extend_from_slice(&w.to_be_bytes());
        }
        let strip: Vec<u16> = vec![1, 3, 0, 0, 0, 1, 256, 0, 2, 0, 256];
        d.extend_from_slice(&65u16.to_be_bytes());
        d.extend_from_slice(&(strip.len() as u16).to_be_bytes());
        for w in strip {
            d.extend_from_slice(&w.to_be_bytes());
        }
        d.extend_from_slice(&0xFFu16.to_be_bytes());
        d[model..model + 4].copy_from_slice(&(vlist as u32).to_be_bytes());
        d[model + 4..model + 8].copy_from_slice(&(plist as u32).to_be_bytes());
        d
    }

    #[test]
    fn reads_vertices_and_strip() {
        let d = triangle_model_be();
        let m = read_model(&Bytes::new(&d, Endian::Big), &FileOffsets, 0x10).unwrap();
        assert_eq!(m.vertex_chunks.len(), 1);
        assert_eq!(m.vertex_chunks[0].vertices[1].position, [1., 0., 0.]);
        assert_eq!(m.vertex_chunks[0].vertices[1].normal, Some([0., 1., 0.]));
        assert_eq!(m.poly_chunks[0], PolyChunk::Texture { kind: 8, flags: 0, texture_id: 5 });
        match &m.poly_chunks[1] {
            PolyChunk::Strip { strips, .. } => {
                assert_eq!(strips.len(), 1);
                assert!(!strips[0].reversed);
                assert_eq!(strips[0].vertices[1].index, 1);
                assert_eq!(strips[0].vertices[1].uv, Some([1.0, 0.0]));
            }
            other => panic!("expected strip, got {other:?}"),
        }
    }

    #[test]
    fn missing_end_or_garbage_errors() {
        let mut d = triangle_model_be();
        // Corrupt the polygon end marker into an unknown type.
        let n = d.len();
        d[n - 1] = 0x33;
        assert!(read_model(&Bytes::new(&d, Endian::Big), &FileOffsets, 0x10).is_err());
    }

    #[test]
    fn vn32_unpack() {
        let n = unpack_vn32((511 << 20) | (0x200 << 10));
        assert!((n[0] - 1.0).abs() < 1e-6);
        assert!((n[1] + 512.0 / 511.0).abs() < 1e-6);
    }
}
