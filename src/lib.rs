use nih_plug::prelude::*;
use std::sync::Arc;

mod dsp;
mod editor;
mod meters;
mod params;

use dsp::{Chain, FrameParams};
use meters::Meters;
use params::{CatchUp, FormalinParams};

pub struct Formalin {
    params: Arc<FormalinParams>,
    meters: Arc<Meters>,
    chain: Chain,
}

impl Default for Formalin {
    fn default() -> Self {
        Self {
            params: Arc::new(FormalinParams::default()),
            meters: Arc::new(Meters::default()),
            // Placeholder; rebuilt with the real sample rate in `initialize`.
            chain: Chain::new(44_100.0),
        }
    }
}

impl Plugin for Formalin {
    const NAME: &'static str = "Formalin";
    const VENDOR: &'static str = "Formalin";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        aux_input_ports: &[],
        aux_output_ports: &[],
        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.meters.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        // Allocation is fine here -- we're not on the audio thread yet.
        self.chain = Chain::new(buffer_config.sample_rate);
        context.set_latency_samples(self.chain.latency_samples());
        true
    }

    fn reset(&mut self) {
        self.chain.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let p = &self.params;
        let tape_stop = p.tape_stop.value();
        let stop_time_s = p.stop_time.value();
        let snap_back = p.catch_up.value() == CatchUp::Snap;
        let mut peak = 0.0f32;

        for mut frame in buffer.iter_samples() {
            let fp = FrameParams {
                wow: p.wow.smoothed.next(),
                flutter: p.flutter.smoothed.next(),
                drive_gain: util::db_to_gain(p.drive.smoothed.next()),
                age: p.age.smoothed.next(),
                semitones: p.pitch.smoothed.next(),
                smear: p.smear.smoothed.next(),
                decay_s: p.decay.smoothed.next(),
                reverb_mix: p.reverb_mix.smoothed.next(),
                tape_stop,
                stop_time_s,
                speed: p.speed.smoothed.next(),
                snap_back,
            };
            let out_gain = p.output.smoothed.next();

            let l = *frame.get_mut(0).unwrap();
            let r = *frame.get_mut(1).unwrap();
            let (l, r) = self.chain.process(l, r, &fp);
            let (l, r) = (l * out_gain, r * out_gain);
            peak = peak.max(l.abs()).max(r.abs());
            *frame.get_mut(0).unwrap() = l;
            *frame.get_mut(1).unwrap() = r;
        }

        self.meters.store(peak, self.chain.transport_speed(), self.chain.transport_lag_s());

        // Keep running on silent input so reverb tails and tape stops finish.
        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for Formalin {
    const CLAP_ID: &'static str = "audio.formalin.tape";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("A glass orb of tape: wow & flutter, saturation, pitch, smeared reverb, and a transport you can grab");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = Some("https://github.com/ColemanSWE/formalin/issues");
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Distortion,
        ClapFeature::Reverb,
    ];
}

impl Vst3Plugin for Formalin {
    const VST3_CLASS_ID: [u8; 16] = *b"FormalinTapeFx01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Distortion,
        Vst3SubCategory::Reverb,
    ];
}

nih_export_clap!(Formalin);
nih_export_vst3!(Formalin);
