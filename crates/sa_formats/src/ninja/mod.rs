//! Sega Ninja library formats: node trees, chunk/basic models, motions.

pub mod basic;
pub mod chunk;
pub mod flatten;
pub mod gc;
pub mod math;
pub mod motion;
pub mod object;
pub mod pose;

use crate::bytes::{Bytes, Endian, FileOffsets};
use crate::error::{invalid, Result};
pub use flatten::{flatten, Mesh, SubMesh};
pub use object::{Attach, ModelKind, Node};

/// One entry of an SA2 `*mdl.prs` (after PRS decompression): a model id and
/// its node tree.
#[derive(Debug, Clone)]
pub struct IndexedModel {
    pub id: i32,
    pub root: Node,
}

/// Parses an SA2 model file: a table of (i32 id, u32 offset) pairs ended by
/// id -1, where offsets are file offsets of `NJS_OBJECT`s. Big-endian on PC.
pub fn read_model_file(data: &[u8], endian: Endian) -> Result<Vec<IndexedModel>> {
    let b = Bytes::new(data, endian);
    let mut out = Vec::new();
    let mut pos = 0;
    loop {
        let id = b.i32(pos)?;
        if id == -1 {
            return Ok(out);
        }
        if out.len() > 4096 {
            return Err(invalid(pos, "model table too long"));
        }
        let off = b.u32(pos + 4)? as usize;
        out.push(IndexedModel { id, root: Node::read(&b, &FileOffsets, off, ModelKind::Chunk)? });
        pos += 8;
    }
}
