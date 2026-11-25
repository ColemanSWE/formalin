//! The service panel: every parameter as a plain early-web form control.
//! Deliberately unstyled next to the scene -- the access hatch, not the show.

use super::style::{display, sigil_font, Sigil, INK, LINK};
use crate::params::{CatchUp, FormalinParams};
use nih_plug::prelude::*;
use nih_plug_egui::egui::{pos2, vec2, Align2, Color32, FontId, Painter, Rect, Sense, Stroke, Ui};

const RULE: Color32 = Color32::from_rgb(0, 0, 0);
const GROOVE: Color32 = Color32::from_rgb(0x9a, 0x9a, 0x96);

/// Draws the panel in `rect`. Returns true if [close] was clicked.
pub fn draw(ui: &Ui, painter: &Painter, rect: Rect, p: &FormalinParams, setter: &ParamSetter) -> bool {
    painter.rect_filled(rect, 0.0, Color32::WHITE);
    painter.line_segment([rect.left_top(), rect.right_top()], Stroke::new(2.0, RULE));

    let head_y = rect.top() + 18.0;
    let t = painter.text(pos2(rect.left() + 14.0, head_y), Align2::LEFT_CENTER, "SERVICE PANEL", display(18.0), INK);
    painter.text(
        pos2(t.right() + 12.0, head_y + 1.0),
        Align2::LEFT_CENTER,
        "for when the orb is not enough",
        FontId::proportional(12.0),
        Color32::from_gray(110),
    );
    let close = link(ui, painter, pos2(rect.right() - 14.0, head_y), "[close]", Align2::RIGHT_CENTER);
    if link(ui, painter, pos2(rect.right() - 74.0, head_y), "[reset all]", Align2::RIGHT_CENTER) {
        reset(setter, &p.wow);
        reset(setter, &p.flutter);
        reset(setter, &p.drive);
        reset(setter, &p.age);
        reset(setter, &p.pitch);
        reset(setter, &p.pitch_pos);
        reset(setter, &p.output);
        reset(setter, &p.smear);
        reset(setter, &p.decay);
        reset(setter, &p.reverb_mix);
        reset(setter, &p.stop_time);
        reset(setter, &p.speed);
        reset(setter, &p.tape_stop);
        reset(setter, &p.catch_up);
    }

    // Grouped like HTML <fieldset>s, read in signal-chain order:
    //   row 1: pitch -> wobble -> tape
    //   row 2: reverb -> transport -> output
    // (Pitch Position decides whether the pitch shift lands before the tape,
    // after the reverb, or split between them.)
    let inner = Rect::from_min_max(pos2(rect.left() + 12.0, rect.top() + 40.0), pos2(rect.right() - 12.0, rect.bottom() - 10.0));
    let row_gap = 14.0;
    let row_h = (inner.height() - row_gap) / 2.0;
    let row1 = Rect::from_min_size(inner.min, vec2(inner.width(), row_h));
    let row2 = Rect::from_min_size(pos2(inner.left(), inner.top() + row_h + row_gap), vec2(inner.width(), row_h));

    let [pitch, wobble, tape] = groups(row1, [2, 2, 2]);
    let [reverb, transport, out] = groups(row2, [3, 3, 1]);

    // Each control gets a sigil (see style::Sigil).
    let c = fieldset(painter, pitch, "PITCH", 2);
    slider(ui, painter, c[0], &p.pitch, setter, "pitch", Sigil::Pitch);
    slider(ui, painter, c[1], &p.pitch_pos, setter, "position", Sigil::PitchPos);

    let c = fieldset(painter, wobble, "WOBBLE", 2);
    slider(ui, painter, c[0], &p.wow, setter, "wow", Sigil::Wow);
    slider(ui, painter, c[1], &p.flutter, setter, "flutter", Sigil::Flutter);

    let c = fieldset(painter, tape, "TAPE", 2);
    slider(ui, painter, c[0], &p.drive, setter, "drive", Sigil::Drive);
    slider(ui, painter, c[1], &p.age, setter, "age", Sigil::Age);

    let c = fieldset(painter, reverb, "REVERB", 3);
    slider(ui, painter, c[0], &p.smear, setter, "smear", Sigil::Smear);
    slider(ui, painter, c[1], &p.decay, setter, "decay", Sigil::Decay);
    slider(ui, painter, c[2], &p.reverb_mix, setter, "wet/dry", Sigil::Wet);

    let c = fieldset(painter, transport, "TRANSPORT", 3);
    slider(ui, painter, c[0], &p.speed, setter, "speed", Sigil::Speed);
    slider(ui, painter, c[1], &p.stop_time, setter, "stop time", Sigil::StopTime);

    let c_out = fieldset(painter, out, "OUTPUT", 1);
    slider(ui, painter, c_out[0], &p.output, setter, "gain", Sigil::Output);

    // The transport's switches.
    let sw = c[2];
    let box_rect = Rect::from_min_size(pos2(sw.left() + 8.0, sw.top() + 9.0), vec2(12.0, 12.0));
    let hit = Rect::from_min_size(box_rect.min, vec2(sw.width() - 16.0, 14.0));
    let resp = ui.interact(hit, ui.id().with("svc-stop"), Sense::click());
    if resp.clicked() {
        setter.begin_set_parameter(&p.tape_stop);
        setter.set_parameter(&p.tape_stop, !p.tape_stop.value());
        setter.end_set_parameter(&p.tape_stop);
    }
    painter.rect_filled(box_rect, 0.0, Color32::WHITE);
    painter.rect_stroke(box_rect, 0.0, Stroke::new(1.0, RULE), egui_stroke_kind());
    if p.tape_stop.value() {
        painter.line_segment([box_rect.left_top() + vec2(2.0, 6.0), box_rect.center_bottom() + vec2(-1.0, -2.0)], Stroke::new(2.0, INK));
        painter.line_segment([box_rect.center_bottom() + vec2(-1.0, -2.0), box_rect.right_top() + vec2(-2.0, 2.0)], Stroke::new(2.0, INK));
    }
    painter.text(box_rect.right_center() + vec2(6.0, 0.0), Align2::LEFT_CENTER, "tape stop", FontId::proportional(14.0), INK);

    let y = sw.top() + 36.0;
    painter.text(pos2(sw.left() + 8.0, y), Align2::LEFT_CENTER, "catch-up:", FontId::proportional(14.0), INK);
    let rush = p.catch_up.value() == CatchUp::Rush;
    let mut x = sw.left() + 8.0;
    for (label, mode, on) in [("rush", CatchUp::Rush, rush), ("snap", CatchUp::Snap, !rush)] {
        let text = if on { format!("({label})") } else { label.to_owned() };
        let font = FontId::monospace(12.0);
        let galley = painter.layout_no_wrap(text.clone(), font.clone(), LINK);
        let r = Rect::from_min_size(pos2(x, y + 10.0), galley.size());
        if ui.interact(r, ui.id().with(("svc-cu", label)), Sense::click()).clicked() {
            setter.begin_set_parameter(&p.catch_up);
            setter.set_parameter(&p.catch_up, mode);
            setter.end_set_parameter(&p.catch_up);
        }
        painter.text(r.min, Align2::LEFT_TOP, text, font, if on { INK } else { LINK });
        if !on {
            painter.line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(1.0, LINK));
        }
        x = r.right() + 10.0;
    }

    close
}

