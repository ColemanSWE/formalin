//! Raw OpenGL rendering of the scene, run inside egui via a paint callback.
//!
//! Three passes each frame, all full-screen triangles:
//!
//! ```text
//! 1. scene.frag  the tank and the orb (raymarched)            -> scene texture
//! 2. ink.frag    feedback buffer: ink in the water and
//!                fingerprints on the glass, ping-ponged       -> ink texture
//! 3. post.frag   the front of the tank: haze, ink, grime,
//!                fixation                                     -> egui's framebuffer
//! ```
//!
//! Every fragment shader gets `#version 150` + `common.glsl` prepended.
//!
//! egui restores its own GL state after the callback, so we only need to
//! unbind what we bind.

use egui_glow::glow::{self, HasContext};
use nih_plug_egui::egui::PaintCallbackInfo;

const VERT: &str = include_str!("shaders/fullscreen.vert");
const SCENE: &str = include_str!("shaders/scene.frag");
const INK: &str = include_str!("shaders/ink.frag");
const COMMON: &str = include_str!("shaders/common.glsl");
const POST: &str = include_str!("shaders/post.frag");

/// Everything the shaders need for one frame. Plain data, copied into the callback.
#[derive(Clone, Copy, Default)]
pub struct Uniforms {
    pub time: f32,
    /// Tape time: integrates the transport's real speed, so the world slows,
    /// stops and runs backwards with the audio.
    pub world: f32,
    /// Orb rotation (column-major 3x3), local -> world.
    pub rot: [f32; 9],
    /// Orb center in screen space (y up, units of window height, origin at center).
    pub orb: [f32; 2],
    /// Orb radius in world units.
    pub orb_r: f32,
    /// Orb silhouette radius in screen space (same units as `orb`).
    pub orb_screen_r: f32,
    pub mouse: [f32; 2],
    pub grab: f32,
    pub squeeze: f32,
    pub wow: f32,
    pub flutter: f32,
    pub drive: f32,
    pub age: f32,
    pub level: f32,
    /// 0 = alive, 1 = stopped dead (static takes over).
    pub dead: f32,
    pub smear: f32,
    /// Wall-clock and tape-time seconds this frame (tape time is negative in reverse).
    pub dt: f32,
    pub dt_world: f32,
    /// Fraction of ink that survives this frame (from Decay).
    pub ink_keep: f32,
    /// Ink emission rate (level x wet/dry); negative pulls ink back in.
    pub ink_emit: f32,
    /// Fraction of fingerprint grease that survives this frame (from Age).
    pub smudge_keep: f32,
}

struct Target {
    fbo: glow::Framebuffer,
    tex: glow::Texture,
}

pub struct SceneGl {
    vao: glow::VertexArray,
    scene: glow::Program,
    ink: glow::Program,
    post: glow::Program,
    /// [scene, accum A, accum B]
    targets: Vec<Target>,
    size: (i32, i32),
    flip: bool,
}

impl SceneGl {
    pub fn new(gl: &glow::Context) -> Result<Self, String> {
        unsafe {
            Ok(Self {
                vao: gl.create_vertex_array()?,
                scene: program(gl, SCENE)?,
                ink: program(gl, INK)?,
                post: program(gl, POST)?,
                targets: Vec::new(),
                size: (0, 0),
                flip: false,
            })
        }
    }

