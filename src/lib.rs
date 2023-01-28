use nih_plug::prelude::*;
use std::sync::Arc;

struct GranularPlugin {
    params: Arc<GranularPluginParams>,
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
    const DESCRIPTION: &'static str = "A granular plugin";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const DEFAULT_INPUT_CHANNELS: u32 = 2;
    const DEFAULT_OUTPUT_CHANNELS: u32 = 2;

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
        for channel_samples in buffer.iter_samples() {
            // Smoothing is optionally built into the parameters themselves
            let gain: f32 = self.params.gain.smoothed.next();

            for sample in channel_samples {
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
    const VST3_CLASS_ID: [u8; 8] = *b"Granular";

    // And don't forget to change these categories, see the docstring on `VST3_CATEGORIES` for more
    // information
    const VST3_CATEGORIES: &'static str = "Fx|Dynamics";
}

nih_export_vst3!(GranularPlugin);

#[cfg(test)]
mod tests {
    use super::*;
    use nih_plug::log::debug;
    use nih_plug::nih_debug_assert_eq;
    use nih_plug::prelude::NoteEvent;

    // Reverb
    // Delay
    // Mod FX
    // Granular
    // Engine and Audio basics
    // *   MIDI Testing
    //     MIDI CC
    #[test]
    fn midi_cc_received_correct() {
        let event: NoteEvent = NoteEvent::MidiCC {
            timing: 2,
            channel: 1,
            cc: 45,
            value: 1.0,
        };

        let intended_cc: u8 = 45;
        let intended_value: f32 = 1.0;
        nih_debug_assert_eq!(event:MidiCC.cc, intended_cc);
        nih_debug_assert_eq!(event:MidiCC.value, intended_value);
    }
    //     MIDI Note
    #[test]
    fn midi_note_received_correct(event: NoteEvent, intended_note: u16) {
        let event: NoteEvent = NoteEvent::NoteOn {
            timing: 2,
            voice_id: None,
            channel: 1,
            note: 100,
            velocity: 127.0,
        };

        let intended_note: u8 = 100;
        nih_debug_assert_eq!(event::NoteOn.note, intended_note);
    }
    //    MIDI filter
    //
    // * Audio testing
    //     Wav file loaded
    #[test]
    fn wav_file_loads_correctly() {
        let reader = hound::WavReader::open("WaveFileLocation.wav").unwrap();
        debug!(reader.samples::<i16>())
    }
    // GUI
}
