#![allow(dead_code)]
#![warn(missing_docs)]
//! A module containing an implementation of a multi delay line.
//! Processes a 1D array of samples into one of equal length, performing Hadamard mixing in the feedback step.
//! Hadamard mixer is a struct that stores a matrix and can perform mixing by multiplying the input vector by the matrix
//! Multi delay line has a vector of delay times and buffers.
//! Will process the input through the delays independently and then mix them using the Hadamard matrix

use crate::delay_buffer::DelayBuffer;
use ndarray::linalg::{general_mat_vec_mul, kron};
use ndarray::{arr1, arr2, Array, Array1, Ix1, Ix2};
use num_traits::Pow;
use std::f32::consts::FRAC_1_SQRT_2;

/// A function generating a Hadamard matrix from given dimension
/// # Parameters
/// * `order`: the order of the matrix, if order is N, an N x N matrix will be returned. Must be a power of 2
pub fn hadamard(order: i8) -> Array<f32, Ix2> {
    let h2 = arr2(&[[1.0, 1.0], [1.0, -1.0]]);
    // if the number of 1s in the binary representation is 1, the number is a perfect power of 2, validates that the order is a power of 2
    assert_eq!(order.count_ones(), 1);

    let power = (order as f32).log2() as u32;
    let hn = match power {
        1 => h2,
        n => kron(&h2, &hadamard(2_i8.pow(n - 1))),
    };
    hn
}

/// A struct which stores a matrix and a scalar and has a method to apply mixing via matrix-vector multiplication
pub struct HadamardMixer {
    matrix: Array<f32, Ix2>,
    order: i8,
    scalar: f32,
}

impl HadamardMixer {
    /// The constructor for HadamardMixer, which takes in an order (number of channels) and returns an instance with the appropriately sized mixing matrix
    /// The hadamard function is extracted because it is recursive and it would not be suitable to call the constructor recursively.
    pub fn new(order: i8) -> Self {
        Self {
            matrix: hadamard(order),
            order,
            scalar: FRAC_1_SQRT_2.pow(order / 2),
        }
    }

    /// A function which accepts a 1D array (vector) and multiplies it by the 2D array (matrix) stored at self.matrix.
    /// This is then scaled by self.scalar and returned.
    pub fn mix(&self, xn: Array1<f32>) -> Array1<f32> {
        let mut mixed = Array::from(vec![0.0; self.order as usize]);
        general_mat_vec_mul(self.scalar, &self.matrix, &xn, 1.0, &mut mixed);

        mixed
    }
}

/// A struct storing functionality relating to delay lines in multiples of 2.
/// Stores a vector of buffers and a vector of times which correspond to delay lines of those times.
/// Stores feedback and mix levels, which are uniform for each delay line.
/// Stores a HadamardMixer which is used to mix the input channels in each feedback loop.
pub struct MultiDelayLine {
    delay_buffers: Vec<DelayBuffer>,
    mixer: HadamardMixer,
    feedback: f32,
    times_samples: Vec<usize>,
    num_channels: i8,
    mix_ratio: f32,
}

impl MultiDelayLine {
    /// Constructor for the multi delay line, which takes a vector of times, number of channels and feedback and mix levels as well as max delay samples, and returns an instance of the class.
    pub fn new(
        times_s: Vec<f32>,
        feedback: f32,
        mix: f32,
        num_channels: i8,
        max_delay_samples: usize,
    ) -> Self {
        Self {
            // creates a vector of buffers initialized to capacity 'max_delay_samples'
            delay_buffers: vec![DelayBuffer::new(max_delay_samples); num_channels as usize],
            mixer: HadamardMixer::new(num_channels),
            feedback,
            times_samples: times_s
                .iter()
                .map(|time| (time * 44100.0) as usize)
                .collect(),
            num_channels,
            mix_ratio: mix,
        }
    }

    /// Processes a vector of samples with delay and feedback Hadamard mixing.
    /// # Parameters
    /// * `xn`: The input array, must be the same length as num_channels and contain floats.
    /// * `do_mixing`: whether to mix the output with a hadamard mixer or not
    pub fn process_with_feedback(&mut self, xn: Array1<f32>, do_mixing: bool) -> Array<f32, Ix1> {
        let mut delayed_vec: Vec<f32> = Vec::new();

        // the delay step, before the mix matrix
        for (index, buffer) in self.delay_buffers.iter().enumerate() {
            let delay_signal: f32 = buffer.read(self.times_samples[index]);
            delayed_vec.push(delay_signal);
        }

        // optional hadamard mixing step
        let scaled_delayed_vec: Vec<f32> = delayed_vec
            .iter()
            .map(|sample| sample * self.feedback)
            .collect();
        let mixed = match do_mixing {
            true => self.mixer.mix(arr1(&scaled_delayed_vec)),
            false => Array1::from_vec(scaled_delayed_vec),
        };
        for (index, buffer) in self.delay_buffers.iter_mut().enumerate() {
            let feedback_signal: f32 = mixed[index];
            buffer.write(xn[index] + feedback_signal);
        }

        // declare variables for mix levels
        let wet_lvl = self.mix_ratio;
        let dry_lvl = 1.0 - self.mix_ratio;

        // rebuild output as 1D array
        let mut yn: Array1<f32> = Array1::from_vec(vec![0.0; self.num_channels as usize]);
        for index in 0..self.num_channels as usize {
            let yn_i = (wet_lvl * delayed_vec[index]) + (dry_lvl * xn[index]);
            yn[index] = yn_i
        }

        yn
    }
}

#[cfg(test)]
mod tests {
    use crate::multi_channel::hadamard;
    use ndarray::arr2;

    #[test]
    fn test_hadamard_construction() {
        let h2 = hadamard(2);
        assert_eq!(h2, arr2(&[[1.0, 1.0], [1.0, -1.0]]));

        let h4 = hadamard(4);
        assert_eq!(
            h4,
            arr2(&[
                [1.0, 1.0, 1.0, 1.0],
                [1.0, -1.0, 1.0, -1.0],
                [1.0, 1.0, -1.0, -1.0],
                [1.0, -1.0, -1.0, 1.0]
            ])
        );

        let h8 = hadamard(8);
        assert_eq!(
            h8,
            arr2(&[
                [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
                [1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0],
                [1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, -1.0],
                [1.0, -1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0],
                [1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0, -1.0],
                [1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0],
                [1.0, 1.0, -1.0, -1.0, -1.0, -1.0, 1.0, 1.0],
                [1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0]
            ])
        );
    }
}
