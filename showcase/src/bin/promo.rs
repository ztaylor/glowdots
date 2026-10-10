//! The glowdots promo video, made with glowdots: a timed deck of title
//! cards and the example scenes, with the color, glass, and camera
//! animated, rendered at 1920x1080 and 60 frames a second, and a soundtrack
//! synthesized to the cuts.
//!
//! ```sh
//! cargo run --release --bin promo -- glowdots-promo.mp4     # needs ffmpeg
//! ```
//!
//! It writes the video with sound, and a silent copy next to it.

use anyhow::{Context as _, Result, anyhow, bail};
use glowdots::font::{REGULAR, SMALL};
use glowdots::math::{Rng, ease_in_out};
use glowdots::render::Camera;
use glowdots::{Bundled, Clock, Context, Deck, Frame, Glass, Layered, Look, MAX, Player, Rect, Rgb, Scene, Size, Transition, art, time};
use glowdots_scenes::{SIZE, demo::Demo, samples, showcase};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const FPS: u32 = 60;
const SCREEN: (u32, u32) = (1920, 1080);
/// The music's tempo: every cut lands on a half-second beat.
const BEAT: f32 = 0.5;
const PANEL_W: i32 = SIZE.w as i32;

// ---- The scenes made for the video ------------------------------------

/// A scene shown for a set time, with a set way in.
struct Timed {
    inner: Box<dyn Scene>,
    seconds: f32,
    transition: Transition,
}

impl Scene for Timed {
    fn name(&self) -> &str {
        self.inner.name()
    }
    fn enter(&mut self, cx: &Context) {
        self.inner.enter(cx);
    }
    fn update(&mut self, cx: &Context, out: &mut Frame) {
        self.inner.update(cx, out);
    }
    /// Half a frame early, so the cut lands on the frame nearest `seconds`.
    fn duration(&self) -> f32 {
        self.seconds - 0.5 / FPS as f32
    }
    fn transition(&self) -> Transition {
        self.transition
    }
    fn look(&self) -> Look {
        self.inner.look()
    }
    fn size(&self) -> Option<Size> {
        self.inner.size()
    }
    fn shades(&self) -> u8 {
        self.inner.shades()
    }
}

fn centered(out: &mut Frame, font: &glowdots::font::Font, text: &str, y: i32, level: u8) {
    let x = (out.w() - font.width(text)) / 2;
    font.draw(out, text, x, y, level);
}

/// The title over a twinkling starfield, with a white-hot glint.
struct Logo;

impl Scene for Logo {
    fn name(&self) -> &str {
        "logo"
    }
    fn update(&mut self, cx: &Context, out: &mut Frame) {
        art::stars(out, out.rect(), cx.t, 0.035);
        let mut layer = out.blank();
        centered(&mut layer, &REGULAR, "GLOWDOTS", 12, MAX);
        art::overlay_outlined(out, &layer);
        glint(out, cx.t - 2.9, Rect::new(14, 10, 66, 21));
    }
    fn duration(&self) -> f32 {
        60.0
    }
}

/// A bright band of white light that sweeps across the lit dots in `area`
/// as `p` goes from 0 to 0.6 seconds.
fn glint(out: &mut Frame, p: f32, area: Rect) {
    if !(0.0..0.6).contains(&p) {
        return;
    }
    let band = area.x0 as f32 - 8.0 + p / 0.6 * (area.w() as f32 + 16.0);
    for y in area.y0..area.y1 {
        for x in area.x0..area.x1 {
            if out.get(x, y) == MAX && (x as f32 - (y - area.y0) as f32 * 0.6 - band).abs() < 2.5 {
                out.set_rgb(x, y, MAX, Rgb(255, 238, 205));
            }
        }
    }
}

/// A big line that types in, and a small line under it.
struct Card {
    name: &'static str,
    big: &'static str,
    small: &'static str,
    big_y: i32,
    small_y: i32,
}

impl Scene for Card {
    fn name(&self) -> &str {
        self.name
    }
    fn update(&mut self, cx: &Context, out: &mut Frame) {
        let typed = (cx.t * 40.0) as usize;
        let big: String = self.big.chars().take(typed).collect();
        let x = (out.w() - REGULAR.width(self.big)) / 2;
        REGULAR.draw(out, &big, x, self.big_y, MAX);
        let small: String = self.small.chars().take(typed.saturating_sub(self.big.len())).collect();
        let x = (out.w() - SMALL.width(self.small)) / 2;
        SMALL.draw(out, &small, x, self.small_y, 2);
    }
    fn duration(&self) -> f32 {
        60.0
    }
}

