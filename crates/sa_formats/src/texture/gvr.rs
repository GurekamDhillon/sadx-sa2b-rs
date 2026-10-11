//! GameCube GVR textures (`GVRT` chunks), as used by SA2 PC models.
//!
//! Header after the 8-byte `GVRT`+length: u16 pad, u8 flags, u8 format,
//! u16 BE width, u16 BE height. Pixel data is tiled in format-specific blocks.

use super::{dxt1_block, expand, rgb565, Rgba};
use crate::bytes::{Bytes, Endian};
use crate::error::{invalid, Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GvrFormat {
    I4 = 0,
    I8 = 1,
    Ia4 = 2,
    Ia8 = 3,
    Rgb565 = 4,
    Rgb5a3 = 5,
    Argb8888 = 6,
    Ci4 = 8,
    Ci8 = 9,
    Cmpr = 14,
}

impl GvrFormat {
    pub fn from_u8(v: u8) -> Option<Self> {
        use GvrFormat::*;
        Some(match v {
            0 => I4,
            1 => I8,
            2 => Ia4,
            3 => Ia8,
            4 => Rgb565,
            5 => Rgb5a3,
            6 => Argb8888,
            8 => Ci4,
            9 => Ci8,
            14 => Cmpr,
            _ => return None,
        })
    }

    /// Block width, block height, bits per pixel.
    fn block(self) -> (u32, u32, u32) {
        use GvrFormat::*;
        match self {
            I4 | Ci4 | Cmpr => (8, 8, 4),
            I8 | Ia4 | Ci8 => (8, 4, 8),
            Ia8 | Rgb565 | Rgb5a3 => (4, 4, 16),
            Argb8888 => (4, 4, 32),
        }
    }
}

/// Parses a `GVRT` chunk (starting at its magic) and decodes the top mip level.
pub fn decode(chunk: &[u8]) -> Result<Rgba> {
    let b = Bytes::new(chunk, Endian::Big);
    let magic = b.slice(0, 4)?;
    if magic != b"GVRT" {
        return Err(Error::BadMagic { expected: "GVRT", found: magic.to_vec() });
    }
    let fmt_byte = b.u8(11)?;
    let format = GvrFormat::from_u8(fmt_byte).ok_or_else(|| invalid(11, format!("unknown GVR format {fmt_byte}")))?;
    let width = b.u16(12)? as u32;
    let height = b.u16(14)? as u32;
    decode_pixels(format, width, height, b.slice(16, chunk.len().saturating_sub(16))?)
}

/// Decodes raw tiled pixel data.
pub fn decode_pixels(format: GvrFormat, width: u32, height: u32, data: &[u8]) -> Result<Rgba> {
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err(invalid(0, format!("bad GVR size {width}x{height}")));
    }
    if matches!(format, GvrFormat::Ci4 | GvrFormat::Ci8) {
        return Err(invalid(0, "palettised GVR textures are not supported"));
    }
    let b = Bytes::new(data, Endian::Big);
    let (bw, bh, bpp) = format.block();
    let block_bytes = (bw * bh * bpp / 8) as usize;
    let mut img = Rgba::new(width, height);
    let mut pos = 0usize;
    for by in (0..height).step_by(bh as usize) {
        for bx in (0..width).step_by(bw as usize) {
            match format {
                GvrFormat::Cmpr => {
                    // 8x8 block = four DXT1 sub-blocks in Z order.
                    for sub in 0..4u32 {
                        let o = pos + sub as usize * 8;
                        let c0 = b.u16(o)?;
                        let c1 = b.u16(o + 2)?;
                        let bits = b.u32(o + 4)?;
                        let px = dxt1_block(c0, c1, |i| ((bits >> (30 - 2 * i)) & 3) as usize, true);
                        let (sx, sy) = (bx + (sub % 2) * 4, by + (sub / 2) * 4);
                        for (i, p) in px.iter().enumerate() {
                            img.put(sx + i as u32 % 4, sy + i as u32 / 4, *p);
                        }
                    }
                }
                GvrFormat::Argb8888 => {
                    for i in 0..16u32 {
                        let ar = pos + i as usize * 2;
                        let gb = ar + 32;
                        let p = [b.u8(ar + 1)?, b.u8(gb)?, b.u8(gb + 1)?, b.u8(ar)?];
                        img.put(bx + i % 4, by + i / 4, p);
                    }
                }
                _ => {
                    for i in 0..bw * bh {
                        let p = texel(format, &b, pos, i)?;
                        img.put(bx + i % bw, by + i / bw, p);
                    }
                }
            }
            pos += block_bytes;
        }
    }
    Ok(img)
}

