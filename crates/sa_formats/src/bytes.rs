//! Bounds-checked, endian-aware reads from a byte slice.

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endian {
    Little,
    Big,
}

/// A view over bytes with a fixed byte order. Every read is bounds-checked.
#[derive(Debug, Clone, Copy)]
pub struct Bytes<'a> {
    pub data: &'a [u8],
    pub endian: Endian,
}

impl<'a> Bytes<'a> {
    pub fn new(data: &'a [u8], endian: Endian) -> Self {
        Bytes { data, endian }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn slice(&self, off: usize, len: usize) -> Result<&'a [u8]> {
        let end = off.checked_add(len).ok_or(Error::Eof(off))?;
        self.data.get(off..end).ok_or(Error::Eof(off))
    }

    fn arr<const N: usize>(&self, off: usize) -> Result<[u8; N]> {
        let mut a: [u8; N] = self.slice(off, N)?.try_into().unwrap();
        if self.endian == Endian::Big {
            a.reverse();
        }
        Ok(a)
    }

    pub fn u8(&self, off: usize) -> Result<u8> {
        self.data.get(off).copied().ok_or(Error::Eof(off))
    }

    pub fn i8(&self, off: usize) -> Result<i8> {
        Ok(self.u8(off)? as i8)
    }

    pub fn u16(&self, off: usize) -> Result<u16> {
        Ok(u16::from_le_bytes(self.arr(off)?))
    }

    pub fn i16(&self, off: usize) -> Result<i16> {
        Ok(i16::from_le_bytes(self.arr(off)?))
    }

    pub fn u32(&self, off: usize) -> Result<u32> {
        Ok(u32::from_le_bytes(self.arr(off)?))
    }

    pub fn i32(&self, off: usize) -> Result<i32> {
        Ok(i32::from_le_bytes(self.arr(off)?))
    }

    pub fn f32(&self, off: usize) -> Result<f32> {
        Ok(f32::from_le_bytes(self.arr(off)?))
    }

    pub fn vec3(&self, off: usize) -> Result<[f32; 3]> {
        Ok([self.f32(off)?, self.f32(off + 4)?, self.f32(off + 8)?])
    }

    /// NUL-terminated string of at most `max` bytes (lossy UTF-8).
    pub fn cstr(&self, off: usize, max: usize) -> Result<String> {
        let avail = self.data.len().saturating_sub(off).min(max);
        let s = self.slice(off, avail)?;
        let end = s.iter().position(|&b| b == 0).unwrap_or(s.len());
        Ok(String::from_utf8_lossy(&s[..end]).into_owned())
    }
}

/// Maps the 32-bit pointers stored inside a file to offsets in its bytes.
pub trait AddressMap {
    /// Offset of `addr` within the data, or `None` for a null/unmapped pointer.
    fn offset(&self, addr: u32) -> Option<usize>;
}

/// Pointers are plain file offsets (SA2 PC `*mdl.prs`, `*mtn.prs`).
#[derive(Debug, Clone, Copy, Default)]
pub struct FileOffsets;

impl AddressMap for FileOffsets {
    fn offset(&self, addr: u32) -> Option<usize> {
        if addr == 0 { None } else { Some(addr as usize) }
    }
}

/// Pointers are file offsets plus a fixed base (SADX/SA1 key-addressed data).
#[derive(Debug, Clone, Copy)]
pub struct Based(pub u32);

impl AddressMap for Based {
    fn offset(&self, addr: u32) -> Option<usize> {
        if addr == 0 { None } else { addr.checked_sub(self.0).map(|o| o as usize) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_both_endians() {
        let d = [0x12, 0x34, 0x56, 0x78];
        assert_eq!(Bytes::new(&d, Endian::Little).u32(0).unwrap(), 0x7856_3412);
        assert_eq!(Bytes::new(&d, Endian::Big).u32(0).unwrap(), 0x1234_5678);
        assert_eq!(Bytes::new(&d, Endian::Big).u16(2).unwrap(), 0x5678);
    }

    #[test]
    fn floats_big_endian() {
        let d = 1.5f32.to_be_bytes();
        assert_eq!(Bytes::new(&d, Endian::Big).f32(0).unwrap(), 1.5);
    }

    #[test]
    fn out_of_bounds_errors() {
        let b = Bytes::new(&[1, 2, 3], Endian::Little);
        assert_eq!(b.u32(0), Err(Error::Eof(0)));
        assert_eq!(b.u16(2), Err(Error::Eof(2)));
        assert_eq!(b.slice(usize::MAX, 2), Err(Error::Eof(usize::MAX)));
    }

    #[test]
    fn cstr_stops_at_nul_and_bounds() {
        let b = Bytes::new(b"abc\0def", Endian::Little);
        assert_eq!(b.cstr(0, 32).unwrap(), "abc");
        assert_eq!(b.cstr(4, 32).unwrap(), "def");
    }

    #[test]
    fn address_maps() {
        assert_eq!(FileOffsets.offset(0), None);
        assert_eq!(FileOffsets.offset(0x10), Some(0x10));
        assert_eq!(Based(0x1000).offset(0x1010), Some(0x10));
        assert_eq!(Based(0x1000).offset(0x10), None);
    }
}
