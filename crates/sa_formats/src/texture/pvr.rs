//! Dreamcast PVR textures (`PVRT` chunks), as used by SADX.
//!
//! Header after `PVRT`+length (little-endian): u8 pixel format, u8 data
//! format, u16 pad, u16 width, u16 height. Supported: 16-bit pixel formats
//! with twiddled, twiddled-mipmap, rectangle and twiddled-rectangle layouts.
//! VQ-compressed and palettised data return an error.

use super::{expand, Rgba};
use crate::bytes::{Bytes, Endian};
use crate::error::{invalid, Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Argb1555 = 0,
    Rgb565 = 1,
    Argb4444 = 2,
}

fn pixel(fmt: PixelFormat, c: u16) -> [u8; 4] {
    let c = c as u32;
    match fmt {
        PixelFormat::Argb1555 => [
            expand((c >> 10) & 31, 5),
            expand((c >> 5) & 31, 5),
            expand(c & 31, 5),
            if c & 0x8000 != 0 { 255 } else { 0 },
        ],
        PixelFormat::Rgb565 => super::rgb565(c as u16),
        PixelFormat::Argb4444 => [
            expand((c >> 8) & 15, 4),
            expand((c >> 4) & 15, 4),
            expand(c & 15, 4),
            expand((c >> 12) & 15, 4),
        ],
    }
}

/// Morton (twiddle) index of (x, y) within a square of side `size`.
pub fn twiddle(x: u32, y: u32) -> u32 {
    let mut out = 0;
    for bit in 0..16 {
        out |= ((y >> bit) & 1) << (2 * bit);
        out |= ((x >> bit) & 1) << (2 * bit + 1);
    }
    out
}

/// Parses a `PVRT` chunk (starting at its magic) and decodes the top level.
pub fn decode(chunk: &[u8]) -> Result<Rgba> {
    let b = Bytes::new(chunk, Endian::Little);
    let magic = b.slice(0, 4)?;
    if magic != b"PVRT" {
        return Err(Error::BadMagic { expected: "PVRT", found: magic.to_vec() });
    }
    let pf = b.u8(8)?;
    let df = b.u8(9)?;
    let width = b.u16(12)? as u32;
    let height = b.u16(14)? as u32;
    if width == 0 || height == 0 || width > 1024 || height > 1024 {
        return Err(invalid(12, format!("bad PVR size {width}x{height}")));
    }
    let fmt = match pf {
        0 => PixelFormat::Argb1555,
        1 => PixelFormat::Rgb565,
        2 => PixelFormat::Argb4444,
        _ => return Err(invalid(8, format!("unsupported PVR pixel format {pf}"))),
    };
    let data = Bytes::new(b.slice(16, chunk.len().saturating_sub(16))?, Endian::Little);
    let mut img = Rgba::new(width, height);
    match df {
        // Square twiddled, optionally with a mip chain stored smallest first.
        1 | 2 => {
            let base = if df == 2 { mip_offset(width) } else { 0 };
            for y in 0..height {
                for x in 0..width {
                    let i = base + twiddle(x, y) as usize * 2;
                    img.put(x, y, pixel(fmt, data.u16(i)?));
                }
            }
        }
        // Rectangle / stride: plain raster.
        9 | 11 => {
            for y in 0..height {
                for x in 0..width {
                    let i = ((y * width + x) * 2) as usize;
                    img.put(x, y, pixel(fmt, data.u16(i)?));
                }
            }
        }
        // Twiddled rectangle: a row or column of square twiddled tiles.
        13 => {
            let side = width.min(height);
            for y in 0..height {
                for x in 0..width {
                    let tile = (x / side) + (y / side);
                    let i = (tile * side * side + twiddle(x % side, y % side)) as usize * 2;
                    img.put(x, y, pixel(fmt, data.u16(i)?));
                }
            }
        }
        3 | 4 | 16 | 17 => return Err(invalid(9, "VQ-compressed PVR textures are not supported")),
        _ => return Err(invalid(9, format!("unsupported PVR data format {df}"))),
    }
    Ok(img)
}

/// Byte offset of the top level in a 16-bit twiddled mip chain.
/// The 1x1 level occupies 2 bytes and is preceded by 2 bytes of padding
/// (6 bytes total before 2x2 for 16bpp chains, per the Katana SDK layout).
fn mip_offset(width: u32) -> usize {
    let mut off = 2usize;
    let mut s = 1u32;
    while s < width {
        off += (s * s * 2) as usize;
        s *= 2;
    }
    off
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(pf: u8, df: u8, w: u16, h: u16, data: &[u8]) -> Vec<u8> {
        let mut c = b"PVRT".to_vec();
        c.extend_from_slice(&((8 + data.len()) as u32).to_le_bytes());
        c.extend_from_slice(&[pf, df, 0, 0]);
        c.extend_from_slice(&w.to_le_bytes());
        c.extend_from_slice(&h.to_le_bytes());
        c.extend_from_slice(data);
        c
    }

    #[test]
    fn twiddle_pattern() {
        assert_eq!(twiddle(0, 0), 0);
        assert_eq!(twiddle(0, 1), 1);
        assert_eq!(twiddle(1, 0), 2);
        assert_eq!(twiddle(1, 1), 3);
        assert_eq!(twiddle(2, 0), 8);
    }

    #[test]
    fn twiddled_square() {
        let mut data = vec![0u8; 8];
        // Pixel (1,0) has twiddle index 2.
        data[4..6].copy_from_slice(&0xF800u16.to_le_bytes());
        let img = decode(&chunk(1, 1, 2, 2, &data)).unwrap();
        assert_eq!(img.get(1, 0), [255, 0, 0, 255]);
        assert_eq!(img.get(0, 1), [0, 0, 0, 255]);
    }

    #[test]
    fn rectangle_raster_and_alpha() {
        let mut data = vec![0u8; 8];
        data[2..4].copy_from_slice(&0x8000u16.to_le_bytes());
        let img = decode(&chunk(0, 9, 2, 2, &data)).unwrap();
        assert_eq!(img.get(1, 0), [0, 0, 0, 255]);
        assert_eq!(img.get(0, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn mip_offset_values() {
        assert_eq!(mip_offset(1), 2);
        assert_eq!(mip_offset(2), 4);
        assert_eq!(mip_offset(4), 12);
    }

    #[test]
    fn rejects_vq() {
        assert!(decode(&chunk(1, 3, 8, 8, &[0; 256])).is_err());
    }
}
