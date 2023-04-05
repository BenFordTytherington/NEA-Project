use crate::multi_channel::{HadamardMixer, MultiDelayLine};
use ndarray::{Array, Array1, Ix1};
use rand::{seq::SliceRandom, thread_rng, Rng};

pub struct Diffuser {
    num_channels: i8,
    mixer: HadamardMixer,
    delay: MultiDelayLine,
}

impl Diffuser {
    pub fn new(num_channels: i8, max_time: f32) -> Self {
        let times: Vec<f32> = (0..num_channels)
            .map(|index| Self::gen_random_time(max_time, num_channels, index))
            .collect();
        Self {
            num_channels,
            mixer: HadamardMixer::new(num_channels),
            delay: MultiDelayLine::new(times, 0.0, 1.0, num_channels, 44100),
        }
    }

    fn gen_random_time(max_time: f32, num_channels: i8, channel: i8) -> f32 {
        // width of one cell division (when splitting the time range from 0 to max_time into segments (num channels)
        let cell_size: f32 = max_time / (num_channels as f32);
        let lower_bound: f32 = cell_size * (channel as f32);
        let upper_bound: f32 = cell_size * (channel as f32 + 1.0);
        // random time in range (lower bound -> upper bound, including the upper bound)
        let time: f32 = thread_rng().gen_range(lower_bound..=upper_bound);
        time
    }

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

    pub fn diffuse(&mut self, xn: Array1<f32>) -> Array<f32, Ix1> {
        let delayed = self.delay.process_with_feedback(xn, false);
        let shuffled = self.shuffle_and_flip(delayed);
        let mixed = self.mixer.mix(shuffled);

        mixed
    }
}

#[cfg(test)]
mod tests {
    use super::Diffuser;
    use crate::samples::PhonicMode;
    use crate::{load_wav, load_wav_float, write_wav, write_wav_float};
    use ndarray::arr1;

    #[test]
    fn test_shuffle_flip() {
        let input = arr1(&[1.0, 0.5, 1.0, 0.25]);
        let diffuser = Diffuser::new(4, 0.02);
        let output = diffuser.shuffle_and_flip(input.clone());
        assert_ne!(input, output);
    }

    #[test]
    #[ignore]
    fn test_diffusion_series() {
        let mut diffusers: Vec<Diffuser> = vec![
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
