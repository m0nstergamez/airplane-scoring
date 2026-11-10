//! A tiny seeded random number generator (SplitMix64).
//! Core never asks the platform for randomness: the game passes a seed in,
//! so the same seed always produces the same weather.

#[derive(Debug, Clone, PartialEq)]
pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        // 24 bits: every value is exactly representable in an f32.
        (self.next_u64() >> 40) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in [lo, hi).
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_gives_the_same_numbers() {
        let (mut a, mut b) = (Rng::new(7), Rng::new(7));
        for _ in 0..100 {
            assert_eq!(a.unit(), b.unit());
        }
    }

    #[test]
    fn different_seeds_give_different_numbers() {
        let (mut a, mut b) = (Rng::new(1), Rng::new(2));
        assert!((0..10).any(|_| a.unit() != b.unit()));
    }

    #[test]
    fn numbers_stay_in_range_and_spread_out() {
        let mut r = Rng::new(42);
        let mut sum = 0.0;
        for _ in 0..10_000 {
            let v = r.range(2.0, 4.0);
            assert!((2.0..4.0).contains(&v), "{v}");
            sum += v;
        }
        let mean = sum / 10_000.0;
        assert!((mean - 3.0).abs() < 0.05, "mean {mean}");
    }
}