    unsafe fn ensure_targets(&mut self, gl: &glow::Context, w: i32, h: i32) -> Result<(), String> {
        if self.size == (w, h) && !self.targets.is_empty() {
            return Ok(());
        }
        for t in self.targets.drain(..) {
            gl.delete_framebuffer(t.fbo);
            gl.delete_texture(t.tex);
        }
        for _ in 0..3 {
            let tex = gl.create_texture()?;
            gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            // Float targets: an 8-bit accumulator would never fully decay.
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA16F as i32,
                w,
                h,
                0,
                glow::RGBA,
                glow::HALF_FLOAT,
                glow::PixelUnpackData::Slice(None),
            );
            for (k, v) in [
                (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
                (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
                (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
                (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
            ] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, k, v as i32);
            }
            let fbo = gl.create_framebuffer()?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(tex), 0);
            if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
                return Err("offscreen framebuffer incomplete".into());
            }
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            self.targets.push(Target { fbo, tex });
        }
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        gl.bind_texture(glow::TEXTURE_2D, None);
        self.size = (w, h);
        Ok(())
    }

    pub fn paint(&mut self, gl: &glow::Context, info: &PaintCallbackInfo, u: &Uniforms) -> Result<(), String> {
        let vp = info.viewport_in_pixels();
        let (w, h) = (vp.width_px.max(1), vp.height_px.max(1));
        unsafe {
            self.ensure_targets(gl, w, h)?;
            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::BLEND);
            gl.bind_vertex_array(Some(self.vao));
            let res = [w as f32, h as f32];

            // 1. Scene
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.targets[0].fbo));
            gl.viewport(0, 0, w, h);
            let p = self.scene;
            gl.use_program(Some(p));
            u2(gl, p, "uRes", res);
            u1(gl, p, "uTime", u.time);
            u1(gl, p, "uWorld", u.world);
            u2(gl, p, "uOrb", u.orb);
            u1(gl, p, "uOrbR", u.orb_r);
            u1(gl, p, "uOrbScreenR", u.orb_screen_r);
            u2(gl, p, "uMouse", u.mouse);
            u1(gl, p, "uGrab", u.grab);
            u1(gl, p, "uSqueeze", u.squeeze);
            u1(gl, p, "uWow", u.wow);
            u1(gl, p, "uFlutter", u.flutter);
            u1(gl, p, "uAge", u.age);
            u1(gl, p, "uLevel", u.level);
            u1(gl, p, "uDead", u.dead);
            if let Some(l) = gl.get_uniform_location(p, "uRot") {
                gl.uniform_matrix_3_f32_slice(Some(&l), false, &u.rot);
            }
            gl.draw_arrays(glow::TRIANGLES, 0, 3);

            // 2. Ink + fingerprints (ping-pong)
            let (prev, cur) = if self.flip { (1, 2) } else { (2, 1) };
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.targets[cur].fbo));
            let p = self.ink;
            gl.use_program(Some(p));
            bind_tex(gl, p, "uPrev", 0, self.targets[prev].tex);
            u2(gl, p, "uRes", res);
            u1(gl, p, "uDt", u.dt);
            u1(gl, p, "uDtWorld", u.dt_world);
            u1(gl, p, "uWorld", u.world);
            u1(gl, p, "uKeep", u.ink_keep);
            u1(gl, p, "uEmit", u.ink_emit);
            u1(gl, p, "uSmear", u.smear);
            u1(gl, p, "uSmudgeKeep", u.smudge_keep);
            u1(gl, p, "uGrab", u.grab);
            u2(gl, p, "uOrb", u.orb);
            u1(gl, p, "uOrbScreenR", u.orb_screen_r);
            u2(gl, p, "uMouse", u.mouse);
            gl.draw_arrays(glow::TRIANGLES, 0, 3);

            // 3. Post, into egui's framebuffer
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(vp.left_px, vp.from_bottom_px, w, h);
            let clip = info.clip_rect_in_pixels();
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(clip.left_px, clip.from_bottom_px, clip.width_px, clip.height_px);
            let p = self.post;
            gl.use_program(Some(p));
            bind_tex(gl, p, "uTex", 0, self.targets[0].tex);
            bind_tex(gl, p, "uInk", 1, self.targets[cur].tex);
            u2(gl, p, "uRes", res);
            u2(gl, p, "uOrb", u.orb);
            u1(gl, p, "uOrbScreenR", u.orb_screen_r);
            u1(gl, p, "uTime", u.time);
            u1(gl, p, "uWorld", u.world);
            u1(gl, p, "uWow", u.wow);
            u1(gl, p, "uFlutter", u.flutter);
            u1(gl, p, "uDrive", u.drive);
            u1(gl, p, "uAge", u.age);
            u1(gl, p, "uDead", u.dead);
            gl.draw_arrays(glow::TRIANGLES, 0, 3);

            self.flip = !self.flip;
            gl.bind_vertex_array(None);
            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.use_program(None);
        }
        Ok(())
    }
}

unsafe fn program(gl: &glow::Context, frag: &str) -> Result<glow::Program, String> {
    let p = gl.create_program()?;
    let mut shaders = Vec::new();
    let frag = format!("#version 150
{COMMON}
{frag}");
    for (kind, src) in [(glow::VERTEX_SHADER, VERT), (glow::FRAGMENT_SHADER, frag.as_str())] {
        let s = gl.create_shader(kind)?;
        gl.shader_source(s, src);
        gl.compile_shader(s);
        if !gl.get_shader_compile_status(s) {
            return Err(format!("shader compile error:\n{}", gl.get_shader_info_log(s)));
        }
        gl.attach_shader(p, s);
        shaders.push(s);
    }
    gl.link_program(p);
    if !gl.get_program_link_status(p) {
        return Err(format!("program link error:\n{}", gl.get_program_info_log(p)));
    }
    for s in shaders {
        gl.detach_shader(p, s);
        gl.delete_shader(s);
    }
    Ok(p)
}

unsafe fn u1(gl: &glow::Context, p: glow::Program, name: &str, v: f32) {
    if let Some(l) = gl.get_uniform_location(p, name) {
        gl.uniform_1_f32(Some(&l), v);
    }
}

unsafe fn u2(gl: &glow::Context, p: glow::Program, name: &str, v: [f32; 2]) {
    if let Some(l) = gl.get_uniform_location(p, name) {
        gl.uniform_2_f32(Some(&l), v[0], v[1]);
    }
}

unsafe fn bind_tex(gl: &glow::Context, p: glow::Program, name: &str, unit: u32, tex: glow::Texture) {
    gl.active_texture(glow::TEXTURE0 + unit);
    gl.bind_texture(glow::TEXTURE_2D, Some(tex));
    if let Some(l) = gl.get_uniform_location(p, name) {
        gl.uniform_1_i32(Some(&l), unit as i32);
    }
}
