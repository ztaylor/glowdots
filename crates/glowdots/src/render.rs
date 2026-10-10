//! GPU renderer.
//!
//! 1. `dots`: draw each DMD value as a round plasma dot, into `scene`.
//! 2. `down`: shrink `scene` to 1/4 size for bloom.
//! 3. `blur`: blur it (horizontal, vertical, twice).
//! 4. `composite`: scene + bloom, to the screen.
//!
//! The shaders are written for OpenGL ES 3.0, which is also WebGL 2. On a
//! desktop OpenGL 3.3 context the same source runs with another
//! `#version` line; see [`Dialect`].

use crate::frame::Size;
use crate::scene::Look;
use anyhow::{Result, anyhow};
use glow::HasContext;

const BLOOM_DIVISOR: u32 = 4;
const BLOOM_STRENGTH: f32 = 0.8;

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
    /// An offscreen copy of the screen, for screenshots.
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

            let frame_tex = texture(&gl, glow::R8, size.w, size.h, glow::NEAREST)?;
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

    /// Upload dot brightness (0..=255), row-major, top row first.
    pub fn upload(&self, bytes: &[u8]) {
        debug_assert_eq!(bytes.len(), self.size.dots());
        unsafe {
            self.gl.bind_texture(glow::TEXTURE_2D, Some(self.frame_tex));
            self.gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                self.size.w as i32,
                self.size.h as i32,
                glow::RED,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(bytes)),
            );
        }
    }

    /// Draw one frame to the screen. `rotate` is quarter turns of the
    /// panel. `progress` (0..1) is the line under the panel; below 0 hides it.
    pub fn draw(&self, rotate: u32, time: f32, look: Look, progress: f32) {
        unsafe { self.draw_to(None, rotate, time, look, progress) }
    }

    /// Draw one frame offscreen, for [`Renderer::read_pixels`].
    pub fn draw_offscreen(&mut self, rotate: u32, time: f32, look: Look, progress: f32) -> Result<()> {
        unsafe {
            if self.shot.is_none() {
                self.shot = Some(target(&self.gl, self.screen.0, self.screen.1)?);
            }
            self.draw_to(self.shot.as_ref(), rotate, time, look, progress);
        }
        Ok(())
    }

    unsafe fn draw_to(&self, out: Option<&Target>, rotate: u32, time: f32, look: Look, progress: f32) {
        let gl = &self.gl;
        let (sw, sh) = self.screen;
        unsafe {
            gl.bind_vertex_array(Some(self.vao));

            // 1. Dots.
            self.begin(self.dots, Some(&self.scene));
            gl.uniform_2_f32(self.loc(self.dots, "u_screen").as_ref(), sw as f32, sh as f32);
            gl.uniform_1_i32(self.loc(self.dots, "u_rotate").as_ref(), (rotate % 4) as i32);
            gl.uniform_1_f32(self.loc(self.dots, "u_time").as_ref(), time);
            let c = look.color;
            gl.uniform_3_f32(self.loc(self.dots, "u_color").as_ref(), c[0], c[1], c[2]);
            gl.uniform_1_f32(self.loc(self.dots, "u_progress").as_ref(), progress);
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
            gl.uniform_1_f32(self.loc(self.composite, "u_bloom_strength").as_ref(), BLOOM_STRENGTH);
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
