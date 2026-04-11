use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct JavaRng {
    seed: i64,
}

impl JavaRng {
    pub fn new(seed: i64) -> Self {
        Self {
            seed: (seed ^ 0x5DEECE66D) & ((1i64 << 48) - 1),
        }
    }

    pub fn new_from_random_seed() -> Self {
        Self {
            seed: JavaRngSupport::generate_unique_seed(),
        }
    }

    pub fn next(&mut self, bits: u32) -> i32 {
        self.seed = (self.seed.wrapping_mul(0x5DEECE66D).wrapping_add(0xB)) & ((1i64 << 48) - 1);
        (self.seed >> (48 - bits)) as i32
    }

    pub fn next_int(&mut self, bound: i32) -> i32 {
        if bound & (bound - 1) == 0 {
            return ((bound as i64 * self.next(31) as i64) >> 31) as i32;
        }
        loop {
            let bits = self.next(31);
            let val = bits % bound;
            if bits - val + (bound - 1) >= 0 {
                return val;
            }
        }
    }

    pub fn next_double(&mut self) -> f64 {
        let hi = self.next(26) as i64;
        let lo = self.next(27) as i64;
        ((hi << 27) + lo) as f64 / ((1i64 << 53) as f64)
    }

    pub fn next_float(&mut self) -> f32 {
        self.next(24) as f32 / (1u32 << 24) as f32
    }
}

pub struct JavaRngSupport;

impl JavaRngSupport {
    pub const GOLDEN_RATIO_64: i64 = -7046029254386353131;
    pub const SILVER_RATIO_64: i64 = 7640891576956012809;
    const SEED_UNIQUIFIER_INITIAL: u64 = 8682522807148012;
    const SEED_MULTIPLIER: u64 = 1181783497276652981;

    #[inline]
    pub fn mix_stafford13(mut z: i64) -> i64 {
        z = (z ^ ((z as u64 >> 30) as i64)).wrapping_mul(-4658895280553007687);
        z = (z ^ ((z as u64 >> 27) as i64)).wrapping_mul(-7723592293110705685);
        z ^ ((z as u64 >> 31) as i64)
    }

    #[inline]
    pub fn upgrade_seed_to_128bit_unmixed(legacy_seed: i64) -> Seed128bit {
        let low_bits = legacy_seed ^ Self::SILVER_RATIO_64;
        let high_bits = low_bits.wrapping_add(Self::GOLDEN_RATIO_64);
        Seed128bit::new(low_bits, high_bits)
    }

    #[inline]
    pub fn upgrade_seed_to_128bit(legacy_seed: i64) -> Seed128bit {
        Self::upgrade_seed_to_128bit_unmixed(legacy_seed).mixed()
    }

    pub fn seed_from_hash_of(input: &str) -> Seed128bit {
        let digest = md5::compute(input.as_bytes());
        let bytes = digest.0;

        let hash_lo = i64::from_be_bytes(bytes[0..8].try_into().expect("slice size is 8"));
        let hash_hi = i64::from_be_bytes(bytes[8..16].try_into().expect("slice size is 8"));

        Seed128bit::new(hash_lo, hash_hi)
    }

    pub fn generate_unique_seed() -> i64 {
        static SEED_UNIQUIFIER: AtomicU64 = AtomicU64::new(JavaRngSupport::SEED_UNIQUIFIER_INITIAL);

        let updated = SEED_UNIQUIFIER
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                Some(current.wrapping_mul(JavaRngSupport::SEED_MULTIPLIER))
            })
            .map(|old| old.wrapping_mul(JavaRngSupport::SEED_MULTIPLIER))
            .unwrap_or_else(|current| current.wrapping_mul(JavaRngSupport::SEED_MULTIPLIER));

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        (updated ^ nanos) as i64
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seed128bit {
    pub seed_lo: i64,
    pub seed_hi: i64,
}

impl Seed128bit {
    #[inline]
    pub const fn new(seed_lo: i64, seed_hi: i64) -> Self {
        Self { seed_lo, seed_hi }
    }

    #[inline]
    pub fn xor(self, lo: i64, hi: i64) -> Self {
        Self {
            seed_lo: self.seed_lo ^ lo,
            seed_hi: self.seed_hi ^ hi,
        }
    }

    #[inline]
    pub fn xor_seed(self, other: Seed128bit) -> Self {
        self.xor(other.seed_lo, other.seed_hi)
    }

    #[inline]
    pub fn mixed(self) -> Self {
        Self {
            seed_lo: JavaRngSupport::mix_stafford13(self.seed_lo),
            seed_hi: JavaRngSupport::mix_stafford13(self.seed_hi),
        }
    }
}
