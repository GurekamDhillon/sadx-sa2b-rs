//! DirectDraw Surface textures, found inside SA2 PC `.pak` archives.
//!
//! Supports DXT1, DXT3, DXT5 and uncompressed 32/24/16-bit masked formats.

use super::{dxt1_block, Rgba};
use crate::bytes::{Bytes, Endian};
use crate::error::{invalid, Error, Result};

const DDPF_FOURCC: u32 = 0x4;
const DDPF_ALPHAPIXELS: u32 = 0x1;

pub fn decode(data: &[u8]) -> Result<Rgba> {
    let b = Bytes::new(data, Endian::Little);
    let magic = b.slice(0, 4)?;
    if magic != b"DDS " {
        return Err(Error::BadMagic { expected: "DDS ", found: magic.to_vec() });
    }
    let height = b.u32(12)?;
    let width = b.u32(16)?;
    if width == 0 || height == 0 || width > 8192 || height > 8192 {
        return Err(invalid(12, format!("bad DDS size {width}x{height}")));
    }
    let pf_flags = b.u32(80)?;
    let fourcc = b.slice(84, 4)?;
    let px = Bytes::new(b.slice(128, data.len().saturating_sub(128))?, Endian::Little);
    let mut img = Rgba::new(width, height);
    if pf_flags & DDPF_FOURCC != 0 {
        let block_size = match fourcc {
            b"DXT1" => 8,
            b"DXT3" | b"DXT5" => 16,
            _ => return Err(invalid(84, format!("unsupported DDS fourcc {:?}", String::from_utf8_lossy(fourcc)))),
        };
        let bw = width.div_ceil(4);
        let bh = height.div_ceil(4);
        for by in 0..bh {
            for bx in 0..bw {
                let o = ((by * bw + bx) * block_size) as usize;
                let colour = if block_size == 16 { o + 8 } else { o };
                let c0 = px.u16(colour)?;
                let c1 = px.u16(colour + 2)?;
                let bits = px.u32(colour + 4)?;
                let mut pixels = dxt1_block(c0, c1, |i| ((bits >> (2 * i)) & 3) as usize, block_size == 8);
                match fourcc {
                    b"DXT3" => {
                        let alpha = u64::from_le_bytes(px.slice(o, 8)?.try_into().unwrap());
                        for (i, p) in pixels.iter_mut().enumerate() {
                            p[3] = (((alpha >> (4 * i)) & 15) * 17) as u8;
                        }
                    }
                    b"DXT5" => {
                        let a0 = px.u8(o)? as u32;
                        let a1 = px.u8(o + 1)? as u32;
                        let mut idx = 0u64;
                        for k in 0..6 {
                            idx |= (px.u8(o + 2 + k)? as u64) << (8 * k);
                        }
                        let table: [u32; 8] = if a0 > a1 {
                            [a0, a1, (6 * a0 + a1) / 7, (5 * a0 + 2 * a1) / 7, (4 * a0 + 3 * a1) / 7, (3 * a0 + 4 * a1) / 7, (2 * a0 + 5 * a1) / 7, (a0 + 6 * a1) / 7]
                        } else {
                            [a0, a1, (4 * a0 + a1) / 5, (3 * a0 + 2 * a1) / 5, (2 * a0 + 3 * a1) / 5, (a0 + 4 * a1) / 5, 0, 255]
                        };
                        for (i, p) in pixels.iter_mut().enumerate() {
                            p[3] = table[((idx >> (3 * i)) & 7) as usize] as u8;
                        }
                    }
                    _ => {}
                }
                for (i, p) in pixels.iter().enumerate() {
                    img.put(bx * 4 + i as u32 % 4, by * 4 + i as u32 / 4, *p);
                }
            }
        }
    } else {
        let bits = b.u32(88)?;
        let masks = [b.u32(92)?, b.u32(96)?, b.u32(100)?, if pf_flags & DDPF_ALPHAPIXELS != 0 { b.u32(104)? } else { 0 }];
        let bpp = match bits {
            16 | 24 | 32 => bits as usize / 8,
            _ => return Err(invalid(88, format!("unsupported DDS bit count {bits}"))),
        };
        for y in 0..height {
            for x in 0..width {
                let o = ((y * width + x) as usize) * bpp;
                let mut v = 0u32;
                for k in 0..bpp {
                    v |= (px.u8(o + k)? as u32) << (8 * k);
                }
                let mut p = [0u8; 4];
                for (c, &m) in masks.iter().enumerate() {
                    p[c] = if m == 0 { 255 } else { scale_mask(v, m) };
                }
                img.put(x, y, p);
            }
        }
    }
    Ok(img)
}

fn scale_mask(v: u32, mask: u32) -> u8 {
    let shift = mask.trailing_zeros();
    let max = mask >> shift;
    (((v & mask) >> shift) * 255 / max) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(w: u32, h: u32, flags: u32, fourcc: &[u8; 4], bits: u32, masks: [u32; 4]) -> Vec<u8> {
        let mut d = vec![0u8; 128];
        d[0..4].copy_from_slice(b"DDS ");
        d[12..16].copy_from_slice(&h.to_le_bytes());
        d[16..20].copy_from_slice(&w.to_le_bytes());
        d[80..84].copy_from_slice(&flags.to_le_bytes());
        d[84..88].copy_from_slice(fourcc);
        d[88..92].copy_from_slice(&bits.to_le_bytes());
        for (i, m) in masks.iter().enumerate() {
            d[92 + i * 4..96 + i * 4].copy_from_slice(&m.to_le_bytes());
        }
        d
    }

    #[test]
    fn bgra32() {
        let mut d = header(1, 1, 0x41, b"\0\0\0\0", 32, [0xFF0000, 0xFF00, 0xFF, 0xFF000000]);
        d.extend_from_slice(&[0x33, 0x22, 0x11, 0x80]);
        assert_eq!(decode(&d).unwrap().get(0, 0), [0x11, 0x22, 0x33, 0x80]);
    }

    #[test]
    fn dxt1_solid() {
        let mut d = header(4, 4, DDPF_FOURCC, b"DXT1", 0, [0; 4]);
        d.extend_from_slice(&0xF800u16.to_le_bytes());
        d.extend_from_slice(&0u16.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(decode(&d).unwrap().get(3, 3), [255, 0, 0, 255]);
    }

    #[test]
    fn dxt5_alpha() {
        let mut d = header(4, 4, DDPF_FOURCC, b"DXT5", 0, [0; 4]);
        d.extend_from_slice(&[200, 100, 1, 0, 0, 0, 0, 0]); // pixel 0 -> a1
        d.extend_from_slice(&0xFFFFu16.to_le_bytes());
        d.extend_from_slice(&0u16.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        let img = decode(&d).unwrap();
        assert_eq!(img.get(0, 0)[3], 100);
        assert_eq!(img.get(1, 0)[3], 200);
    }

    #[test]
    fn rejects_unknown() {
        assert!(decode(b"XXXX").is_err());
        let d = header(4, 4, DDPF_FOURCC, b"ATI2", 0, [0; 4]);
        assert!(decode(&d).is_err());
    }
}
