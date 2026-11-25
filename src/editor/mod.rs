//! The Formalin window.
//!
//! The whole background is an OpenGL scene (see `gl.rs` and `shaders/`):
//! a specimen tank with a glass orb you can grab, seen through a CRT the
//! effect damages. egui draws the text and the service panel on top.

mod gl;
mod orb;
mod panel;
mod style;

use crate::meters::Meters;
use crate::params::FormalinParams;
use gl::{SceneGl, Uniforms};
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Ui};
use nih_plug_egui::{create_egui_editor, EguiState};
use orb::Orb;
use std::sync::{Arc, Mutex};
use style::{display, sigil_font, Sigil, COOL, DIM, HOT, PAPER};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 450;
const PANEL_H: f32 = 236.0;

/// Orb radius in world units, and the camera it's seen through (must match scene.frag).
const ORB_R: f32 = 0.6;
const CAM_DIST: f32 = 3.0;
const FOCAL: f32 = 1.5;

pub fn default_state() -> Arc<EguiState> {
    EguiState::from_size(WIDTH, HEIGHT)
}

/// The GL renderer is created lazily on the first frame (we need the context),
/// and dropped whenever the window reopens with a fresh context.
enum Renderer {
    Pending,
    Ready(SceneGl),
    Failed(String),
}

struct Gui {
    renderer: Arc<Mutex<Renderer>>,
    orb: Orb,
    last_time: f64,
    world: f64,
    level: f32,
    dead: f32,
    panel_open: bool,
    panel_t: f32,
}

pub fn create(params: Arc<FormalinParams>, meters: Arc<Meters>) -> Option<Box<dyn Editor>> {
    let gui = Gui {
        renderer: Arc::new(Mutex::new(Renderer::Pending)),
        orb: Orb::new(),
        last_time: 0.0,
        world: 0.0,
        level: 0.0,
        dead: 0.0,
        panel_open: false,
        panel_t: 0.0,
    };
    create_egui_editor(
        params.editor_state.clone(),
        gui,
        |ctx, gui| {
            style::install_fonts(ctx);
            *gui.renderer.lock().unwrap() = Renderer::Pending;
        },
        move |ctx, setter, gui| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ctx, |ui| frame(ui, setter, gui, &params, &meters));
        },
    )
}

