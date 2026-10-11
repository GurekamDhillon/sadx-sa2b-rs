//! SA2 PC `.pak` archives (`resource/gd_PC/PRS/*.pak`).
//!
//! Layout (little-endian): magic `01 'p' 'a' 'k'`, padding to 0x39, file
//! count, then per file: long path, short name, two equal sizes. File data
//! follows the table in the same order. Members are usually DDS textures
//! plus a `.inf` index.

use crate::bytes::{Bytes, Endian};
use crate::error::{invalid, Error, Result};

pub const MAGIC: &[u8; 4] = b"\x01pak";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PakEntry {
    pub long_path: String,
    pub name: String,
    pub data: Vec<u8>,
}

pub fn parse(data: &[u8]) -> Result<Vec<PakEntry>> {
    let b = Bytes::new(data, Endian::Little);
    let magic = b.slice(0, 4)?;
    if magic != MAGIC {
        return Err(Error::BadMagic { expected: "\\x01pak", found: magic.to_vec() });
    }
    let count = b.u32(0x39)? as usize;
    if count > 0x10000 {
        return Err(invalid(0x39, format!("implausible file count {count}")));
    }
    let mut pos = 0x3D;
    let mut table = Vec::with_capacity(count);
    for _ in 0..count {
        let plen = b.u32(pos)? as usize;
        let long_path = String::from_utf8_lossy(b.slice(pos + 4, plen)?).into_owned();
        pos += 4 + plen;
        let nlen = b.u32(pos)? as usize;
        let name = String::from_utf8_lossy(b.slice(pos + 4, nlen)?).into_owned();
        pos += 4 + nlen;
        let size = b.u32(pos)? as usize;
        let size2 = b.u32(pos + 4)? as usize;
        if size != size2 {
            return Err(invalid(pos, format!("size mismatch {size} != {size2}")));
        }
        pos += 8;
        table.push((long_path, name, size));
    }
    let mut out = Vec::with_capacity(count);
    for (long_path, name, size) in table {
        let data = b.slice(pos, size)?.to_vec();
        pos += size;
        out.push(PakEntry { long_path, name, data });
    }
    Ok(out)
}

/// Builds a PAK archive. Used for tests and tooling.
pub fn build(entries: &[(&str, &str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.resize(0x39, 0);
    out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for (long, name, data) in entries {
        for s in [long, name] {
            out.extend_from_slice(&(s.len() as u32).to_le_bytes());
            out.extend_from_slice(s.as_bytes());
        }
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    }
    for (_, _, data) in entries {
        out.extend_from_slice(data);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let bytes = build(&[("..\\a\\x.inf", "a\\x.inf", b"inf"), ("..\\a\\y.dds", "a\\y.dds", b"DDS data")]);
        let e = parse(&bytes).unwrap();
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].name, "a\\x.inf");
        assert_eq!(e[1].data, b"DDS data");
    }

    #[test]
    fn bad_magic_and_truncation() {
        assert!(matches!(parse(b"nope"), Err(Error::BadMagic { .. })));
        let bytes = build(&[("l", "n", b"0123456789")]);
        assert!(matches!(parse(&bytes[..bytes.len() - 1]), Err(Error::Eof(_))));
    }
}
