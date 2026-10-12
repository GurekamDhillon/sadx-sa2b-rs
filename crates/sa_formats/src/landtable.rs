//! Level geometry tables (`LandTable` + `COL` entries).
//!
//! SA2 (little-endian, inside `Data_DLL.dll`):
//! `i16 col_count, i16 visible_count, i16, i16 anim_count, i16 attrs,
//! i16 tex_loaded, f32 clip, COL* cols, void* anims, char* tex_name,
//! NJS_TEXLIST* texlist`. COL (0x20): center[3], radius, NJS_OBJECT*,
//! i32, i32, u32 surface flags. The first `visible_count` entries hold visual
//! models: GameCube Ginja or chunk models (detected per entry, see
//! `sniff_visual`). The rest hold basic models (collision).

use crate::bytes::{AddressMap, Bytes, Endian};
use crate::error::{invalid, Result};
use crate::ninja::basic::BasicVariant;
use crate::ninja::{flatten, Mesh, ModelKind, Node};

pub mod surface {
    pub const SOLID: u32 = 0x1;
    pub const WATER: u32 = 0x2;
    pub const DIGGABLE: u32 = 0x20;
    pub const UNCLIMBABLE: u32 = 0x80;
    pub const STAIRS: u32 = 0x100;
    pub const HURT: u32 = 0x400;
    pub const FEET_SOUND: u32 = 0x800;
    pub const CANNOT_LAND: u32 = 0x1000;
    pub const WATER_NO_ALPHA: u32 = 0x2000;
    pub const NO_SHADOW: u32 = 0x8000;
    pub const ACCELERATE: u32 = 0x100000;
    pub const NO_FOG: u32 = 0x400000;
    pub const DYNAMIC: u32 = 0x8000000;
    pub const VISIBLE: u32 = 0x80000000;
}

#[derive(Debug, Clone)]
pub struct Col {
    pub center: [f32; 3],
    pub radius: f32,
    pub flags: u32,
    pub model: Node,
}

#[derive(Debug, Clone)]
pub struct LandTable {
    pub clip_distance: f32,
    pub texture_name: Option<String>,
    pub cols: Vec<Col>,
}

/// A flattened collision triangle soup with per-triangle surface flags.
#[derive(Debug, Clone, Default)]
pub struct CollisionMesh {
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    /// One entry per triangle.
    pub flags: Vec<u32>,
}

impl LandTable {
    /// Reads an SA2 landtable at file offset `off` (pointers mapped by `map`).
    pub fn read_sa2(data: &[u8], map: &dyn AddressMap, off: usize) -> Result<LandTable> {
        let b = Bytes::new(data, Endian::Little);
        let count = b.i16(off)?;
        if count < 0 || count > 0x4000 {
            return Err(invalid(off, format!("bad COL count {count}")));
        }
        // A visible count of -1 means "decide per entry by the VISIBLE flag".
        let visible = b.i16(off + 2)?;
        let cols_ptr = b.u32(off + 0x10)?;
        let texture_name = map.offset(b.u32(off + 0x18)?).map(|o| b.cstr(o, 64)).transpose()?;
        let mut cols = Vec::with_capacity(count as usize);
        if let Some(c) = map.offset(cols_ptr) {
            for i in 0..count as usize {
                let e = c + i * 0x20;
                let flags = b.u32(e + 0x1C)?;
                let is_visual = if visible < 0 { flags & surface::VISIBLE != 0 } else { i < visible as usize };
                let kind = if is_visual { sniff_visual(&b, map, e)? } else { ModelKind::Basic(BasicVariant::Sa1) };
                let obj = map.offset(b.u32(e + 0x10)?).ok_or_else(|| invalid(e, "COL without model"))?;
                cols.push(Col { center: b.vec3(e)?, radius: b.f32(e + 12)?, flags, model: Node::read(&b, map, obj, kind)? });
            }
        }
        Ok(LandTable { clip_distance: b.f32(off + 0x0C)?, texture_name, cols })
    }

