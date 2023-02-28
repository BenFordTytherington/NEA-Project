use crate::delay_buffer::DelayBuffer;

pub struct DelayLine {
    buffer: DelayBuffer,
    delay_samples: usize,
    internal_feedback: f64,
    mix_ratio: f64,
}

impl DelayLine {
    pub fn new(
        max_delay_samples: usize,
        delay_samples: usize,
        internal_feedback: f64,
        mix_ratio: f64,
    ) -> Self {
        Self {
            buffer: DelayBuffer::new(max_delay_samples),
            delay_samples,
            internal_feedback,
            mix_ratio,
        }
    }

    pub fn process_with_feedback(&mut self, xn: f64) -> (f64, f64) {
        let delay_signal: f64 = self.buffer.read(self.delay_samples);
        let feedback_signal: f64 = delay_signal * self.internal_feedback;

        self.buffer.write(xn + feedback_signal);

        let wet_lvl = self.mix_ratio;
        let dry_lvl = 1.0 - self.mix_ratio;
        let yn = (wet_lvl * delay_signal) + (dry_lvl * xn);
        (yn, yn)
    }

    pub fn set_delay_samples(&mut self, delay_samples: usize) {
        self.delay_samples = delay_samples;
    }

    pub fn set_internal_feedback(&mut self, internal_feedback: f64) {
        self.internal_feedback = internal_feedback;
    }

    pub fn set_mix_ratio(&mut self, mix_ratio: f64) {
        self.mix_ratio = mix_ratio;
    }
}

pub struct StereoDelay {
    left_dl: DelayLine,
    right_dl: DelayLine,
}

impl StereoDelay {
    pub fn new(sample_rate: f64, delay_seconds: f64) -> Self {
        let max_delay_samples = sample_rate as usize + 1;
        let delay_samples = (sample_rate * delay_seconds) as usize;
        let left_dl = DelayLine::new(max_delay_samples, delay_samples, 0.25, 0.25);
        let right_dl = DelayLine::new(max_delay_samples, delay_samples, 0.25, 0.25);
        Self { left_dl, right_dl }
    }

    pub fn process(&mut self, in_sample_l: f64, in_sample_r: f64) -> (f64, f64) {
        let (out_left, _left_feedback) = self.left_dl.process_with_feedback(in_sample_l);

        let (out_right, _right_feedback) = self.right_dl.process_with_feedback(in_sample_r);
        (out_left, out_right)
    }
}