/// Rolling waves in 16 shades under "ANY COLOR"; the video cycles the look.
struct Colors;

impl Scene for Colors {
    fn name(&self) -> &str {
        "colors"
    }
    fn update(&mut self, cx: &Context, out: &mut Frame) {
        for y in 0..out.h() {
            for x in 0..out.w() {
                let (fx, fy) = (x as f32, y as f32);
                let v = 0.5 + 0.25 * (fx * 0.11 + cx.t * 2.2).sin() + 0.25 * (fx * 0.05 - fy * 0.24 + cx.t * 1.3).sin();
                out.set(x, y, out.level(v * 0.7));
            }
        }
        let mut layer = out.blank();
        let bright = layer.max();
        centered(&mut layer, &REGULAR, "ANY COLOR", 12, bright);
        art::overlay_outlined(out, &layer);
    }
    fn duration(&self) -> f32 {
        60.0
    }
    fn shades(&self) -> u8 {
        16
    }
}

/// "ANY GLASS", and the name of the glass the video is showing.
struct Glasses;

impl Scene for Glasses {
    fn name(&self) -> &str {
        "glass"
    }
    fn update(&mut self, cx: &Context, out: &mut Frame) {
        centered(out, &REGULAR, "ANY GLASS", 4, MAX);
        let label = match cx.t {
            t if t < 1.0 => "PLASMA",
            t if t < 2.0 => "LED",
            _ => "WORN",
        };
        centered(out, &REGULAR, label, 18, MAX);
        out.fill(Rect::new(26, 14, 54, 15), 1);
    }
    fn duration(&self) -> f32 {
        60.0
    }
}

/// Where glowdots runs: an icon and a name.
struct Platform {
    name: &'static str,
    label: &'static str,
}

impl Platform {
    fn icon(&self, out: &mut Frame, t: f32) {
        let (cx, cy) = (40.0f32, 8.0f32);
        let ring = |out: &mut Frame, x0: f32, y0: f32, r: f32, level: u8| {
            for y in (y0 - r - 1.0) as i32..=(y0 + r + 1.0) as i32 {
                for x in (x0 - r - 1.0) as i32..=(x0 + r + 1.0) as i32 {
                    let d = (x as f32 - x0).hypot(y as f32 - y0);
                    if (d - r).abs() < 0.55 {
                        out.set(x, y, level);
                    }
                }
            }
        };
        match self.name {
            "window" => {
                let r = Rect::new(28, 1, 52, 16);
                out.fill(Rect::new(r.x0, r.y0, r.x1, r.y0 + 3), 1);
                for x in r.x0..r.x1 {
                    out.set(x, r.y0, MAX);
                    out.set(x, r.y1 - 1, MAX);
                }
                for y in r.y0..r.y1 {
                    out.set(r.x0, y, MAX);
                    out.set(r.x1 - 1, y, MAX);
                }
                for k in 0..3 {
                    out.set(r.x0 + 2 + k * 2, r.y0 + 1, MAX);
                }
                // A tiny clock inside, its colon blinking.
                let x = r.x0 + (r.w() - SMALL.width("10:08")) / 2 + 1;
                SMALL.draw(out, "10:08", x, r.y0 + 6, MAX);
                if (t * 2.0) as i32 % 2 == 1 {
                    SMALL.draw(out, ":", x + SMALL.width("10") + 1, r.y0 + 6, 1);
                }
            }
            "browser" => {
                // A globe, turning.
                ring(out, cx, cy, 7.0, MAX);
                for y in 1..16 {
                    out.set(40, y, 2);
                }
                for x in 33..48 {
                    out.set(x, 8, 2);
                }
                for k in 0..2 {
                    let w = 4.0 * ((t * 2.0 + k as f32 * 1.6).sin()).abs() + 0.5;
                    for y in 1..16 {
                        let dy = (y as f32 - cy) / 7.0;
                        let x = cx + w * (1.0 - dy * dy).max(0.0).sqrt() * if k == 0 { 1.0 } else { -1.0 };
                        out.set(x.round() as i32, y, 1);
                    }
                }
                for y in [4, 12] {
                    for x in 34..47 {
                        if ((x as f32 - cx).hypot(y as f32 - cy) - 7.0) < -0.5 {
                            out.set(x, y, 1);
                        }
                    }
                }
            }
            _ => {
                // A raspberry: leaves on top, and a berry of round drupelets
                // that catch the light.
                let berry = glowdots::Sprite::from_rows(&[
                    "  ##       ##  ",
                    "   ###   ###   ",
                    "    ##= =##    ",
                    "      ===      ",
                    "    =#.#.#=    ",
                    "   #.#=#=#.#   ",
                    "  =#=#.#.#=#=  ",
                    "  #.#=#=#=#.#  ",
                    "  =#=#.#.#=#=  ",
                    "  .#.#=#=#.#.  ",
                    "   =#=#.#=#=   ",
                    "    #.#=#.#    ",
                    "     =#.#=     ",
                    "      =#=      ",
                    "       .       ",
                ]);
                out.blit(&berry, 33, 1);
            }
        }
    }
}

