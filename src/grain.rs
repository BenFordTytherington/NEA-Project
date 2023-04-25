use crate::smoothers::{NoSmoother, Smoother};

//  * re-pitch
//     - Resample the audio at the correct rate for pitch shifting and linearly interpolate
//       to bring back to correct pitch
//     - adjustable in real time (hopefully)
//  * Slice
//     - split the indices into more grain objects, using either random lengths divided into the time
//       or uniform sizes.
//     - adjust playback order and specify per grain, the envelope time, pitch, reverse, etc...
//
//  * read and write heads
//     - potential for using concurrency (multi-threaded, simd etc...) for reading and processing multiple grains or samples at once
//

// Grain
//  * Reverse
//     - reverse the list of indices range in order to reverse the playback of the sample.
//     -
//  * Loop
//  * Smooth (windowing)

fn sub_vec<T: Copy>(v: Vec<T>, l: usize, u: usize) -> Vec<T> {
    assert_ne!(l, u);
    assert!(u <= v.len());

    (l..u).map(|index| v[index]).collect()
}

pub struct Grain {
    audio_buffer: &'static Vec<i16>,
    upper_index: usize,
    lower_index: usize,
    reverse: bool,
    looping: bool,
    smoother: Box<dyn Smoother>,
}

impl Grain {
    pub fn new(audio_buffer: &'static Vec<i16>) -> Self {
        Self {
            audio_buffer,
            upper_index: audio_buffer.len(),
            lower_index: 0,
            reverse: false,
            looping: false,
            smoother: Box::new(NoSmoother::new()),
        }
    }
    pub fn get_samples(&self) -> Vec<i16> {
        let mut sub_samples = sub_vec::<i16>(
            self.audio_buffer.clone(),
            self.lower_index,
            self.upper_index,
        );
        match self.reverse {
            true => {
                sub_samples.reverse();
                sub_samples
            }
            false => sub_samples,
        }
    }

    pub fn set_reverse(&mut self, on_off: bool) {
        self.reverse = on_off;
    }

    pub fn set_smoothing(&mut self, smoother_object: impl Smoother + 'static) {
        self.smoother = Box::new(smoother_object);
    }
}

#[cfg(test)]
mod tests {
    use crate::grain::Grain;
    use crate::load_wav;
    use crate::smoothers::NoSmoother;
    use once_cell::sync::Lazy;

    #[test]
    fn test_init() {
        static AUDIO_BUFFER: Lazy<Vec<i16>> = Lazy::new(|| load_wav("tests/amen_br.wav").unwrap());
        let mut grain = Grain::new(&AUDIO_BUFFER);
    }

    #[test]
    fn test_get() {
        static AUDIO_BUFFER: Lazy<Vec<i16>> = Lazy::new(|| load_wav("tests/amen_br.wav").unwrap());

        let mut grain = Grain::new(&AUDIO_BUFFER);
        let grain_samples = grain.get_samples();
    }

    #[test]
    fn test_set() {
        static AUDIO_BUFFER: Lazy<Vec<i16>> = Lazy::new(|| load_wav("tests/amen_br.wav").unwrap());

        let mut grain = Grain::new(&AUDIO_BUFFER);
        let mut grain_original_samples = grain.get_samples();

        grain.set_reverse(true);
        grain_original_samples.reverse();
        assert_eq!(grain.get_samples(), grain_original_samples);

        grain.set_smoothing(NoSmoother::new())
    }
}
