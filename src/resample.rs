//! A Module containing structs and functions for resampling audio
//! Primarily used for pitch shifting.

use crate::interpolators::{hermite_interpolate, lanczos_window, lerp};

/// Struct performing linear interpolation given an input slice and pitch factor to resample by.
pub struct LinearResampler<'a> {
    buffer: &'a [i16],
    position: f64,
    pitch_factor: f64,
}

impl<'a> LinearResampler<'a> {
    /// Constructor for linear resampler which takes an input slice and pitch factor to resample by
    pub fn new(collection: &'a [i16], pitch_factor: f64) -> Self {
        Self {
            buffer: collection,
            position: 0.0,
            pitch_factor,
        }
    }

    /// Setter for repitching factor as a ratio to the original frequency
    pub fn set_factor(&mut self, factor: f64) {
        self.pitch_factor = factor;
    }

    /// Setter for buffer by a lifetime annotated slice
    pub fn set_buffer(&mut self, buffer: &'a [i16]) {
        self.buffer = buffer;
    }

    /// Getter for the current position of the resampler
    pub fn get_position(&self) -> f64 {
        self.position
    }

    /// increments the resampler and loops index if over the length of the buffer.
    ///
    /// Returns true if the buffer was looped
    pub fn increment(&mut self) -> bool {
        self.position += self.pitch_factor;
        if self.position >= (self.buffer.len() - 1) as f64 {
            self.position -= self.buffer.len() as f64 - 1.0;
            return true;
        }
        false
    }

    pub fn get_pitch_freq(&self) -> f64 {
        self.pitch_factor
    }
}

impl<'a> Iterator for LinearResampler<'a> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= (self.buffer.len() - 1) as f64 {
            self.position -= self.buffer.len() as f64 - 1.0;
        }

        // performs linear interpolation between that index and the next, by the fractional part
        let index = self.position.floor() as usize;
        let sample = lerp(
            self.buffer[index] as f32,
            self.buffer[index + 1] as f32,
            self.position.fract() as f32,
        );
        // position increased by pitch factor in order to stretch the sample by the amount of pitch factor.
        self.position += self.pitch_factor;

        Some(sample)
    }
}

/// Struct performing Lanczos interpolation using the Lanczos window function
pub struct LanczosResampler<'a> {
    buffer: &'a [i16],
    position: f64,
    pitch_factor: f64,
    window_size: u16,
}

impl<'a> LanczosResampler<'a> {
    /// Constructor for the Lanczos resampler, with input slice and pitch factor to interpolate by
    pub fn new(collection: &'a [i16], pitch_factor: f64, window_size: u16) -> Self {
        Self {
            buffer: collection,
            position: 0.0,
            pitch_factor,
            window_size,
        }
    }

    /// Setter for repitching factor as a ratio to the original frequency
    pub fn set_factor(&mut self, factor: f64) {
        self.pitch_factor = factor;
    }

    /// Setter for buffer by a lifetime annotated slice
    pub fn set_buffer(&mut self, buffer: &'a [i16]) {
        self.buffer = buffer;
    }
}

impl<'a> Iterator for LanczosResampler<'a> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.buffer.len() as f64 {
            self.position -= self.buffer.len() as f64;
        }

        // Any window size could be chosen, which will affect the interpolation result.
        // 3 is a sensible default value
        let window_size = self.window_size as i32;
        // Where the window will be centered for kernel interpolation
        let input_position = self.position;
        // The leftmost sample to interpolate (likely fractional index)
        let start = input_position - window_size as f64;
        // The rightmost sample to interpolate (likely fractional index)
        let end = input_position + window_size as f64;

        // initializing sum variables for the kernel interpolation.
        let mut sum = 0.0;
        let mut total_weight = 0.0;

        // Iterate over the window space and take a weighted average weighted by the Lanczos window
        for i in start.ceil() as isize..=end.floor() as isize {
            // ignores indices outside the buffer range.
            if i >= 0 && i < self.buffer.len() as isize {
                let x = (input_position - i as f64) as f32;
                let weight = lanczos_window(x, window_size as f32);
                sum += self.buffer[i as usize] as f32 * weight;
                // total weight is used to keep track of weights which may change, with the start or end of buffer
                total_weight += weight;
            }
        }

        // advance position by fractional index.
        self.position += self.pitch_factor;
        // return the average from the weighted average function.
        Some(sum / total_weight)
    }
}

