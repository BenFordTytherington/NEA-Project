#![allow(dead_code)]
#![warn(missing_docs)]

use crate::delay_buffer::DelayBuffer;
use crate::filter::LowpassFilter;

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
    pub fn process_with_feedback(&mut self, xn: f32) -> (f32, f32) {
        let delay_signal: f32 = self.buffer.read(self.delay_samples);
        let feedback_signal: f32 = self.filter.process(delay_signal) * self.internal_feedback;

        self.buffer.write(xn + feedback_signal);

        let wet_lvl = self.mix_ratio;
        let dry_lvl = 1.0 - self.mix_ratio;

        // yn is the output notation from block diagrams
        let yn = (wet_lvl * delay_signal) + (dry_lvl * xn);
        (yn, yn)
    }

    pub fn get_delay_samples(&self) -> usize {
        self.delay_samples
    }

    pub fn get_delay_seconds(&self) -> f32 {
        self.delay_samples as f32 / 44100_f32
    }

    pub fn set_delay_samples(&mut self, delay_samples: usize) {
        self.delay_samples = delay_samples;
    }

    pub fn set_internal_feedback(&mut self, internal_feedback: f32) {
        self.internal_feedback = internal_feedback;
    }

    pub fn set_mix_ratio(&mut self, mix_ratio: f32) {
        self.mix_ratio = mix_ratio;
    }
}

/// An enum used for time divisions relative to a bar.
/// Whole is 1 bar
/// Half is 2 beats
/// Quarter is 1 beat
/// Eighth is an Eighth note or half a beat
/// Sixteenth is a Sixteenth note or a quarter of a beat
pub enum TimeDiv {
    Whole,
    Half,
    Quarter,
    Eighth,
    Sixteenth,
}

/// A function to calculate the amount of time in seconds that a given unit of music time takes provided the bpm
/// # Parameters
/// * `bpm`: The tempo in beats per minute
/// * `division`: The time division relative to a bar
/// * `dotted`: Whether or not to dot the note (meaning multiply its length by 1.5)
fn calculate_s_synced(bpm: i32, division: TimeDiv, dotted: bool) -> f32 {
    let divisor: f32 = match division {
        TimeDiv::Whole => 1.0,
        TimeDiv::Half => 2.0,
        TimeDiv::Quarter => 4.0,
        TimeDiv::Eighth => 8.0,
        TimeDiv::Sixteenth => 16.0,
    };

    let measure_length_s: f32 = 240.0 / bpm as f32;
    let mut length = measure_length_s / divisor;
    if dotted {
        length *= 1.5
    }
    length
}

/// A struct capturing full delay functionality with independent left and right delay lines.
pub struct StereoDelay {
    left_dl: DelayLine,
    right_dl: DelayLine,
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
        sample_rate: f64,
        delay_seconds_l: f64,
        delay_seconds_r: f64,
        feedback: f32,
        mix: f32,
    ) -> Self {
        let max_delay_samples = sample_rate as usize + 1;

        // conversion between seconds and samples using provided sample rate
        let delay_samples_l = (sample_rate * delay_seconds_l) as usize;
        let delay_samples_r = (sample_rate * delay_seconds_r) as usize;

        let left_dl = DelayLine::new(max_delay_samples, delay_samples_l, feedback, mix);
        let right_dl = DelayLine::new(max_delay_samples, delay_samples_r, feedback, mix);
        Self { left_dl, right_dl }
    }

    /// Constructs a new StereoDelay object with 2 delay lines which have separate delay times, specified as a time division
    /// # Parameters
    /// * `sample_rate`: The sample rate to use in Hz
    /// * `delay_div_l`: The length of the left delay line as a time division
    /// * `dotted_left`: Whether or not to dot the note of the time division of the left delay line
    /// * `delay_div_r`: The length of the right delay line as a time division
    /// * `dotted_right`: Whether or not to dot the note of the time division of the right delay line
    /// * `feedback`: The internal feedback multiplier for `DelayLine`
    /// * `mix`: The internal wet/dry mix level for `DelayLine`
    pub fn new_sync(
        sample_rate: f32,
        bpm: i32,
        delay_div_left: TimeDiv,
        dotted_left: bool,
        delay_div_right: TimeDiv,
        dotted_right: bool,
        feedback: f32,
        mix: f32,
    ) -> Self {
        let max_delay_samples = sample_rate as usize + 1;

        let delay_seconds_l = calculate_s_synced(bpm, delay_div_left, dotted_left);
        let delay_seconds_r = calculate_s_synced(bpm, delay_div_right, dotted_right);

        // conversion between seconds and samples using provided sample rate
        let delay_samples_l = (sample_rate * delay_seconds_l) as usize;
        let delay_samples_r = (sample_rate * delay_seconds_r) as usize;

        let left_dl = DelayLine::new(max_delay_samples, delay_samples_l, feedback, mix);
        let right_dl = DelayLine::new(max_delay_samples, delay_samples_r, feedback, mix);
        Self { left_dl, right_dl }
    }

    /// Returns a tuple of samples (left, right) which have been processed through the delay line
    pub fn process(&mut self, in_sample_l: f32, in_sample_r: f32) -> (f32, f32) {
        let (out_left, _) = self.left_dl.process_with_feedback(in_sample_l);

        let (out_right, _) = self.right_dl.process_with_feedback(in_sample_r);
        (out_left, out_right)
    }
}

#[cfg(test)]
mod tests {
    use crate::delay_line::{calculate_s_synced, TimeDiv};

    #[test]
    fn test_time_calculator() {
        let correct_times: Vec<f32> = vec![1.714, 0.857, 0.429, 0.214, 0.107];
        let calc_times: Vec<f32> = [
            TimeDiv::Whole,
            TimeDiv::Half,
            TimeDiv::Quarter,
            TimeDiv::Eighth,
            TimeDiv::Sixteenth,
        ]
        .into_iter()
        .map(|time_d| calculate_s_synced(140, time_d, false))
        .collect();

        for index in 0..5 {
            let diff = (correct_times[index] - calc_times[index]).abs();
            assert!(diff <= 0.001)
        }
    }
}
