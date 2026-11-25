//! Fonts and colours.

use nih_plug_egui::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontId};
use std::sync::Arc;

pub const COREPIX: &str = "corepix";
pub const ESOTERICA: &str = "esoterica";

pub const HOT: Color32 = Color32::from_rgb(0xff, 0x2d, 0x9a);
pub const COOL: Color32 = Color32::from_rgb(0x2f, 0xd8, 0xff);
pub const PAPER: Color32 = Color32::from_rgb(0xeb, 0xea, 0xe6);
pub const DIM: Color32 = Color32::from_rgb(0x9a, 0xb8, 0xb4);
pub const INK: Color32 = Color32::from_rgb(0x11, 0x11, 0x12);
pub const LINK: Color32 = Color32::from_rgb(0x00, 0x00, 0xee);

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        COREPIX.to_owned(),
        Arc::new(FontData::from_static(include_bytes!("../../assets/fonts/Corepix.ttf"))),
    );
    fonts.font_data.insert(
        ESOTERICA.to_owned(),
        Arc::new(FontData::from_static(include_bytes!("../../assets/fonts/Esoterica.ttf"))),
    );
    // Fall back to egui's fonts for glyphs ours don't have (arrows, symbols).
    let fallback = fonts.families[&FontFamily::Proportional].clone();
    for name in [COREPIX, ESOTERICA] {
        let mut family = vec![name.to_owned()];
        family.extend(fallback.iter().cloned());
        fonts.families.insert(FontFamily::Name(name.into()), family);
    }
    ctx.set_fonts(fonts);
}

pub fn corepix(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(COREPIX.into()))
}

pub fn esoterica(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(ESOTERICA.into()))
}