/// Splits a row into N groups sized by how many controls each holds.
fn groups<const N: usize>(row: Rect, cells: [usize; N]) -> [Rect; N] {
    const GAP: f32 = 10.0;
    let total: usize = cells.iter().sum();
    let cell_w = (row.width() - GAP * (N as f32 - 1.0)) / total as f32;
    let mut x = row.left();
    cells.map(|n| {
        let r = Rect::from_min_size(pos2(x, row.top()), vec2(cell_w * n as f32, row.height()));
        x = r.right() + GAP;
        r
    })
}

/// A box with its legend set into the top border, like an HTML <fieldset>.
/// Returns `n` equal cells inside it, split by light dividers.
fn fieldset(painter: &Painter, rect: Rect, legend: &str, n: usize) -> Vec<Rect> {
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, RULE), egui_stroke_kind());
    let galley = painter.layout_no_wrap(legend.to_owned(), display(12.0), INK);
    let at = pos2(rect.left() + 8.0, rect.top() - galley.size().y / 2.0);
    painter.rect_filled(Rect::from_min_size(at - vec2(4.0, 0.0), galley.size() + vec2(8.0, 0.0)), 0.0, Color32::WHITE);
    painter.galley(at, galley, INK);

    let w = rect.width() / n as f32;
    (0..n)
        .map(|i| {
            let cell = Rect::from_min_size(pos2(rect.left() + w * i as f32, rect.top() + 4.0), vec2(w, rect.height() - 4.0));
            if i > 0 {
                painter.line_segment(
                    [pos2(cell.left(), rect.top() + 10.0), pos2(cell.left(), rect.bottom() - 8.0)],
                    Stroke::new(1.0, Color32::from_gray(200)),
                );
            }
            cell
        })
        .collect()
}

