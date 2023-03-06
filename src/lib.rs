extern crate core;

mod delay_buffer;
mod delay_line;
mod samples;

use samples::PhonicMode;
use std::num::NonZeroU32;

use crate::delay_line::{DelayLine, StereoDelay};
use hound;
use nih_plug::buffer::SamplesIter;
use nih_plug::plugin;
use nih_plug::prelude::*;
use std::sync::Arc;

struct GranularPlugin {
    params: Arc<GranularPluginParams>,
    delay: StereoDelay,
}

#[derive(Params)]
struct GranularPluginParams {
    /// The parameter's ID is used to identify the parameter in the wrappred plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.
    #[id = "gain"]
    pub gain: FloatParam,
}

impl Default for GranularPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(GranularPluginParams::default()),
            delay: StereoDelay::new(44100.0, 0.2, 0.3),
        }
    }
}

impl Default for GranularPluginParams {
    fn default() -> Self {
        Self {
            // This gain is stored as linear gain. NIH-plug comes with useful conversion functions
            // to treat these kinds of parameters as if we were dealing with decibels. Storing this
            // as decibels is easier to work with, but requires a conversion for every sample.
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(30.0),
                    // This makes the range appear as if it was linear when displaying the values as
                    // decibels
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            // Because the gain parameter is stored as linear gain instead of storing the value as
            // decibels, we need logarithmic smoothing
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            // There are many predefined formatters we can use here. If the gain was stored as
            // decibels instead of as a linear gain value, we could have also used the
            // `.with_step_size(0.1)` function to get internal rounding.
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
        }
    }
}