fn frame(ui: &mut Ui, setter: &ParamSetter, gui: &mut Gui, p: &FormalinParams, meters: &Meters) {
    let rect = ui.max_rect();
    let h = rect.height();
    let now = ui.input(|i| i.time);
    let dt = (now - gui.last_time).clamp(0.0, 0.1) as f32;
    gui.last_time = now;

    // -- What the audio thread is doing --
    let (peak, dsp_speed, lag_s) = meters.load();
    let (dsp_speed, lag_s) = debug_preview(gui, dsp_speed, lag_s);
    gui.level = (gui.level * (-dt * 5.0).exp()).max(peak.min(1.5));
    // Pitch applied after the tape slows (or speeds) the room itself, so the
    // tank's caustics, particles and ink run at that rate too. The orb, being
    // the tape, keeps its own speed.
    let room = (p.pitch.value() * p.pitch_pos.value() / 12.0).exp2();
    gui.world += (dt * dsp_speed * room) as f64;
    // Fixation sets in slowly once the tape is stopped, and clears faster.
    let dead_target = 1.0 - (dsp_speed.abs() * 5.0).min(1.0);
    let rate = if dead_target > gui.dead { 1.2 } else { 3.5 };
    gui.dead += (dead_target - gui.dead) * (1.0 - (-dt * rate).exp());
    let panel_target = if gui.panel_open { 1.0 } else { 0.0 };
    gui.panel_t += (panel_target - gui.panel_t) * (1.0 - (-dt * 10.0).exp());

    // -- The orb's place on screen (screen space: y up, units of height) --
    let pitch_n = p.pitch.value() / 12.0;
    let orb_center = [0.0, -0.02 + pitch_n * 0.08 + gui.panel_t * 0.26];
    let orb_r = ORB_R * (1.0 - 0.45 * gui.panel_t);
    let c3 = [orb_center[0] * CAM_DIST / FOCAL, orb_center[1] * CAM_DIST / FOCAL];
    let d = (c3[0] * c3[0] + c3[1] * c3[1] + CAM_DIST * CAM_DIST).sqrt();
    let screen_r = orb_r * FOCAL / (d * d - orb_r * orb_r).sqrt();
    let to_px = |sp: [f32; 2]| pos2(rect.center().x + sp[0] * h, rect.center().y - sp[1] * h);
    let center_px = to_px(orb_center);
    let radius_px = screen_r * h;

    let panel_rect = Rect::from_min_size(pos2(rect.left(), rect.bottom() - PANEL_H * gui.panel_t), vec2(rect.width(), PANEL_H));
    let orb_rect = Rect::from_center_size(center_px, vec2(radius_px * 2.0, radius_px * 2.0));
    let orb_resp = ui.interact(orb_rect, ui.id().with("orb"), Sense::click_and_drag());
    let over_panel = gui.panel_t > 0.01 && ui.input(|i| i.pointer.hover_pos()).is_some_and(|pos| panel_rect.contains(pos));
    if !over_panel {
        gui.orb.update(ui, &orb_resp, center_px, radius_px, dt, dsp_speed, p, setter);
    }
    let pointer = ui.input(|i| i.pointer.hover_pos()).unwrap_or(center_px);
    let mouse = [(pointer.x - rect.center().x) / h, -(pointer.y - rect.center().y) / h];
    let hovering_orb = pointer.distance(center_px) <= radius_px;
    ui.ctx().set_cursor_icon(if gui.orb.is_held() {
        egui::CursorIcon::Grabbing
    } else if hovering_orb && !over_panel {
        egui::CursorIcon::Grab
    } else {
        egui::CursorIcon::Default
    });

    // -- The scene --
    let wow = p.wow.value();
    let u = Uniforms {
        time: (now % 10_000.0) as f32,
        world: (gui.world % 1000.0) as f32,
        rot: gui.orb.rotation(wow * 0.12 * (now as f32 * 1.3).sin()),
        orb: orb_center,
        orb_r,
        orb_screen_r: screen_r,
        mouse,
        grab: gui.orb.grab_amount,
        squeeze: p.drive.value() / 24.0,
        wow,
        flutter: p.flutter.value(),
        drive: p.drive.value() / 24.0,
        age: p.age.value(),
        level: gui.level,
        dead: gui.dead,
        smear: p.smear.value(),
        dt,
        dt_world: dt * dsp_speed * room,
        // Ink lasts about as long as the reverb tail, measured in tape time.
        ink_keep: (-dt * dsp_speed.abs() / (0.15 + p.decay.value() * 0.5)).exp(),
        ink_emit: gui.level.min(1.0) * (0.2 + 1.8 * p.reverb_mix.value()) * dsp_speed.signum(),
        // Fresh tape: prints vanish in a blink. Old tape: they stay for ages.
        smudge_keep: (-dt / (0.3 + 40.0 * p.age.value().powi(2))).exp(),
    };
    let u = debug_preview_uniforms(u);
    let renderer = gui.renderer.clone();
    ui.painter().add(egui::PaintCallback {
        rect,
        callback: Arc::new(egui_glow::CallbackFn::new(move |info, painter| {
            let gl = painter.gl();
            let mut r = renderer.lock().unwrap();
            if let Renderer::Pending = *r {
                *r = match SceneGl::new(gl) {
                    Ok(s) => Renderer::Ready(s),
                    Err(e) => Renderer::Failed(e),
                };
            }
            if let Renderer::Ready(scene) = &mut *r {
                if let Err(e) = scene.paint(gl, &info, &u) {
                    *r = Renderer::Failed(e);
                }
            }
        })),
    });

    // -- Overlay --
    let painter = ui.painter();
    if let Renderer::Failed(e) = &*gui.renderer.lock().unwrap() {
        painter.text(rect.center(), Align2::CENTER_CENTER, format!("GL error:\n{e}"), FontId::monospace(11.0), Color32::RED);
    }

    specimen_label(painter, rect.left_top() + vec2(20.0, 18.0));

    let (sigil, mode, color) = if gui.dead > 0.6 {
        (Sigil::Fixed, "FIXED".to_owned(), HOT)
    } else if dsp_speed < -0.02 {
        (Sigil::Rewind, format!("REW  X{:.2}", -dsp_speed), COOL)
    } else if dsp_speed > 1.02 && lag_s > 0.005 {
        (Sigil::CatchUp, format!("CATCH-UP  X{dsp_speed:.2}"), COOL)
    } else {
        (Sigil::Play, format!("PLAY  X{dsp_speed:.2}"), PAPER)
    };
    let tr = rect.right_top() + vec2(-24.0, 24.0);
    let mode_rect = painter.text(tr, Align2::RIGHT_TOP, mode, display(18.0), color);
    painter.text(mode_rect.left_center() + vec2(-8.0, 0.0), Align2::RIGHT_CENTER, sigil.glyph(), sigil_font(24.0), color);
    painter.text(tr + vec2(0.0, 24.0), Align2::RIGHT_TOP, format!("LAG {lag_s:.2}s"), display(13.0), DIM);
    let readout = format!(
        "DRIVE {:.1}dB   PITCH {:+.2}st {}   {}",
        p.drive.value(),
        p.pitch.value(),
        p.pitch_pos.to_string().to_uppercase(),
        if p.tape_stop.value() { "TAPE STOP ON" } else { "" }
    );
    painter.text(tr + vec2(0.0, 44.0), Align2::RIGHT_TOP, readout, display(11.0), DIM);

    if gui.panel_t < 0.98 {
        let by = rect.bottom() - 18.0 - PANEL_H * gui.panel_t;
        let mut x = rect.left() + 24.0;
        let hints = [
            (Sigil::Hold, "HOLD TO STOP"),
            (Sigil::Spin, "SPIN TO SCRUB"),
            (Sigil::Squeeze, "RIGHT-DRAG TO SQUEEZE"),
            (Sigil::Sink, "SCROLL TO SINK"),
        ];
        for (sigil, hint) in hints {
            let g = painter.text(pos2(x, by), Align2::LEFT_CENTER, sigil.glyph(), sigil_font(16.0), DIM);
            let t = painter.text(pos2(g.right() + 5.0, by + 1.0), Align2::LEFT_CENTER, hint, display(11.0), DIM);
            x = t.right() + 16.0;
        }
        let link_pos = pos2(rect.right() - 24.0, by);
        let galley = painter.layout_no_wrap("[service panel]".into(), display(12.0), PAPER);
        let r = Align2::RIGHT_CENTER.anchor_size(link_pos, galley.size());
        let resp = ui.interact(r, ui.id().with("open-panel"), Sense::click());
        let c = if resp.hovered() { HOT } else { PAPER };
        painter.text(r.min, Align2::LEFT_TOP, "[service panel]", display(12.0), c);
        if resp.clicked() {
            gui.panel_open = !gui.panel_open;
        }
    }

    if gui.panel_t > 0.01 && panel::draw(ui, painter, panel_rect, p, setter) {
        gui.panel_open = false;
    }
}

