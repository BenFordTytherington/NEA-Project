//! Multi-Mode Low Frequency Oscillator (MMLFO) module with the following features:
/// * WaveForms
///      - square
///      - triangle
///      - sine
///      - S&H circuit
/// * frequency (Hz)
/// * sync (time div enum)
/// * get current sample / step current index
use crate::delay_line::calculate_s_synced;
use crate::delay_line::TimeDiv;
use rand::{thread_rng, Rng};
use std::f32::consts::PI;
use std::io::Seek;

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

#[derive(Default, PartialEq, Debug)]
/// An enum of available LFO modes, excluding the Sample and Hold mode,
/// which is implemented separately
pub enum LFOMode {
    #[default]
    Sine,
    Triangle,
    Square,
}

impl LFOMode {
    /// Returns an f32 returning closure which also takes an f32 as a parameter,
    /// corresponding with the LFO mode
    pub fn get_function(&self) -> fn(f32) -> f32 {
        match self {
            LFOMode::Sine => |x| (0.5 * (2.0 * PI * x).sin()) + 0.5,
            LFOMode::Triangle => |x| (2.0 * ((x + 0.25) - ((x + 0.25) + 0.5).floor()).abs()),
            LFOMode::Square => |x| match x {
                x if (0.0 <= x && x < 0.5) => 1.0,
                x if x == 0.5 => 0.5,
                _ => 0.0,
            },
        }
    }
}

/// Struct representing an LFO object, which uses discrete samples and can be synchronized.
/// # Attributes
/// * `mode`: The waveform selector for the LFO, an enum variant of LFOMode
///
/// * `sync`: A boolean deciding whether to use the LFOs time sync or the frequency in Hz
///
/// * `time_div`: A time division to use for the frequency of the LFO, enum variant of TimeDiv
///
/// * `dotted`: A boolean deciding whether or not to make the time division dotted (1.5X length)
///
/// * `freq_hz`: The LFO frequency in Hz. Will be automatically set if a time division and BPM are used
///
/// * `sample_rate`: The sample rate the LFO will be played back at in Hz
///
/// * `bpm`: The beats per minute that the time division is based on
///
/// * `function`: The closure object used to populate the discrete function vector with samples
///
/// * `current_index`: The index used to iterate over the discrete function buffer to get the next sample
///
/// * `discrete_func`: The rendered buffer of sampled waveform,
/// will have the length needed for 1 period of the waveform at the correct frequency
pub struct MMLFO {
    mode: LFOMode,
    sync: bool,
    time_div: TimeDiv,
    dotted: bool,
    freq_hz: f32,
    sample_rate: f32,
    bpm: i32,
    function: fn(f32) -> f32,
    current_index: usize,
    discrete_func: Vec<f32>,
}

impl MMLFO {
    /// The constructor for the LFO with sync and mode as the parameters
    /// ## Default values
    /// * time division: Undotted Quarter note
    /// * frequency: 500Hz
    /// * sample rate: 44100Hz
    /// * bpm: 120
    /// * unpopulated discrete function as this is determined by the enum get_function method
    pub fn new(sync: bool, mode: LFOMode) -> Self {
        let mut instance = Self {
            mode,
            sync,
            time_div: TimeDiv::Quarter,
            dotted: false,
            freq_hz: 500.0,
            sample_rate: 44100.0,
            bpm: 120,
            function: |x| x,
            current_index: 0,
            discrete_func: vec![1.0; 44100],
        };
        // this populates the discrete function and function as well as updates frequency if in sync mode
        instance.update_state();
        instance
    }

    /// Updates fields of the struct that need recomputing after a set operation.
    ///
    /// Function is updated from the mode enum variant
    ///
    /// Frequency is updated from the sync parameter set
    ///
    /// The discrete function is regenerated
    fn update_state(&mut self) {
        self.function = self.mode.get_function();
        self.freq_hz = match self.sync {
            true => 1.0 / calculate_s_synced(self.bpm, self.time_div.clone(), self.dotted),
            false => self.freq_hz,
        };

        let period_samples = self.sample_rate / (self.freq_hz);
        self.discrete_func = vec![1.0; period_samples as usize];

        for x in 0..(period_samples as usize) {
            self.discrete_func[x] = (self.function)(x as f32 / period_samples)
        }
    }

    /// Returns the next value from the discrete buffer and cycles the index to 0 if necessary
    pub fn get_next_value(&mut self) -> f32 {
        let value = self.discrete_func[self.current_index];
        let period = self.sample_rate / (self.freq_hz);
        self.current_index = (self.current_index + 1) % (period as usize);
        value
    }

