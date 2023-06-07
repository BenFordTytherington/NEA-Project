use crate::resample::semitone_to_hz_ratio;

/// Receive mock midi message (not NIH plug format) and for that message, get variety of things:
///     - get frequency in hz
///     - get frequency ratio relative to middle c, which is used for reptiching
///     - set a gate signal
/// Store a queue of midi messages
/// Implement a sequencer.

pub struct NoteMessage {
    note: Option<u8>,
    time_s: f32,
}

impl NoteMessage {
    pub fn valid_name(name: &str) -> bool {
        match name.len() {
            i if i <= 3 => (),
            _ => return false,
        };
        match name.chars().nth(0) {
            Some(a) => match a {
                'A'..='G' | 'a'..='g' => (),
                _ => return false,
            },
            None => return false,
        };
        match name.chars().nth(1) {
            Some(a) => match a {
                '#' | 'b' => (),
                '0'..='8' => (),
                _ => return false,
            },
            None => return false,
        };
        match name.chars().nth(2) {
            Some(a) => match a {
                '0'..='8' => (),
                _ => return false,
            },
            None => (),
        };
        true
    }

    pub fn midi_note_from_name(name: &str) -> u8 {
        let notes = if !Self::valid_name(name) {
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
            _ => panic!("Invalid note name"),
        };
        (note + (21 + octave) as i8) as u8
    }

    pub fn new(name: &str, time: f32) -> Self {
        Self {
            note: Some(Self::midi_note_from_name(name)),
            time_s: time,
        }
    }

    pub fn get_time(&self) -> f32 {
        self.time_s
    }

    pub fn get_note(&self) -> u8 {
        match self.note {
            Some(note) => note,
            None => 0,
        }
    }

    const NONE: Self = Self {
        note: None,
        time_s: 0.0,
    };
}

pub struct MidiManager {
    current_event: NoteMessage,
    current_timer: f32,
}

impl MidiManager {
    pub fn new() -> Self {
        Self {
            current_event: NoteMessage::NONE,
            current_timer: 0.0,
        }
    }

    pub fn set_note_event(&mut self, event: NoteMessage) {
        self.current_timer = event.get_time();
        self.current_event = event;
    }

    pub fn tick(&mut self) {
        self.current_timer -= 1.0 / 44100.0;
        if self.current_timer < 0.0 {
            self.current_event = NoteMessage::NONE
        }
    }

    pub fn get_gate(&self) -> bool {
        match self.current_event.get_note() {
            0 => false,
            _ => true,
        }
    }

    pub fn get_ratio(&self) -> f32 {
        let note = self.current_event.get_note() as i8;
        // 72 is the midi number of C5 - middle C
        let semitones = -1 * (72 - note);
        semitone_to_hz_ratio(semitones)
    }

    pub fn get_semitones(&self) -> i8 {
        let note = self.current_event.get_note() as i8;
        // 72 is the midi number of C5 - middle C
        -1 * (72 - note)
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
