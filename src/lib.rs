//! A crate containing the main code for the plugin and some helper functions.
//! GranularPlugin is the main plugin, using the NIH-plug framework to build to the VST3 and CLAP formats.
//! stat() is used for integration tests.
//! load_wav() and its float counterpart load samples from a .wav file.
//! write_wav() and its float counterpart write samples to a .wav file.
#![warn(missing_docs)]

extern crate core;

pub mod delay_buffer;
pub mod delay_line;
pub mod diffusion;
pub mod envelope;
pub mod filter;
pub mod grain;
pub mod interpolators;
pub mod lfo;
pub mod midi;
pub mod modulation;
pub mod multi_channel;
pub mod resample;
pub mod reverb;
pub mod samples;
pub mod saturation;
pub mod smoothers;
pub mod timing;

use samples::PhonicMode;
use std::num::NonZeroU32;

use crate::delay_line::StereoDelay;
use hound::{Error, SampleFormat, WavReader, WavSpec, WavWriter};
use nih_plug::prelude::*;
use std::sync::Arc;

/// The struct used for the main plugin.
/// # Attributes
/// * `params`: An Arc containing an instance of `GranularPluginParams`
/// * `delay`: An instance of `StereoDelay` storing the plugins delay processor
struct GranularPlugin {
    params: Arc<GranularPluginParams>,
    delay: StereoDelay,
}

/// The parameters for the main plugin, returned in an Arc type.
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

            let (processed_l, processed_r) = self.delay.process(left, right, true, true);
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

/// Function used in integration tests to ensure the code can be accessed from an external module
pub fn stat() -> i16 {
    200
}

/// loads a wav file from string path and returns a result type possibly containing a vector of integer samples
/// # Returns
/// * A result type containing either a vector of i16 samples or a hound error
/// # Parameters
/// * `path`: A string containing the relative path to the file to be loaded (must include .wav file extension)
pub fn load_wav(path: &str) -> Result<Vec<i16>, Error> {
    let mut reader = WavReader::open(path)
        .expect("Test audio should be in tests directory and have the path specified");
    let mut samples: Vec<i16> = vec![];

    // turbofish used to get samples as i16 type
    for sample in reader.samples::<i16>() {
        match sample {
            Ok(s) => samples.push(s),
            Err(e) => return Err(e),
        };
    }

    Ok(samples)
}

/// loads a wav file from string path and returns a result type possibly containing a vector of float samples
/// # Returns
/// * A result type containing either a vector of f32 samples or a hound error
/// # Parameters
/// * `path`: A string containing the relative path to the file to be loaded (must include .wav file extension)
pub fn load_wav_float(path: &str) -> Result<Vec<f32>, Error> {
    let mut reader = WavReader::open(path)
        .expect("Test audio should be in tests directory and have the path specified");
    let mut samples: Vec<f32> = vec![];

    // turbofish used to get samples as i16 type
    for sample in reader.samples::<f32>() {
        match sample {
            Ok(s) => samples.push(s),
            Err(e) => return Err(e),
        };
    }

    Ok(samples)
}

/// writes to a wav file at string path from integer samples
/// # Parameters
/// * `path`: A string containing the relative path to the file to be written to (must include .wav file extension)
/// * `samples`: A vector of i16 samples which will be written to the file
/// * `mode`: An enum variant determining whether sample vector is stereo or mono (interleaved or not)
pub fn write_wav(path: &str, samples: Vec<i16>, mode: PhonicMode) {
    let channels: u16 = match mode {
        PhonicMode::Mono => 1,
        PhonicMode::Stereo => 2,
    };

    let spec = WavSpec {
        channels,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec).expect("could not create writer");

    for sample in samples {
        writer
            .write_sample(sample)
            .expect("error occurred while writing sample");
    }
    writer.finalize().expect("issue with finalization")
}

/// writes to a wav file at string path from float samples
/// # Parameters
/// * `path`: A string containing the relative path to the file to be written to (must include .wav file extension)
/// * `samples`: A vector of f32 samples which will be written to the file
/// * `mode`: An enum variant determining whether sample vector is stereo or mono (interleaved or not)
pub fn write_wav_float(path: &str, samples: Vec<f32>, mode: PhonicMode) {
    let channels: u16 = match mode {
        PhonicMode::Mono => 1,
        PhonicMode::Stereo => 2,
    };

    let spec = WavSpec {
        channels,
        sample_rate: 44100,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };

    let mut writer = WavWriter::create(path, spec).expect("could not create writer");

    for sample in samples {
        writer
            .write_sample(sample)
            .expect("error occurred while writing sample");
    }
    writer.finalize().expect("issue with finalization")
}

