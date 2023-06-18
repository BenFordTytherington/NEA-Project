//! Module which mocks MIDI messages in a very simple sense, optimized to have predetermined timing
//! Contains a struct for mock midi messages, called NoteMessage.
//! This only allows for Midi Note messages, with no note off message, and predetermined timing.
//! This struct interfaces with the interpolator method of repitching.

use crate::resample::semitone_to_hz_ratio;

/// Note message which contains an optional midi note number and duration in seconds
///
/// The note being `None` can be interpreted as the note being gate off, which is used for the gate behaviour of various objects
///
/// The time is used by the midi manager to determine when to load the next note, in a sort of sequence behaviour
pub struct NoteMessage {
    note: Option<u8>,
    time_s: f32,
}

impl NoteMessage {
    /// Function using pattern matching on string, to determine if it is a valid musical note
    /// ## Examples:
    /// 'C5' True
    /// 'H2' False
    /// 'Db6' True
    /// 'E#4' True (evaluates to F4)
    /// 'C' False (an octave needs to be specified)
    pub fn valid_name(name: &str) -> bool {
        // A note name will never be more than 3 characters: Note, Accidental?, Octave
        match name.len() {
            i if i <= 3 => (),
            _ => return false,
        };
        // Matching the first character
        match name.chars().next() {
            Some(a) => match a {
                // The letter name must be a letter between a and g, and is case insensitive
                'A'..='G' | 'a'..='g' => (),
                _ => return false,
            },
            None => return false,
        };
        // Matching the second character
        match name.chars().next() {
            Some(a) => match a {
                // Second may be an accidental
                '#' | 'b' => (),
                // Or an octave
                '0'..='8' => (),
                _ => return false,
            },
            None => return false,
        };
        // Match the last character if it exists
        if let Some(a) = name.chars().next() {
            match a {
                '0'..='8' => (),
                _ => return false,
            }
        };
        // if all previous checks pass, name is valid
        true
    }

    /// Converts a musical note name to a midi note value
    pub fn midi_note_from_name(name: &str) -> u8 {
        if !Self::valid_name(name) {
            panic!("Invalid note name")
        };
        // gets the last character of name, then converts to digit base 10, and multiplies by 12 (octave in semitones)
        let octave = (name
            .chars()
            .nth(name.len() - 1)
            .unwrap()
            .to_digit(10)
            .unwrap()
            * 12) as u8;
        // Converts the note name to an offset from 0, where A is 0. This is where MIDI starts (A0)
        // Also allows for matching accidentals as the same note such as E# and F or Db and C#
        let note = match &name[..name.len() - 1] {
            "A" => 0_i8,
            "A#" | "Bb" => 1,
            "B" | "Cb" => 2,
            "C" | "B#" => -9,
            "C#" | "Db" => -8,
            "D" => -7,
            "D#" | "Eb" => -6,
            "E" | "Fb" => -5,
            "F" | "E#" => -4,
            "F#" | "Gb" => -3,
            "G" => -2,
            "G#" | "Ab" => -1,
            // This should be unreachable
            _ => panic!("Invalid note name"),
        };
        // 21 is the first MIDI note message, as lower than this is used for control values
        (note + (21 + octave) as i8) as u8
    }

    /// The constructor for a midi note given a valid note name and the duration in seconds
    pub fn new(name: &str, time: f32) -> Self {
        Self {
            note: Some(Self::midi_note_from_name(name)),
            time_s: time,
        }
    }

    /// Getter for the current timing of a NoteMessage
    pub fn get_time(&self) -> f32 {
        self.time_s
    }

    /// Get the midi note value or 0
    pub fn get_note(&self) -> u8 {
        self.note.unwrap_or(0)
    }

    /// Reusable constant instance with no note, to save time in removing Note to a gateless value
    const NONE: Self = Self {
        note: None,
        time_s: 0.0,
    };
}

/// Struct which manages midi notes and can output a frequency ratio for repitching.
pub struct MidiManager {
    current_event: NoteMessage,
    current_timer: f32,
}

impl MidiManager {
    /// Constructor with a default value of no midi message
    pub fn new() -> Self {
        Self {
            current_event: NoteMessage::NONE,
            current_timer: 0.0,
        }
    }

    /// Set the current note event given an instance of NoteMessage
    pub fn set_note_event(&mut self, event: NoteMessage) {
        self.current_timer = event.get_time();
        self.current_event = event;
    }

    /// Decrease the timer, used for gate signals, uses 44100Hz sample rate.
    pub fn tick(&mut self) {
        self.current_timer -= 1.0 / 44100.0;
        if self.current_timer < 0.0 {
            self.current_event = NoteMessage::NONE
        }
    }

