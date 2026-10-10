//! GPU renderer.
//!
//! 1. `dots`: draw each dot as a round plasma dot, into `scene`.
//! 2. `down`: shrink `scene` to 1/4 size for bloom.
//! 3. `blur`: blur it (horizontal, vertical, twice).
//! 4. `composite`: scene + bloom, to the screen.
//!
//! The shaders are written for OpenGL ES 3.0, which is also WebGL 2. On a
//! desktop OpenGL 3.3 context the same source runs with another
//! `#version` line; see [`Dialect`].

use crate::frame::Size;
use crate::glass::Glass;
use crate::scene::Look;
use anyhow::{Result, anyhow};
use glow::HasContext;

const BLOOM_DIVISOR: u32 = 4;

/// Which GLSL the context speaks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dialect {
    /// OpenGL ES 3.0 and WebGL 2: `#version 300 es`.
    Es300,
    /// Desktop OpenGL 3.3 core: `#version 330 core`.
    Core330,
}

impl Dialect {
    fn header(self) -> &'static str {
        match self {
            Dialect::Es300 => "#version 300 es\n",
            Dialect::Core330 => "#version 330 core\n",
        }
    }
}

/// Everything about one drawn frame besides the dots.
#[derive(Clone, Copy, Debug)]
pub struct View {
    /// Quarter turns of the panel on the screen (0..3).
    pub rotate: u32,
    /// Seconds, for the shimmer.
    pub time: f32,
    pub look: Look,
    pub glass: Glass,
    /// The line under the panel (0..1), or none.
    pub progress: Option<f32>,
    pub camera: Camera,
}

impl Default for View {
    fn default() -> Self {
        Self { rotate: 0, time: 0.0, look: Look::WILLIAMS, glass: Glass::CLASSIC, progress: None, camera: Camera::default() }
    }
}

/// Where the viewer looks: the whole panel, or closer in. For videos and
/// effects; a display normally leaves it at its default.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Camera {
    /// 1 fits the panel to the screen; 4 is four times closer.
    pub zoom: f32,
    /// The point on the panel, in dots, at the center of the screen. `None`
    /// is the panel's center.
    pub focus: Option<[f32; 2]>,
}

impl Default for Camera {
    fn default() -> Self {
        Self { zoom: 1.0, focus: None }
    }
}

/// Where the panel sits on the screen: whole pixels per dot, centred.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Fit {
    /// Screen pixels per dot.
    pub pitch: f32,
    /// The panel's top-left corner, in pixels of the screen turned to landscape.
    pub origin: [f32; 2],
}

impl Fit {
    pub fn new(screen: (u32, u32), size: Size, rotate: u32) -> Fit {
        let (sx, sy) = (screen.0 as f32, screen.1 as f32);
        let land = if rotate % 2 == 1 { [sy, sx] } else { [sx, sy] };
        let pitch = (land[0] / size.w as f32).min(land[1] / size.h as f32).floor().max(1.0);
        let origin = [((land[0] - size.w as f32 * pitch) * 0.5).floor(), ((land[1] - size.h as f32 * pitch) * 0.5).floor()];
        Fit { pitch, origin }
    }

    /// Where the panel sits on the screen through a camera. With the default
    /// camera it is [`Fit::new`].
    pub fn with_camera(screen: (u32, u32), size: Size, rotate: u32, camera: Camera) -> Fit {
        let base = Fit::new(screen, size, rotate);
        if camera == Camera::default() {
            return base;
        }
        let (sx, sy) = (screen.0 as f32, screen.1 as f32);
        let land = if rotate % 2 == 1 { [sy, sx] } else { [sx, sy] };
        let pitch = base.pitch * camera.zoom.max(0.01);
        let focus = camera.focus.unwrap_or([size.w as f32 / 2.0, size.h as f32 / 2.0]);
        Fit { pitch, origin: [land[0] / 2.0 - focus[0] * pitch, land[1] / 2.0 - focus[1] * pitch] }
    }

    /// Where a point on the screen (pixels, top-left origin) is on the
    /// panel, in dots.
    pub fn to_panel(&self, screen: (u32, u32), rotate: u32, x: f32, y: f32) -> (f32, f32) {
        let (sx, sy) = (screen.0 as f32, screen.1 as f32);
        let q = match rotate % 4 {
            0 => (x, y),
            1 => (sy - y, x),
            2 => (sx - x, sy - y),
            _ => (y, sx - x),
        };
        ((q.0 - self.origin[0]) / self.pitch, (q.1 - self.origin[1]) / self.pitch)
    }
}

