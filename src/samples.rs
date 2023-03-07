/// An enum used to store state of either stereophonic or monophonic in audio structs
#[derive(Default)]
pub enum PhonicMode {
    #[default]
    Stereo,
    Mono,
}
/// A trait implemented on structs that hold samples
/// # Methods
/// * `get_frames`: a function which returns a vector of tuple containing left and right samples
/// * `from_mono`: constructs a struct with interleaved samples created from a single mono vector of samples
/// * `from_stereo`: constructs a struct with interleaved samples from 2 mono left and right vectors
pub trait Samples<T> {
    fn get_frames(&self) -> Vec<(T, T)>;

    fn from_mono(samples: Vec<T>) -> Self;

    fn from_stereo(left: Vec<T>, right: Vec<T>) -> Self;
}

/// A generic helper function to interleave 2 vectors of equal length into a single vector
fn interleave<T: Copy>(left: Vec<T>, right: Vec<T>) -> Vec<T> {
    assert_eq!(left.len(), right.len());
    let mut output: Vec<T> = Vec::new();
    for index in 0..left.len() {
        output.push(left[index]);
        output.push(right[index]);
    }
    output
}

/// A struct storing a vector of integer samples with associated methods and constructors
#[derive(Default)]
pub struct IntSamples {
    samples: Vec<i16>,
}

impl IntSamples {
    /// Constructs an IntSamples instance from interleaved samples
    pub fn new(samples: Vec<i16>) -> Self {
        Self { samples }
    }

    /// Gets a copy of the samples for processing
    pub fn samples(&self) -> Vec<i16> {
        self.samples.clone()
    }
}

/// A struct storing a vector of float samples with associated methods and constructors
#[derive(Default)]
pub struct FloatSamples {
    samples: Vec<f32>,
}

impl FloatSamples {
    /// Constructs a FloatSamples instance from interleaved samples
    pub fn new(samples: Vec<f32>) -> Self {
        Self { samples }
    }

    /// Gets a copy of the samples for processing
    pub fn samples(&self) -> Vec<f32> {
        self.samples.clone()
    }
}

// the default preference will be to work with stereo samples as either i16 or f64
// Samples implements methods to create stereo from mono and to return frames of stereo samples

impl Samples<i16> for IntSamples {
    fn get_frames(&self) -> Vec<(i16, i16)> {
        let mut frames: Vec<(i16, i16)> = Vec::new();
        for f in self.samples.chunks(2) {
            match f {
                [a, b] => frames.push((*a, *b)),
                _ => panic!("Sample vector is empty or has uneven length"),
            }
        }
        frames
    }

    /// Constructs a stereo sample object by duplicating the mono input and interleaving
    fn from_mono(samples: Vec<i16>) -> Self {
        let left = samples.clone();
        let right = samples.clone();
        Self {
            samples: interleave(left, right),
        }
    }

    /// Constructs a stereo sample object by interleaving samples
    fn from_stereo(left: Vec<i16>, right: Vec<i16>) -> Self {
        Self {
            samples: interleave(left, right),
        }
    }
}

impl Samples<f32> for FloatSamples {
    fn get_frames(&self) -> Vec<(f32, f32)> {
        let mut frames: Vec<(f32, f32)> = Vec::new();
        for f in self.samples.chunks(2) {
            match f {
                [a, b] => frames.push((*a, *b)),
                _ => panic!("Sample vector is empty or has uneven length"),
            }
        }
        frames
    }

    /// Constructs a stereo sample object by duplicating the mono input and interleaving
    fn from_mono(samples: Vec<f32>) -> Self {
        let left = samples.clone();
        let right = samples.clone();
        Self {
            samples: interleave(left, right),
        }
    }

    /// Constructs a stereo sample object by interleaving samples
    fn from_stereo(left: Vec<f32>, right: Vec<f32>) -> Self {
        Self {
            samples: interleave(left, right),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::samples::{FloatSamples, IntSamples, Samples};

    #[test]
    fn test_int_new() {
        let samples = IntSamples::new(vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5]);
        assert_eq!(samples.samples, vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5])
    }

    #[test]
    fn test_int_from_mono() {
        let samples = IntSamples::from_mono(vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(samples.samples, vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5])
    }

    #[test]
    fn test_int_from_stereo() {
        let samples = IntSamples::from_stereo(vec![0, 1, 2, 3, 4, 5], vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(samples.samples, vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5])
    }

    #[test]
    fn test_int_get_frames() {
        let samples = IntSamples::new(vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5]);
        assert_eq!(
            samples.get_frames(),
            vec![(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5)]
        )
    }

    #[test]
    fn test_float_new() {
        let samples = FloatSamples::new(vec![
            0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 4.0, 4.0, 5.0, 5.0,
        ]);
        assert_eq!(
            samples.samples,
            vec![0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 4.0, 4.0, 5.0, 5.0]
        )
    }

    #[test]
    fn test_float_from_mono() {
        let samples = FloatSamples::from_mono(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(
            samples.samples,
            vec![0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 4.0, 4.0, 5.0, 5.0]
        )
    }

    #[test]
    fn test_float_from_stereo() {
        let samples = FloatSamples::from_stereo(
            vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
            vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
        );
        assert_eq!(
            samples.samples,
            vec![0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 4.0, 4.0, 5.0, 5.0]
        )
    }

    #[test]
    fn test_float_get_frames() {
        let samples = FloatSamples::new(vec![
            0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 4.0, 4.0, 5.0, 5.0,
        ]);
        assert_eq!(
            samples.get_frames(),
            vec![
                (0.0, 0.0),
                (1.0, 1.0),
                (2.0, 2.0),
                (3.0, 3.0),
                (4.0, 4.0),
                (5.0, 5.0)
            ]
        )
    }
}