/// Struct that performs CHSI (cubic Hermite spline interpolation)
pub struct HermiteResampler<'a> {
    buffer: &'a [i16],
    pitch_factor: f32,
    position: f32,
}

impl<'a> HermiteResampler<'a> {
    /// Constructor for CHSI resampler taking an audio buffer and a pitch factor
    pub fn new(input: &'a [i16], pitch_factor: f32) -> Self {
        HermiteResampler {
            buffer: input,
            pitch_factor,
            position: 0.0,
        }
    }

    /// Setter for repitching factor as a ratio to the original frequency
    pub fn set_factor(&mut self, factor: f32) {
        self.pitch_factor = factor;
    }

    /// Setter for buffer object by a lifetime annotated slice
    pub fn set_buffer(&mut self, buffer: &'a [i16]) {
        self.buffer = buffer;
    }
}

impl<'a> Iterator for HermiteResampler<'a> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.buffer.len() as f32 {
            self.position -= self.buffer.len() as f32;
        }

        // assign the start position to the current position index, which advances by scale pitch factor
        let input_position = self.position;
        // convert position to usize index with floor
        let index = input_position.floor() as usize;
        // T is the interpolation factor (difference between real position and integer one)
        // Same technique as lerp function
        let t = input_position - index as f32;

        // acounting for the start of the array, p0 may need to be selected as 0
        // otherwise, it samples at index - 1
        let p0 = if index == 0 {
            self.buffer[0]
        } else {
            self.buffer[index - 1]
        };

        // p1 is the sample at the current index value of the array
        let p1 = self.buffer[index];

        // accounting for the end of the array, may need to be decreased
        // otherwise, samples at the index after the base index
        let p2 = if index >= self.buffer.len() - 1 {
            self.buffer[self.buffer.len() - 1]
        } else {
            self.buffer[index + 1]
        };

        let p3 = if index >= self.buffer.len() - 2 {
            self.buffer[self.buffer.len() - 1]
        } else {
            self.buffer[index + 2]
        };

        // advancing the position by the pitch factor
        self.position += self.pitch_factor;

        Some(hermite_interpolate(
            p0 as f32,
            p1 as f32,
            p2 as f32,
            p3 as f32,
            self.pitch_factor,
            t,
        ))
    }
}

/// Returns the ration of the note `step` semitones above a root.
/// Example:
///
/// ` semitone_to_hz_ratio(12) -> 2.0 `
/// ` semitone_to_hz_ratio(-12) -> 0.5 `
pub fn semitone_to_hz_ratio(step: i8) -> f32 {
    2.0_f32.powf(step as f32 / 12.0)
}

#[cfg(test)]
mod tests {
    use crate::resample::{
        semitone_to_hz_ratio, HermiteResampler, LanczosResampler, LinearResampler,
    };
    use crate::samples::PhonicMode;
    use crate::{load_wav, write_wav};
    use plotters::prelude::*;
    use rustfft::num_traits::Signed;
    use rustfft::{num_complex::Complex, FftPlanner};
    use std::ops::Neg;
    use test_case::test_case;

    #[test]
    fn repitch_vec() {
        let samples: Vec<i16> = load_wav("tests/sine.wav").unwrap();

        let mut root = LinearResampler::new(&samples, 1.0);
        let mut third = LinearResampler::new(&samples, 5.0 / 4.0);
        let mut fifth = LinearResampler::new(&samples, 3.0 / 2.0);
        let mut sub_oct = LinearResampler::new(&samples, 0.25);

        let mut out: Vec<i16> = Vec::new();

        for _ in 0..samples.len() {
            out.push(
                (root.next().unwrap_or(0.0)
                    + third.next().unwrap_or(0.0)
                    + fifth.next().unwrap_or(0.0)) as i16
                    / 3,
            )
        }

        write_wav("tests/debug/sine_harmony_linear.wav", out, PhonicMode::Mono);
    }