struct Target {
    fbo: glow::Framebuffer,
    tex: glow::Texture,
    w: u32,
    h: u32,
}

impl Target {
    unsafe fn delete(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_framebuffer(self.fbo);
            gl.delete_texture(self.tex);
        }
    }
}

pub struct Renderer {
    gl: glow::Context,
    vao: glow::VertexArray,
    frame_tex: glow::Texture,
    size: Size,
    dots: glow::Program,
    down: glow::Program,
    blur: glow::Program,
    composite: glow::Program,
    scene: Target,
    bloom: [Target; 2],
    /// An offscreen copy of the screen, for captures.
    shot: Option<Target>,
    screen: (u32, u32),
}

impl Renderer {
    /// A renderer for a `size` panel on a `screen` of that many pixels.
    pub fn new(gl: glow::Context, dialect: Dialect, screen: (u32, u32), size: Size) -> Result<Self> {
        let vert = include_str!("shaders/fullscreen.vert");
        unsafe {
            let dots = link(&gl, dialect, vert, include_str!("shaders/dots.frag"))?;
            let down = link(&gl, dialect, vert, include_str!("shaders/down.frag"))?;
            let blur = link(&gl, dialect, vert, include_str!("shaders/blur.frag"))?;
            let composite = link(&gl, dialect, vert, include_str!("shaders/composite.frag"))?;

            let frame_tex = texture(&gl, glow::RGBA8, size.w, size.h, glow::NEAREST)?;
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

            let (scene, bloom) = targets(&gl, screen)?;
            let vao = gl.create_vertex_array().map_err(|e| anyhow!(e))?;
            Ok(Self { gl, vao, frame_tex, size, dots, down, blur, composite, scene, bloom, shot: None, screen })
        }
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn screen(&self) -> (u32, u32) {
        self.screen
    }

    /// Draw a panel of another size from now on.
    pub fn set_size(&mut self, size: Size) -> Result<()> {
        if size == self.size {
            return Ok(());
        }
        unsafe {
            let tex = texture(&self.gl, glow::RGBA8, size.w, size.h, glow::NEAREST)?;
            self.gl.delete_texture(self.frame_tex);
            self.frame_tex = tex;
        }
        self.size = size;
        Ok(())
    }

    /// The screen changed size: remake the render targets.
    pub fn resize(&mut self, screen: (u32, u32)) -> Result<()> {
        if screen == self.screen || screen.0 == 0 || screen.1 == 0 {
            return Ok(());
        }
        unsafe {
            let (scene, bloom) = targets(&self.gl, screen)?;
            self.scene.delete(&self.gl);
            for b in &self.bloom {
                b.delete(&self.gl);
            }
            if let Some(s) = self.shot.take() {
                s.delete(&self.gl);
            }
            self.scene = scene;
            self.bloom = bloom;
        }
        self.screen = screen;
        Ok(())
    }

    /// Upload the dots: per dot, its own colour (0, 0, 0 for the look's)
    /// and brightness, as RGBA bytes, row-major, top row first. See
    /// [`Phosphor::rgba`](crate::Phosphor::rgba).
    pub fn upload(&self, rgba: &[u8]) {
        debug_assert_eq!(rgba.len(), self.size.dots() * 4);
        unsafe {
            self.gl.bind_texture(glow::TEXTURE_2D, Some(self.frame_tex));
            self.gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                self.size.w as i32,
                self.size.h as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(rgba)),
            );
        }
    }

    /// Draw one frame to the screen.
    pub fn draw(&self, view: &View) {
        unsafe { self.draw_to(None, view) }
    }

    /// Draw one frame offscreen, for [`Renderer::read_pixels`].
    pub fn draw_offscreen(&mut self, view: &View) -> Result<()> {
        unsafe {
            if self.shot.is_none() {
                self.shot = Some(target(&self.gl, self.screen.0, self.screen.1)?);
            }
            self.draw_to(self.shot.as_ref(), view);
        }
        Ok(())
    }

    unsafe fn draw_to(&self, out: Option<&Target>, view: &View) {
        let gl = &self.gl;
        let (sw, sh) = self.screen;
        let fit = Fit::with_camera(self.screen, self.size, view.rotate, view.camera);
        let g = &view.glass;
        unsafe {
            gl.bind_vertex_array(Some(self.vao));

            // 1. Dots.
            let p = self.dots;
            self.begin(p, Some(&self.scene));
            gl.uniform_2_f32(self.loc(p, "u_screen").as_ref(), sw as f32, sh as f32);
            gl.uniform_1_i32(self.loc(p, "u_rotate").as_ref(), (view.rotate % 4) as i32);
            gl.uniform_1_f32(self.loc(p, "u_time").as_ref(), view.time);
            gl.uniform_1_f32(self.loc(p, "u_pitch").as_ref(), fit.pitch);
            gl.uniform_2_f32(self.loc(p, "u_origin").as_ref(), fit.origin[0], fit.origin[1]);
            gl.uniform_1_f32(self.loc(p, "u_progress").as_ref(), view.progress.unwrap_or(-1.0));
            let c = view.look.color();
            gl.uniform_3_f32(self.loc(p, "u_color").as_ref(), c[0], c[1], c[2]);
            let palette: Vec<f32> = view.look.palette_steps().map_or(vec![0.0; 48], |p| p.iter().flatten().copied().collect());
            gl.uniform_3_f32_slice(self.loc(p, "u_palette").as_ref(), &palette);
            gl.uniform_1_f32(self.loc(p, "u_palette_weight").as_ref(), view.look.palette_weight());
            for (name, v) in [
                ("u_dot_size", g.dot_size),
                ("u_roundness", g.roundness),
                ("u_halo", g.halo),
                ("u_halo_width", g.halo_width.max(0.01)),
                ("u_unlit", g.unlit),
                ("u_variation", g.variation),
                ("u_flicker", g.flicker),
                ("u_gamma", g.gamma),
                ("u_hot_core", g.hot_core),
            ] {
                gl.uniform_1_f32(self.loc(p, name).as_ref(), v);
            }
            self.bind_tex(0, self.frame_tex);
            gl.draw_arrays(glow::TRIANGLES, 0, 3);

            // 2. Shrink.
            self.begin(self.down, Some(&self.bloom[0]));
            gl.uniform_2_f32(self.loc(self.down, "u_texel").as_ref(), 1.0 / sw as f32, 1.0 / sh as f32);
            self.bind_tex(0, self.scene.tex);
            gl.draw_arrays(glow::TRIANGLES, 0, 3);

            // 3. Blur: 0 -> 1 horizontal, 1 -> 0 vertical, twice.
            let (bw, bh) = (self.bloom[0].w as f32, self.bloom[0].h as f32);
            for _ in 0..2 {
                for (src, dst, step) in [(0, 1, (1.0 / bw, 0.0)), (1, 0, (0.0, 1.0 / bh))] {
                    self.begin(self.blur, Some(&self.bloom[dst]));
                    gl.uniform_2_f32(self.loc(self.blur, "u_step").as_ref(), step.0, step.1);
                    self.bind_tex(0, self.bloom[src].tex);
                    gl.draw_arrays(glow::TRIANGLES, 0, 3);
                }
            }

            // 4. Composite.
            self.begin(self.composite, out);
            gl.uniform_1_i32(self.loc(self.composite, "u_scene").as_ref(), 0);
            gl.uniform_1_i32(self.loc(self.composite, "u_bloom").as_ref(), 1);
            gl.uniform_1_f32(self.loc(self.composite, "u_bloom_strength").as_ref(), g.bloom);
            self.bind_tex(0, self.scene.tex);
            self.bind_tex(1, self.bloom[0].tex);
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }
    }

    /// Read back the last offscreen draw as RGBA, top row first.
    pub fn read_pixels(&self) -> Vec<u8> {
        let (w, h) = (self.screen.0 as usize, self.screen.1 as usize);
        let mut buf = vec![0u8; w * h * 4];
        unsafe {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, self.shot.as_ref().map(|t| t.fbo));
            self.gl.read_pixels(0, 0, w as i32, h as i32, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(Some(&mut buf)));
        }
        // GL rows start at the bottom; flip so row 0 is the top.
        buf.chunks_exact(w * 4).rev().flatten().copied().collect()
    }

    unsafe fn begin(&self, program: glow::Program, target: Option<&Target>) {
        let gl = &self.gl;
        unsafe {
            let (fbo, w, h) = match target {
                Some(t) => (Some(t.fbo), t.w, t.h),
                None => (None, self.screen.0, self.screen.1),
            };
            gl.bind_framebuffer(glow::FRAMEBUFFER, fbo);
            gl.viewport(0, 0, w as i32, h as i32);
            gl.use_program(Some(program));
        }
    }

    unsafe fn bind_tex(&self, unit: u32, tex: glow::Texture) {
        unsafe {
            self.gl.active_texture(glow::TEXTURE0 + unit);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        }
    }

    fn loc(&self, program: glow::Program, name: &str) -> Option<glow::UniformLocation> {
        unsafe { self.gl.get_uniform_location(program, name) }
    }
}

