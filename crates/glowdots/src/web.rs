//! Play a deck in a web page (feature `web`, target `wasm32`): a canvas
//! with WebGL 2, drawn every animation frame at the screen's pixel
//! density. Taps on the canvas work like everywhere else; once the canvas
//! has focus (a tap gives it focus), the arrow keys, Enter, and letters go
//! to the deck.
//!
//! ```ignore
//! let canvas: web_sys::HtmlCanvasElement = /* from the page */;
//! let web = glowdots::web::WebPlayer::start(Player::new(deck), canvas)?;
//! // Later, from a button:
//! web.with(|p| p.deck.next());
//! ```
//!
//! The page should size the canvas with CSS; the drawing buffer follows it.

#![cfg(all(feature = "web", target_arch = "wasm32"))]

use crate::player::Player;
use crate::render::{Dialect, Renderer};
use crate::scene::Key;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, KeyboardEvent, PointerEvent, WebGl2RenderingContext};

/// A callback for `requestAnimationFrame`.
type FrameCallback = Closure<dyn FnMut(f64)>;

/// The highest pixel density drawn, to keep big screens fast.
const MAX_DPR: f64 = 2.0;

struct State {
    player: Player,
    renderer: Renderer,
    canvas: HtmlCanvasElement,
    /// The last animation frame's time stamp, in milliseconds.
    last: Option<f64>,
}

impl State {
    /// The drawing buffer size for the canvas's size on the page.
    fn backing(canvas: &HtmlCanvasElement) -> (u32, u32) {
        let dpr = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio()).min(MAX_DPR);
        let w = (canvas.client_width() as f64 * dpr).round().max(1.0) as u32;
        let h = (canvas.client_height() as f64 * dpr).round().max(1.0) as u32;
        (w, h)
    }

    fn frame(&mut self, stamp: f64) {
        let dt = self.last.map_or(1.0 / 60.0, |l| ((stamp - l) / 1000.0).clamp(0.0, 0.1)) as f32;
        self.last = Some(stamp);
        let size = Self::backing(&self.canvas);
        if size != self.renderer.screen() {
            self.canvas.set_width(size.0);
            self.canvas.set_height(size.1);
            if let Err(e) = self.renderer.resize(size) {
                web_sys::console::error_1(&format!("resize: {e}").into());
            }
        }
        self.player.step(dt);
        self.player.draw(&self.renderer);
    }
}

/// A player running in a canvas. Clones share the same player.
#[derive(Clone)]
pub struct WebPlayer {
    state: Rc<RefCell<State>>,
}

impl WebPlayer {
    /// Start playing `player` in `canvas`, and keep playing every animation
    /// frame for as long as the page is open.
    pub fn start(player: Player, canvas: HtmlCanvasElement) -> Result<WebPlayer, JsValue> {
        let options = js_sys::Object::new();
        for (k, v) in [("antialias", false), ("alpha", false), ("depth", false), ("stencil", false)] {
            js_sys::Reflect::set(&options, &k.into(), &v.into())?;
        }
        let gl = canvas
            .get_context_with_context_options("webgl2", &options)?
            .ok_or_else(|| JsValue::from_str("this browser has no WebGL 2"))?
            .dyn_into::<WebGl2RenderingContext>()?;
        let screen = State::backing(&canvas);
        canvas.set_width(screen.0);
        canvas.set_height(screen.1);
        let renderer = Renderer::new(glow::Context::from_webgl2_context(gl), Dialect::Es300, screen, player.deck.size())
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let state = Rc::new(RefCell::new(State { player, renderer, canvas: canvas.clone(), last: None }));
        let web = WebPlayer { state };
        web.listen(&canvas)?;
        web.animate();
        Ok(web)
    }

    /// Do something with the player: change the glass, go to a scene...
    pub fn with<R>(&self, f: impl FnOnce(&mut Player) -> R) -> R {
        f(&mut self.state.borrow_mut().player)
    }

    fn listen(&self, canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
        let state = self.state.clone();
        let tap = Closure::<dyn FnMut(PointerEvent)>::new(move |e: PointerEvent| {
            let mut s = state.borrow_mut();
            let _ = s.canvas.focus();
            let dpr = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio()).min(MAX_DPR) as f32;
            let screen = s.renderer.screen();
            s.player.tap(screen, e.offset_x() as f32 * dpr, e.offset_y() as f32 * dpr);
        });
        canvas.add_event_listener_with_callback("pointerdown", tap.as_ref().unchecked_ref())?;
        tap.forget();

        let state = self.state.clone();
        let keys = Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
            let key = match e.key().as_str() {
                "ArrowRight" | " " => Key::Right,
                "ArrowLeft" => Key::Left,
                "ArrowUp" => Key::Up,
                "ArrowDown" => Key::Down,
                "Enter" => Key::Select,
                k if k.chars().count() == 1 => Key::Char(k.chars().next().unwrap_or(' ')),
                _ => return,
            };
            e.prevent_default();
            state.borrow_mut().player.key(key);
        });
        canvas.add_event_listener_with_callback("keydown", keys.as_ref().unchecked_ref())?;
        keys.forget();
        if canvas.tab_index() < 0 {
            canvas.set_tab_index(0);
        }
        Ok(())
    }

    fn animate(&self) {
        // The callback asks for the next frame from inside itself, so it
        // holds a handle to itself.
        let slot: Rc<RefCell<Option<FrameCallback>>> = Rc::new(RefCell::new(None));
        let again = slot.clone();
        let state = self.state.clone();
        *slot.borrow_mut() = Some(Closure::new(move |stamp: f64| {
            state.borrow_mut().frame(stamp);
            if let (Some(w), Some(cb)) = (web_sys::window(), again.borrow().as_ref()) {
                let _ = w.request_animation_frame(cb.as_ref().unchecked_ref());
            }
        }));
        if let (Some(w), Some(cb)) = (web_sys::window(), slot.borrow().as_ref()) {
            let _ = w.request_animation_frame(cb.as_ref().unchecked_ref());
        }
    }
}
