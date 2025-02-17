//! Module containing a struct that performs saturation on a given input, with a threshold level and mixes the output
use std::ops::Neg;

/// A struct which stores 2 fields and uses them to saturate (clip) an input
/// ## Attributes:
/// * `threshold`: The amplitude (f32) at which signals will be clipped
/// * `mix_ratio`: Ratio between 1 and 0 of how much saturated signal is mixed in (1 is full clipping and 0 is dry)
pub struct Saturator {
    threshold: f32,
    mix_ratio: f32,
}

impl Saturator {
    /// Constructor given a mix ratio and threshold
    pub fn new(threshold: f32, mix_ratio: f32) -> Self {
        Self {
            threshold,
            mix_ratio,
        }
    }

    /// Setter for saturator ...
    pub fn set_threshold(&mut self, threshold: f32) {
        self.threshold = threshold;
    }

    /// Setter for saturator ...mix ratio (must be between 0 and 1
    pub fn set_mix_ratio(&mut self, mix_ratio: f32) {
        self.mix_ratio = mix_ratio.clamp(0.0, 1.0);
    }

    /// Takes an f32 input using saturation
    pub fn process(&self, xn: f32) -> f32 {
        let value = match xn {
            xn if xn > self.threshold => self.threshold,
            xn if xn < self.threshold.neg() => self.threshold.neg(),
            _ => xn,
        };
        (self.mix_ratio * value) + ((1.0 - self.mix_ratio) * xn)
    }
}

#[cfg(test)]
mod tests {
    use crate::delay_line::StereoDelay;
    use crate::samples::{IntSamples, PhonicMode, Samples};
    use crate::saturation::Saturator;
    use crate::{load_wav, write_wav};

    #[test]
    fn generate_saturation_example() {
        let input = load_wav("tests/amen_br.wav").unwrap();

        let mut out: Vec<i16> = Vec::new();

        let saturator = Saturator::new(i16::MAX as f32 / 16.0, 0.5);

        for sample in input {
            out.push(saturator.process(sample as f32) as i16);
        }

        write_wav(
            "tests/debug/saturator_demo_32nd_reduction_half_mix.wav",
            out,
            PhonicMode::Stereo,
        );
    }

    #[test]
    fn test_in_delay() {
        let input = load_wav("tests/kalimba.wav").unwrap();
        let stereo = IntSamples::new(input);

        let mut out: Vec<i16> = Vec::new();

        let mut delay = StereoDelay::new(44100.0, 0.2, 0.3, 0.85, 0.7);
        delay.set_saturation_factor(8.0);

        for (l, r) in stereo.get_frames() {
            let (left, right) = delay.process(l as f32, r as f32, true, true);
            out.push(left as i16);
            out.push(right as i16);
        }

        write_wav("tests/debug/saturated_delay.wav", out, PhonicMode::Stereo);
    }
}
