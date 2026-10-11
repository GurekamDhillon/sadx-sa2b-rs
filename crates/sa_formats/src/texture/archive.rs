//! GVM (GameCube) and PVM (Dreamcast) texture archives.
//!
//! Header: magic `GVMH`/`PVMH`, u32 little-endian header length, u16 flags,
//! u16 count (big-endian in GVM, little-endian in PVM), then one entry per
//! texture. Texture chunks (optionally prefixed by `GBIX`/`GCIX`) follow at
//! `8 + header length`.

use super::{gvr, pvr, Rgba};
use crate::bytes::{Bytes, Endian};
use crate::error::{invalid, Error, Result};

const FLAG_GLOBAL_INDEX: u16 = 0x1;
const FLAG_DIMENSIONS: u16 = 0x2;
const FLAG_FORMATS: u16 = 0x4;
const FLAG_NAMES: u16 = 0x8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    Gvm,
    Pvm,
}

#[derive(Debug, Clone)]
pub struct ArchiveTexture {
    pub name: String,
    pub global_index: Option<u32>,
    /// The `GVRT`/`PVRT` chunk, starting at its magic.
    pub chunk: Vec<u8>,
}

impl ArchiveTexture {
    pub fn decode(&self, kind: ArchiveKind) -> Result<Rgba> {
        match kind {
            ArchiveKind::Gvm => gvr::decode(&self.chunk),
            ArchiveKind::Pvm => pvr::decode(&self.chunk),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Archive {
    pub kind: ArchiveKind,
    pub textures: Vec<ArchiveTexture>,
}

pub fn parse(data: &[u8]) -> Result<Archive> {
    let le = Bytes::new(data, Endian::Little);
    let magic = le.slice(0, 4)?;
    let (kind, endian, tex_magic) = match magic {
        b"GVMH" => (ArchiveKind::Gvm, Endian::Big, b"GVRT"),
        b"PVMH" => (ArchiveKind::Pvm, Endian::Little, b"PVRT"),
        _ => return Err(Error::BadMagic { expected: "GVMH or PVMH", found: magic.to_vec() }),
    };
    let b = Bytes::new(data, endian);
    let header_len = le.u32(4)? as usize;
    let flags = b.u16(8)?;
    let count = b.u16(10)? as usize;
    let entry_size = 2
        + if flags & FLAG_NAMES != 0 { 28 } else { 0 }
        + if flags & FLAG_FORMATS != 0 { 2 } else { 0 }
        + if flags & FLAG_DIMENSIONS != 0 { 2 } else { 0 }
        + if flags & FLAG_GLOBAL_INDEX != 0 { 4 } else { 0 };
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let e = 12 + i * entry_size;
        let name = if flags & FLAG_NAMES != 0 { b.cstr(e + 2, 28)? } else { String::new() };
        let gi = if flags & FLAG_GLOBAL_INDEX != 0 { Some(b.u32(e + entry_size - 4)?) } else { None };
        entries.push((name, gi));
    }
    let mut pos = 8 + header_len;
    let mut textures = Vec::with_capacity(count);
    for (name, global_index) in entries {
        // Skip to the next texture chunk, passing any GBIX/GCIX and padding.
        loop {
            let tag = le.slice(pos, 4)?;
            if tag == tex_magic {
                break;
            }
            if tag == b"GBIX" || tag == b"GCIX" {
                pos += 8 + le.u32(pos + 4)? as usize;
            } else if tag == [0, 0, 0, 0] {
                pos += 4;
            } else {
                return Err(invalid(pos, format!("expected texture chunk, found {tag:02x?}")));
            }
        }
        let len = le.u32(pos + 4)? as usize;
        let chunk = le.slice(pos, 8 + len)?.to_vec();
        pos += 8 + len;
        textures.push(ArchiveTexture { name, global_index, chunk });
    }
    Ok(Archive { kind, textures })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gvr_chunk() -> Vec<u8> {
        let mut c = b"GVRT".to_vec();
        c.extend_from_slice(&(8u32 + 32).to_le_bytes());
        c.extend_from_slice(&[0, 0, 0, 4, 0, 4, 0, 4]);
        c.extend_from_slice(&[0xF8, 0x00].repeat(16));
        c
    }

    #[test]
    fn parses_gvm_with_names_and_gbix() {
        let mut d = b"GVMH".to_vec();
        let header_len = 4 + 38;
        d.extend_from_slice(&(header_len as u32).to_le_bytes());
        d.extend_from_slice(&0x000Fu16.to_be_bytes());
        d.extend_from_slice(&1u16.to_be_bytes());
        d.extend_from_slice(&0u16.to_be_bytes());
        let mut name = b"red".to_vec();
        name.resize(28, 0);
        d.extend_from_slice(&name);
        d.extend_from_slice(&[0x01, 0x04, 0x00, 0x22]);
        d.extend_from_slice(&1234u32.to_be_bytes());
        d.extend_from_slice(b"GBIX");
        d.extend_from_slice(&8u32.to_le_bytes());
        d.extend_from_slice(&[0; 8]);
        d.extend_from_slice(&gvr_chunk());
        let a = parse(&d).unwrap();
        assert_eq!(a.textures.len(), 1);
        assert_eq!(a.textures[0].name, "red");
        assert_eq!(a.textures[0].global_index, Some(1234));
        assert_eq!(a.textures[0].decode(a.kind).unwrap().get(0, 0), [255, 0, 0, 255]);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse(b"ABCD").is_err());
    }
}
