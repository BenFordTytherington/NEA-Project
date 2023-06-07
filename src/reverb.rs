//! A module bringing together various pre-written components into a reverb algorithm.
//! Currently not working due to un-found bug.
//!
//! Uses FDN architecture and is heavily based on the article "Let's write a reverb" by Geraint Luff of Signal Smith audio

use crate::diffusion::Diffuser;
use crate::multi_channel::MultiDelayLine;
use ndarray::arr1;

/// Struct combining multi delay, and diffusers into an FDN reverb.
///
/// Has a single multi delay line used with feedback to increase echo density
///
/// Has a vector of Diffusers, usually between 3 - 7, Used to blend / smear audio to create the reverb effect.
/// CURRENTLY WIP.
pub struct Reverb {
    delay: MultiDelayLine,
    diffusers: Vec<Diffuser>,
}

impl Default for Reverb {
    fn default() -> Self {
        Self {
            delay: MultiDelayLine::new(
                vec![
                    0.13658298, 0.17436438, 0.10935726, 0.13564646, 0.10045976, 0.19373563,
                    0.14323634, 0.11213523,
                ],
                0.85,
                1.0,
                8,
                44100,
            ),
            diffusers: vec![
                Diffuser::new(8, 0.020),
                Diffuser::new(8, 0.040),
                Diffuser::new(8, 0.080),
                Diffuser::new(8, 0.160),
            ],
        }
    }
}

impl Reverb {
    /// Constructor for the reverb struct.
    ///
    /// Default Values:
    ///
    /// * delay: 8 Channel multi delay with random chosen times between 100ms and 200ms
    ///
    /// * Diffusers: 4 series, 8 Channel diffusers with maximum times doubling each diffuser
    ///     from 20ms up to 160ms
    pub fn new(diffuser_count: usize, diffuser_start: f32, channels: u8) -> Self {
        Self {
            delay: MultiDelayLine::new(
                vec![
                    0.13658298, 0.17436438, 0.10935726, 0.13564646, 0.10045976, 0.19373563,
                    0.14323634, 0.11213523,
                ],
                0.85,
                1.0,
                channels,
                44100,
            ),
            diffusers: (0..diffuser_count)
                .map(|index| Diffuser::new(channels, diffuser_start * (index + 1) as f32))
                .collect(),
        }
    }

    /// Process a single float by duplicating it to all channels and performing the reverb algorithm
    /// First the sample is passed through the diffuser series.
    ///
    /// Then it is delayed with feedback and mixed down with the dry signal by the mix parameter.
    pub fn process(&mut self, xn: f32, mix: f32) -> f32 {
        let read_sample = xn;
        let mut read_sample_array = arr1(&[read_sample; 8]);

        for diffuser in &mut self.diffusers {
            let write_sample_array;
            let diffused = diffuser.diffuse(read_sample_array.clone());
            write_sample_array = diffused;
            read_sample_array = write_sample_array.clone();
        }

        let delayed = self.delay.process_with_feedback(read_sample_array, true);

        ((1.0 - mix) * xn) + (mix * delayed.sum())
    }
}

#[cfg(test)]
mod tests {
    use crate::reverb::Reverb;
    use crate::samples::PhonicMode;
    use crate::{load_wav, write_wav};

    #[test]
    #[ignore]
    fn test_reverb() {
        let mut input = load_wav("tests/kalimba.wav").expect("error loading file");
        input.extend(&[0; 44100 * 4]);

        let mut reverb = Reverb::new(4, 0.02, 8);
        let mut output: Vec<i16> = Vec::new();
        for sample in input {
            output.push(reverb.process(sample as f32, 1.0) as i16)
        }
        write_wav(
            "tests/kalimba_reverb_test_less_diffusion.wav",
            output,
            PhonicMode::Stereo,
        );
    }
}
