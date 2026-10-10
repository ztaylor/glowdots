//! Small helpers: random numbers, hashing, and easing.

/// Small xorshift random numbers, so scenes need no extra crate. Seed it
/// with any non-zero number.
pub struct Rng(pub u32);

impl Rng {
    /// A random number in 0..1.
    pub fn next_f32(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1 << 24) as f32
    }

    /// A random 32-bit number.
    pub fn next_u32(&mut self) -> u32 {
        self.next_f32();
        self.0
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_f32()
    }
}

/// A fixed pseudo-random value in 0..1 for a grid position. The same
/// (x, y) always gives the same value, so patterns hold still.
pub fn hash(x: u32, y: u32) -> f32 {
    let mut h = x.wrapping_mul(0x9e37_79b1) ^ y.wrapping_mul(0x85eb_ca77);
    h ^= h >> 15;
    h = h.wrapping_mul(0xc2b2_ae3d);
    h ^= h >> 13;
    (h & 0xffff) as f32 / 65536.0
}

pub fn ease_out(p: f32) -> f32 {
    let p = p.clamp(0.0, 1.0);
    1.0 - (1.0 - p) * (1.0 - p) * (1.0 - p)
}

pub fn ease_in(p: f32) -> f32 {
    let p = p.clamp(0.0, 1.0);
    p * p * p
}

pub fn ease_in_out(p: f32) -> f32 {
    let p = p.clamp(0.0, 1.0);
    if p < 0.5 { 4.0 * p * p * p } else { 1.0 - (-2.0 * p + 2.0).powi(3) / 2.0 }
}