    /// Returns a boolean based on whether the note is a valid note or 0, which indicates an empty event
    pub fn get_gate(&self) -> bool {
        !matches!(self.current_event.get_note(), 0)
    }

    /// Get the ratio between the current note and middle C (C5), assume the original pitch of your sample is this.
    pub fn get_ratio(&self) -> f32 {
        let note = self.current_event.get_note() as i8;
        // 72 is the midi number of C5 - middle C
        let semitones = -(72 - note);
        semitone_to_hz_ratio(semitones)
    }

    /// Get the number of semitones from middle C
    pub fn get_semitones(&self) -> i8 {
        let note = self.current_event.get_note() as i8;
        // 72 is the midi number of C5 - middle C
        -(72 - note)
    }
}

#[cfg(test)]
mod tests {
    use crate::grain::{GrainManager, GrainMode};
    use crate::midi::{MidiManager, NoteMessage};
    use crate::resample::LinearResampler;
    use crate::samples::PhonicMode;
    use crate::{load_wav, write_wav};
    use once_cell::sync::Lazy;
    use std::collections::VecDeque;

    #[test]
    fn test_note_name() {
        println!("C1: {}", NoteMessage::valid_name("C1"));
        println!("C9: {}", NoteMessage::valid_name("C9"));
        println!("D#4: {}", NoteMessage::valid_name("D#4"));
        println!("H3: {}", NoteMessage::valid_name("H3"));
        println!("dogs: {}", NoteMessage::valid_name("dogs"));
    }

    #[test]
    fn test_midi_conversion() {
        assert_eq!(NoteMessage::midi_note_from_name("C5"), 72);
        assert_eq!(NoteMessage::midi_note_from_name("F#7"), 102);
        assert_eq!(
            NoteMessage::midi_note_from_name("F#7"),
            NoteMessage::midi_note_from_name("Gb7")
        );
    }

    #[test]
    fn test_pitch() {
        let input = load_wav("tests/kalimba.wav").unwrap();
        let mut resampler = LinearResampler::new(&input, 1.0);
        let mut midi_manager = MidiManager::new();

        let mut out: Vec<i16> = Vec::new();

        let mut events = VecDeque::from([
            NoteMessage::new("C5", 1.0),
            NoteMessage::new("E5", 1.0),
            NoteMessage::new("G5", 1.0),
            NoteMessage::new("B5", 1.0),
        ]);
        for _ in 0..(44100 * 4) {
            if midi_manager.get_gate() {
                out.push(resampler.next().unwrap() as i16);
            } else {
                println!("Setting note event to {}", events[0].get_note());
                midi_manager.set_note_event(match events.pop_front() {
                    Some(event) => event,
                    None => NoteMessage::NONE,
                });
                resampler.set_factor(midi_manager.get_ratio() as f64);
            }
            midi_manager.tick();
        }

        write_wav(
            "tests/debug/kalimba_chord_with_midi.wav",
            out,
            PhonicMode::Stereo,
        );
    }
    #[test]
    fn test_with_grains() {
        static AUDIO_BUFFER: Lazy<Vec<i16>> = Lazy::new(|| load_wav("tests/handpan.wav").unwrap());

        let mut manager = GrainManager::new(GrainMode::Cascade(0, 0));
        manager.populate_grains(4, &AUDIO_BUFFER, GrainMode::Cascade(0, 44100));
        manager.set_attack(0.0);
        manager.set_decay(0.0);
        manager.set_sustain(1.0);
        manager.set_release(2.0);
        manager.set_makeup_gain(8.0);

        let mut out: Vec<i16> = Vec::new();

        let mut midi_manager = MidiManager::new();

        let mut events = VecDeque::from([
            NoteMessage::new("C5", 2.0),
            NoteMessage::new("E5", 2.0),
            NoteMessage::new("G5", 2.0),
            NoteMessage::new("B5", 2.0),
        ]);

        manager.gate_trigger(true);
        for _ in 0..4 {
            midi_manager.set_note_event(match events.pop_front() {
                Some(event) => event,
                None => NoteMessage::NONE,
            });
            manager.set_global_pitch(midi_manager.get_semitones());
            for _ in 0..(44100 * 4) {
                out.push(manager.get_next_sample());
                midi_manager.tick();
            }
        }

        write_wav(
            "tests/debug/granular_pitch_with_midi_3.wav",
            out,
            PhonicMode::Stereo,
        );
    }
}
