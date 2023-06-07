//! A module containing useful structs and functions for time based conversions
//! Contains functions / structs used to convert between seconds, samples and tempo based time.

/// An enum used for time divisions relative to a bar.
#[derive(Clone, Default)]
pub enum TimeDiv {
    /// A bar
    Whole,
    /// Half a bar
    Half,
    #[default]
    /// A quarter of a bar ( A beat )
    Quarter,
    /// An eight of a bar ( A half note )
    Eighth,
    /// A sixteenth of a bar ( Quarter note )
    Sixteenth,
}

/// An enum containing variants for different note modifiers, regular, dotted and triplet.
///
/// Non exhaustive because an option for generic tuplets may be added.
#[non_exhaustive]
#[derive(Default, Clone)]
pub enum NoteModifier {
    #[default]
    /// A normal note (1 X normal length)
    Regular,
    /// A dotted note (1.5 X normal length)
    Dotted,
    /// A triplet (0.666 X normal length)
    Triplet,
}

/// A struct that contains all the necessary information about a note timing and can be converted to seconds
/// ## Attributes:
/// * `division`: A time division enum variant (multiple of a bar)
///
/// * `bpm`: The bpm (beats per minute) of the timing in order to tempo sync.
///
/// * `modifier`: A NoteModifier variant, which differentiates different types of notes (triplet, dotted, regular)
#[derive(Clone)]
pub struct Timing {
    division: TimeDiv,
    bpm: i16,
    modifier: NoteModifier,
}

impl Timing {
    /// Constructor for Timing struct.
    ///
    /// Takes a time division, bpm and note modifier and returns a Timing struct.
    pub fn new(div: TimeDiv, bpm: i16, modifier: NoteModifier) -> Self {
        Self {
            division: div,
            bpm,
            modifier,
        }
    }

    /// A method to calculate the amount of time in seconds that the instance of Timing takes to complete
    pub fn to_seconds(&self) -> f32 {
        let bar_length_seconds: f32 = 240.0 / self.bpm as f32; // 4 beats at the bpm in seconds is 60 / bpm (1 beat) x 4 or 240 / bpm
        let divisor = match self.division {
            TimeDiv::Whole => 1.0,
            TimeDiv::Half => 2.0,
            TimeDiv::Quarter => 4.0,
            TimeDiv::Eighth => 8.0,
            TimeDiv::Sixteenth => 16.0,
        };

        let scalar = match self.modifier {
            NoteModifier::Regular => 1.0,
            NoteModifier::Dotted => 3.0 / 2.0,
            NoteModifier::Triplet => 2.0 / 3.0,
        };
        (bar_length_seconds / divisor) * scalar
    }

    /// Return the timing object as a number of samples at a sample rate (parameter)
    pub fn to_samples(&self, sample_rate: f32) -> usize {
        (self.to_seconds() * sample_rate) as usize
    }

    /// A setter for the time division. Accepts a time division enum variant as a parameter
    pub fn set_division(&mut self, division: TimeDiv) {
        self.division = division;
    }

    /// A setter for the bpm. Accepts an i16 as a parameter
    pub fn set_bpm(&mut self, bpm: i16) {
        self.bpm = bpm;
    }

    /// A setter for the note modifier. Accepts a note modifier enum variant as a parameter
    pub fn set_modifier(&mut self, modifier: NoteModifier) {
        self.modifier = modifier;
    }

    /// Getter for time division. Returns a `TimeDiv` variant
    pub fn division(&self) -> TimeDiv {
        self.division.clone()
    }

    /// Getter for BPM. Returns an i16
    pub fn bpm(&self) -> i16 {
        self.bpm
    }

    /// Getter for note modifier. Returns a `NoteModifier`
    pub fn modifier(&self) -> NoteModifier {
        self.modifier.clone()
    }
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            division: Default::default(),
            bpm: 120,
            modifier: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TimeDiv, Timing};
    use crate::timing::NoteModifier;
    #[test]
    fn test_time_calculator() {
        let correct_times: Vec<f32> = vec![1.714, 0.857, 0.429, 0.214, 0.107];
        let calc_times: Vec<f32> = [
            TimeDiv::Whole,
            TimeDiv::Half,
            TimeDiv::Quarter,
            TimeDiv::Eighth,
            TimeDiv::Sixteenth,
        ]
        .into_iter()
        .map(|time_d| Timing::new(time_d, 140, NoteModifier::Regular).to_seconds())
        .collect();

        for index in 0..5 {
            let diff = (correct_times[index] - calc_times[index]).abs();
            assert!(diff <= 0.001)
        }
    }
}