fn texel(format: GvrFormat, b: &Bytes, block: usize, i: u32) -> Result<[u8; 4]> {
    use GvrFormat::*;
    Ok(match format {
        I4 => {
            let byte = b.u8(block + i as usize / 2)?;
            let v = if i % 2 == 0 { byte >> 4 } else { byte & 15 };
            let l = expand(v as u32, 4);
            [l, l, l, 255]
        }
        I8 => {
            let l = b.u8(block + i as usize)?;
            [l, l, l, 255]
        }
        Ia4 => {
            let byte = b.u8(block + i as usize)?;
            let l = expand((byte & 15) as u32, 4);
            [l, l, l, expand((byte >> 4) as u32, 4)]
        }
        Ia8 => {
            let a = b.u8(block + i as usize * 2)?;
            let l = b.u8(block + i as usize * 2 + 1)?;
            [l, l, l, a]
        }
        Rgb565 => rgb565(b.u16(block + i as usize * 2)?),
        Rgb5a3 => {
            let c = b.u16(block + i as usize * 2)? as u32;
            if c & 0x8000 != 0 {
                [expand((c >> 10) & 31, 5), expand((c >> 5) & 31, 5), expand(c & 31, 5), 255]
            } else {
                [expand((c >> 8) & 15, 4), expand((c >> 4) & 15, 4), expand(c & 15, 4), expand((c >> 12) & 7, 3)]
            }
        }
        Argb8888 | Cmpr | Ci4 | Ci8 => unreachable!("handled by caller"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(format: u8, w: u16, h: u16, data: &[u8]) -> Vec<u8> {
        let mut c = b"GVRT".to_vec();
        c.extend_from_slice(&((8 + data.len()) as u32).to_le_bytes());
        c.extend_from_slice(&[0, 0, 0, format]);
        c.extend_from_slice(&w.to_be_bytes());
        c.extend_from_slice(&h.to_be_bytes());
        c.extend_from_slice(data);
        c
    }

    #[test]
    fn rgb565_tiles_4x4() {
        // 8x4 image = two 4x4 blocks: first all red, second all blue.
        let mut data = Vec::new();
        data.extend(std::iter::repeat([0xF8, 0x00]).take(16).flatten());
        data.extend(std::iter::repeat([0x00, 0x1F]).take(16).flatten());
        let img = decode(&chunk(4, 8, 4, &data)).unwrap();
        assert_eq!(img.get(0, 0), [255, 0, 0, 255]);
        assert_eq!(img.get(3, 3), [255, 0, 0, 255]);
        assert_eq!(img.get(4, 0), [0, 0, 255, 255]);
    }

    #[test]
    fn argb8888_split_planes() {
        let mut data = vec![0u8; 64];
        data[0] = 0x80; // A of pixel 0
        data[1] = 0x11; // R
        data[32] = 0x22; // G
        data[33] = 0x33; // B
        let img = decode(&chunk(6, 4, 4, &data)).unwrap();
        assert_eq!(img.get(0, 0), [0x11, 0x22, 0x33, 0x80]);
    }

    #[test]
    fn cmpr_sub_block_order() {
        // 8x8: sub-blocks TL red, TR green, BL blue, BR white; all selector 0.
        let mut data = Vec::new();
        for c in [0xF800u16, 0x07E0, 0x001F, 0xFFFF] {
            data.extend_from_slice(&c.to_be_bytes());
            data.extend_from_slice(&0u16.to_be_bytes());
            data.extend_from_slice(&0u32.to_be_bytes());
        }
        let img = decode(&chunk(14, 8, 8, &data)).unwrap();
        assert_eq!(img.get(0, 0), [255, 0, 0, 255]);
        assert_eq!(img.get(7, 0), [0, 255, 0, 255]);
        assert_eq!(img.get(0, 7), [0, 0, 255, 255]);
        assert_eq!(img.get(7, 7), [255, 255, 255, 255]);
    }

    #[test]
    fn rgb5a3_both_modes() {
        let mut data = vec![0u8; 32];
        data[0..2].copy_from_slice(&0xFC00u16.to_be_bytes()); // opaque red
        data[2..4].copy_from_slice(&0x0F00u16.to_be_bytes()); // alpha 0, red 15
        let img = decode(&chunk(5, 4, 4, &data)).unwrap();
        assert_eq!(img.get(0, 0), [255, 0, 0, 255]);
        assert_eq!(img.get(1, 0), [255, 0, 0, 0]);
    }

    #[test]
    fn errors() {
        assert!(decode(b"PVRT0000").is_err());
        assert!(decode(&chunk(7, 4, 4, &[0; 32])).is_err());
        assert!(decode(&chunk(9, 8, 4, &[0; 32])).is_err());
        assert!(matches!(decode(&chunk(4, 8, 8, &[0; 10])), Err(Error::Eof(_))));
    }
}