unsafe fn targets(gl: &glow::Context, screen: (u32, u32)) -> Result<(Target, [Target; 2])> {
    unsafe {
        let (bw, bh) = ((screen.0 / BLOOM_DIVISOR).max(1), (screen.1 / BLOOM_DIVISOR).max(1));
        let scene = target(gl, screen.0, screen.1)?;
        let bloom = [target(gl, bw, bh)?, target(gl, bw, bh)?];
        Ok((scene, bloom))
    }
}

unsafe fn texture(gl: &glow::Context, format: u32, w: u32, h: u32, filter: u32) -> Result<glow::Texture> {
    unsafe {
        let tex = gl.create_texture().map_err(|e| anyhow!(e))?;
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        for (k, v) in [
            (glow::TEXTURE_MIN_FILTER, filter),
            (glow::TEXTURE_MAG_FILTER, filter),
            (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
            (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
        ] {
            gl.tex_parameter_i32(glow::TEXTURE_2D, k, v as i32);
        }
        gl.tex_storage_2d(glow::TEXTURE_2D, 1, format, w as i32, h as i32);
        Ok(tex)
    }
}

unsafe fn target(gl: &glow::Context, w: u32, h: u32) -> Result<Target> {
    unsafe {
        let tex = texture(gl, glow::RGBA8, w, h, glow::LINEAR)?;
        let fbo = gl.create_framebuffer().map_err(|e| anyhow!(e))?;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(tex), 0);
        if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
            return Err(anyhow!("framebuffer {w}x{h} is incomplete"));
        }
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        Ok(Target { fbo, tex, w, h })
    }
}

