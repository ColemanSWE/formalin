//! Uses fonts from `assets/fonts/private/` when both are present (they're
//! git-ignored; see the README there), otherwise the bundled OFL fonts.

use std::path::Path;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(formalin_private_fonts)");
    println!("cargo::rerun-if-changed=assets/fonts/private");

    let dir = Path::new("assets/fonts/private");
    if dir.join("Corepix.ttf").is_file() && dir.join("Esoterica.ttf").is_file() {
        println!("cargo::rustc-cfg=formalin_private_fonts");
    }
}
