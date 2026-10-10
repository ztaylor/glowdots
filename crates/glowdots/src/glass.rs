//! The glass: how dots look on the screen. A [`Look`](crate::Look) says
//! what colour a scene's dots are; the glass is the display itself: the
//! size and shape of its dots, how they glow, shimmer, and fade.

/// Settings for how the display draws its dots. [`Glass::CLASSIC`] is a
/// 1990s plasma panel.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Glass {
    /// Dot radius, in dots: 0.5 makes neighbours touch.
    pub dot_size: f32,
    /// 1 for round plasma dots, 0 for square LED pixels, or between.
    pub roundness: f32,
    /// The tight glow around each lit dot.
    pub halo: f32,
    /// How far the tight glow reaches, in dots.
    pub halo_width: f32,
    /// How visible unlit dots are (0 = black).
    pub unlit: f32,
    /// Dot-to-dot difference in brightness, like an old panel (0 = none).
    pub variation: f32,
    /// Fast shimmer of the plasma (0 = steady).
    pub flicker: f32,
    /// Spreads the shades apart; higher makes dim shades dimmer.
    pub gamma: f32,
    /// How far bright dot centres lean to white (0 = not at all).
    pub hot_core: f32,
    /// The wide, soft glow around everything (0 = none).
    pub bloom: f32,
    /// How long a dot takes to go dark after it goes off, in seconds.
    pub afterglow: f32,
}

impl Glass {
    /// A 1990s plasma panel: round orange-hot dots, a soft glow, a little
    /// shimmer, and a quick afterglow.
    pub const CLASSIC: Glass = Glass {
        dot_size: 0.43,
        roundness: 1.0,
        halo: 0.12,
        halo_width: 0.45,
        unlit: 0.065,
        variation: 0.18,
        flicker: 0.05,
        gamma: 1.6,
        hot_core: 0.6,
        bloom: 0.8,
        afterglow: 0.05,
    };

    /// An LED replacement panel: crisp rounded-square pixels, steady, with
    /// little glow and almost no afterglow.
    pub const LED: Glass = Glass {
        dot_size: 0.42,
        roundness: 0.35,
        halo: 0.05,
        halo_width: 0.35,
        unlit: 0.035,
        variation: 0.03,
        flicker: 0.0,
        gamma: 1.3,
        hot_core: 0.25,
        bloom: 0.45,
        afterglow: 0.008,
    };

    /// A tired plasma panel in a dark arcade: uneven dots, more shimmer,
    /// a big glow, and a long afterglow.
    pub const WORN: Glass = Glass {
        dot_size: 0.44,
        roundness: 1.0,
        halo: 0.17,
        halo_width: 0.55,
        unlit: 0.09,
        variation: 0.38,
        flicker: 0.1,
        gamma: 1.7,
        hot_core: 0.5,
        bloom: 1.15,
        afterglow: 0.11,
    };
}

impl Default for Glass {
    fn default() -> Self {
        Glass::CLASSIC
    }
}