unsafe fn link(gl: &glow::Context, dialect: Dialect, vert: &str, frag: &str) -> Result<glow::Program> {
    unsafe {
        let program = gl.create_program().map_err(|e| anyhow!(e))?;
        let mut shaders = Vec::new();
        for (kind, src) in [(glow::VERTEX_SHADER, vert), (glow::FRAGMENT_SHADER, frag)] {
            let shader = gl.create_shader(kind).map_err(|e| anyhow!(e))?;
            gl.shader_source(shader, &format!("{}{src}", dialect.header()));
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                return Err(anyhow!("shader compile failed:\n{}", gl.get_shader_info_log(shader)));
            }
            gl.attach_shader(program, shader);
            shaders.push(shader);
        }
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            return Err(anyhow!("shader link failed:\n{}", gl.get_program_info_log(program)));
        }
        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }
        Ok(program)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taps_land_on_the_right_dot_at_every_rotation() {
        let size = Size::new(80, 32);
        // A portrait screen with the panel turned a quarter, like the Pi.
        for (screen, rotate) in [((800, 400), 0), ((400, 800), 1), ((800, 400), 2), ((400, 800), 3)] {
            let fit = Fit::new(screen, size, rotate);
            assert_eq!(fit.pitch, 10.0);
            // The centre of the screen is the centre of the panel.
            let (x, y) = fit.to_panel(screen, rotate, screen.0 as f32 / 2.0, screen.1 as f32 / 2.0);
            assert_eq!((x, y), (40.0, 16.0), "rotate {rotate}");
        }
        // A camera at 2x on the panel's top-left corner puts that corner in
        // the middle of the screen.
        let close = Fit::with_camera((800, 400), size, 0, Camera { zoom: 2.0, focus: Some([0.0, 0.0]) });
        assert_eq!((close.pitch, close.origin), (20.0, [400.0, 200.0]));
        assert_eq!(Fit::with_camera((800, 400), size, 0, Camera::default()), Fit::new((800, 400), size, 0));
        // Rotated a quarter, the top of a portrait screen is the panel's right.
        let fit = Fit::new((400, 800), size, 1);
        let (x, _) = fit.to_panel((400, 800), 1, 200.0, 10.0);
        assert!(x > 70.0);
    }
}