    #[test]
    fn find_difference() {
        let first = load_wav("tests/debug/kalimba_minus_two_octave_linear.wav").unwrap();
        let second = load_wav("tests/debug/kalimba_minus_two_octave_hermite.wav").unwrap();
        let third = load_wav("tests/debug/kalimba_minus_two_octave_lanczos.wav").unwrap();

        let first_second_diff: Vec<i32> = (0..first.len())
            .map(|index| (first[index] - second[index]).abs() as i32)
            .collect();
        let first_third_diff: Vec<i32> = (0..first.len())
            .map(|index| (first[index] - third[index]).abs() as i32)
            .collect();
        let second_third_diff: Vec<i32> = (0..second.len())
            .map(|index| (second[index] - third[index]).abs() as i32)
            .collect();

        let diff_sum_1 = first_second_diff.iter().sum::<i32>();
        let diff_sum_2 = first_third_diff.iter().sum::<i32>();
        let diff_sum_3 = second_third_diff.iter().sum::<i32>();

        println!(
            "linear - hermite {}, per sample {}",
            diff_sum_1,
            diff_sum_1 / first.len() as i32
        );
        println!(
            "linear - lanczos {}, per sample {}",
            diff_sum_2,
            diff_sum_2 / first.len() as i32
        );
        println!(
            "hermite - lanczos {}, per sample {}",
            diff_sum_3,
            diff_sum_3 / second.len() as i32
        );
    }

    #[test_case(3)]
    #[test_case(4)]
    #[test_case(5)]
    #[test_case(10)]
    #[test_case(100)]
    #[ignore]
    fn test_lanczos(window_size: u16) {
        let samples: Vec<i16> = load_wav("tests/kalimba.wav").unwrap();

        let mut resampler = LanczosResampler::new(&samples, 0.25, window_size);
        let output: Vec<i16> = resampler.map(|sample| sample as i16).collect();
        write_wav(
            format!("tests/debug/lanczos_quarter_window_{}.wav", window_size).as_str(),
            output,
            PhonicMode::Stereo,
        )
    }

    #[test]
    fn plot_frequencies() {
        let signal: Vec<i16> = load_wav("tests/debug/lanczos_quarter_window_3.wav").unwrap();

        let mut complex_signal: Vec<Complex<f32>> = signal
            .iter()
            .map(|sample| Complex::new(*sample as f32, 0.0))
            .collect();

        let mut planner = FftPlanner::new();

        let fft = planner.plan_fft_forward(complex_signal.len());

        fft.process(&mut complex_signal);

        let magnitudes: Vec<i64> = complex_signal
            .iter()
            .map(|z| (z.re * z.re + z.im * z.im).sqrt() as i64)
            .collect();

        let root = BitMapBackend::new("tests/debug/lanczos_width_3_plot.png", (1600, 1200))
            .into_drawing_area();
        root.fill(&WHITE).expect("could not fill window");

        let mut chart = ChartBuilder::on(&root)
            .caption("Frequency Spectrum", ("Arial", 24))
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(60)
            .build_cartesian_2d(0..7000_i64, 0..(*magnitudes.iter().max().unwrap()))
            .expect("could not create chart");

        chart
            .configure_mesh()
            .draw()
            .expect("could not configure mesh and draw");

        chart
            .draw_series(LineSeries::new(
                magnitudes
                    .iter()
                    .enumerate()
                    .map(|(i, &y)| ((i * 44100 / magnitudes.len()) as i64, y)),
                &RED,
            ))
            .expect("could not draw plot");
    }

    #[test]
    fn create_chromatic_steps() {
        let input: Vec<i16> = load_wav("tests/sine.wav").unwrap();

        let mut out: Vec<i16> = Vec::new();

        for shift in 1..=12 as i8 {
            let freq_shift = semitone_to_hz_ratio(shift) as f64;

            let mut resampler = LinearResampler::new(&input, freq_shift);
            let pitched: Vec<i16> = resampler.take(22050).map(|sample| sample as i16).collect();
            out.extend(pitched)
        }

        for shift in 1..=12 as i8 {
            let freq_shift = semitone_to_hz_ratio(13 - shift) as f64;

            let mut resampler = LinearResampler::new(&input, freq_shift);
            let pitched: Vec<i16> = resampler.take(22050).map(|sample| sample as i16).collect();
            out.extend(pitched)
        }

        for shift in 1..=12 as i8 {
            let freq_shift = semitone_to_hz_ratio(shift.neg()) as f64;

            let mut resampler = LinearResampler::new(&input, freq_shift);
            let pitched: Vec<i16> = resampler.take(22050).map(|sample| sample as i16).collect();
            out.extend(pitched)
        }

        write_wav("tests/debug/chromatic_sweep.wav", out, PhonicMode::Mono);
    }
}