    /// Setter for sample rate in Hz
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.update_state();
    }

    /// Setter for time division as a multiple of a bar
    pub fn set_time_div(&mut self, time_div: TimeDiv, dotted: bool) {
        self.sync = true;
        self.time_div = time_div;
        self.dotted = dotted;
        self.update_state();
    }

    /// Setter for the BPM (beats per minute)
    pub fn set_bpm(&mut self, bpm: i32) {
        self.bpm = bpm;
        self.update_state();
    }

    /// Setter to enable sync in the LFO
    pub fn set_sync(&mut self, sync: bool) {
        self.sync = sync;
        self.update_state();
    }

    /// Setter for frequency in Hz
    pub fn set_frequency_hz(&mut self, freq_hz: f32) {
        self.sync = false;
        self.freq_hz = freq_hz;
        self.update_state();
    }

    /// Setter for the waveform mode. Accepts an `LFOMode` as a parameter
    pub fn set_mode(&mut self, mode: LFOMode) {
        self.mode = mode;
        self.update_state();
    }
}

/// Sample and Hold struct accessing a 2 second noise buffer, which is generated at construction
/// ## Attributes
/// * `noise_buffer`: A vector of noise samples, used to generate the stepped random output
///
/// * `current_value`: Stores the current value to be output, used in iterpolation if slewed
///
/// * `current_index`: The index used to iterate over the noise buffer
///
/// * `frequency_hz`: The noise sampling frequency in Hz
///
/// * `last_value`: The last value sampled, used for interpolation in the slew limiting logic
///
/// * `interpolate`: The value between 0 and 1 used to interpolate between sample changes
///
/// * `slew`: Bool used to decide whether to perform slew rate limiting or not
///
/// * `slew_time_s`: The time in seconds that it should take for a transition between samples.
pub struct SampleAndHold {
    noise_buffer: Vec<f32>,
    current_value: f32,
    current_index: usize,
    frequency_hz: f32,
    last_value: f32,
    interpolate: f32,
    slew: bool,
    slew_time_s: f32,
}

impl SampleAndHold {
    /// The constructor for the S&H circuit, takes no parameters.
    /// ## Default settings:
    /// * noise buffer: 2 seconds of random between 0 and 1
    ///
    /// * all indices and values: 0
    ///
    /// * slew: false
    ///
    /// * slew time: 0.25s
    pub fn new() -> Self {
        let mut rng = thread_rng();
        Self {
            noise_buffer: (0..88200).map(|_| rng.gen()).collect(),
            current_value: 0.0,
            current_index: 0,
            frequency_hz: 1.0,
            last_value: 0.0,
            interpolate: 0.0,
            slew: false,
            slew_time_s: 0.25,
        }
    }

    /// Function that samples the noise buffer and stores it in the current value.
    /// Updates the last value and interpolation index
    fn sample(&mut self) {
        self.last_value = self.current_value;
        self.current_value = self.noise_buffer[self.current_index];
        self.interpolate = 0.0;
    }

    /// Increase the index and take it mod 2 seconds in samples, which loops the index to 0 after a full duration
    fn advance(&mut self) {
        self.current_index = (self.current_index + 1) % 88200;
    }

    /// Setter for frequency in Hz
    pub fn set_freq(&mut self, freq: f32) {
        self.frequency_hz = freq;
    }

    /// Getter for the next sample value, will produce stepped random voltage
    pub fn get_next_value(&mut self) -> f32 {
        self.advance();

        let period_samples = ((1.0 / self.frequency_hz) * 44100.0) as usize;
        if self.current_index % (period_samples) == 0 {
            self.sample();
        }

        if self.slew {
            if self.interpolate >= 1.0 {
                self.interpolate = 0.0;
                self.last_value = self.current_value;
                return self.current_value;
            } else {
                self.interpolate += (1.0 / (44100.0 * self.slew_time_s));
                return ((1.0 - self.interpolate) * self.last_value)
                    + (self.interpolate * self.current_value);
            }
        } else {
            return self.current_value;
        }
    }

    /// Setter for toggling slew on or off
    pub fn set_slew(&mut self, on_off: bool) {
        self.slew = on_off;
    }

    /// Setter for slew time in seconds
    pub fn set_slew_time(&mut self, time_s: f32) {
        self.slew_time_s = time_s;
    }
}

#[cfg(test)]
mod tests {
    use crate::delay_line::{StereoDelay, TimeDiv};
    use crate::filter::LowpassFilter;
    use crate::lfo::{LFOMode, SampleAndHold, MMLFO};
    use crate::samples::{IntSamples, PhonicMode, Samples};
    use crate::{load_wav, write_wav, write_wav_float};
    use std::f32::consts::PI;
    use test_case::test_case;

    #[test]
    fn test_lfo_init() {
        let lfo_sin = MMLFO::new(false, LFOMode::Sine);
        let lfo_sqr = MMLFO::new(false, LFOMode::Square);
        let lfo_tri = MMLFO::new(false, LFOMode::Triangle);
    }

