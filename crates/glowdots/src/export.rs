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
/// `fps` frames per second (20, 25, or 50 keep GIF timing exact). The
/// frames are kept in memory until the GIF is written.
///
/// The GIF has one palette, learned from the first frame, and each frame
/// stores only the pixels that changed visibly (glow that shifts by a
/// shade or two is left as it was), so calm scenes stay small. The plasma
/// shimmer changes dots every frame; set the glass's `flicker` to 0 for
/// the smallest files.
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

    // Play the whole clip first, so the palette can learn from all of it.
    let count = (seconds * fps as f32).round().max(1.0) as usize;
    let mut frames = Vec::with_capacity(count);
    for _ in 0..count {
        step(player);
        frames.push(player.capture(r)?);
    }
    let (w, h) = frames[0].dimensions();
    let (w16, h16) = (u16::try_from(w)?, u16::try_from(h)?);
    // 255 colours learned from up to 12 frames across the clip; index 255
    // means "unchanged".
    let picks = count.min(12);
    let sample: Vec<u8> = (0..picks).flat_map(|k| frames[k * count / picks].as_raw().iter().copied()).collect();
    let nq = NeuQuant::new(10, 255, &sample);
    let mut palette = nq.color_map_rgb();
    palette.resize(256 * 3, 0);
    const SAME: u8 = 255;

    let file = std::io::BufWriter::new(std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?);
    let mut gif = gif::Encoder::new(file, w16, h16, &palette)?;
    gif.set_repeat(gif::Repeat::Infinite)?;
    let delay = (100 / fps) as u16; // centiseconds
    // What a viewer sees at each pixel now, as palette indexes.
    let mut prev: Option<Vec<u8>> = None;
    let color = |i: u8| [palette[i as usize * 3] as i32, palette[i as usize * 3 + 1] as i32, palette[i as usize * 3 + 2] as i32];
    // Pixels closer than this (summed over the channels) count as unchanged.
    const TOLERANCE: i32 = 24;
    for img in &frames {
        let idx: Vec<u8> = img.as_raw().chunks_exact(4).map(|p| nq.index_of(p) as u8).collect();
        let frame = match &mut prev {
            None => gif::Frame { width: w16, height: h16, buffer: Cow::Borrowed(&idx), delay, ..Default::default() },
            Some(prev) => {
                // Only the box around the pixels that changed visibly, and
                // inside it, "unchanged" wherever the pixel looks the same.
                let changed: Vec<bool> = idx
                    .iter()
                    .zip(prev.iter())
                    .map(|(&n, &o)| {
                        let (a, b) = (color(n), color(o));
                        (a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs() > TOLERANCE
                    })
                    .collect();
                for (shown, (&n, &c)) in prev.iter_mut().zip(idx.iter().zip(&changed)) {
                    if c {
                        *shown = n;
                    }
                }
                let changed = |x: u32, y: u32| changed[(y * w + x) as usize];
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
        if prev.is_none() {
            prev = Some(idx);
        }
    }
    Ok(())
}