impl Scene for Platform {
    fn name(&self) -> &str {
        self.name
    }
    fn update(&mut self, cx: &Context, out: &mut Frame) {
        self.icon(out, cx.t);
        centered(out, &SMALL, if self.name == "pi" { "RUNS ON" } else { "RUNS IN" }, 18, 2);
        centered(out, &REGULAR, self.label, 24, MAX);
    }
    fn duration(&self) -> f32 {
        60.0
    }
}

/// The demo's plasma, tunnel, and torus, cut short, each opening with its flash.
struct DemoCut(Demo);

impl Scene for DemoCut {
    fn name(&self) -> &str {
        "democut"
    }
    fn enter(&mut self, cx: &Context) {
        self.0.enter(cx);
    }
    fn update(&mut self, cx: &Context, out: &mut Frame) {
        // The demo's parts start at 8 s (plasma), 15 s (tunnel), and 22 s (torus).
        let t = match cx.t {
            t if t < 1.25 => 8.0 + t,
            t if t < 2.5 => 15.0 + t - 1.25,
            t => 22.0 + t - 2.5,
        };
        self.0.update(&cx.with_time(cx.dt, t), out);
    }
    fn duration(&self) -> f32 {
        60.0
    }
}

/// The end card, then the power going down.
struct Outro;

impl Scene for Outro {
    fn name(&self) -> &str {
        "outro"
    }
    fn update(&mut self, cx: &Context, out: &mut Frame) {
        if cx.t > OUTRO_DARK {
            return;
        }
        art::stars(out, out.rect(), cx.t, 0.03);
        let mut layer = out.blank();
        centered(&mut layer, &REGULAR, "GLOWDOTS", 4, MAX);
        let typed = ((cx.t - 0.4).max(0.0) * 40.0) as usize;
        for (line, y, level) in [("OPEN SOURCE · RUST", 16, 2), ("TRY IT IN A BROWSER", 23, MAX)] {
            let shown: String = line.chars().take(typed).collect();
            let x = (layer.w() - SMALL.width(line)) / 2;
            SMALL.draw(&mut layer, &shown, x, y, level);
        }
        art::overlay_outlined(out, &layer);
        glint(out, cx.t - 2.0, Rect::new(14, 2, 66, 13));
    }
    fn duration(&self) -> f32 {
        60.0
    }
}

/// When, into the end card, the display goes dark.
const OUTRO_DARK: f32 = 4.4;

// ---- The video's timeline ---------------------------------------------

