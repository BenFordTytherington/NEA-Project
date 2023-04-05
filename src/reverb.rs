use crate::diffusion::Diffuser;
use crate::load_wav;
use crate::multi_channel::MultiDelayLine;
use ndarray::{arr1, Array1};

pub struct Reverb {
    delay: MultiDelayLine,
    diffusers: Vec<Diffuser>,
}

impl Reverb {
    pub fn new() -> Self {
        Self {
            delay: MultiDelayLine::new(
                vec![
                    0.136582985935935,
                    0.174364382824060,
                    0.109357268469463,
                    0.135646466920643,
                    0.100459768235823,
                    0.193735635293646,
                    0.14323634964359,
                    0.11213523623693,
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
    pub fn process(&mut self, xn: f32, mix: f32) -> f32 {
        let read_sample = xn.clone();
        let mut read_sample_array = arr1(&[read_sample; 8]);

        for diffuser in &mut self.diffusers {
            let mut write_sample_array;
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
    fn test_reverb() {
        let mut input = load_wav("tests/kalimba.wav").expect("error loading file");
        input.extend(&[0; 44100 * 4]);

        let mut reverb = Reverb::new();
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
