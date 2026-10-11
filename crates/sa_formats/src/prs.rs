//! PRS: Sega's LZ77 variant used for most SA1/SA2 files.
//!
//! The stream interleaves little-endian control bytes (read LSB first) with
//! data bytes. Control bits select a literal, a short copy (offset within
//! 256 bytes, length 2-5) or a long copy (offset within 8191 bytes, length
//! 3-256). A long copy with a zero word ends the stream.

use crate::error::{Error, Result};

const LONG_WINDOW: usize = 0x1FFF;
const SHORT_WINDOW: usize = 0xFF;
const MAX_LEN: usize = 0x100;

struct Reader<'a> {
    src: &'a [u8],
    pos: usize,
    ctrl: u8,
    bits_left: u8,
}

impl<'a> Reader<'a> {
    fn byte(&mut self) -> Result<u8> {
        let b = *self.src.get(self.pos).ok_or(Error::Eof(self.pos))?;
        self.pos += 1;
        Ok(b)
    }

    fn bit(&mut self) -> Result<bool> {
        if self.bits_left == 0 {
            self.ctrl = self.byte()?;
            self.bits_left = 8;
        }
        let b = self.ctrl & 1 != 0;
        self.ctrl >>= 1;
        self.bits_left -= 1;
        Ok(b)
    }
}

/// Decompress a PRS stream.
pub fn decompress(src: &[u8]) -> Result<Vec<u8>> {
    let mut r = Reader { src, pos: 0, ctrl: 0, bits_left: 0 };
    let mut out: Vec<u8> = Vec::with_capacity(src.len() * 4);
    loop {
        if r.bit()? {
            out.push(r.byte()?);
            continue;
        }
        let (back, len) = if r.bit()? {
            let lo = r.byte()? as usize;
            let hi = r.byte()? as usize;
            let word = lo | (hi << 8);
            if word == 0 {
                return Ok(out);
            }
            let back = 0x2000 - (word >> 3);
            let len = match word & 7 {
                0 => r.byte()? as usize + 1,
                n => n + 2,
            };
            (back, len)
        } else {
            let len = ((r.bit()? as usize) << 1 | r.bit()? as usize) + 2;
            let back = 0x100 - r.byte()? as usize;
            (back, len)
        };
        if back > out.len() {
            return Err(crate::error::invalid(r.pos, format!("copy reaches {back} bytes back with only {} output", out.len())));
        }
        let start = out.len() - back;
        for i in 0..len {
            out.push(out[start + i]);
        }
    }
}

struct Writer {
    out: Vec<u8>,
    ctrl_pos: usize,
    bit: u8,
}

impl Writer {
    fn new() -> Self {
        Writer { out: vec![0], ctrl_pos: 0, bit: 0 }
    }

    fn put_bit(&mut self, b: bool) {
        if self.bit == 8 {
            self.ctrl_pos = self.out.len();
            self.out.push(0);
            self.bit = 0;
        }
        if b {
            self.out[self.ctrl_pos] |= 1 << self.bit;
        }
        self.bit += 1;
    }
}

/// Compress data to a PRS stream (greedy longest match).
pub fn compress(src: &[u8]) -> Vec<u8> {
    let mut w = Writer::new();
    let mut i = 0;
    while i < src.len() {
        let (back, len) = longest_match(src, i);
        if len >= 2 && back <= SHORT_WINDOW && len <= 5 {
            w.put_bit(false);
            w.put_bit(false);
            let n = len - 2;
            w.put_bit(n & 2 != 0);
            w.put_bit(n & 1 != 0);
            w.out.push((0x100 - back) as u8);
            i += len;
        } else if len >= 3 {
            w.put_bit(false);
            w.put_bit(true);
            let off = (0x2000 - back) << 3;
            if len <= 9 {
                let word = off | (len - 2);
                w.out.extend_from_slice(&(word as u16).to_le_bytes());
            } else {
                w.out.extend_from_slice(&(off as u16).to_le_bytes());
                w.out.push((len - 1) as u8);
            }
            i += len;
        } else {
            w.put_bit(true);
            w.out.push(src[i]);
            i += 1;
        }
    }
    w.put_bit(false);
    w.put_bit(true);
    w.out.extend_from_slice(&[0, 0]);
    w.out
}

fn longest_match(src: &[u8], i: usize) -> (usize, usize) {
    let max_len = MAX_LEN.min(src.len() - i);
    let mut best = (0, 0);
    let lo = i.saturating_sub(LONG_WINDOW);
    for start in (lo..i).rev() {
        let mut len = 0;
        while len < max_len && src[start + len] == src[i + len] {
            len += 1;
        }
        if len > best.1 {
            best = (i - start, len);
            if len == max_len {
                break;
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lcg(seed: &mut u32) -> u32 {
        *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        *seed >> 16
    }

    #[test]
    fn empty_round_trips() {
        assert_eq!(decompress(&compress(&[])).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn repetitive_data_round_trips_and_shrinks() {
        let data: Vec<u8> = (0..5000).map(|i| (i % 7) as u8).collect();
        let c = compress(&data);
        assert!(c.len() < data.len() / 10);
        assert_eq!(decompress(&c).unwrap(), data);
    }

    #[test]
    fn random_data_round_trips() {
        let mut seed = 1;
        for size in [1, 2, 3, 255, 256, 257, 9000] {
            // Low-entropy random bytes so all copy kinds occur.
            let data: Vec<u8> = (0..size).map(|_| (lcg(&mut seed) % 4) as u8).collect();
            assert_eq!(decompress(&compress(&data)).unwrap(), data, "size {size}");
        }
    }

    #[test]
    fn hand_built_stream_decodes() {
        // ctrl 0b0100_0011: literal a, literal b, short copy (len 2, back 2),
        // literal c, then a second ctrl byte for the end marker (bits 0,1).
        let stream = [0b0100_0011, b'a', b'b', 0xFE, b'c', 0x01, 0, 0];
        assert_eq!(decompress(&stream).unwrap(), b"ababc");
    }

    #[test]
    fn truncated_stream_errors() {
        let c = compress(b"hello hello hello");
        assert!(matches!(decompress(&c[..c.len() - 2]), Err(Error::Eof(_))));
    }

    #[test]
    fn copy_before_start_errors() {
        // Short copy as first token.
        assert!(matches!(decompress(&[0b0000_0000, 0xFF]), Err(Error::Invalid { .. })));
    }
}
