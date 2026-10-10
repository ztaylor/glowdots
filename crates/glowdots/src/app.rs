//! A window that runs a deck (feature `desktop`).
//!
//! Keys: Right or Space for the next scene, Left for the one before,
//! F for fullscreen, Esc or Q to quit. A click or tap on the right half
//! goes forward, on the left half back.

#![cfg(feature = "desktop")]

use crate::deck::Deck;
use crate::frame::{Frame, Phosphor};
use crate::render::{Dialect, Renderer};
use anyhow::{Context, Result, anyhow};
use glutin::config::{Config, ConfigTemplateBuilder, GlConfig};
use glutin::context::{ContextApi, ContextAttributesBuilder, GlContext, NotCurrentGlContext, PossiblyCurrentContext, Version};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::surface::{GlSurface, Surface, SurfaceAttributesBuilder, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasWindowHandle;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, KeyEvent, MouseButton, Touch, TouchPhase, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Fullscreen, Window, WindowId};

/// How the window runs.
pub struct Options {
    pub title: String,
    /// Quarter turns of the panel on the screen (0..3). A portrait panel
    /// mounted sideways uses 1 or 3.
    pub rotate: u32,
    pub fullscreen: bool,
    /// Window size in logical pixels, when not fullscreen.
    pub window: (u32, u32),
    /// Draw one frame at this many seconds in, save it, and stop.
    pub screenshot: Option<Screenshot>,
}

pub struct Screenshot {
    pub path: PathBuf,
    pub seconds: f32,
}

impl Default for Options {
    fn default() -> Self {
        Self { title: "glowdots".into(), rotate: 0, fullscreen: false, window: (1200, 480), screenshot: None }
    }
}

