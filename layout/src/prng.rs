//! A small seeded generator (splitmix64): no dependency, same numbers on every target.

/// Deterministic pseudo-random numbers.
#[derive(Clone, Debug)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// A generator that starts from `seed`.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The next number in `[0, 1)`.
    pub fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        // The top 53 bits, exactly representable: a uniform number in [0, 1).
        #[allow(clippy::cast_precision_loss, reason = "a 53-bit value is exact in f64")]
        let unit = (z >> 11) as f64 / 9_007_199_254_740_992.0;
        unit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_numbers() {
        let mut a = SplitMix64::new(42);
        let mut b = SplitMix64::new(42);
        let xs: Vec<f64> = (0..8).map(|_| a.next_f64()).collect();
        let ys: Vec<f64> = (0..8).map(|_| b.next_f64()).collect();
        assert_eq!(xs, ys);
        assert!(xs.iter().all(|x| (0.0..1.0).contains(x)));
        assert!(xs.windows(2).any(|w| w[0].to_bits() != w[1].to_bits()));
    }

    #[test]
    fn another_seed_other_numbers() {
        let mut a = SplitMix64::new(1);
        let mut b = SplitMix64::new(2);
        assert_ne!(a.next_f64().to_bits(), b.next_f64().to_bits());
    }

    #[test]
    fn known_first_value() {
        // splitmix64(0): 0xE220A8397B1DCDAF, as the reference implementation gives.
        let mut g = SplitMix64::new(0);
        let x = g.next_f64();
        #[allow(clippy::cast_precision_loss, reason = "a 53-bit value is exact in f64")]
        let expected = (0xE220_A839_7B1D_CDAF_u64 >> 11) as f64 / 9_007_199_254_740_992.0;
        assert_eq!(x, expected);
        // The mean of many draws is near one half.
        let mean = (0..10_000).map(|_| g.next_f64()).sum::<f64>() / 10_000.0;
        assert!((mean - 0.5).abs() < 0.02, "{mean}");
    }
}