/// The deck, in order: each scene, how long, and how it comes in.
fn deck() -> Result<Deck> {
    let mut pool = showcase()?;
    let mut take = |name: &str| -> Result<Box<dyn Scene>> {
        let i = pool.iter().position(|s| s.name() == name).ok_or_else(|| anyhow!("no scene {name}"))?;
        Ok(pool.remove(i))
    };
    let card = |name, big, small, big_y, small_y| -> Box<dyn Scene> { Box::new(Card { name, big, small, big_y, small_y }) };
    use Transition::*;
    let parts: Vec<(Box<dyn Scene>, f32, Transition)> = vec![
        (Box::new(Logo), 4.0, Cut),
        (card("pinball", "1990S PINBALL", "DOT MATRIX DISPLAYS", 7, 20), 1.5, Wipe),
        (card("anything", "FOR ANYTHING", "YOU WANT TO SHOW", 8, 20), 1.5, Wipe),
        (take("clock")?, 2.0, Dissolve),
        (take("weather")?, 2.0, Dissolve),
        (take("lake")?, 2.5, Wipe),
        (take("news")?, 2.5, Wipe),
        (take("sports")?, 1.5, Dissolve),
        (take("status")?, 1.5, Wipe),
        (card("json", "FEED IT JSON", "FROM ANYTHING", 8, 20), 1.5, Wipe),
        (Box::new(glowdots_scenes::events::scene()), 2.5, Dissolve),
        (Box::new(Colors), 3.0, Wipe),
        (Box::new(Glasses), 3.0, Dissolve),
        (Box::new(Platform { name: "window", label: "A WINDOW" }), 1.0, Push(1)),
        (Box::new(Platform { name: "browser", label: "A BROWSER" }), 1.0, Push(1)),
        (Box::new(Platform { name: "pi", label: "A RASPBERRY PI" }), 1.0, Push(1)),
        (Box::new(DemoCut(Demo::new())), 4.0, Ink),
        // Longer than the video, so the deck never comes back around.
        (Box::new(Outro), 9.0, Dissolve),
    ];
    let scenes =
        parts.into_iter().map(|(inner, seconds, transition)| Box::new(Timed { inner, seconds, transition }) as Box<dyn Scene>).collect();
    // The events scene shows the launch.
    let launch = Bundled::new().with(
        "events.json",
        r#"{"items": [{"title": "glowdots 0.1", "venue": "github.com/ztaylor", "time": "Out now", "category": "Rust"}]}"#,
    );
    // The clock rolls over to 10:08 one second into its scene.
    let start = time::parse_local("2026-10-09 22:07:52").map_err(|e| anyhow!(e))?;
    Ok(Deck::new(SIZE, scenes)?.with_feeds(Layered::new().then(launch).then(samples())).with_clock(Clock::stepped(start)))
}

fn lerp(a: f32, b: f32, p: f32) -> f32 {
    a + (b - a) * p
}

fn mix_glass(a: Glass, b: Glass, p: f32) -> Glass {
    Glass {
        dot_size: lerp(a.dot_size, b.dot_size, p),
        roundness: lerp(a.roundness, b.roundness, p),
        halo: lerp(a.halo, b.halo, p),
        halo_width: lerp(a.halo_width, b.halo_width, p),
        unlit: lerp(a.unlit, b.unlit, p),
        variation: lerp(a.variation, b.variation, p),
        flicker: lerp(a.flicker, b.flicker, p),
        gamma: lerp(a.gamma, b.gamma, p),
        hot_core: lerp(a.hot_core, b.hot_core, p),
        bloom: lerp(a.bloom, b.bloom, p),
        afterglow: lerp(a.afterglow, b.afterglow, p),
    }
}

/// Through a list of (time, value) keys, holding each and blending over
/// `blend` seconds into the next.
fn keyed<T: Copy>(keys: &[(f32, T)], t: f32, blend: f32, mix: impl Fn(T, T, f32) -> T) -> T {
    let mut v = keys[0].1;
    for w in keys.windows(2) {
        let (t1, b) = w[1];
        if t >= t1 - blend {
            v = if t >= t1 { b } else { mix(w[0].1, b, ease_in_out((t - (t1 - blend)) / blend)) };
        }
    }
    v
}