/// Open a window and run the deck until the window closes.
pub fn run(deck: Deck, options: Options) -> Result<()> {
    let event_loop = EventLoop::new().context("creating the event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let frame = Frame::new(deck.size());
    let phosphor = Phosphor::new(deck.size());
    let mut app = App {
        options,
        deck,
        frame,
        phosphor,
        gpu: None,
        cursor: PhysicalPosition::new(0.0, 0.0),
        start: Instant::now(),
        last: Instant::now(),
        result: Ok(()),
    };
    event_loop.run_app(&mut app).context("running the event loop")?;
    app.result
}

struct Gpu {
    window: Window,
    surface: Surface<WindowSurface>,
    context: PossiblyCurrentContext,
    renderer: Renderer,
}

struct App {
    options: Options,
    deck: Deck,
    frame: Frame,
    phosphor: Phosphor,
    gpu: Option<Gpu>,
    cursor: PhysicalPosition<f64>,
    start: Instant,
    last: Instant,
    result: Result<()>,
}

impl App {
    fn init(&self, event_loop: &ActiveEventLoop) -> Result<Gpu> {
        let (w, h) = self.options.window;
        let mut attrs = Window::default_attributes().with_title(&self.options.title).with_inner_size(LogicalSize::new(w, h));
        if self.options.fullscreen {
            attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }
        if self.options.screenshot.is_some() {
            attrs = attrs.with_visible(false);
        }
        let template = ConfigTemplateBuilder::new().with_alpha_size(8);
        let (window, config) = DisplayBuilder::new()
            .with_window_attributes(Some(attrs))
            .build(event_loop, template, pick_config)
            .map_err(|e| anyhow!("creating the window and GL display: {e}"))?;
        let window = window.ok_or_else(|| anyhow!("no window was created"))?;
        let handle = window.window_handle()?.as_raw();
        let display = config.display();

        // The shaders are GL ES 3.0. Ask for that first; desktops that
        // cannot (macOS) get OpenGL 3.3 core and the same shaders with
        // another #version line.
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

        let surface_attrs =
            window.build_surface_attributes(SurfaceAttributesBuilder::<WindowSurface>::new()).map_err(|e| anyhow!("{e}"))?;
        let surface = unsafe { display.create_window_surface(&config, &surface_attrs).context("creating the GL surface")? };
        let context = not_current.make_current(&surface).context("making the GL context current")?;
        let _ = surface.set_swap_interval(&context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()));

        let gl = unsafe { glow::Context::from_loader_function_cstr(|s| display.get_proc_address(s)) };
        let size = window.inner_size();
        let renderer = Renderer::new(gl, dialect, (size.width.max(1), size.height.max(1)), self.deck.size())?;
        eprintln!("gl: {dialect:?}, screen {}x{}, panel {}", size.width, size.height, self.deck.size());
        Ok(Gpu { window, surface, context, renderer })
    }

    /// Advance the deck to now and draw.
    fn tick(&mut self) {
        let Some(gpu) = &mut self.gpu else { return };
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().min(0.1);
        self.last = now;
        self.deck.update(dt, &mut self.frame);
        self.phosphor.update(&self.frame, dt);
        gpu.renderer.upload(self.phosphor.bytes());
        gpu.renderer.draw(self.options.rotate, (now - self.start).as_secs_f32(), self.deck.look(), self.deck.progress());
        if let Err(e) = gpu.surface.swap_buffers(&gpu.context) {
            eprintln!("swap: {e}");
        }
    }

    /// Step to the asked time at 60 fps, save one frame, and stop.
    fn screenshot(&mut self, shot: &Screenshot) -> Result<()> {
        let gpu = self.gpu.as_mut().ok_or_else(|| anyhow!("no GPU"))?;
        let dt = 1.0 / 60.0;
        let steps = (shot.seconds / dt).round().max(1.0) as usize;
        for _ in 0..steps {
            self.deck.update(dt, &mut self.frame);
            self.phosphor.update(&self.frame, dt);
        }
        gpu.renderer.upload(self.phosphor.bytes());
        gpu.renderer.draw_offscreen(self.options.rotate, shot.seconds, self.deck.look(), self.deck.progress())?;
        let (w, h) = gpu.renderer.screen();
        let img = image::RgbaImage::from_raw(w, h, gpu.renderer.read_pixels()).ok_or_else(|| anyhow!("bad framebuffer size"))?;
        let img = match self.options.rotate % 4 {
            1 => image::imageops::rotate90(&img),
            2 => image::imageops::rotate180(&img),
            3 => image::imageops::rotate270(&img),
            _ => img,
        };
        img.save(&shot.path).with_context(|| format!("saving {}", shot.path.display()))?;
        eprintln!("saved {}", shot.path.display());
        Ok(())
    }

    /// A click or touch at a screen position: forward on the right half
    /// of the picture, back on the left half.
    fn tap(&mut self, at: PhysicalPosition<f64>) {
        let Some(gpu) = &self.gpu else { return };
        let (sw, sh) = gpu.renderer.screen();
        let (x, y) = ((at.x / sw as f64) as f32, (at.y / sh as f64) as f32);
        // Where the tap lands across the picture as the viewer sees it.
        let lx = match self.options.rotate % 4 {
            0 => x,
            1 => 1.0 - y,
            2 => 1.0 - x,
            _ => y,
        };
        if lx < 0.5 { self.deck.back() } else { self.deck.next() }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }
        match self.init(event_loop) {
            Ok(gpu) => self.gpu = Some(gpu),
            Err(e) => {
                self.result = Err(e);
                event_loop.exit();
                return;
            }
        }
        if let Some(shot) = self.options.screenshot.take() {
            self.result = self.screenshot(&shot);
            event_loop.exit();
            return;
        }
        self.start = Instant::now();
        self.last = self.start;
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
                    Key::Named(NamedKey::ArrowRight | NamedKey::Space) => self.deck.next(),
                    Key::Named(NamedKey::ArrowLeft) => self.deck.back(),
                    Key::Character("f") => {
                        if let Some(gpu) = &self.gpu {
                            let full = gpu.window.fullscreen().is_none().then_some(Fullscreen::Borderless(None));
                            gpu.window.set_fullscreen(full);
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

/// Prefer a config without multisampling: the dots are drawn in the shader.
fn pick_config(configs: Box<dyn Iterator<Item = Config> + '_>) -> Config {
    configs.reduce(|best, c| if c.num_samples() < best.num_samples() { c } else { best }).expect("at least one GL config")
}
