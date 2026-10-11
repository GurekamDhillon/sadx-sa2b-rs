//! Ninja motions (`NJS_MOTION`): per-node keyframe tracks.
//!
//! Header: mdata ptr, u32 frame count, u16 type flags, u16 inp_fn (low 4
//! bits = number of track kinds). `mdata` holds, per node, one pointer per
//! enabled kind followed by one u32 key count per kind.

use crate::bytes::{AddressMap, Bytes, Endian, FileOffsets};
use crate::error::{invalid, Result};

pub mod mtype {
    pub const POS: u16 = 0x1;
    pub const ANG: u16 = 0x2;
    pub const SCL: u16 = 0x4;
    pub const VEC: u16 = 0x8;
    pub const VERT: u16 = 0x10;
    pub const NORM: u16 = 0x20;
    pub const TARGET: u16 = 0x40;
    pub const ROLL: u16 = 0x80;
    pub const ANGLE: u16 = 0x100;
    pub const LIGHT_COLOR: u16 = 0x200;
    pub const LIGHT_INTENSITY: u16 = 0x400;
    pub const SPOT: u16 = 0x800;
    pub const POINT: u16 = 0x1000;
    pub const QUAT: u16 = 0x2000;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key<T> {
    pub frame: u32,
    pub value: T,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NodeTracks {
    pub position: Vec<Key<[f32; 3]>>,
    /// BAMS angles.
    pub rotation: Vec<Key<[i32; 3]>>,
    pub scale: Vec<Key<[f32; 3]>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Motion {
    pub frame_count: u32,
    pub type_flags: u16,
    pub nodes: Vec<NodeTracks>,
}

const MAX_KEYS: usize = 1 << 16;

impl Motion {
    /// Reads a motion at `off` for a skeleton of `node_count` nodes.
    pub fn read(b: &Bytes, map: &dyn AddressMap, off: usize, node_count: usize) -> Result<Motion> {
        let mdata = b.u32(off)?;
        let frame_count = b.u32(off + 4)?;
        let type_flags = b.u16(off + 8)?;
        let kinds = (b.u16(off + 10)? & 0xF) as usize;
        if node_count > 4096 {
            return Err(invalid(off, format!("implausible node count {node_count}")));
        }
        let order = [mtype::POS, mtype::ANG, mtype::SCL, mtype::VEC, mtype::VERT, mtype::NORM, mtype::TARGET, mtype::ROLL, mtype::ANGLE, mtype::LIGHT_COLOR, mtype::LIGHT_INTENSITY, mtype::SPOT, mtype::POINT, mtype::QUAT];
        let enabled: Vec<u16> = order.into_iter().filter(|f| type_flags & f != 0).take(kinds).collect();
        let mut nodes = vec![NodeTracks::default(); node_count];
        if let Some(md) = map.offset(mdata) {
            let entry = kinds * 8;
            for (n, tracks) in nodes.iter_mut().enumerate() {
                let e = md + n * entry;
                for (k, kind) in enabled.iter().enumerate() {
                    let ptr = b.u32(e + k * 4)?;
                    let count = b.u32(e + kinds * 4 + k * 4)? as usize;
                    let Some(keys) = map.offset(ptr) else { continue };
                    if count > MAX_KEYS {
                        return Err(invalid(e, format!("implausible key count {count}")));
                    }
                    match *kind {
                        mtype::POS => tracks.position = read_vec_keys(b, keys, count)?,
                        mtype::SCL => tracks.scale = read_vec_keys(b, keys, count)?,
                        mtype::ANG => {
                            tracks.rotation = (0..count)
                                .map(|i| {
                                    let o = keys + i * 16;
                                    Ok(Key { frame: b.u32(o)?, value: [b.i32(o + 4)?, b.i32(o + 8)?, b.i32(o + 12)?] })
                                })
                                .collect::<Result<_>>()?
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(Motion { frame_count, type_flags, nodes })
    }
}

fn read_vec_keys(b: &Bytes, off: usize, count: usize) -> Result<Vec<Key<[f32; 3]>>> {
    (0..count).map(|i| Ok(Key { frame: b.u32(off + i * 16)?, value: b.vec3(off + i * 16 + 4)? })).collect()
}

/// One entry of an SA2 `*mtn.prs`: an animation id and its motion.
#[derive(Debug, Clone)]
pub struct IndexedMotion {
    pub id: u16,
    pub motion: Motion,
}

/// Parses an SA2 motion file: a table of (u16 id, u16 node count, u32
/// offset) entries ended by id 0xFFFF. Big-endian on PC.
pub fn read_motion_file(data: &[u8], endian: Endian) -> Result<Vec<IndexedMotion>> {
    let b = Bytes::new(data, endian);
    let mut out = Vec::new();
    let mut pos = 0;
    loop {
        let id = b.u16(pos)?;
        if id == 0xFFFF {
            return Ok(out);
        }
        if out.len() > 4096 {
            return Err(invalid(pos, "motion table too long"));
        }
        let nodes = b.u16(pos + 2)? as usize;
        let off = b.u32(pos + 4)? as usize;
        out.push(IndexedMotion { id, motion: Motion::read(&b, &FileOffsets, off, nodes)? });
        pos += 8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_pos_and_ang_tracks() {
        // Table, motion header, mdata (1 node, 2 kinds), keys.
        let mut d = Vec::new();
        let be32 = |d: &mut Vec<u8>, v: u32| d.extend_from_slice(&v.to_be_bytes());
        d.extend_from_slice(&[0x00, 0x07, 0x00, 0x01]);
        be32(&mut d, 16);
        d.extend_from_slice(&[0xFF, 0xFF, 0, 0, 0, 0, 0, 0]);
        // Motion at 16.
        be32(&mut d, 28);
        be32(&mut d, 10);
        d.extend_from_slice(&[0x00, 0x03, 0x00, 0x02]);
        // mdata at 28: pos ptr, ang ptr, pos count, ang count.
        be32(&mut d, 44);
        be32(&mut d, 60);
        be32(&mut d, 1);
        be32(&mut d, 1);
        // pos key at 44.
        be32(&mut d, 0);
        for f in [1.0f32, 2.0, 3.0] {
            d.extend_from_slice(&f.to_be_bytes());
        }
        // ang key at 60.
        be32(&mut d, 9);
        for a in [0x4000i32, 0, -0x4000] {
            d.extend_from_slice(&a.to_be_bytes());
        }
        let m = read_motion_file(&d, Endian::Big).unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].id, 7);
        assert_eq!(m[0].motion.frame_count, 10);
        assert_eq!(m[0].motion.nodes[0].position[0].value, [1., 2., 3.]);
        assert_eq!(m[0].motion.nodes[0].rotation[0], Key { frame: 9, value: [0x4000, 0, -0x4000] });
    }
}