/// Set the camera, glass, and look for the moment `t` seconds into
/// `scene`.
fn direct(player: &mut Player, scene: &str, t: f32) {
    player.camera = Camera::default();
    player.glass = Glass::CLASSIC;
    player.look = None;
    match scene {
        "logo" => {
            // From a macro shot of the first O, back out to the whole panel.
            let p = ease_in_out(t / 3.0);
            player.camera = Camera { zoom: (14f32.ln() * (1.0 - p)).exp(), focus: Some([lerp(30.5, 40.0, p), lerp(15.5, 16.0, p)]) };
        }
        "colors" => {
            let keys = [
                (0.0, Look::WILLIAMS),
                (0.6, Look::GREEN),
                (1.1, Look::BLUE),
                (1.6, Look::heat()),
                (2.1, Look::ocean()),
                (2.7, Look::WILLIAMS),
            ];
            player.look = Some(keyed(&keys, t, 0.2, Look::mix));
        }
        "glass" => {
            let keys = [(0.0, Glass::CLASSIC), (1.0, Glass::LED), (2.0, Glass::WORN), (2.9, Glass::CLASSIC)];
            player.glass = keyed(&keys, t, 0.25, mix_glass);
            // Push in on the glass's name, and back out for the cut.
            let zoom = if t < 2.6 { lerp(1.0, 2.2, ease_in_out((t - 0.2) / 2.2)) } else { lerp(2.2, 1.0, ease_in_out((t - 2.6) / 0.35)) };
            player.camera = Camera { zoom, focus: Some([40.0, lerp(16.0, 22.0, ((zoom - 1.0) / 1.2).clamp(0.0, 1.0))]) };
        }
        "outro" => {
            // The power goes down: dots fade slowly, then the glass goes dark.
            let p = ((t - OUTRO_DARK) / 0.9).clamp(0.0, 1.0);
            player.glass.afterglow = lerp(Glass::CLASSIC.afterglow, 0.45, ((t - OUTRO_DARK + 0.1) * 10.0).clamp(0.0, 1.0));
            player.glass.unlit = lerp(Glass::CLASSIC.unlit, 0.0, p);
            player.glass.bloom = lerp(Glass::CLASSIC.bloom, 0.4, p);
            player.camera = Camera { zoom: lerp(1.0, 0.92, ease_in_out((t - 3.0) / 2.5)), focus: None };
        }
        _ => {}
    }
}

// ---- Rendering --------------------------------------------------------

fn main() -> Result<()> {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "glowdots-promo.mp4".into()));
    let silent = out.with_file_name(format!("{}-silent.mp4", out.file_stem().and_then(|s| s.to_str()).unwrap_or("promo")));
    let video = out.with_extension("video.mp4");
    let audio = out.with_extension("wav");
    check_text()?;

    let total: f32 = 41.5;
    let mut player = Player::new(deck()?);
    player.progress_line = false;

    let frames = (total * FPS as f32).round() as usize;
    let mut ffmpeg = Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgba", "-s", &format!("{}x{}", SCREEN.0, SCREEN.1)])
        .args(["-framerate", &FPS.to_string(), "-i", "-"])
        .args(["-c:v", "libx264", "-preset", "slow", "-crf", "17", "-maxrate", "24M", "-bufsize", "48M"])
        .args(["-pix_fmt", "yuv420p", "-profile:v", "high", "-movflags", "+faststart"])
        .arg(&video)
        .stdin(Stdio::piped())
        .spawn()
        .context("starting ffmpeg (is it installed?)")?;
    let mut pipe = ffmpeg.stdin.take().ok_or_else(|| anyhow!("no ffmpeg input"))?;

    // Where each scene starts, as it happens: for the soundtrack.
    let mut cuts: Vec<(String, f32)> = Vec::new();
    let dt = 1.0 / FPS as f32;
    glowdots::app::offscreen(SIZE, SCREEN, |r| {
        let mut current = String::new();
        let mut started = 0.0f32;
        for i in 0..frames {
            let t = i as f32 * dt;
            // Step first, so a cut on this frame shows its first frame.
            player.step(dt);
            let name = player.deck.current().name().to_string();
            if name != current {
                current = name.clone();
                started = t;
                cuts.push((name, t));
            }
            direct(&mut player, &current, t - started);
            let img = player.capture(r)?;
            pipe.write_all(img.as_raw())?;
            if i % (FPS as usize * 5) == 0 {
                eprintln!("{:>4.1} s  {}", t, current);
            }
        }
        Ok(())
    })?;
    drop(pipe);
    if !ffmpeg.wait()?.success() {
        bail!("ffmpeg could not encode the video");
    }

    let (left, right) = soundtrack(&cuts, total);
    write_wav(&audio, &left, &right)?;
    let status = Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-i"])
        .arg(&video)
        .arg("-i")
        .arg(&audio)
        .args(["-c:v", "copy", "-af", "loudnorm=I=-14:TP=-1.5:LRA=11", "-ar", "48000", "-c:a", "aac", "-b:a", "192k"])
        .args(["-shortest", "-movflags", "+faststart"])
        .arg(&out)
        .status()?;
    if !status.success() {
        bail!("ffmpeg could not add the sound");
    }
    std::fs::rename(&video, &silent)?;
    std::fs::remove_file(&audio)?;
    eprintln!("cuts: {}", cuts.iter().map(|(n, t)| format!("{n} {t:.2}")).collect::<Vec<_>>().join(", "));
    eprintln!("wrote {} and {}", out.display(), silent.display());
    Ok(())
}