    /// Visible geometry (entries flagged `VISIBLE`) as one mesh.
    pub fn visual_mesh(&self) -> Mesh {
        let mut out = Mesh::default();
        for c in self.cols.iter().filter(|c| c.flags & surface::VISIBLE != 0) {
            append(&mut out, flatten(&c.model));
        }
        out
    }

    /// Solid/water geometry with surface flags per triangle.
    pub fn collision_mesh(&self) -> CollisionMesh {
        let mut out = CollisionMesh::default();
        for c in self.cols.iter().filter(|c| c.flags & (surface::SOLID | surface::WATER) != 0) {
            let m = flatten(&c.model);
            let base = out.positions.len() as u32;
            out.positions.extend_from_slice(&m.positions);
            out.indices.extend(m.indices.iter().map(|i| i + base));
            out.flags.extend(std::iter::repeat(c.flags).take(m.triangle_count()));
        }
        out
    }
}

/// Visual entries are chunk or GameCube models; the header hint is not
/// reliable, so look at the first byte of the vertex data: chunk vertex
/// types are 32..=50, GC vertex sets start with an attribute id (<= 0x0D)
/// or the 0xFF terminator.
fn sniff_visual(b: &Bytes, map: &dyn AddressMap, col: usize) -> Result<ModelKind> {
    let kind = (|| {
        let obj = map.offset(b.u32(col + 0x10).ok()?)?;
        let attach = map.offset(b.u32(obj + 4).ok()?)?;
        let first = b.u8(map.offset(b.u32(attach).ok()?)?).ok()?;
        Some(if (32..=50).contains(&first) { ModelKind::Chunk } else { ModelKind::Gc })
    })();
    Ok(kind.unwrap_or(ModelKind::Chunk))
}

fn append(out: &mut Mesh, m: Mesh) {
    let base = out.positions.len() as u32;
    let first = out.indices.len() as u32;
    out.src_slot.extend(m.src_slot);
    out.src_node.extend(m.src_node);
    out.positions.extend(m.positions);
    out.normals.extend(m.normals);
    out.uvs.extend(m.uvs);
    out.indices.extend(m.indices.iter().map(|i| i + base));
    out.submeshes.extend(m.submeshes.into_iter().map(|mut s| {
        s.first_index += first;
        s
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytes::FileOffsets;
    use crate::ninja::basic;

    #[test]
    fn reads_collision_entry() {
        // Basic quad model at 0x10 (from the basic tests), node, COL, table.
        let mut d = basic::tests::quad_model_le();
        let node = d.len();
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&0x10u32.to_le_bytes());
        d.extend_from_slice(&[0; 24]);
        for _ in 0..3 {
            d.extend_from_slice(&1f32.to_le_bytes());
        }
        d.extend_from_slice(&[0; 8]);
        let col = d.len();
        d.extend_from_slice(&[0; 16]);
        d.extend_from_slice(&(node as u32).to_le_bytes());
        d.extend_from_slice(&[0; 8]);
        d.extend_from_slice(&surface::SOLID.to_le_bytes());
        let table = d.len();
        d.extend_from_slice(&1i16.to_le_bytes());
        d.extend_from_slice(&0i16.to_le_bytes());
        d.extend_from_slice(&[0; 8]);
        d.extend_from_slice(&1000f32.to_le_bytes());
        d.extend_from_slice(&(col as u32).to_le_bytes());
        d.extend_from_slice(&[0; 12]);
        let lt = LandTable::read_sa2(&d, &FileOffsets, table).unwrap();
        assert_eq!(lt.cols.len(), 1);
        assert_eq!(lt.clip_distance, 1000.0);
        let cm = lt.collision_mesh();
        assert_eq!(cm.indices.len(), 6);
        assert_eq!(cm.flags, vec![surface::SOLID; 2]);
        assert_eq!(lt.visual_mesh().triangle_count(), 0);
    }

    #[test]
    fn rejects_bad_count() {
        let mut d = vec![0u8; 0x20];
        d[0..2].copy_from_slice(&(-5i16).to_le_bytes());
        assert!(LandTable::read_sa2(&d, &FileOffsets, 0).is_err());
    }
}
