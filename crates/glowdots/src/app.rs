//! A desktop window that plays a deck (feature `desktop`, not on the web),
//! and offscreen rendering for screenshots and GIFs.
//!
//! Keys: Right or Space for the next scene, Left for the one before, Up,
//! Down, Enter, and letters go to the scene, F toggles fullscreen, Esc or
//! Q quits. A click or tap on the right half goes forward, on the left
//! half back (unless the scene uses it).

#![cfg(all(feature = "desktop", not(target_arch = "wasm32")))]

use crate::frame::Size;
use crate::player::Player;
use crate::render::{Dialect, Renderer};
use crate::scene::Key as DeckKey;
use anyhow::{Context, Result, anyhow};
use glutin::config::{Config, ConfigTemplateBuilder, GlConfig};
use glutin::context::{ContextApi, ContextAttributesBuilder, GlContext, NotCurrentGlContext, PossiblyCurrentContext, Version};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::surface::{GlSurface, Surface, SurfaceAttributesBuilder, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasWindowHandle;
use std::num::NonZeroU32;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, KeyEvent, MouseButton, Touch, TouchPhase, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Fullscreen, Window, WindowAttributes, WindowId};

/// How the window opens.
pub struct Options {
    pub title: String,
    pub fullscreen: bool,
    /// Window size in logical pixels, when not fullscreen.
    pub window: (u32, u32),
}

impl Default for Options {
    fn default() -> Self {
        Self { title: "glowdots".into(), fullscreen: false, window: (1200, 480) }
    }
}

/// Open a window and play until it closes.
pub fn run(player: Player, options: Options) -> Result<()> {
    let event_loop = EventLoop::new().context("creating the event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App { options, player, gpu: None, cursor: PhysicalPosition::new(0.0, 0.0), last: Instant::now(), result: Ok(()) };
    event_loop.run_app(&mut app).context("running the event loop")?;
    app.result
}

/// Make a GL context with no window on screen and a renderer for a `panel`
/// drawn on `screen` pixels, and run `f` with it. For screenshots and GIFs
/// (see [`crate::export`]).
///
/// A program can do this once (winit cannot make a second event loop), or
/// run a window instead. To render several things, do them all in `f`;
/// [`Renderer::set_size`] and [`Renderer::resize`] change the panel and
/// the picture size.
pub fn offscreen<T>(panel: Size, screen: (u32, u32), f: impl FnOnce(&mut Renderer) -> Result<T>) -> Result<T> {
    let event_loop = EventLoop::new().context("creating the event loop")?;
    let mut app = Offscreen { panel, screen, f: Some(f), result: None };
    event_loop.run_app(&mut app).context("running the event loop")?;
    app.result.unwrap_or_else(|| Err(anyhow!("the offscreen renderer did not start")))
}

struct Gpu {
    window: Window,
    surface: Surface<WindowSurface>,
    context: PossiblyCurrentContext,
    renderer: Renderer,
}

/// A window with a current GL context, and a renderer for `panel`.
fn create_gpu(event_loop: &ActiveEventLoop, attrs: WindowAttributes, panel: Size, screen: Option<(u32, u32)>) -> Result<Gpu> {
    let template = ConfigTemplateBuilder::new().with_alpha_size(8);
    let (window, config) = DisplayBuilder::new()
        .with_window_attributes(Some(attrs))
        .build(event_loop, template, pick_config)
        .map_err(|e| anyhow!("creating the window and GL display: {e}"))?;
    let window = window.ok_or_else(|| anyhow!("no window was created"))?;
    let handle = window.window_handle()?.as_raw();
    let display = config.display();

    // The shaders are GL ES 3.0. Ask for that first; desktops that cannot
    // (macOS) get OpenGL 3.3 core and the same shaders with another
    // #version line.
    let gles = ContextAttributesBuilder::new().with_context_api(ContextApi::Gles(Some(Version::new(3, 0)))).build(Some(handle));
    let core = ContextAttributesBuilder::new().with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3)))).build(Some(handle));
    let not_current = unsafe {
        display
            .create_context(&config, &gles)
            .or_else(|_| display.create_context(&config, &core))
            .context("creating a GL context (GL ES 3.0 or OpenGL 3.3 is necessary)")?
    };
    let dialect = match not_current.context_api() {
        ContextApi::Gles(_) => Dialect::Es300,
        ContextApi::OpenGl(_) => Dialect::Core330,
    };

    let surface_attrs = window.build_surface_attributes(SurfaceAttributesBuilder::<WindowSurface>::new()).map_err(|e| anyhow!("{e}"))?;
    let surface = unsafe { display.create_window_surface(&config, &surface_attrs).context("creating the GL surface")? };
    let context = not_current.make_current(&surface).context("making the GL context current")?;
    let _ = surface.set_swap_interval(&context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()));

    let gl = unsafe { glow::Context::from_loader_function_cstr(|s| display.get_proc_address(s)) };
    let size = window.inner_size();
    let screen = screen.unwrap_or((size.width.max(1), size.height.max(1)));
    let renderer = Renderer::new(gl, dialect, screen, panel)?;
    Ok(Gpu { window, surface, context, renderer })
}