/// Create a vector of floats distributed uniformly between a minimum and maximum in N channels. Returns a vector of length `channels`
pub fn distribute_uniform(channels: i8, min: f32, max: f32) -> Vec<f32> {
    let float_channels = channels as f32;
    let delta = max - min;
    (0..channels)
        .map(|ch_num| ((ch_num as f32 / float_channels) * (delta)) + min)
        .collect()
}

/// Create a vector of floats distributed exponentially between a minimum and maximum in N channels. Returns a vector of length `channels`
pub fn distribute_exponential(channels: i8, delay_base: f32) -> Vec<f32> {
    let float_channels = channels as f32;
    (0..channels)
        .map(|ch_num| 2.0_f32.powf(ch_num as f32 / float_channels) * delay_base)
        .collect()
}

nih_export_vst3!(GranularPlugin);
nih_export_clap!(GranularPlugin);

#[cfg(test)]
mod tests {
    use crate::delay_line::StereoDelay;
    use crate::multi_channel::MultiDelayLine;
    use crate::samples::{IntSamples, PhonicMode, Samples};
    use crate::timing::{NoteModifier, TimeDiv, Timing};
    use crate::{load_wav, write_wav};
    use ndarray::Array1;
    use test_case::test_case;

    // Reverb Algorithm
    #[test]
    #[ignore]
    fn test_multi_channel_delay() {
        let mut in_samples = load_wav("tests/kalimba.wav").unwrap();
        in_samples.extend_from_slice(&[0; (44100 * 6)]);

        let mut delay = MultiDelayLine::new(
            vec![0.03237569, 0.05574729, 0.05872747, 0.08126467],
            0.8,
            0.25,
            4,
            44100,
        );

        let mut out_samples = Vec::new();
        for sample in in_samples.iter_mut() {
            let sample_vec = Array1::from(vec![*sample as f32; 4]);
            let out_sample = delay.process_with_feedback(sample_vec, true);
            let summed: f32 = out_sample.iter().sum();
            out_samples.push(summed as i16 / 4);
        }

        write_wav(
            "tests/kalimba_2_series.wav",
            out_samples,
            PhonicMode::Stereo,
        )
    }
    // Delay Algorithm
    #[test_case(
        "tests/kalimba_filter_5KHz_delay.wav",
        Timing::new(TimeDiv::Quarter, 80, NoteModifier::Regular),
        Timing::new(TimeDiv::Quarter, 80, NoteModifier::Dotted),
        0.65,
        0.45;
        "amen break through delay with feedback filter. Eighth and dotted eighth times. 55% feedback 45% mix"
    )]
    #[ignore]
    /// Test which renders the effects of the delay algorithm to a file based on an input file
    fn test_delay(
        filename: &str,
        timing_left: Timing,
        timing_right: Timing,
        feedback: f32,
        mix: f32,
    ) {
        // the delay times are chosen based on time divisions at the tempo of the audio being processed
        let mut delay = StereoDelay::new_sync(44100.0, timing_left, timing_right, feedback, mix);

        // creating a sample struct with the test audio (amen break)
        let in_samples =
            IntSamples::new(load_wav("tests/kalimba.wav").expect("error occurred loading file"));

        // initializing output vectors in stereo
        let mut out_l: Vec<i16> = Vec::new();
        let mut out_r: Vec<i16> = Vec::new();

        // process frames of stereo audio and write them to the output for left and right
        for (left, right) in in_samples.get_frames() {
            let (l, r) = delay.process(left as f32, right as f32, true, false);
            out_l.push(l as i16);
            out_r.push(r as i16);
        }

        // processing tail time seconds worth of 0s to capture the tail of the delay
        for _ in 0..(44100 * 3) {
            let (l, r) = delay.process(0.0, 0.0, true, false);
            out_l.push(l as i16);
            out_r.push(r as i16);
        }

        // initialize new sample vector from stereo inputs. from stereo interleaves the samples into a single vector
        let out_samples = IntSamples::from_stereo(&out_l, &out_r);
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

    #[test]
    #[ignore]
    // Utility rather than an actual test
    fn strip_start() {
        let in_samples = load_wav("tests/sine.wav").unwrap();
        let mut out: Vec<i16> = Vec::new();
        let mut found_start = false;

        for sample in in_samples {
            if !found_start {
                if sample == 0 {
                    continue;
                } else {
                    found_start = true
                }
            } else {
                out.push(sample);
            }
        }
        let _stereo_samples = IntSamples::from_mono(&out);
        write_wav("tests/sine.wav", out, PhonicMode::Mono);
    }
}
