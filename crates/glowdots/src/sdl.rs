//! Play a deck full screen through SDL2 (feature `sdl`).
//!
//! On a Raspberry Pi with no desktop, SDL's KMSDRM driver draws straight
//! to the panel; run from a text console, SSH, or a systemd service. It
//! also works in a desktop session. Taps on a touch screen work like
//! clicks; keys: Right and Left move between scenes, Esc or Q quits.

#![cfg(all(feature = "sdl", not(target_arch = "wasm32")))]

use crate::frame::Size;
use crate::player::Player;
use crate::render::{Dialect, Renderer};
use crate::scene::Key;
use anyhow::{Result, anyhow};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::{GLContext, GLProfile, Window};
use std::time::{Duration, Instant};

/// How the screen is driven.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Print the frame rate once a minute.
    pub log_fps: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self { log_fps: true }
    }
}

struct Screen {
    sdl: sdl2::Sdl,
    window: Window,
    _context: GLContext,
    renderer: Renderer,
}

/// A full-screen window with a GL ES 3.0 context (or OpenGL 3.3 where GL ES
/// is not there), and a renderer for `panel` on `screen` pixels (default:
/// the whole screen).
fn open(panel: Size, screen: Option<(u32, u32)>, visible: bool) -> Result<Screen> {
    let sdl = sdl2::init().map_err(|e| anyhow!(e))?;
    let video = sdl.video().map_err(|e| anyhow!(e))?;
    let bounds = video.display_bounds(0).map_err(|e| anyhow!(e))?;
    let mut last_error = String::new();
    for (profile, version, dialect) in [(GLProfile::GLES, (3, 0), Dialect::Es300), (GLProfile::Core, (3, 3), Dialect::Core330)] {
        let attr = video.gl_attr();
        attr.set_context_profile(profile);
        attr.set_context_version(version.0, version.1);
        attr.set_double_buffer(true);
        let mut builder = video.window("glowdots", bounds.width(), bounds.height());
        builder.opengl();
        if visible {
            builder.fullscreen_desktop();
        } else {
            builder.hidden();
        }
        let window = match builder.build() {
            Ok(w) => w,
            Err(e) => {
                last_error = e.to_string();
                continue;
            }
        };
        let context = match window.gl_create_context() {
            Ok(c) => c,
            Err(e) => {
                last_error = e;
                continue;
            }
        };
        let _ = video.gl_set_swap_interval(1);
        sdl.mouse().show_cursor(false);
        let gl = unsafe { glow::Context::from_loader_function(|s| video.gl_get_proc_address(s) as *const _) };
        let screen = screen.unwrap_or_else(|| window.drawable_size());
        eprintln!("video driver: {}, screen {}x{}, {dialect:?}", video.current_video_driver(), screen.0, screen.1);
        let renderer = Renderer::new(gl, dialect, screen, panel)?;
        return Ok(Screen { sdl, window, _context: context, renderer });
    }
    Err(anyhow!("no GL ES 3.0 or OpenGL 3.3 context: {last_error}"))
}

/// Play full screen until Esc, Q, or the program is stopped.
pub fn run(mut player: Player, options: Options) -> Result<()> {
    let s = open(player.deck.size(), None, true)?;
    let mut events = s.sdl.event_pump().map_err(|e| anyhow!(e))?;
    let screen = s.renderer.screen();
    let start = Instant::now();
    let mut last = start;
    let (mut frames, mut fps_since) = (0u32, start);
    'run: loop {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape | Keycode::Q), .. } => break 'run,
                Event::KeyDown { keycode: Some(Keycode::Right), .. } => player.key(Key::Right),
                Event::KeyDown { keycode: Some(Keycode::Left), .. } => player.key(Key::Left),
                Event::KeyDown { keycode: Some(Keycode::Up), .. } => player.key(Key::Up),
                Event::KeyDown { keycode: Some(Keycode::Down), .. } => player.key(Key::Down),
                Event::KeyDown { keycode: Some(Keycode::Return), .. } => player.key(Key::Select),
                // Touch is 0..1 across the panel's own axes, like the screen's.
                Event::FingerDown { x, y, .. } => player.tap(screen, x * screen.0 as f32, y * screen.1 as f32),
                Event::MouseButtonDown { x, y, .. } => player.tap(screen, x as f32, y as f32),
                _ => {}
            }
        }
        let now = Instant::now();
        let dt = (now - last).as_secs_f32().min(0.1);
        last = now;
        player.step(dt);
        player.draw(&s.renderer);
        s.window.gl_swap_window();

        frames += 1;
        if options.log_fps && now - fps_since >= Duration::from_secs(60) {
            eprintln!("{:.1} fps", frames as f32 / (now - fps_since).as_secs_f32());
            (frames, fps_since) = (0, now);
        }
    }
    Ok(())
}

/// Like [`crate::app::offscreen`], through SDL: run `f` with a renderer for
/// `panel` on `screen` pixels (default: the screen's size).
pub fn offscreen<T>(panel: Size, screen: Option<(u32, u32)>, f: impl FnOnce(&mut Renderer) -> Result<T>) -> Result<T> {
    // A real full-screen window: the KMSDRM driver has no hidden ones.
    let mut s = open(panel, screen, true)?;
    f(&mut s.renderer)
}
