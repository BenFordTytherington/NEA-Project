#![allow(dead_code)]
#![warn(missing_docs)]
//! A module containing structs for a delay line and delay processor.
//! Delay line implements a delay line with dynamic delay times and a first order low-pass in the feedback loop.
//! Stereo Delay implements a delay processor that has 2 delay independently timed delay lines and processes stereo sample pairs.
//! Both use f32 samples

use crate::delay_buffer::DelayBuffer;
use crate::filter::LowpassFilter;
use crate::saturation::Saturator;
use crate::timing::Timing;

/// A delay line which can process inputs with internal feedback and internal filtering as well as dry/wet mix control
/// # Attributes
/// * `buffer`: A delay buffer object storing samples
/// * `delay_samples`: Number of samples to delay input by
/// * `internal_feedback`: Internal feedback multiplier **do not exceed 1 - may create infinite feedback and clipping**
/// * `mix_ratio`: Ratio of dry to wet (ratio of 1 is 100% wet) **do not exceed 1**
/// * `filter`: A lowpass filter applied in the feedback loop
#[derive(Debug)]
pub struct DelayLine {
    buffer: DelayBuffer,
    delay_samples: usize,
    internal_feedback: f32,
    mix_ratio: f32,
    filter: LowpassFilter,
}

impl DelayLine {
    /// Constructor for DelayLine
    /// # Parameters
    /// * `max_delay_samples`: The maximum number of delay samples to be used in the `DelayBuffer`
    /// * `delay_samples`: The number of samples to delay the signal by
    /// * `internal_feedback`: Float between 0 and 1 to multiply feedback signal by
    /// * `mix_ratio`: Float between 0 and 1 to multiply feedback signal by
    pub fn new(
        max_delay_samples: usize,
        delay_samples: usize,
        internal_feedback: f32,
        mix_ratio: f32,
    ) -> Self {
        Self {
            buffer: DelayBuffer::new(max_delay_samples),
            delay_samples,
            internal_feedback,
            mix_ratio,
            filter: LowpassFilter::new(5000.0, 44100.0, max_delay_samples),
        }
    }

    /// A function which processes a single sample and returns a tuple of 2 processed samples
    /// # Parameters
    /// * `xn`: The input sample to be processed, named this way because of the nomenclature on block diagrams and difference equations
    pub fn process_with_feedback(&mut self, xn: f32, do_filtering: bool) -> (f32, f32) {
        let delay_signal: f32 = self.buffer.read(self.delay_samples);
        let feedback_signal: f32 = match do_filtering {
            true => self.filter.process(delay_signal) * self.internal_feedback,
            false => delay_signal * self.internal_feedback,
        };

        self.buffer.write(xn + feedback_signal);

        let wet_lvl = self.mix_ratio;
        let dry_lvl = 1.0 - self.mix_ratio;

        // yn is the output notation from block diagrams
        let yn = (wet_lvl * delay_signal) + (dry_lvl * xn);
        (yn, yn)
    }

    #[allow(missing_docs)]
    pub fn get_delay_samples(&self) -> usize {
        self.delay_samples
    }

    #[allow(missing_docs)]
    pub fn delay_samples(&self) -> &usize {
        &self.delay_samples
    }

    #[allow(missing_docs)]
    pub fn get_delay_seconds(&self) -> f32 {
        self.delay_samples as f32 / 44100_f32
    }

    #[allow(missing_docs)]
    pub fn set_delay_samples(&mut self, delay_samples: usize) {
        self.delay_samples = delay_samples;
    }

    #[allow(missing_docs)]
    pub fn set_internal_feedback(&mut self, internal_feedback: f32) {
        self.internal_feedback = internal_feedback;
    }

    #[allow(missing_docs)]
    pub fn set_mix_ratio(&mut self, mix_ratio: f32) {
        self.mix_ratio = mix_ratio;
    }
}

/// A struct capturing full delay functionality with independent left and right delay lines.
pub struct StereoDelay {
    left_dl: DelayLine,
    right_dl: DelayLine,
    sample_rate: f32,
    saturator: Saturator,
}