impl Plugin for GranularPlugin {
    const NAME: &'static str = "Granular Plugin";
    const VENDOR: &'static str = "Ben Ford";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "17bford@tythy.school";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[new_nonzero_u32(2)],
        ..AudioIOLayout::const_default()
    }];

    const DEFAULT_AUX_INPUTS: Option<AuxiliaryIOConfig> = None;
    const DEFAULT_AUX_OUTPUTS: Option<AuxiliaryIOConfig> = None;

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn accepts_bus_config(&self, config: &BusConfig) -> bool {
        // This works with any symmetrical IO layout
        config.num_input_channels == config.num_output_channels && config.num_input_channels > 0
    }

    fn initialize(
        &mut self,
        _bus_config: &BusConfig,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // Resize buffers and perform other potentially expensive initialization operations here.
        // The `reset()` function is always called right after this function. You can remove this
        // function if you do not need it.
        true
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for channel in buffer.iter_samples() {
            let gain = self.params.gain.smoothed.next();

            for sample in channel {
                *sample *= gain;
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for GranularPlugin {
    const CLAP_ID: &'static str = "com.your-domain.granular-plugin";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("A granular synthesis plugin with reverb, delay and modulation fx");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for GranularPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"GranularPluginBF";

    // And don't forget to change these categories, see the docstring on `VST3_CATEGORIES` for more
    // information
    const VST3_CATEGORIES: &'static str = "Fx|Dynamics";
}

pub fn stat() -> i16 {
    return 200;
}

pub fn load_wav(path: &str) -> Result<Vec<i16>, &str> {
    let mut reader = hound::WavReader::open(path)
        .expect("Test audio should be in tests directory and have the path specified");
    let mut samples: Vec<i16> = vec![];

    // turbofish used to get samples as i16 type
    for sample in reader.samples::<i16>() {
        match sample {
            Ok(s) => samples.push(s),
            Err(_) => return Err("Error occurred during file load"),
        };
    }

    Ok(samples)
}

pub fn write_wav(path: &str, samples: Vec<i16>, mode: PhonicMode) {
    let channels: u16 = match mode {
        PhonicMode::Mono => 1,
        PhonicMode::Stereo => 2,
    };

    // although the current system exclusively uses stereo channels and will create stereo from mono input by doubling,
    // mono writing could be useful for testing in the future.

    // all wav writing in the project will currently by done with i16 for convenience
    let spec = hound::WavSpec {
        channels,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec).expect("could not create writer");

    for sample in samples {
        writer
            .write_sample(sample)
            .expect("error occurred while writing sample");
    }
    writer.finalize().expect("issue with finalization")
}

nih_export_vst3!(GranularPlugin);
nih_export_clap!(GranularPlugin);

#[cfg(test)]
mod tests {
    use crate::samples::{PhonicMode, Samples};
    use crate::{delay_line, load_wav, samples, write_wav};
    use nih_plug::nih_debug_assert_eq;
    use nih_plug::prelude::NoteEvent;
    use test_case::test_case;

    // Reverb Algorithm
    // Delay Algorithm
    #[test]
    /// Test which renders the effects of the delay algorithm to a file based on an input file
    fn test_delay() {
        // the delay times are chosen based on time divisions at the tempo of the audio being processed
        let mut delay = delay_line::StereoDelay::new(44100.0, 0.21127, 0.10563);

        // creating a sample struct with the test audio (amen break)
        let in_samples = samples::IntSamples::new(
            load_wav("tests/amen_br.wav").expect("error occurred loading file"),
        );

        // initializing output vectors in stereo
        let mut out_l: Vec<i16> = Vec::new();
        let mut out_r: Vec<i16> = Vec::new();

        // process frames of stereo audio and write them to the output for left and right
        for (left, right) in in_samples.get_frames() {
            let (l, r) = delay.process(left as f64, right as f64);
            out_l.push(l as i16);
            out_r.push(r as i16);
        }

        // initialize new sample vector from stereo inputs. from stereo interleaves the samples into a single vector
        let out_samples = samples::IntSamples::from_stereo(out_l, out_r);
        write_wav(
            "tests/amen_br_stereo.wav",
            out_samples.samples(),
            PhonicMode::Stereo,
        );
    }

    // Modulation Algorithm
    // Granular Engine
    // Audio / MIDI basics
    // *   MIDI tests
    //     MIDI CC
    const TIMING: u32 = 5;

    #[test]
    fn midi_cc_conversion_correct() {
        let event: NoteEvent = NoteEvent::MidiCC {
            timing: TIMING,
            channel: 1,
            cc: 2,
            value: 0.5,
        };
        nih_debug_assert_eq!(
            NoteEvent::from_midi(TIMING, event.as_midi().unwrap()).unwrap(),
            event
        )
    }
    //     MIDI Note

    #[test_case(0.0 ; "minimum velocity value")]
    #[test_case(127.0 ; "maximum velocity value")]
    fn midi_note_on_conversion_correct(velocity: f32) {
        let event: NoteEvent = NoteEvent::NoteOn {
            timing: TIMING,
            voice_id: None,
            channel: 1,
            note: 100,
            velocity,
        };
        nih_debug_assert_eq!(
            NoteEvent::from_midi(TIMING, event.as_midi().unwrap()).unwrap(),
            event
        )
    }

    #[test_case(0.0 ; "minimum velocity value")]
    #[test_case(127.0 ; "maximum velocity value")]
    fn midi_note_off_conversion_correct(velocity: f32) {
        let event: NoteEvent = NoteEvent::NoteOff {
            timing: TIMING,
            voice_id: None,
            channel: 1,
            note: 100,
            velocity,
        };
        nih_debug_assert_eq!(
            NoteEvent::from_midi(TIMING, event.as_midi().unwrap()).unwrap(),
            event
        )
    }
    // * Audio testing
    //     Wav file loading
    #[test]
    fn wav_file_loads_correctly() {
        load_wav("tests/amen_br.wav").expect("wav file loaded incorrectly");
    }

    #[test]
    #[should_panic]
    fn wav_file_loads_incorrectly() {
        load_wav("doesnt/exist.wav").expect("wav file loaded incorrectly");
    }

    // GUI
}
