//! A module providing a struct for diffusing audio using a polarity shuffle and Hadamard mix technique
//! Diffuser takes an array input and uses lin-alg to perform Hadamard mixer multiplication.
//! Shuffles channels and randomly decides whether to flip polarity.
//! Based on the article "let's write a reverb" by Geraint Luff of signal smith audio
use crate::multi_channel::{HadamardMixer, MultiDelayLine};
use ndarray::{Array, Array1, Ix1};
use rand::{seq::SliceRandom, thread_rng, Rng};

/// A struct that has a mixing object and a multi delay line, performs diffusion of an array of audio samples.
///
/// Delays using multi delay line, shuffles and flips polarity and then mixes using the Hadamard mixer
pub struct Diffuser {
    mixer: HadamardMixer,
    delay: MultiDelayLine,
}

impl Diffuser {
    /// Constructor for the Diffuser struct.
    ///
    /// Takes parameters of number of channels (for the hadamard mixer) and max_time, for setting up the delay line
    pub fn new(num_channels: u8, max_time: f32) -> Self {
        let times: Vec<f32> = (0..num_channels)
            .map(|index| Self::gen_random_time(max_time, num_channels, index))
            .collect();
        Self {
            mixer: HadamardMixer::new(num_channels),
            delay: MultiDelayLine::new(times, 0.0, 1.0, num_channels, 44100),
        }
    }

    /// Generate N random times in a range so that each even Nth division of the range has exactly one time in it.
    fn gen_random_time(max_time: f32, num_channels: u8, channel: u8) -> f32 {
        // width of one cell division (when splitting the time range from 0 to max_time into segments (num channels)
        let cell_size: f32 = max_time / (num_channels as f32);
        let lower_bound: f32 = cell_size * (channel as f32);
        let upper_bound: f32 = cell_size * (channel as f32 + 1.0);
        // random time in range (lower bound -> upper bound, including the upper bound)
        let time: f32 = thread_rng().gen_range(lower_bound..=upper_bound);
        time
    }

    /// Function which takes a 1D array of samples and randomly reorders the channels as well as probabilistically flips polarity
    ///
    ///
    /// E.G:
    ///
    ///     [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    ///
    /// -> `[2, -4 6, 9, -10, 3, 1, 5, -7, 8]`
    ///
    pub fn shuffle_and_flip(&self, xn: Array1<f32>) -> Array<f32, Ix1> {
        let mut rng = thread_rng();
        let polarities = [-1.0, 1.0];
        let mut indices: Vec<usize> = (0..xn.len()).collect();
        indices.shuffle(&mut rng);
        indices
            .iter()
            .map(|index| {
                xn[*index]
                    * match polarities.choose(&mut rng) {
                        Some(polarity) => *polarity,
                        None => 1.0,
                    }
            })
            .collect()
    }

    /// Function combining all the steps for diffusion into a single process.
    pub fn diffuse(&mut self, xn: Array1<f32>) -> Array<f32, Ix1> {
        let delayed = self.delay.process_with_feedback(xn, false);
        let shuffled = self.shuffle_and_flip(delayed);
        self.mixer.mix(shuffled)
    }
}

#[cfg(test)]
mod tests {
    use super::Diffuser;
    use crate::samples::PhonicMode;
    use crate::{load_wav, write_wav};
    use ndarray::arr1;

    #[test]
    fn test_shuffle_flip() {
        let input = arr1(&[1.0, 0.5, 1.0, 0.25]);
        let diffuser = Diffuser::new(4, 0.02);
        let output = diffuser.shuffle_and_flip(input.clone());
        assert_ne!(input, output);
        assert_ne!(input.sum(), output.sum())
    }

    #[test]
    #[ignore]
    fn test_diffusion_series() {
        let diffusers: Vec<Diffuser> = vec![
            Diffuser::new(8, 0.048),
            Diffuser::new(8, 0.096),
            Diffuser::new(8, 0.192),
            Diffuser::new(8, 0.384),
        ];

        let mut input = load_wav("tests/impulse.wav").expect("file loaded incorrectly");
        input.extend(&[0; 44100 * 4]);

        // clone input into the last_samples variable for use in the series iteration.
        let mut read_samples = input.clone();

        // iterate over diffusers in vector, declared above
        for mut diffuser in diffusers {
            let mut write_samples: Vec<i16> = Vec::new();

            for sample in read_samples {
                let sample_array = arr1(&[sample as f32; 8]);
                let diffused = diffuser.diffuse(sample_array);
                write_samples.push(diffused.sum() as i16);
            }
            read_samples = write_samples.clone();
        }

        write_wav(
            "tests/4_series_diffused_click_doubling_8ch.wav",
            read_samples,
            PhonicMode::Mono,
        );
    }
}
