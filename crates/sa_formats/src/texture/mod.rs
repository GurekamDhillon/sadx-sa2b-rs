//! Textures: decoded RGBA8 images plus the console and PC texture formats.

pub mod archive;
pub mod dds;
pub mod gvr;
pub mod pvr;

/// A decoded image: `width * height` pixels, RGBA8, rows top to bottom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Rgba {
    pub fn new(width: u32, height: u32) -> Self {
        Rgba { width, height, pixels: vec![0; (width * height * 4) as usize] }
    }

    pub fn put(&mut self, x: u32, y: u32, p: [u8; 4]) {
        if x < self.width && y < self.height {
            let i = ((y * self.width + x) * 4) as usize;
            self.pixels[i..i + 4].copy_from_slice(&p);
        }
    }

    pub fn get(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        self.pixels[i..i + 4].try_into().unwrap()
    }
}

pub(crate) fn expand(v: u32, bits: u32) -> u8 {
    // Replicate high bits into low bits: 5-bit 31 -> 255, 0 -> 0.
    let v = v << (8 - bits);
    (v | (v >> bits)) as u8
}

pub(crate) fn rgb565(c: u16) -> [u8; 4] {
    [expand((c >> 11) as u32 & 31, 5), expand((c >> 5) as u32 & 63, 6), expand(c as u32 & 31, 5), 255]
}

/// Decodes one 4x4 DXT1-style colour block given two 565 endpoints and a
/// per-pixel 2-bit selector function. Returns 16 RGBA pixels in raster order.
pub(crate) fn dxt1_block(c0: u16, c1: u16, selector: impl Fn(usize) -> usize, dxt1_alpha: bool) -> [[u8; 4]; 16] {
    let a = rgb565(c0);
    let b = rgb565(c1);
    let mix = |wa: u32, wb: u32, d: u32| -> [u8; 4] {
        let f = |i: usize| ((a[i] as u32 * wa + b[i] as u32 * wb) / d) as u8;
        [f(0), f(1), f(2), 255]
    };
    let palette = if c0 > c1 || !dxt1_alpha {
        [a, b, mix(2, 1, 3), mix(1, 2, 3)]
    } else {
        [a, b, mix(1, 1, 2), [0, 0, 0, 0]]
    };
    let mut out = [[0u8; 4]; 16];
    for (i, px) in out.iter_mut().enumerate() {
        *px = palette[selector(i)];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_full_range() {
        assert_eq!(expand(31, 5), 255);
        assert_eq!(expand(0, 5), 0);
        assert_eq!(expand(63, 6), 255);
        assert_eq!(expand(15, 4), 255);
    }

    #[test]
    fn rgb565_primaries() {
        assert_eq!(rgb565(0xF800), [255, 0, 0, 255]);
        assert_eq!(rgb565(0x07E0), [0, 255, 0, 255]);
        assert_eq!(rgb565(0x001F), [0, 0, 255, 255]);
    }

    #[test]
    fn dxt1_transparent_mode() {
        // c0 <= c1 selects 3-colour + transparent palette.
        let px = dxt1_block(0x0000, 0xFFFF, |_| 3, true);
        assert_eq!(px[0], [0, 0, 0, 0]);
        let px = dxt1_block(0xFFFF, 0x0000, |_| 0, true);
        assert_eq!(px[5], [255, 255, 255, 255]);
    }
}