/// Every line of the title cards must fit the panel.
fn check_text() -> Result<()> {
    let lines = [
        (&REGULAR, "GLOWDOTS"),
        (&REGULAR, "1990S PINBALL"),
        (&SMALL, "DOT MATRIX DISPLAYS"),
        (&REGULAR, "FOR ANYTHING"),
        (&SMALL, "YOU WANT TO SHOW"),
        (&REGULAR, "FEED IT JSON"),
        (&SMALL, "FROM ANYTHING"),
        (&REGULAR, "ANY COLOR"),
        (&REGULAR, "ANY GLASS"),
        (&REGULAR, "A WINDOW"),
        (&REGULAR, "A BROWSER"),
        (&REGULAR, "A RASPBERRY PI"),
        (&SMALL, "OPEN SOURCE · RUST"),
        (&SMALL, "TRY IT IN A BROWSER"),
    ];
    for (font, line) in lines {
        let w = font.width(line);
        if w > PANEL_W - 2 {
            bail!("{line:?} is {w} dots wide; the panel is {PANEL_W}");
        }
    }
    Ok(())
}

// ---- The soundtrack ---------------------------------------------------

const RATE: u32 = 48_000;

struct Mix {
    left: Vec<f32>,
    right: Vec<f32>,
}

impl Mix {
    fn new(seconds: f32) -> Self {
        let n = (seconds * RATE as f32) as usize;
        Self { left: vec![0.0; n], right: vec![0.0; n] }
    }

    /// Add a sound at `at` seconds: `f(t)` for t in 0..len, panned -1..1.
    fn add(&mut self, at: f32, len: f32, gain: f32, pan: f32, mut f: impl FnMut(f32) -> f32) {
        let (gl, gr) = (((1.0 - pan) * 0.25 * std::f32::consts::PI).cos() * gain, ((1.0 + pan) * 0.25 * std::f32::consts::PI).cos() * gain);
        let start = (at * RATE as f32) as usize;
        let n = ((len * RATE as f32) as usize).min(self.left.len().saturating_sub(start));
        for k in 0..n {
            let v = f(k as f32 / RATE as f32);
            self.left[start + k] += v * gl;
            self.right[start + k] += v * gr;
        }
    }
}

fn midi(n: f32) -> f32 {
    440.0 * 2f32.powf((n - 69.0) / 12.0)
}

/// A one-pole low-pass filter, for softening square and saw waves.
struct LowPass {
    y: f32,
    a: f32,
}

impl LowPass {
    fn new(cutoff: f32) -> Self {
        Self { y: 0.0, a: 1.0 - (-2.0 * std::f32::consts::PI * cutoff / RATE as f32).exp() }
    }
    fn run(&mut self, x: f32) -> f32 {
        self.y += self.a * (x - self.y);
        self.y
    }
}

