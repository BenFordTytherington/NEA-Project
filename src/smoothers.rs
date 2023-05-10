//! Trait and Structs for performing window functional smoothing on f32 samples
use num_traits::{pow, Pow};
use std::f32::consts::PI;

/// Trait for a smoother object, with associated window length and a method to get the next sample from the window.
pub trait Smoother {
    fn get_index(&self, index: usize) -> f32;
    fn set_length(&mut self, length: usize) {}
}

/// Struct designed to act as a bypass in places where a type of `dyn Smoother` is required
pub struct NoSmoother {}

impl NoSmoother {
    pub fn new() -> Self {
        Self {}
    }
}

impl Smoother for NoSmoother {
    /// Returns 1.0 always, as this performs no smoothing
    fn get_index(&self, _: usize) -> f32 {
        1.0
    }

    fn set_length(&mut self, _: usize) {}
}

/// A struct which performs Hann window smoothing, using a discrete vector of samples of the window function
pub struct HannSmoother {
    length: usize,
    discrete: Vec<f32>,
}

impl HannSmoother {
    pub fn new() -> Self {
        Self {
            length: 0,
            discrete: Vec::new(),
        }
    }
}

impl Smoother for HannSmoother {
    /// Getter for the next sample from the discrete function
    fn get_index(&self, index: usize) -> f32 {
        self.discrete[index]
    }

    /// Setter for the length of the window function.
    /// Also recomputes the discrete function with the new length, so should be used sparingly.
    fn set_length(&mut self, length: usize) {
        self.discrete.clear();
        self.length = length;
        let delta: f32 = 1.0 / (length as f32);
        for index in 0..length {
            let cos_1: f32 = (PI * ((index as f32 * delta) - 0.5)).cos();
            self.discrete.push(cos_1.pow(2))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::samples::PhonicMode;
    use crate::smoothers::{HannSmoother, Smoother};
    use crate::{load_wav, write_wav};

    #[test]
    #[ignore]
    fn gen_smooth() {
        let samples = load_wav("tests/sine.wav").unwrap();

        let mut hann = HannSmoother::new();
        hann.set_length(samples.len());
        let mut out: Vec<i16> = Vec::new();

        for (index, sample) in samples.iter().enumerate() {
            out.push((*sample as f32 * hann.get_index(index)) as i16)
        }

        write_wav("tests/debug/hann_test.wav", out, PhonicMode::Mono)
    }
}
