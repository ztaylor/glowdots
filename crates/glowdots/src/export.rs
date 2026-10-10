//! Pictures of a player: a PNG of one moment, or an animated GIF. Use it
//! with an offscreen renderer (see [`app::offscreen`](crate::app::offscreen)).

use crate::player::Player;
use crate::render::Renderer;
use anyhow::{Context as _, Result};
use std::path::Path;

/// Play to `at` seconds and save that frame as a PNG.
pub fn png(player: &mut Player, r: &mut Renderer, at: f32, path: &Path) -> Result<()> {
    player.skip(at);
    if player.time() == 0.0 {
        player.step(1.0 / 60.0);
    }
    let img = player.capture(r)?;
    img.save(path).with_context(|| format!("saving {}", path.display()))
}

/// Record `seconds` of play, starting now, as a looping animated GIF at
/// `fps` frames per second (25 or 50 keep GIF timing exact).
#[cfg(feature = "gif")]
pub fn gif(player: &mut Player, r: &mut Renderer, seconds: f32, fps: u32, path: &Path) -> Result<()> {
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Delay, Frame};
    let fps = fps.clamp(1, 50);
    // Step at least 50 times a second, so the afterglow stays smooth.
    let substeps = 50u32.div_ceil(fps).max(1);
    let dt = 1.0 / (fps * substeps) as f32;
    let file = std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?;
    let mut gif = GifEncoder::new_with_speed(std::io::BufWriter::new(file), 10);
    gif.set_repeat(Repeat::Infinite)?;
    let delay = Delay::from_numer_denom_ms(1000, fps);
    for _ in 0..(seconds * fps as f32).round() as u32 {
        for _ in 0..substeps {
            player.step(dt);
        }
        gif.encode_frame(Frame::from_parts(player.capture(r)?, 0, 0, delay))?;
    }
    Ok(())
}