/// Prefer a config without multisampling: the dots are drawn in the shader.
fn pick_config(configs: Box<dyn Iterator<Item = Config> + '_>) -> Config {
    configs.reduce(|best, c| if c.num_samples() < best.num_samples() { c } else { best }).expect("at least one GL config")
}

struct App {
    options: Options,
    player: Player,
    gpu: Option<Gpu>,
    cursor: PhysicalPosition<f64>,
    last: Instant,
    result: Result<()>,
}

impl App {
    /// Play to now and draw.
    fn tick(&mut self) {
        let Some(gpu) = &mut self.gpu else { return };
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().min(0.1);
        self.last = now;
        self.player.step(dt);
        self.player.draw(&gpu.renderer);
        if let Err(e) = gpu.surface.swap_buffers(&gpu.context) {
            eprintln!("swap: {e}");
        }
    }

    fn tap(&mut self, at: PhysicalPosition<f64>) {
        if let Some(gpu) = &self.gpu {
            let screen = gpu.renderer.screen();
            self.player.tap(screen, at.x as f32, at.y as f32);
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }
        let (w, h) = self.options.window;
        let mut attrs = Window::default_attributes().with_title(&self.options.title).with_inner_size(LogicalSize::new(w, h));
        if self.options.fullscreen {
            attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }
        match create_gpu(event_loop, attrs, self.player.deck.size(), None) {
            Ok(gpu) => {
                let (sw, sh) = gpu.renderer.screen();
                eprintln!("screen {sw}x{sh}, panel {}, {} shades", self.player.deck.size(), self.player.deck.shades());
                self.gpu = Some(gpu);
                self.last = Instant::now();
            }
            Err(e) => {
                self.result = Err(e);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    if let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
                        gpu.surface.resize(&gpu.context, w, h);
                        if let Err(e) = gpu.renderer.resize((size.width, size.height)) {
                            eprintln!("resize: {e}");
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput { event: KeyEvent { logical_key, state: ElementState::Pressed, repeat: false, .. }, .. } => {
                match logical_key.as_ref() {
                    Key::Named(NamedKey::Escape) | Key::Character("q") => event_loop.exit(),
                    Key::Character("f") => {
                        if let Some(gpu) = &self.gpu {
                            let full = gpu.window.fullscreen().is_none().then_some(Fullscreen::Borderless(None));
                            gpu.window.set_fullscreen(full);
                        }
                    }
                    Key::Named(NamedKey::ArrowRight | NamedKey::Space) => self.player.key(DeckKey::Right),
                    Key::Named(NamedKey::ArrowLeft) => self.player.key(DeckKey::Left),
                    Key::Named(NamedKey::ArrowUp) => self.player.key(DeckKey::Up),
                    Key::Named(NamedKey::ArrowDown) => self.player.key(DeckKey::Down),
                    Key::Named(NamedKey::Enter) => self.player.key(DeckKey::Select),
                    Key::Character(s) => {
                        if let Some(c) = s.chars().next() {
                            self.player.key(DeckKey::Char(c));
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::CursorMoved { position, .. } => self.cursor = position,
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => self.tap(self.cursor),
            WindowEvent::Touch(Touch { phase: TouchPhase::Started, location, .. }) => self.tap(location),
            WindowEvent::RedrawRequested => self.tick(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(gpu) = &self.gpu {
            gpu.window.request_redraw();
        }
    }
}

struct Offscreen<F, T> {
    panel: Size,
    screen: (u32, u32),
    f: Option<F>,
    result: Option<Result<T>>,
}

impl<F: FnOnce(&mut Renderer) -> Result<T>, T> ApplicationHandler for Offscreen<F, T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let Some(f) = self.f.take() else { return };
        let attrs = Window::default_attributes().with_title("glowdots").with_visible(false).with_inner_size(LogicalSize::new(64, 64));
        self.result = Some(create_gpu(event_loop, attrs, self.panel, Some(self.screen)).and_then(|mut gpu| f(&mut gpu.renderer)));
        event_loop.exit();
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
}
