#![allow(dead_code)]
#![warn(missing_docs)]
//! Implementing a first order filter with transfer function H(S) = w_0 / s + w_0
//! x, y and a0 ... are used due to their correspondence with difference equations

#[derive(Debug)]
/// The coefficients of a first order filter where a0 is normalized to 1
pub struct LPCoefficients {
    a1: f32,
    b0: f32,
    b1: f32,
}

impl LPCoefficients {
    /// A function that generates coefficients given cutoff frequency and sample rate
    pub fn new(cutoff_freq: f32, sample_rate: f32) -> Self {
        let dt: f32 = 1.0 / sample_rate;
        let a0 = (cutoff_freq * dt) + 2.0;
        Self {
            a1: (2.0 - cutoff_freq * dt) / a0,
            b0: (cutoff_freq * dt) / a0,
            b1: (cutoff_freq * dt) / a0,
        }
    }

    #[allow(missing_docs)]
    pub fn get_coeffs(&self) -> (f32, f32, f32) {
        (self.a1, self.b0, self.b1)
    }

    /// Recompute the filter coefficients based on a change in cutoff frequency and or sample rate
    pub fn recompute(&mut self, cutoff_freq: f32, sample_rate: f32) {
        let dt: f32 = 1.0 / sample_rate;
        let a0: f32 = (cutoff_freq * dt) + 2.0;
        self.a1 = (2.0 - cutoff_freq * dt) / a0;
        self.b0 = (cutoff_freq * dt) / a0;
        self.b1 = (cutoff_freq * dt) / a0;
    }
}

#[derive(Debug)]
/// A struct used to process input signals through a first order lowpass filter
pub struct LowpassFilter {
    x: Vec<f32>,
    y: Vec<f32>,
    n: usize,
    coeffs: LPCoefficients,
}

impl LowpassFilter {
    /// A constructor for a new lowpass filter given buffer capacity, cutoff frequency and sample rate
    pub fn new(cutoff_freq: f32, sample_rate: f32, capacity: usize) -> Self {
        Self {
            x: vec![0.0; capacity],
            y: vec![0.0; capacity],
            n: 1,
            coeffs: LPCoefficients::new(cutoff_freq, sample_rate),
        }
    }

    /// Function to move the index through the buffer with wrapping to form a circular buffer
    fn advance(&mut self) {
        self.n = (self.n + 1) % self.x.len();
    }

    /// A function to process a single input (given as f32) through the lowpass filter
    pub fn process(&mut self, xn: f32) -> f32 {
        // increase the index (with wrapping)
        self.advance();

        // assigning to local variables to shorten expressions
        let (a1, b0, b1) = self.coeffs.get_coeffs();
        let n = self.n;

        self.x[n] = xn;
        match n {
            n if n == 0 => {
                self.y[n] =
                    a1 * self.y.last().unwrap() + b0 * self.x[n] + b1 * self.x.last().unwrap()
            }
            _ => self.y[n] = a1 * self.y[n - 1] + b0 * self.x[n] + b1 * self.x[n - 1],
        };
        self.y[n]
    }

    /// Setter for filter cutoff frequency. Wrapper for recompute coefficients
    pub fn set_cutoff(&mut self, cutoff_freq: f32, sample_rate: f32) {
        self.coeffs.recompute(cutoff_freq, sample_rate)
    }
}

#[cfg(test)]
mod tests {
    use crate::filter::LowpassFilter;
    use crate::samples::PhonicMode;
    use crate::{load_wav, write_wav};

    #[test]
    fn test_lp() {
        let in_samples: Vec<f32> = load_wav("tests/noise.wav")
            .unwrap()
            .iter()
            .map(|x| *x as f32)
            .collect();
        let mut filter = LowpassFilter::new(600.0, 44100.0, 44100);
        let mut out_samples: Vec<i16> = Vec::new();

        for xn in &in_samples {
            let yn = filter.process(*xn);
            out_samples.push(yn as i16);
        }
        write_wav(
            "tests/filtered_noise_600hz.wav",
            out_samples,
            PhonicMode::Stereo,
        )
    }
}