    #[test]
    fn test_lfo_setters() {
        let mut lfo = MMLFO::new(true, LFOMode::Sine);

        lfo.set_mode(LFOMode::Triangle);
        assert_eq!(lfo.mode, LFOMode::Triangle);

        lfo.set_bpm(130);
        assert_eq!(lfo.bpm, 130);

        lfo.set_time_div(TimeDiv::Eighth, false);

        lfo.set_sample_rate(88200.0);
        assert_eq!(lfo.sample_rate, 88200.0);

        lfo.set_frequency_hz(800.0);
        assert_eq!(lfo.freq_hz, 800.0);
    }

    #[test_case(LFOMode::Sine ; "sin lfo")]
    #[test_case(LFOMode::Triangle ; "tri lfo")]
    #[test_case(LFOMode::Square ; "sqr lfo")]
    #[ignore]
    fn generate_lfo_examples(mode: LFOMode) {
        let mut lfo = MMLFO::new(true, mode);
        lfo.set_bpm(141);
        lfo.update_state();

        let mut filter = LowpassFilter::new(2000.0, 44100.0, 44100);

        let samples = load_wav("tests/amen_br.wav").unwrap();

        let mut out: Vec<i16> = Vec::new();

        for sample in samples {
            filter.set_cutoff((20000.0 * lfo.get_next_value()), 44100.0);
            out.push((filter.process(sample as f32)) as i16);
        }
        let mode_name = match lfo.mode {
            LFOMode::Sine => "sin",
            LFOMode::Triangle => "tri",
            LFOMode::Square => "sqr",
        };

        write_wav(
            format!("tests/amen_br_{}_filter.wav", mode_name).as_str(),
            out,
            PhonicMode::Stereo,
        )
    }

    #[test]
    #[ignore]
    fn generate_chorus_examples() {
        let mut lfo_1 = MMLFO::new(false, LFOMode::Sine);
        // lfo_1.set_bpm(141);
        // lfo_1.set_time_div(TimeDiv::Whole, false);
        lfo_1.set_frequency_hz(0.25);
        lfo_1.update_state();

        let mut lfo_2 = MMLFO::new(false, LFOMode::Sine);
        // lfo_2.set_bpm(141);
        // lfo_2.set_time_div(TimeDiv::Whole, false);
        lfo_1.set_frequency_hz(0.25);
        lfo_2.update_state();

        const DEPTH: f32 = 0.5;
        const D_BASE_1: f32 = 0.002;
        const D_BASE_2: f32 = 0.0025;

        let mut delay = StereoDelay::new(44100.0, D_BASE_1 as f64, D_BASE_2 as f64, 0.5, 0.5);
        let samples = load_wav("tests/amen_br.wav").unwrap();
        let stereo_samples = IntSamples::new(samples);

        let mut out: Vec<i16> = Vec::new();

        for (left, right) in stereo_samples.get_frames() {
            let lfo_value_1 = lfo_1.get_next_value();
            let lfo_value_2 = lfo_2.get_next_value();

            let d_time_1 = D_BASE_1 * (DEPTH * lfo_value_1 + 1.0);
            let d_time_2 = D_BASE_2 * (DEPTH * lfo_value_2 + 1.0);

            delay.set_time_left(d_time_1);
            delay.set_time_right(d_time_2);

            let (left, right) = delay.process(left as f32, right as f32, true);
            out.push(left as i16);
            out.push(right as i16);
        }

        write_wav("tests/amen_br_flange_filter.wav", out, PhonicMode::Stereo);
    }

    #[test_case(LFOMode::Sine ; "sin lfo")]
    #[test_case(LFOMode::Triangle ; "tri lfo")]
    #[test_case(LFOMode::Square ; "sqr lfo")]
    #[ignore]
    fn render_waveforms(mode: LFOMode) {
        let mut lfo = MMLFO::new(false, mode);
        lfo.update_state();
        let mut out: Vec<i16> = Vec::new();

        for _ in (0..88200) {
            out.push((5000.0 * lfo.get_next_value()) as i16)
        }

        let mode_name = match lfo.mode {
            LFOMode::Sine => "sin",
            LFOMode::Triangle => "tri",
            LFOMode::Square => "sqr",
        };

        write_wav(
            format!("tests/debug/lfo_{}.wav", mode_name).as_str(),
            out,
            PhonicMode::Mono,
        )
    }

    #[test]
    fn render_snh() {
        let mut snh = SampleAndHold::new();
        let mut out: Vec<i16> = Vec::new();

        snh.set_freq(2.0);

        for _ in 0..(44100 * 6) {
            out.push((5000.0 * snh.get_next_value()) as i16)
        }

        write_wav("tests/debug/lfo_snh_no_slew.wav", out, PhonicMode::Mono);

        let mut out = Vec::new();
        snh.set_slew(true);

        for _ in 0..(44100 * 6) {
            out.push((5000.0 * snh.get_next_value()) as i16)
        }

        write_wav("tests/debug/lfo_snh_slew.wav", out, PhonicMode::Mono);
    }
}