/// Debug builds only: `FORMALIN_PREVIEW=panel,still,rew,held,hot,aged` (any
/// combination) forces states so the standalone can be screenshotted without
/// touching the mouse.
fn preview_has(flag: &str) -> bool {
    cfg!(debug_assertions)
        && std::env::var("FORMALIN_PREVIEW").is_ok_and(|v| v.split(',').any(|f| f.trim() == flag))
}

fn debug_preview(gui: &mut Gui, speed: f32, lag_s: f32) -> (f32, f32) {
    if preview_has("panel") {
        gui.panel_open = true;
    }
    if preview_has("still") {
        (0.0, 1.4)
    } else if preview_has("rew") {
        (-0.8, 2.1)
    } else {
        (speed, lag_s)
    }
}

fn debug_preview_uniforms(mut u: Uniforms) -> Uniforms {
    if preview_has("held") {
        u.grab = 1.0;
        u.mouse = [u.orb[0] + 0.06, u.orb[1] + 0.05];
    }
    if preview_has("hot") {
        u.drive = 1.0;
        u.squeeze = 1.0;
        u.level = 0.9;
        u.ink_emit = 2.0;
        u.smear = 1.0;
    }
    if preview_has("aged") {
        u.age = 1.0;
        u.smudge_keep = 1.0;
    }
    u
}

/// The title, as a typed museum specimen label stuck to the front of the tank.
fn specimen_label(painter: &Painter, at: Pos2) {
    const CARD: Color32 = Color32::from_rgb(0xe8, 0xe2, 0xcf);
    const TYPE: Color32 = Color32::from_rgb(0x22, 0x1f, 0x1a);
    const RED: Color32 = Color32::from_rgb(0x9a, 0x1b, 0x1b);
    let card = Rect::from_min_size(at, vec2(232.0, 70.0));
    painter.rect_filled(card.translate(vec2(2.0, 3.0)), 1.0, Color32::from_black_alpha(90));
    painter.rect_filled(card, 1.0, CARD);
    painter.rect_stroke(card.shrink(3.0), 0.0, egui::Stroke::new(1.0, TYPE.gamma_multiply(0.6)), egui::StrokeKind::Inside);
    let x = card.left() + 12.0;
    painter.text(pos2(x, card.top() + 10.0), Align2::LEFT_TOP, "FORMALIN", display(28.0), TYPE);
    painter.line_segment([pos2(x, card.top() + 42.0), pos2(card.right() - 12.0, card.top() + 42.0)], egui::Stroke::new(1.0, RED));
    painter.text(pos2(x, card.top() + 46.0), Align2::LEFT_TOP, "PRESERVATIVE FOR MAGNETIC TAPE", display(10.0), TYPE);
    painter.text(
        pos2(card.right() - 12.0, card.top() + 14.0),
        Align2::RIGHT_TOP,
        concat!("LOT ", env!("CARGO_PKG_VERSION")),
        FontId::monospace(9.0),
        RED,
    );
}