fn soundtrack(cuts: &[(String, f32)], total: f32) -> (Vec<f32>, Vec<f32>) {
    let tau = std::f32::consts::TAU;
    let mut drums = Mix::new(total);
    let mut music = Mix::new(total);
    let mut echo = Mix::new(total);
    let mut rng = Rng(0x5eed_5eed);
    let mut noise = move || rng.next_f32() * 2.0 - 1.0;
    let at = |name: &str| cuts.iter().find(|c| c.0 == name).map_or(0.0, |c| c.1);
    let groove_start = at("pinball");
    let demo_start = at("democut");
    let outro_start = at("outro");
    let groove_end = outro_start + OUTRO_DARK;

    // A minor, F, C, G: a bar (4 beats, 2 seconds) each.
    let chords: [[f32; 4]; 4] = [[57.0, 60.0, 64.0, 69.0], [53.0, 57.0, 60.0, 65.0], [48.0, 52.0, 55.0, 60.0], [55.0, 59.0, 62.0, 67.0]];
    let roots = [33.0, 29.0, 36.0, 31.0];

    // The intro: a pad swells, noise rises, bells for the glint, a boom.
    for &n in &chords[0][..3] {
        for detune in [-0.06, 0.06] {
            let f = midi(n + detune);
            let mut lp = LowPass::new(900.0);
            let mut ph = 0.0;
            music.add(0.0, groove_start + 1.0, 0.05, detune * 8.0, |t| {
                ph = (ph + f / RATE as f32) % 1.0;
                let env = (t / 2.5).min(1.0) * (1.0 - ((t - groove_start + 0.2) / 1.0).clamp(0.0, 1.0));
                lp.run(ph * 2.0 - 1.0) * env
            });
        }
    }
    {
        let mut lp = LowPass::new(300.0);
        let mut ph = 0.0;
        drums.add(0.5, groove_start - 0.5, 0.22, 0.0, |t| {
            let p = t / (groove_start - 0.5);
            lp.a = 1.0 - (-tau * (300.0 * 20f32.powf(p)) / RATE as f32).exp();
            ph = (ph + (110.0 * 8f32.powf(p)) / RATE as f32) % 1.0;
            (lp.run(noise()) * 0.7 + (ph * tau).sin() * 0.2) * p * p
        });
    }
    for (k, f) in [2093.0, 2637.0, 3136.0, 4186.0, 5274.0].into_iter().enumerate() {
        let start = at("logo") + 2.9 + k as f32 * 0.07;
        music.add(start, 1.4, 0.08, (k as f32 - 2.0) * 0.3, move |t| (tau * f * t).sin() * (-t * 4.0).exp());
    }
    // The boom as the groove starts.
    drums.add(groove_start, 2.0, 0.9, 0.0, |t| (tau * 38.0 * t).sin() * (-t * 1.6).exp() + noise() * 0.25 * (-t * 5.0).exp());

    // The groove: a beat every half second until the end card goes dark.
    let mut kicks = Vec::new();
    let mut beat = 0;
    loop {
        let t0 = groove_start + beat as f32 * BEAT;
        if t0 >= groove_end - 0.01 {
            break;
        }
        let chord = (beat / 4) % 4;
        let demo = t0 >= demo_start;
        // Kick.
        kicks.push(t0);
        let mut ph = 0.0;
        drums.add(t0, 0.35, 0.85, 0.0, move |t| {
            ph += (46.0 + 130.0 * (-t * 32.0).exp()) / RATE as f32;
            (ph * tau).sin() * (-t * 7.0).exp()
        });
        // Snare on the 2 and the 4.
        if beat % 2 == 1 {
            let mut prev = 0.0;
            drums.add(t0, 0.25, 0.32, 0.1, |t| {
                let n = noise();
                let hp = n - prev;
                prev = n;
                hp * (-t * 17.0).exp() + (tau * 190.0 * t).sin() * (-t * 26.0).exp() * 0.6
            });
        }
        // Hats: eighths, sixteenths in the demo.
        let hats: &[f32] = if demo { &[0.125, 0.25, 0.375] } else { &[0.25] };
        for &off in hats {
            let mut prev = 0.0;
            drums.add(t0 + off, 0.06, 0.16, -0.3, |t| {
                let n = noise();
                let hp = n - prev;
                prev = n;
                hp * (-t * 80.0).exp()
            });
        }
        // Bass: eighth notes on the root, up an octave on the off-beat.
        for (k, octave) in [(0.0, 0.0), (0.25, 12.0)] {
            let f = midi(roots[chord] + octave);
            let mut lp = LowPass::new(520.0);
            let mut ph = 0.0;
            music.add(t0 + k, 0.24, 0.24, 0.0, move |t| {
                ph = (ph + f / RATE as f32) % 1.0;
                let sq = if ph < 0.5 { 1.0 } else { -1.0 };
                lp.run(sq) * (1.0 - (t / 0.24).powi(4)) * (t * 300.0).min(1.0)
            });
        }
        // The arpeggio, from the montage on: sixteenths up the chord.
        if t0 >= at("clock") {
            for k in 0..4 {
                let n = chords[chord][(beat * 4 + k) % 4] + 12.0;
                let f = midi(n);
                let mut lp = LowPass::new(if demo { 5200.0 } else { 3200.0 });
                let mut ph = 0.0;
                let pan = if k % 2 == 0 { -0.35 } else { 0.35 };
                echo.add(t0 + k as f32 * BEAT / 4.0, 0.2, 0.085, pan, move |t| {
                    ph = (ph + f / RATE as f32) % 1.0;
                    lp.run(if ph < 0.3 { 1.0 } else { -1.0 }) * (-t * 13.0).exp()
                });
            }
        }
        beat += 1;
    }

    // A soft pad under the groove, and the last chord held as it ends.
    let bars = ((groove_end - groove_start) / (4.0 * BEAT)).ceil() as usize;
    for bar in 0..=bars {
        let t0 = groove_start + bar as f32 * 4.0 * BEAT;
        let last = bar == bars || t0 + 4.0 * BEAT >= groove_end;
        let len = if last { 3.0 } else { 4.0 * BEAT + 0.05 };
        for &n in &chords[bar % 4][..3] {
            let f = midi(n - 12.0);
            let mut lp = LowPass::new(700.0);
            let mut ph = 0.0;
            music.add(t0.min(groove_end), len, 0.035, 0.0, move |t| {
                ph = (ph + f / RATE as f32) % 1.0;
                let release = if last { (-t * 1.2).exp() } else { 1.0 };
                lp.run(ph * 2.0 - 1.0) * (t * 8.0).min(1.0) * release
            });
        }
        if last {
            break;
        }
    }

    // A tick at every cut, and the power going down at the end.
    for (_, t0) in cuts.iter().skip(1) {
        music.add(*t0, 0.05, 0.05, 0.0, |t| (tau * 2400.0 * t).sin() * (-t * 110.0).exp());
    }
    let mut ph = 0.0;
    music.add(groove_end, 1.1, 0.22, 0.0, move |t| {
        ph += 620.0 * (-t * 3.5).exp() / RATE as f32;
        (ph * tau).sin() * (-t * 2.6).exp()
    });

    // A dotted-eighth echo on the arpeggio, bouncing side to side.
    let delay = (0.375 * RATE as f32) as usize;
    for i in delay..echo.left.len() {
        echo.left[i] += echo.right[i - delay] * 0.38;
        echo.right[i] += echo.left[i - delay] * 0.38;
    }

    // Duck the music under each kick, mix, and keep it under full scale.
    let n = drums.left.len();
    let (mut left, mut right) = (vec![0.0; n], vec![0.0; n]);
    let mut k = 0;
    for i in 0..n {
        let t = i as f32 / RATE as f32;
        while k + 1 < kicks.len() && kicks[k + 1] <= t {
            k += 1;
        }
        let since = kicks.get(k).map_or(9.0, |&kt| if kt <= t { t - kt } else { 9.0 });
        let duck = 1.0 - 0.45 * (-since * 9.0).exp();
        let fade = (t / 0.02).min(1.0) * ((total - t) / 0.3).clamp(0.0, 1.0);
        left[i] = ((drums.left[i] + (music.left[i] + echo.left[i]) * duck) * 1.1).tanh() * fade;
        right[i] = ((drums.right[i] + (music.right[i] + echo.right[i]) * duck) * 1.1).tanh() * fade;
    }
    let peak = left.iter().chain(&right).fold(0.0f32, |m, v| m.max(v.abs())).max(1e-6);
    for v in left.iter_mut().chain(right.iter_mut()) {
        *v *= 0.89 / peak;
    }
    (left, right)
}

fn write_wav(path: &Path, left: &[f32], right: &[f32]) -> Result<()> {
    let n = left.len() as u32;
    let mut bytes = Vec::with_capacity(44 + n as usize * 4);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + n * 4).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&RATE.to_le_bytes());
    bytes.extend_from_slice(&(RATE * 4).to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(n * 4).to_le_bytes());
    for (l, r) in left.iter().zip(right) {
        bytes.extend_from_slice(&((l.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
        bytes.extend_from_slice(&((r.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    std::fs::write(path, bytes).with_context(|| format!("writing {}", path.display()))
}
