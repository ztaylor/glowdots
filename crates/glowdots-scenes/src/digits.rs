//! Big, smooth digits (10x18 dots) for the clock and the weather, drawn by
//! `tools/big_digits.py` at 8 px per dot and shaded by ink coverage.

use anyhow::Result;
use glowdots::Sprite;

/// The digit PNGs, drawn by tools/big_digits.py at 8 px per dot.
const DIGITS: [&[u8]; 10] = [
    include_bytes!("../assets/fonts/digits/0.png"),
    include_bytes!("../assets/fonts/digits/1.png"),
    include_bytes!("../assets/fonts/digits/2.png"),
    include_bytes!("../assets/fonts/digits/3.png"),
    include_bytes!("../assets/fonts/digits/4.png"),
    include_bytes!("../assets/fonts/digits/5.png"),
    include_bytes!("../assets/fonts/digits/6.png"),
    include_bytes!("../assets/fonts/digits/7.png"),
    include_bytes!("../assets/fonts/digits/8.png"),
    include_bytes!("../assets/fonts/digits/9.png"),
];
const COLON: &[u8] = include_bytes!("../assets/fonts/digits/colon.png");

/// The digits 0 to 9.
pub fn big() -> Result<Vec<Sprite>> {
    DIGITS.iter().map(|png| Sprite::art_from_bytes(png)).collect()
}

/// The clock's colon.
pub fn colon() -> Result<Sprite> {
    Sprite::art_from_bytes(COLON)
}
