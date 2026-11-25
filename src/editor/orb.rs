//! The orb: a little physics object whose spin *is* the tape speed.
//!
//! - Hold it: your grip drags its spin to zero (a tape stop under your hand).
//! - Drag sideways while holding: it follows your hand (scrub; left = reverse).
//! - Let go: it keeps its momentum, then a "motor" pulls it back to normal speed.
//! - Right-drag up/down: squeeze it (Drive).
//! - Scroll over it: sink or raise it (Pitch).
//! - Double-click: toggle the Tape Stop switch.
//!
//! While you're touching it, the orb writes the Speed parameter (so the DAW can
//! record it). Otherwise it just mirrors what the audio thread is doing.

use crate::params::FormalinParams;
use nih_plug::prelude::*;
use nih_plug_egui::egui::{Pos2, Response, Ui};

/// Spin rate (rad/s) at normal tape speed.
pub const IDLE_OMEGA: f32 = 0.9;
/// How hard your hand grips: higher = the orb matches your hand faster.
const GRIP: f32 = 7.0;
/// How quickly a released orb spins back up to normal.
const MOTOR: f32 = 1.6;

pub struct Orb {
    yaw: f32,
    tilt: f32,
    tilt_vel: f32,
    omega: f32,
    hand_omega: f32,
    last_pos: Option<Pos2>,
    grabbed: bool,
    squeezing: bool,
    /// True while we hold a begin/end gesture on the Speed parameter.
    owning_speed: bool,
    /// Smoothed 0..1 for visuals.
    pub grab_amount: f32,
}

impl Orb {
    pub fn new() -> Self {
        Self {
            yaw: 0.0,
            tilt: 0.25,
            tilt_vel: 0.0,
            omega: IDLE_OMEGA,
            hand_omega: 0.0,
            last_pos: None,
            grabbed: false,
            squeezing: false,
            owning_speed: false,
            grab_amount: 0.0,
        }
    }

    /// Local -> world rotation, column-major for GLSL.
    pub fn rotation(&self, wobble: f32) -> [f32; 9] {
        let (s, c) = (-self.yaw).sin_cos();
        let (st, ct) = (self.tilt + wobble).sin_cos();
        [c, st * s, -ct * s, 0.0, ct, st, s, -st * c, ct * c]
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        ui: &Ui,
        resp: &Response,
        center: Pos2,
        radius: f32,
        dt: f32,
        dsp_speed: f32,
        p: &FormalinParams,
        setter: &ParamSetter,
    ) {
        let (primary, secondary, pos, origin) = ui.input(|i| {
            (i.pointer.primary_down(), i.pointer.secondary_down(), i.pointer.interact_pos(), i.pointer.press_origin())
        });
        let pressed_on_orb =
            resp.is_pointer_button_down_on() && origin.is_some_and(|o| o.distance(center) <= radius);

        // -- Grab / spin --
        if pressed_on_orb && primary {
            if !self.grabbed {
                self.grabbed = true;
                self.last_pos = pos;
                self.hand_omega = self.omega;
                if !self.owning_speed {
                    setter.begin_set_parameter(&p.speed);
                    self.owning_speed = true;
                }
            }
            if let (Some(now), Some(last)) = (pos, self.last_pos) {
                let d = now - last;
                let hand = if dt > 0.0 { d.x / radius / dt } else { 0.0 };
                self.hand_omega += (hand - self.hand_omega) * 0.35;
                self.tilt_vel += d.y / radius * 6.0;
                self.last_pos = Some(now);
            }
            self.omega += (self.hand_omega - self.omega) * (1.0 - (-dt * GRIP).exp());
        } else if self.grabbed {
            self.grabbed = false;
            self.last_pos = None;
        }

        if self.owning_speed {
            if !self.grabbed {
                self.omega += (IDLE_OMEGA - self.omega) * (1.0 - (-dt * MOTOR).exp());
            }
            if !self.grabbed && (self.omega - IDLE_OMEGA).abs() < 0.003 {
                self.omega = IDLE_OMEGA;
                setter.set_parameter(&p.speed, 1.0);
                setter.end_set_parameter(&p.speed);
                self.owning_speed = false;
            } else {
                setter.set_parameter(&p.speed, (self.omega / IDLE_OMEGA).clamp(-2.0, 2.0));
            }
        } else {
            // Not touching it: mirror the real transport (Tape Stop, rush, automation).
            self.omega = dsp_speed * IDLE_OMEGA;
        }

        // -- Squeeze (Drive) --
        if pressed_on_orb && secondary && !primary {
            if !self.squeezing {
                self.squeezing = true;
                self.last_pos = pos;
                setter.begin_set_parameter(&p.drive);
            }
            if let (Some(now), Some(last)) = (pos, self.last_pos) {
                let v = (p.drive.value() - (now.y - last.y) * 0.08).clamp(0.0, 24.0);
                setter.set_parameter(&p.drive, v);
                self.last_pos = Some(now);
            }
        } else if self.squeezing {
            self.squeezing = false;
            self.last_pos = None;
            setter.end_set_parameter(&p.drive);
        }

        // -- Sink (Pitch) --
        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let v = (p.pitch.value() + scroll * 0.01).clamp(-12.0, 12.0);
                setter.begin_set_parameter(&p.pitch);
                setter.set_parameter(&p.pitch, (v * 100.0).round() / 100.0);
                setter.end_set_parameter(&p.pitch);
            }
        }

        if resp.double_clicked() {
            setter.begin_set_parameter(&p.tape_stop);
            setter.set_parameter(&p.tape_stop, !p.tape_stop.value());
            setter.end_set_parameter(&p.tape_stop);
        }

        // -- Motion --
        self.yaw = (self.yaw + self.omega * dt) % std::f32::consts::TAU;
        self.tilt_vel += (-(self.tilt - 0.25) * 20.0 - self.tilt_vel * 3.0) * dt;
        self.tilt = (self.tilt + self.tilt_vel * dt).clamp(-0.9, 1.2);
        let target = if self.grabbed { 1.0 } else { 0.0 };
        self.grab_amount += (target - self.grab_amount) * (1.0 - (-dt * 12.0).exp());
    }

    pub fn is_held(&self) -> bool {
        self.grabbed || self.squeezing
    }
}
