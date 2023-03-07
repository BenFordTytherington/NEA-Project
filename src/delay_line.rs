use crate::delay_buffer::DelayBuffer;

/// A delay line which can process inputs with internal feedback
/// #Attributes
/// * `buffer`: a delay buffer object storing samples
/// * `delay_samples`: number of samples to delay input by
/// * `internal_feedback`: internal feedback multiplier **do not exceed 1 - may create infinite feedback**
/// * `mix_ratio`: ratio of dry to wet (ratio of 1 is 100% wet) **do not exceed 1**
pub struct DelayLine {
    buffer: DelayBuffer,
    delay_samples: usize,
    internal_feedback: f32,
    mix_ratio: f32,
}

impl DelayLine {
    /// Constructor for DelayLine
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
        }
    }

    pub fn process_with_feedback(&mut self, xn: f32) -> (f32, f32) {
        let delay_signal: f32 = self.buffer.read(self.delay_samples);
        let feedback_signal: f32 = delay_signal * self.internal_feedback;

        self.buffer.write(xn + feedback_signal);

        let wet_lvl = self.mix_ratio;
        let dry_lvl = 1.0 - self.mix_ratio;

        // yn is the output notation from block diagrams
        let yn = (wet_lvl * delay_signal) + (dry_lvl * xn);
        (yn, yn)
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

pub struct StereoDelay {
    left_dl: DelayLine,
    right_dl: DelayLine,
}

impl StereoDelay {
    /// Constructs a new StereoDelay object with 2 delay lines which have separate delay times, specified in ms
    pub fn new(sample_rate: f64, delay_seconds_l: f64, delay_seconds_r: f64) -> Self {
        let max_delay_samples = sample_rate as usize + 1;

        // conversion between seconds and samples using provided sample rate
        let delay_samples_l = (sample_rate * delay_seconds_l) as usize;
        let delay_samples_r = (sample_rate * delay_seconds_r) as usize;

        let left_dl = DelayLine::new(max_delay_samples, delay_samples_l, 0.25, 0.25);
        let right_dl = DelayLine::new(max_delay_samples, delay_samples_r, 0.25, 0.25);
        Self { left_dl, right_dl }
    }

    /// Returns a tuple of samples (left, right) which have been processed through the delay line
    pub fn process(&mut self, in_sample_l: f32, in_sample_r: f32) -> (f32, f32) {
        let (out_left, _) = self.left_dl.process_with_feedback(in_sample_l);

        let (out_right, _) = self.right_dl.process_with_feedback(in_sample_r);
        (out_left, out_right)
    }
}