impl StereoDelay {
    /// Constructs a new StereoDelay object with 2 delay lines which have separate delay times, specified in ms
    /// # Parameters
    /// * `sample_rate`: The sample rate to use in Hz
    /// * `delay_seconds_l`: The length of the left delay line in seconds
    /// * `delay_seconds_r`: The length of the right delay line in seconds
    /// * `feedback`: The internal feedback multiplier for `DelayLine`
    /// * `mix`: The internal wet/dry mix level for `DelayLine`
    pub fn new(
        sample_rate: f32,
        delay_seconds_l: f64,
        delay_seconds_r: f64,
        feedback: f32,
        mix: f32,
    ) -> Self {
        let max_delay_samples = sample_rate as usize + 1;

        // conversion between seconds and samples using provided sample rate
        let delay_samples_l = (sample_rate as f64 * delay_seconds_l) as usize;
        let delay_samples_r = (sample_rate as f64 * delay_seconds_r) as usize;

        let left_dl = DelayLine::new(max_delay_samples, delay_samples_l, feedback, mix);
        let right_dl = DelayLine::new(max_delay_samples, delay_samples_r, feedback, mix);
        Self {
            left_dl,
            right_dl,
            sample_rate,
            saturator: Saturator::new(i16::MAX as f32 / 64.0, 0.5),
        }
    }

    /// Constructs a new StereoDelay object with 2 delay lines which have separate delay times, specified as a time division
    /// # Parameters
    /// * `sample_rate`: The sample rate to use in Hz
    ///
    /// * `timing_left`: A timing object used to represent the time at a bpm for the left delay to repeat
    ///
    /// * `timing_right`: A timing object used to represent the time at a bpm for the right delay to repeat
    ///
    /// * `feedback`: The internal feedback multiplier for `DelayLine`
    ///
    /// * `mix`: The internal wet/dry mix level for `DelayLine`
    ///
    pub fn new_sync(
        sample_rate: f32,
        timing_left: Timing,
        timing_right: Timing,
        feedback: f32,
        mix: f32,
    ) -> Self {
        let max_delay_samples = sample_rate as usize + 1;

        let delay_seconds_l = timing_left.to_seconds();
        let delay_seconds_r = timing_right.to_seconds();

        // conversion between seconds and samples using provided sample rate
        let delay_samples_l = (sample_rate * delay_seconds_l) as usize;
        let delay_samples_r = (sample_rate * delay_seconds_r) as usize;

        let left_dl = DelayLine::new(max_delay_samples, delay_samples_l, feedback, mix);
        let right_dl = DelayLine::new(max_delay_samples, delay_samples_r, feedback, mix);
        Self {
            left_dl,
            right_dl,
            sample_rate,
            saturator: Saturator::new(i16::MAX as f32 / 64.0, 0.5),
        }
    }

    /// Returns a tuple of samples (left, right) which have been processed through the delay line
    pub fn process(
        &mut self,
        in_sample_l: f32,
        in_sample_r: f32,
        do_filtering: bool,
        saturate: bool,
    ) -> (f32, f32) {
        let (out_left, _) = self
            .left_dl
            .process_with_feedback(in_sample_l, do_filtering);

        let (out_right, _) = self
            .right_dl
            .process_with_feedback(in_sample_r, do_filtering);
        match saturate {
            false => (out_left, out_right),
            true => (
                self.saturator.process(out_left),
                self.saturator.process(out_right),
            ),
        }
    }

    /// Setter for left delay line time in seconds
    pub fn set_time_left(&mut self, time_s: f32) {
        self.left_dl.delay_samples = (self.sample_rate * time_s) as usize
    }

    /// Setter for right delay line time in seconds
    pub fn set_time_right(&mut self, time_s: f32) {
        self.right_dl.delay_samples = (self.sample_rate * time_s) as usize
    }

    /// Sets the saturation factor as a fraction of the sample maximum (i16::MAX)
    pub fn set_saturation_factor(&mut self, factor: f32) {
        self.saturator.set_threshold(i16::MAX as f32 / factor);
    }

    /// Getter for the delay times as a pair, to avoid repeating the get time function for both delay lines
    pub fn get_times(&self) -> (f32, f32) {
        (
            self.left_dl.get_delay_seconds(),
            self.right_dl.get_delay_seconds(),
        )
    }
}
