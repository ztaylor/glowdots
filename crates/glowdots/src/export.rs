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
///
/// The GIF has one palette, learned from the first frame, and each frame
/// stores only the pixels that changed, so calm scenes stay small. The
/// plasma shimmer changes dots every frame; set the glass's `flicker` to 0
/// for the smallest files.
#[cfg(feature = "gif")]
pub fn gif(player: &mut Player, r: &mut Renderer, seconds: f32, fps: u32, path: &Path) -> Result<()> {
    use color_quant::NeuQuant;
    use std::borrow::Cow;

    let fps = fps.clamp(1, 50);
    // Step at least 50 times a second, so the afterglow stays smooth.
    let substeps = 50u32.div_ceil(fps).max(1);
    let dt = 1.0 / (fps * substeps) as f32;
    let step = |player: &mut Player| {
        for _ in 0..substeps {
            player.step(dt);
        }
    };

    step(player);
    let mut img = player.capture(r)?;
    let (w, h) = img.dimensions();
    let (w16, h16) = (u16::try_from(w)?, u16::try_from(h)?);
    // 255 colours from the first frame; index 255 means "unchanged".
    let nq = NeuQuant::new(10, 255, img.as_raw());
    let mut palette = nq.color_map_rgb();
    palette.resize(256 * 3, 0);
    const SAME: u8 = 255;

    let file = std::io::BufWriter::new(std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?);
    let mut gif = gif::Encoder::new(file, w16, h16, &palette)?;
    gif.set_repeat(gif::Repeat::Infinite)?;
    let delay = (100 / fps) as u16; // centiseconds
    let mut prev: Option<Vec<u8>> = None;
    for i in 0..(seconds * fps as f32).round().max(1.0) as u32 {
        if i > 0 {
            step(player);
            img = player.capture(r)?;
        }
        let idx: Vec<u8> = img.as_raw().chunks_exact(4).map(|p| nq.index_of(p) as u8).collect();
        let frame = match &prev {
            None => gif::Frame { width: w16, height: h16, buffer: Cow::Borrowed(&idx), delay, ..Default::default() },
            Some(prev) => {
                // Only the box around the pixels that changed, and inside it,
                // "unchanged" wherever the pixel is the same.
                let changed = |x: u32, y: u32| idx[(y * w + x) as usize] != prev[(y * w + x) as usize];
                let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
                for y in 0..h {
                    for x in 0..w {
                        if changed(x, y) {
                            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
                        }
                    }
                }
                if x0 >= x1 {
                    (x0, y0, x1, y1) = (0, 0, 1, 1);
                }
                let mut buffer = Vec::with_capacity(((x1 - x0) * (y1 - y0)) as usize);
                for y in y0..y1 {
                    for x in x0..x1 {
                        buffer.push(if changed(x, y) { idx[(y * w + x) as usize] } else { SAME });
                    }
                }
                gif::Frame {
                    left: x0 as u16,
                    top: y0 as u16,
                    width: (x1 - x0) as u16,
                    height: (y1 - y0) as u16,
                    buffer: Cow::Owned(buffer),
                    delay,
                    transparent: Some(SAME),
                    dispose: gif::DisposalMethod::Keep,
                    ..Default::default()
                }
            }
        };
        gif.write_frame(&frame)?;
        prev = Some(idx);
    }
    Ok(())
}
