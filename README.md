# Formalin

A glass orb of tape. CLAP + VST3, written in Rust with [nih-plug].

```
in -> pitch -> wow/flutter -> saturation -> smeared reverb -> transport -> out
```

The window is an OpenGL scene: a specimen tank holding a glass orb with
something alive inside, seen through a CRT that the effect damages. The orb's
spin *is* the tape speed.

| Gesture | Does |
|---|---|
| Hold the orb | Your grip drags it to a halt: a tape stop under your hand |
| Drag sideways while holding | Scrub; drag left to play backwards |
| Let go | It keeps its momentum, then spins back up and catches up to live |
| Right-drag up/down | Squeeze it (Drive) |
| Scroll over it | Sink or raise it (Pitch) |
| Double-click | Toggle the Tape Stop switch |

The orb writes the automatable **Speed** parameter, so the DAW can record a
performance. Everything else is in the **service panel** (bottom right).

The picture follows the sound: wow sways the image, flutter jitters lines,
drive splits colour, age clouds the water, smear leaves phosphor trails, and
a stopped tape dissolves to static. The tank's caustics and particles move on
tape time, so they slow, stop and run backwards with the audio.

| Param | What it does |
|---|---|
| Wow / Flutter | Slow wobble + fast jitter of tape speed (modulated delay) |
| Drive / Age | Asymmetric tanh saturation / high-end loss of worn tape |
| Pitch | Real-time pitch shift, -12..+12 semitones |
| Speed | Read-head speed, -2..2 (the orb). Below 1 builds lag behind live |
| Catch-up | Rush (play fast until caught up) or Snap (crossfade back to live) |
| Tape Stop / Stop Time | Coast the transport to a halt; automate it for drops |
| Smear / Decay / Reverb Mix | Diffused FDN reverb: blur amount, RT60, wet level |
| Output | Output gain |

## Build

```sh
cargo test --lib                          # DSP unit tests
cargo xtask bundle formalin --release    # -> target/bundled/
```

## Install (Windows)

Copy from `target/bundled/`:
- `Formalin.vst3` -> `C:\Program Files\Common Files\VST3\`
- `Formalin.clap` -> `C:\Program Files\Common Files\CLAP\`

Then rescan plugins in your DAW.

### Faster dev loop

Link the whole build folder into the VST3 folder once (admin PowerShell).
Hosts scan subfolders, so every `cargo xtask bundle` shows up, and the link
survives renames:

```powershell
New-Item -ItemType Junction -Path "C:\Program Files\Common Files\VST3\formalin-dev" -Target "<repo>\target\bundled"
```

Remove the plugin from your project before rebuilding (the DAW locks the DLL
while it's loaded), then re-add it.

For UI work you can skip the DAW entirely:

```sh
cargo run --features standalone --bin formalin_standalone -- --backend dummy
```

## Layout

- `src/lib.rs`: plugin glue (nih-plug traits, process loop)
- `src/params.rs`: parameters. **Don't change `#[id]`s once you use it in a project.**
- `src/dsp/`: plain-Rust DSP, one file per effect, no nih-plug types
- `src/meters.rs`: lock-free values the audio thread publishes for the GUI
- `src/editor/`: the window. `orb.rs` (physics + gestures), `gl.rs` (three
  render passes), `shaders/` (GLSL 150), `panel.rs` (service panel)
- `assets/fonts/`: Corepix (display) and Esoterica (a symbol font: A-L only)

Debug builds of the standalone accept `FORMALIN_PREVIEW=panel|still|rew` to
force a UI state for screenshots.

## Ideas / next steps

- **Hiss + head bump**: noise floor and a low-mid resonance in `saturation.rs`
- **Oversampling** around the saturator to cut aliasing at high drive
- **Modulated FDN delays** in the reverb for a more chorused, lush tail
- **Better pitch shifter**: pick grain splice points by autocorrelation (less warble)
- **More gestures**: shake to trigger flutter bursts, pinch to tear the membrane
- **MIDI**: a note-on grabs the orb, note-off lets go
- **Presets**, stored as named "specimens"

## License

Plugins that export VST3 through nih-plug link GPLv3 code (`vst3-sys`), so
this project is GPL-3.0-or-later. A CLAP-only build could use any license.

[nih-plug]: https://github.com/robbert-vdh/nih-plug
