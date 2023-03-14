extern crate core;

mod delay_buffer;
mod delay_line;
mod filter;
mod samples;

use samples::PhonicMode;
use std::num::NonZeroU32;

use crate::delay_line::StereoDelay;
use hound;
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
            delay: StereoDelay::new(44100.0, 0.2, 0.3, 0.4, 0.5),
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

    const MIDI_INPUT: MidiConfig = MidiConfig::None;

    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();

    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
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
        for mut channel_samples in buffer.iter_samples() {
            let left = *channel_samples.get_mut(0).unwrap();
            let right = *channel_samples.get_mut(1).unwrap();

            let (processed_l, processed_r) = self.delay.process(left, right);
            *channel_samples.get_mut(0).unwrap() = processed_l;
            *channel_samples.get_mut(1).unwrap() = processed_r;
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

    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Delay];
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
    use crate::delay_line::TimeDiv;
    use crate::samples::{PhonicMode, Samples};
    use crate::{delay_line, load_wav, samples, write_wav};
    use test_case::test_case;

    // Reverb Algorithm
    // Delay Algorithm
    #[test_case(
        "tests/kalimba_filter_5KHz_delay.wav",
        80,
        TimeDiv::Sixteenth,
        false,
        TimeDiv::Eighth,
        true,
        0.65,
        0.45;
        "amen break through delay with feedback filter. Eighth and dotted eighth times. 55% feedback 45% mix"
    )]
    /// Test which renders the effects of the delay algorithm to a file based on an input file
    fn test_delay(
        filename: &str,
        bpm: i32,
        delay_div_left: TimeDiv,
        dotted_left: bool,
        delay_div_right: TimeDiv,
        dotted_right: bool,
        feedback: f32,
        mix: f32,
    ) {
        // the delay times are chosen based on time divisions at the tempo of the audio being processed
        let mut delay = delay_line::StereoDelay::new_sync(
            44100.0,
            bpm,
            delay_div_left,
            dotted_left,
            delay_div_right,
            dotted_right,
            feedback,
            mix,
        );

        // creating a sample struct with the test audio (amen break)
        let in_samples = samples::IntSamples::new(
            load_wav("tests/kalimba.wav").expect("error occurred loading file"),
        );

        // initializing output vectors in stereo
        let mut out_l: Vec<i16> = Vec::new();
        let mut out_r: Vec<i16> = Vec::new();

        // process frames of stereo audio and write them to the output for left and right
        for (left, right) in in_samples.get_frames() {
            let (l, r) = delay.process(left as f32, right as f32);
            out_l.push(l as i16);
            out_r.push(r as i16);
        }

        // processing tail time seconds worth of 0s to capture the tail of the delay
        for _ in 0..(44100 * 3) {
            let (l, r) = delay.process(0.0, 0.0);
            out_l.push(l as i16);
            out_r.push(r as i16)
        }

        // initialize new sample vector from stereo inputs. from stereo interleaves the samples into a single vector
        let out_samples = samples::IntSamples::from_stereo(out_l, out_r);
        write_wav(filename, out_samples.samples(), PhonicMode::Stereo);
    }

    // Modulation Algorithm
    // Granular Engine
    // Audio / MIDI basics
    // *   MIDI tests
    //     MIDI CC
    // * Audio testing
    //     Wav file loading
    #[test]
    #[ignore]
    fn wav_file_loads_correctly() {
        load_wav("tests/amen_br.wav").expect("wav file loaded incorrectly");
    }

    #[test]
    #[should_panic]
    #[ignore]
    fn wav_file_loads_incorrectly() {
        load_wav("doesnt/exist.wav").expect("wav file loaded incorrectly");
    }

    // GUI
}
