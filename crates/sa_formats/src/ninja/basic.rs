//! Ninja basic models (`NJS_MODEL`), used by SADX and by landtable collision.
//!
//! Model: points ptr, normals ptr, i32 point count, meshsets ptr,
//! materials ptr, u16 meshset count, u16 material count, center[3], radius
//! (SADX adds a 4-byte buffer pointer). Meshset: u16 type/material,
//! u16 primitive count, then index/attr/normal/colour/uv pointers
//! (SADX adds a buffer pointer).

use crate::bytes::{AddressMap, Bytes};
use crate::error::{invalid, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasicVariant {
    /// Dreamcast SA1 / SA2 landtable layout: meshset 0x18 bytes.
    Sa1,
    /// SADX PC layout: meshset 0x1C bytes.
    Sadx,
}

impl BasicVariant {
    fn meshset_size(self) -> usize {
        match self {
            BasicVariant::Sa1 => 0x18,
            BasicVariant::Sadx => 0x1C,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolyType {
    Triangles,
    Quads,
    NStrip,
    Strip,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Meshset {
    pub material: u16,
    pub poly_type: PolyType,
    /// One entry per primitive. Strips keep their own vertex lists.
    pub polys: Vec<Vec<u16>>,
    /// Strip winding flags (bit 15 of each strip header).
    pub reversed: Vec<bool>,
    /// UVs per corner, in the same order as the flattened `polys` indices.
    pub uvs: Option<Vec<[f32; 2]>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub diffuse: u32,
    pub texture_id: u32,
    pub attr_flags: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BasicModel {
    pub points: Vec<[f32; 3]>,
    pub normals: Option<Vec<[f32; 3]>>,
    pub meshsets: Vec<Meshset>,
    pub materials: Vec<Material>,
    pub center: [f32; 3],
    pub radius: f32,
}

const MAX_POINTS: usize = 1 << 20;

pub fn read_model(b: &Bytes, map: &dyn AddressMap, off: usize, variant: BasicVariant) -> Result<BasicModel> {
    let npoints = b.i32(off + 8)?;
    if npoints < 0 || npoints as usize > MAX_POINTS {
        return Err(invalid(off + 8, format!("bad point count {npoints}")));
    }
    let npoints = npoints as usize;
    let read_vecs = |ptr: u32| -> Result<Option<Vec<[f32; 3]>>> {
        match map.offset(ptr) {
            None => Ok(None),
            Some(p) => (0..npoints).map(|i| b.vec3(p + i * 12)).collect::<Result<Vec<_>>>().map(Some),
        }
    };
    let points = read_vecs(b.u32(off)?)?.unwrap_or_default();
    let normals = read_vecs(b.u32(off + 4)?)?;
    let nmeshsets = b.u16(off + 0x14)? as usize;
    let nmats = b.u16(off + 0x16)? as usize;
    let mut materials = Vec::with_capacity(nmats);
    if let Some(m) = map.offset(b.u32(off + 0x10)?) {
        for i in 0..nmats {
            let o = m + i * 0x14;
            materials.push(Material { diffuse: b.u32(o)?, texture_id: b.u32(o + 0x0C)?, attr_flags: b.u32(o + 0x10)? });
        }
    }
    let mut meshsets = Vec::with_capacity(nmeshsets);
    if let Some(ms) = map.offset(b.u32(off + 0x0C)?) {
        for i in 0..nmeshsets {
            meshsets.push(read_meshset(b, map, ms + i * variant.meshset_size())?);
        }
    }
    Ok(BasicModel { points, normals, meshsets, materials, center: b.vec3(off + 0x18)?, radius: b.f32(off + 0x24)? })
}

fn read_meshset(b: &Bytes, map: &dyn AddressMap, off: usize) -> Result<Meshset> {
    let type_mat = b.u16(off)?;
    let count = b.u16(off + 2)? as usize;
    let poly_type = match type_mat >> 14 {
        0 => PolyType::Triangles,
        1 => PolyType::Quads,
        2 => PolyType::NStrip,
        _ => PolyType::Strip,
    };
    let mut polys = Vec::with_capacity(count);
    let mut reversed = Vec::new();
    let mut corners = 0usize;
    if let Some(mut p) = map.offset(b.u32(off + 4)?) {
        for _ in 0..count {
            let poly: Vec<u16> = match poly_type {
                PolyType::Triangles | PolyType::Quads => {
                    let n = if poly_type == PolyType::Triangles { 3 } else { 4 };
                    let v = (0..n).map(|k| b.u16(p + k * 2)).collect::<Result<_>>()?;
                    p += n * 2;
                    v
                }
                PolyType::NStrip | PolyType::Strip => {
                    let head = b.u16(p)?;
                    let n = (head & 0x7FFF) as usize;
                    reversed.push(head & 0x8000 != 0);
                    let v = (0..n).map(|k| b.u16(p + 2 + k * 2)).collect::<Result<_>>()?;
                    p += 2 + n * 2;
                    v
                }
            };
            corners += poly.len();
            polys.push(poly);
        }
    }
    let uvs = match map.offset(b.u32(off + 0x14)?) {
        Some(u) => Some((0..corners).map(|i| Ok([b.i16(u + i * 4)? as f32 / 256.0, b.i16(u + i * 4 + 2)? as f32 / 256.0])).collect::<Result<Vec<_>>>()?),
        None => None,
    };
    Ok(Meshset { material: type_mat & 0x3FFF, poly_type, polys, reversed, uvs })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::bytes::{Endian, FileOffsets};

    /// Little-endian SA1-layout basic model at 0x10: one quad (two triangles).
    pub fn quad_model_le() -> Vec<u8> {
        let mut d = vec![0u8; 0x10];
        let model = d.len();
        d.extend_from_slice(&[0; 0x28]);
        let points = d.len();
        for p in [[0f32, 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]] {
            for f in p {
                d.extend_from_slice(&f.to_le_bytes());
            }
        }
        let idx = d.len();
        for i in [0u16, 1, 2, 3] {
            d.extend_from_slice(&i.to_le_bytes());
        }
        let meshset = d.len();
        d.extend_from_slice(&((1u16 << 14) | 2).to_le_bytes());
        d.extend_from_slice(&1u16.to_le_bytes());
        d.extend_from_slice(&(idx as u32).to_le_bytes());
        d.extend_from_slice(&[0; 16]);
        let w = |d: &mut Vec<u8>, at: usize, v: u32| d[at..at + 4].copy_from_slice(&v.to_le_bytes());
        w(&mut d, model, points as u32);
        w(&mut d, model + 8, 4);
        w(&mut d, model + 0x0C, meshset as u32);
        d[model + 0x14..model + 0x16].copy_from_slice(&1u16.to_le_bytes());
        d
    }

    #[test]
    fn reads_quad() {
        let d = quad_model_le();
        let m = read_model(&Bytes::new(&d, Endian::Little), &FileOffsets, 0x10, BasicVariant::Sa1).unwrap();
        assert_eq!(m.points.len(), 4);
        assert_eq!(m.meshsets[0].poly_type, PolyType::Quads);
        assert_eq!(m.meshsets[0].material, 2);
        assert_eq!(m.meshsets[0].polys, vec![vec![0, 1, 2, 3]]);
    }

    #[test]
    fn rejects_negative_count() {
        let mut d = quad_model_le();
        d[0x18..0x1C].copy_from_slice(&(-1i32).to_le_bytes());
        assert!(read_model(&Bytes::new(&d, Endian::Little), &FileOffsets, 0x10, BasicVariant::Sa1).is_err());
    }
}