fn egui_stroke_kind() -> nih_plug_egui::egui::StrokeKind {
    nih_plug_egui::egui::StrokeKind::Inside
}

fn reset<P: Param>(setter: &ParamSetter, param: &P) {
    setter.begin_set_parameter(param);
    setter.set_parameter(param, param.default_plain_value());
    setter.end_set_parameter(param);
}

fn link(ui: &Ui, painter: &Painter, at: nih_plug_egui::egui::Pos2, text: &str, align: Align2) -> bool {
    let font = display(12.0);
    let galley = painter.layout_no_wrap(text.to_owned(), font.clone(), LINK);
    let r = align.anchor_size(at, galley.size());
    let resp = ui.interact(r, ui.id().with(("link", text)), Sense::click());
    let color = if resp.hovered() { Color32::from_rgb(0xee, 0, 0) } else { LINK };
    painter.text(r.min, Align2::LEFT_TOP, text, font, color);
    painter.line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(1.0, color));
    resp.clicked()
}

/// A plain horizontal slider: label, value, a groove and a black thumb.
/// Click or drag to set, double-click to reset.
fn slider<P: Param>(ui: &Ui, painter: &Painter, cell: Rect, param: &P, setter: &ParamSetter, label: &str, sigil: Sigil) {
    let track = Rect::from_min_max(pos2(cell.left() + 10.0, cell.bottom() - 18.0), pos2(cell.right() - 10.0, cell.bottom() - 14.0));
    let resp = ui.interact(cell, ui.id().with(("svc", label)), Sense::click_and_drag());

    let set_from_pointer = |setter: &ParamSetter| {
        if let Some(pos) = resp.interact_pointer_pos() {
            let v = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0);
            setter.set_parameter(param, param.preview_plain(v));
        }
    };
    if resp.drag_started() {
        setter.begin_set_parameter(param);
    }
    if resp.dragged() {
        set_from_pointer(setter);
    }
    if resp.drag_stopped() {
        setter.end_set_parameter(param);
    }
    if resp.double_clicked() {
        reset(setter, param);
    } else if resp.clicked() {
        setter.begin_set_parameter(param);
        set_from_pointer(setter);
        setter.end_set_parameter(param);
    }

    let v = param.unmodulated_normalized_value();
    painter.text(pos2(cell.left() + 8.0, cell.top() + 14.0), Align2::LEFT_CENTER, label, FontId::proportional(14.0), INK);
    painter.text(pos2(cell.left() + 8.0, cell.top() + 34.0), Align2::LEFT_CENTER, sigil.glyph(), sigil_font(18.0), INK);
    painter.text(
        pos2(cell.right() - 8.0, cell.top() + 34.0),
        Align2::RIGHT_CENTER,
        param.normalized_value_to_string(v, true),
        FontId::monospace(12.0),
        INK,
    );
    painter.rect_filled(track, 0.0, GROOVE);
    painter.line_segment([track.left_top(), track.right_top()], Stroke::new(1.0, Color32::from_gray(70)));
    let x = track.left() + v * track.width();
    let thumb = Rect::from_center_size(pos2(x, track.center().y), vec2(8.0, 16.0));
    painter.rect_filled(thumb, 0.0, if resp.hovered() || resp.dragged() { Color32::from_gray(40) } else { INK });
}
