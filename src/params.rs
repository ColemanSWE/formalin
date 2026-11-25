use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::Arc;

/// Parameter IDs (`#[id = ...]`) are what hosts save in projects.
/// Never change them once you've used the plugin in a real session.
#[derive(Params)]
pub struct FormalinParams {
    /// Window size, saved with the project. Not a host-automatable parameter.
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    // -- Transport wobble --
    #[id = "wow"]
    pub wow: FloatParam,
    #[id = "flutter"]
    pub flutter: FloatParam,

    // -- Tape saturation --
    #[id = "drive"]
    pub drive: FloatParam,
    #[id = "age"]
    pub age: FloatParam,

    // -- Pitch & transport --
    #[id = "pitch"]
    pub pitch: FloatParam,
    #[id = "stop"]
    pub tape_stop: BoolParam,
    #[id = "stoptime"]
    pub stop_time: FloatParam,
    /// The orb. 1 = normal, 0 = held still, negative = reverse.
    #[id = "speed"]
    pub speed: FloatParam,
    #[id = "catchup"]
    pub catch_up: EnumParam<CatchUp>,

    // -- Smeared reverb --
    #[id = "smear"]
    pub smear: FloatParam,
    #[id = "decay"]
    pub decay: FloatParam,
    #[id = "verbmix"]
    pub reverb_mix: FloatParam,

    #[id = "output"]
    pub output: FloatParam,
}

impl Default for FormalinParams {
    fn default() -> Self {
        Self {
            editor_state: crate::editor::default_state(),

            wow: percent("Wow", 0.3),
            flutter: percent("Flutter", 0.2),

            drive: FloatParam::new("Drive", 6.0, FloatRange::Linear { min: 0.0, max: 24.0 })
                .with_smoother(SmoothingStyle::Linear(50.0))
                .with_step_size(0.1)
                .with_unit(" dB"),
            age: percent("Age", 0.3),

            pitch: FloatParam::new("Pitch", 0.0, FloatRange::Linear { min: -12.0, max: 12.0 })
                .with_smoother(SmoothingStyle::Linear(80.0))
                .with_step_size(0.01)
                .with_unit(" st"),
            tape_stop: BoolParam::new("Tape Stop", false),
            stop_time: FloatParam::new(
                "Stop Time",
                1.5,
                FloatRange::Skewed { min: 0.1, max: 4.0, factor: FloatRange::skew_factor(-1.0) },
            )
            .with_unit(" s")
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            speed: FloatParam::new("Speed", 1.0, FloatRange::Linear { min: -2.0, max: 2.0 })
                .with_smoother(SmoothingStyle::Linear(30.0))
                .with_unit("x")
                .with_value_to_string(formatters::v2s_f32_rounded(2)),
            catch_up: EnumParam::new("Catch-up", CatchUp::Rush),

            smear: percent("Smear", 0.6),
            decay: FloatParam::new(
                "Decay",
                4.0,
                FloatRange::Skewed { min: 0.3, max: 15.0, factor: FloatRange::skew_factor(-1.5) },
            )
            .with_smoother(SmoothingStyle::Linear(100.0))
            .with_unit(" s")
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            reverb_mix: percent("Reverb Mix", 0.25),

            output: FloatParam::new(
                "Output",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-24.0),
                    max: util::db_to_gain(12.0),
                    factor: FloatRange::gain_skew_factor(-24.0, 12.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
        }
    }
}

/// What happens when the tape is back to normal speed but behind live.
#[derive(Enum, PartialEq, Clone, Copy)]
pub enum CatchUp {
    /// Play fast until caught up (a pitch-up swoop).
    Rush,
    /// Crossfade straight back to live.
    Snap,
}

fn percent(name: &str, default: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min: 0.0, max: 1.0 })
        .with_smoother(SmoothingStyle::Linear(50.0))
        .with_unit("%")
        .with_value_to_string(formatters::v2s_f32_percentage(0))
        .with_string_to_value(formatters::s2v_f32_percentage())
}
