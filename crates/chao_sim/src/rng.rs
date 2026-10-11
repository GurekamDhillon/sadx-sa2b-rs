//! Deterministic random numbers (xorshift32), so a simulation replays
//! exactly from the same seed (e.g. the Chao's own `Seed` field).

#[derive(Debug, Clone)]
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9 } else { seed })
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    /// Uniform integer in `lo..=hi`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next_u32() % (hi - lo + 1) as u32) as i32
    }

    /// Uniform float in `0..1`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_in_range() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..1000 {
            let v = a.range(-2, 2);
            assert_eq!(v, b.range(-2, 2));
            assert!((-2..=2).contains(&v));
            let u = a.unit();
            b.unit();
            assert!((0.0..1.0).contains(&u));
        }
    }
}
