//! Fonts, symbols and colours.
//!
//! Two font sets, chosen at build time by `build.rs`:
//! - default: Pirata One (display) + Noto Sans Symbols (alchemical sigils), both OFL
//! - `formalin_private_fonts`: your own Corepix + Esoterica from `assets/fonts/private/`

use nih_plug_egui::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontId};
use std::sync::Arc;

const DISPLAY: &str = "display";
const SIGILS: &str = "sigils";

#[cfg(not(formalin_private_fonts))]
const DISPLAY_TTF: &[u8] = include_bytes!("../../assets/fonts/PirataOne-Regular.ttf");
#[cfg(not(formalin_private_fonts))]
const SIGILS_TTF: &[u8] = include_bytes!("../../assets/fonts/NotoSansSymbols.ttf");

#[cfg(formalin_private_fonts)]
const DISPLAY_TTF: &[u8] = include_bytes!("../../assets/fonts/private/Corepix.ttf");
#[cfg(formalin_private_fonts)]
const SIGILS_TTF: &[u8] = include_bytes!("../../assets/fonts/private/Esoterica.ttf");

pub const HOT: Color32 = Color32::from_rgb(0xff, 0x2d, 0x9a);
pub const COOL: Color32 = Color32::from_rgb(0x2f, 0xd8, 0xff);
pub const PAPER: Color32 = Color32::from_rgb(0xeb, 0xea, 0xe6);
pub const DIM: Color32 = Color32::from_rgb(0x9a, 0xb8, 0xb4);
pub const INK: Color32 = Color32::from_rgb(0x11, 0x11, 0x12);
pub const LINK: Color32 = Color32::from_rgb(0x00, 0x00, 0xee);

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(DISPLAY.to_owned(), Arc::new(FontData::from_static(DISPLAY_TTF)));
    fonts.font_data.insert(SIGILS.to_owned(), Arc::new(FontData::from_static(SIGILS_TTF)));
    // Fall back to egui's fonts for any glyph ours don't have.
    let fallback = fonts.families[&FontFamily::Proportional].clone();
    for name in [DISPLAY, SIGILS] {
        let mut family = vec![name.to_owned()];
        family.extend(fallback.iter().cloned());
        fonts.families.insert(FontFamily::Name(name.into()), family);
    }
    ctx.set_fonts(fonts);
}

pub fn display(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(DISPLAY.into()))
}

pub fn sigil_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SIGILS.into()))
}

/// Every symbol the UI draws. Alchemical meanings in the default set; with the
/// private fonts each maps to one of Esoterica's A-L dingbats instead.
#[derive(Clone, Copy)]
pub enum Sigil {
    Play,
    Fixed,
    Rewind,
    CatchUp,
    Hold,
    Spin,
    Squeeze,
    Sink,
    Wow,
    Flutter,
    Drive,
    Age,
    Pitch,
    PitchPos,
    Smear,
    Decay,
    Wet,
    StopTime,
    Speed,
    Output,
}

impl Sigil {
    #[cfg(not(formalin_private_fonts))]
    pub fn glyph(self) -> &'static str {
        match self {
            Sigil::Play => "\u{1F71A}",    // gold (the sun)
            Sigil::Fixed => "\u{263D}",    // the moon (alchemical silver)
            Sigil::Rewind => "\u{1F704}",  // water, flowing down
            Sigil::CatchUp => "\u{1F701}", // air, rising
            Sigil::Hold => "\u{1F714}",    // salt: the principle of fixity
            Sigil::Spin => "\u{263F}",     // mercury: quicksilver
            Sigil::Squeeze => "\u{1F702}", // fire
            Sigil::Sink => "\u{1F703}",    // earth
            Sigil::Wow => "\u{1F704}",     // water
            Sigil::Flutter => "\u{1F701}", // air
            Sigil::Drive => "\u{1F702}",   // fire
            Sigil::Age => "\u{1F71E}",     // crocus of iron: rust, like tape oxide
            Sigil::Pitch => "\u{1F703}",   // earth
            Sigil::PitchPos => "\u{1F73A}", // two points joined: before <-> after
            Sigil::Smear => "\u{1F705}",   // aqua fortis
            Sigil::Decay => "\u{1F70D}",   // sulfur
            Sigil::Wet => "\u{1F708}",     // aqua vitae
            Sigil::StopTime => "\u{1F714}", // salt
            Sigil::Speed => "\u{263F}",    // mercury
            Sigil::Output => "\u{1F71A}",  // gold
        }
    }

    #[cfg(formalin_private_fonts)]
    pub fn glyph(self) -> &'static str {
        // Esoterica: A hand, B chalice, C sun face, D money bag, E clover, F cards,
        // G moon, H poison, I hexagram, J sunburst, K dice, L eye.
        match self {
            Sigil::Play => "J",
            Sigil::Fixed => "G",
            Sigil::Rewind => "L",
            Sigil::CatchUp => "C",
            Sigil::Hold => "A",
            Sigil::Spin => "K",
            Sigil::Squeeze => "H",
            Sigil::Sink => "G",
            Sigil::Wow => "G",
            Sigil::Flutter => "E",
            Sigil::Drive => "H",
            Sigil::Age => "F",
            Sigil::Pitch => "I",
            Sigil::PitchPos => "I",
            Sigil::Smear => "L",
            Sigil::Decay => "C",
            Sigil::Wet => "B",
            Sigil::StopTime => "A",
            Sigil::Speed => "K",
            Sigil::Output => "D",
        }
    }
}
