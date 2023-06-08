//! A module containing 3 Structs and an Enum
//! Grain:
//!     The Grain struct represents a grain of audio data that can be played back.
//!     It contains various fields including an audio buffer, upper and lower index values,
//!     grain ID, a reverse flag, a smoother object, and others.
//!
//! Id Manager:
//!     The IdManager struct manages IDs by keeping track of the next available ID in its next_id field.
//!     Used to distribute an index to each grain created by the grain manager
//!
//! Grain Manager:
//!     This code defines a GrainManager struct with several fields,
//!     including an IdManager to assign unique IDs to each grain, a Vec of Grain objects,
//!     and various parameters to control the behavior of the grains.
//!     3 Modes:
//!         - Cloud:
//!             For Cloud mode, each grain is assigned a random lower and upper index within a range
//!             determined by variation and start_index, and some additional random parameters are set.
//!         - Sequence:
//!             For Sequence mode, the audio buffer is divided into equal-sized grains,
//!             and each grain is assigned a lower and upper index based on its position in the sequence.
//!         - Cascade:
//!             For Cascade mode, the audio buffer is divided into equal-sized chunks,
//!             and each grain is assigned a lower and upper index based on its position in the cascade.
//!
use crate::envelope::ADSREnvelope;
use crate::interpolators::lerp;
use crate::resample::{semitone_to_hz_ratio, LinearResampler};
use crate::smoothers::{HannSmoother, Smoother};
use rand::prelude::{thread_rng, Rng, SliceRandom};

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

// Grain
//  * Reverse
//     - reverse the list of indices range in order to reverse the playback of the sample.
//     -
//  * Loop
//  * Smooth (windowing)

/// Struct used to assign an index to an object, keeping track of a sequence of objects using a next_id variable
/// Increments ID by 1 each time.
#[derive(Default)]
pub struct IdManager {
    next_id: usize,
}

impl IdManager {
    /// Constructor for ID manager
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    /// Function simply returning the next index. Currently just increments by 1 and returns the index
    pub fn get_next_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

/// Struct used to store a fragment of an audio buffer, without copying it's data.
/// Can have parameters set with fairly low overhead.
/// ## Attributes
/// * `audio_buffer`: A reference to a static audio buffer using i16 samples
///
/// * `upper_index`: The last index stored in the grain, must be greater than lower_index,
///     and must be less than the audio buffer length
///
/// * `lower_index`: The first index stored in the grain, must be lesser than upper_index,
///     and must be greater than 0
///
/// * `grain_id`: An ID assigned by an ID manager, used to keep track of sequence in the grain playback code
///
/// * `next_id`: The next grains ID in sequence, used in sequential playback and looping. By default is reset to instance index + 1
///
/// * `index_mod`: The index modulo, used to loop indices back to 0 in sequence playback
///
/// * `reverse`: A flag used to determine whether to reverse sample playback
///
/// * `looping`: A flag used to determine whether or not to loop the grain. Sets the next grain ID to itself if true
///
/// * `smoother`: An object implementing the `Smoother` trait, dynamically dispatched and therefore heap allocated.
///     Performs smoothing using an index based approach.
///
/// * `smooth_factor`: Depth parameter for the smoothing objects result, range 0 - 1. Will cause no smoothing at 0. Will cause full smoothing at 1
///
/// * `current_index`: The current index of sample playback, used to iterate through the grains samples
///
/// * `lock_playback`: A boolean which if true prevents the playback of a grain being interrupted by index setters
///
/// * `next_upper`: Used with the `lock_playback` bool to store the next upper index to set, which will be applied once the grain finishes playback
///
/// * `next_lower`: Used with the `lock_playback` bool to store the next lower index to set, which will be applied once the grain finishes playback
pub struct Grain {
    audio_buffer: &'static Vec<i16>,
    upper_index: usize,
    lower_index: usize,
    grain_id: usize,
    next_id: usize,
    index_mod: usize,
    reverse: bool,
    looping: bool,
    smoother: Box<dyn Smoother>,
    smooth_factor: f32,
    current_index: usize,
    lock_playback: bool,
    next_upper: usize,
    next_lower: usize,
    resampler: LinearResampler<'static>,
    pitch_enable: bool,
}

impl Grain {
    /// Constructor for a grain, accepts a reference to a static audio buffer,
    /// an ID to assign and the index_mod (number of grains used for sequence playback)
    /// ## Default settings:
    /// * lower and upper index: The start and end of the buffer respectively
    ///
    /// * next_id: the instance ID + 1
    ///
    /// * reverse and loop: false
    ///
    /// * smoother: HannSmoother
    ///
    /// * smooth factor: 35%
    ///
    /// * index parameters: 0
    ///
    /// * lock_playback: false (enabled in populate grains function)
    ///
    pub fn new(audio_buffer: &'static Vec<i16>, id: usize, index_mod: usize, pitch: i8) -> Self {
        Self {
            audio_buffer,
            upper_index: audio_buffer.len(),
            lower_index: 0,
            grain_id: id,
            next_id: (id + 1) % index_mod,
            index_mod,
            reverse: false,
            looping: false,
            smoother: Box::new(HannSmoother::new()),
            smooth_factor: 1.0,
            current_index: 0,
            lock_playback: false,
            next_upper: 0,
            next_lower: 0,
            resampler: LinearResampler::new(
                audio_buffer.as_slice(),
                semitone_to_hz_ratio(pitch) as f64,
            ),
            pitch_enable: true,
        }
    }

    /// Return the next sample of playback, may be from a sequential grain or multiple grains, with output averaged.
    /// Optional smoothing through the `smoothed` Boolean
    pub fn get_next_sample(&mut self, smoothed: bool) -> i16 {
        match self.pitch_enable {
            false => {
                let index = match self.reverse {
                    true => self.upper_index - self.current_index,
                    false => self.lower_index + self.current_index,
                };

                let value = match smoothed {
                    true => {
                        (self.audio_buffer[index] as f32
                            * ((self.smooth_factor * self.smoother.get_index(self.current_index))
                                + (1.0 - self.smooth_factor))) as i16
                    }
                    false => self.audio_buffer[index],
                };
                self.current_index = (self.current_index + 1) % (self.len());

                // runs after 1 full loop of the grain
                if self.current_index == 0 {
                    self.lower_index = self.next_lower;
                    self.upper_index = self.next_upper;
                    self.smoother.set_length(self.len());
                }

                value
            }
            true => {
                let index: f32 = match self.reverse {
                    true => self.upper_index as f32 - self.resampler.get_position() as f32,
                    false => self.lower_index as f32 + self.resampler.get_position() as f32,
                };

                let sample = lerp(
                    self.audio_buffer[index.floor() as usize] as f32,
                    self.audio_buffer[index.floor() as usize + 1] as f32,
                    index.fract(),
                );

                let smooth_value = lerp(
                    self.smoother.get_index(index.floor() as usize),
                    self.smoother.get_index(index.floor() as usize + 1),
                    index.fract(),
                );

                let value = match smoothed {
                    true => {
                        (sample
                            * ((self.smooth_factor * smooth_value) + (1.0 - self.smooth_factor)))
                            as i16
                    }
                    false => sample as i16,
                };

                // runs after 1 full loop of the grain
                if self.resampler.increment() {
                    self.lower_index = self.next_lower;
                    self.upper_index = self.next_upper;
                    self.smoother.set_length(self.len());
                }

                value
            }
        }
    }

    /// Setter for next grain ID to be loaded after a full playback
    fn set_next_id(&mut self, id: usize) {
        self.next_id = id;
    }

    /// Set the read index for samples from this grain
    pub fn set_sample_index(&mut self, index: usize) {
        self.current_index = index;
    }

    /// Toggle the reverse status of the grain on or off
    pub fn set_reverse(&mut self, on_off: bool) {
        self.reverse = on_off;
    }

    /// Lock playback, meaning that setting an index will not occur until after grain playback has completed
    pub fn lock_playback(&mut self) {
        self.lock_playback = true;
    }

    /// Unlock playback, meaning that setting an index will immediately take effect
    pub fn unlock_playback(&mut self) {
        self.lock_playback = false;
    }

    /// Set the looping boolean to on or off and adjust the next_id field accordingly:
    ///     enabled => instance ID (causing it to restart once finished)
    ///     disabled => instance ID + 1 (causing it to jump to next grain in sequence once finished)
    pub fn set_looping(&mut self, on_off: bool) {
        self.looping = on_off;
        match self.looping {
            true => self.set_next_id(self.grain_id),
            false => self.set_next_id((self.grain_id + 1) % self.index_mod),
        };
    }

    /// The length of the grain in samples, used to adjust smoother settings
    pub fn len(&self) -> usize {
        self.upper_index - self.lower_index
    }

    /// Boolean function returning if the length of the grain contains no samples
    pub fn is_empty(&self) -> bool {
        self.upper_index <= self.lower_index
    }

    /// Setter to assign a new smoother object to the grain.
    pub fn set_smoothing(&mut self, smoother_object: impl Smoother + 'static) {
        self.smoother = Box::new(smoother_object);
        self.smoother.set_length(self.len());
    }

    /// Setter for the smoothing factor as a percentage between 0 and 1
    pub fn set_smooth_factor(&mut self, factor: f32) {
        self.smooth_factor = factor;
    }

    /// Set the upper index of the grain, including logic for locked playback
    pub fn set_upper_index(&mut self, upper_index: usize) {
        match self.lock_playback {
            true => self.next_upper = upper_index,
            false => {
                self.upper_index = upper_index;
                self.next_upper = upper_index;
                self.smoother.set_length(self.len());
                self.resampler
                    .set_buffer(&self.audio_buffer[self.lower_index..=self.upper_index])
            }
        }
    }

    /// Set the lower index of the grain, including logic for locked playback
    pub fn set_lower_index(&mut self, lower_index: usize) {
        match self.lock_playback {
            true => self.next_lower = lower_index,
            false => {
                self.lower_index = lower_index;
                self.next_lower = lower_index;
                self.smoother.set_length(self.len());
                self.resampler
                    .set_buffer(&self.audio_buffer[self.lower_index..self.upper_index])
            }
        }
    }

    /// Update the length of the smoother object with the grains current length.
    /// Usually called after a setter method is run.
    pub fn update_smoother(&mut self) {
        self.smoother.set_length(self.len());
    }

    /// Set the resamplers pitch as a number of semitones
    pub fn set_pitch(&mut self, pitch: i8) {
        self.resampler
            .set_factor(semitone_to_hz_ratio(pitch) as f64);
    }

    /// Set the resamplers pitch as a frequency ratio
    pub fn set_pitch_freq(&mut self, freq: f32) {
        self.resampler.set_factor(freq as f64);
    }

    /// Set the grain position in the sample without changing length.
    ///
    /// Abides playback lock setting.
    ///
    /// If the pos with the current length would put the upper index outside the audio buffer range,
    /// The position closest to the end with allowed length (last index being last index of audio buffer)
    /// is used.
    pub fn set_pos(&mut self, pos: usize) {
        let len = self.len();
        match self.lock_playback {
            true => {
                if (pos + len) >= self.audio_buffer.len() - 1 {
                    self.next_lower = self.audio_buffer.len() - len;
                    self.next_upper = self.audio_buffer.len() - 1;
                } else {
                    self.next_lower = pos;
                    self.next_upper = pos + len;
                }
            }
            false => {
                if (pos + len) >= self.audio_buffer.len() - 1 {
                    self.lower_index = self.audio_buffer.len() - len;
                    self.upper_index = self.audio_buffer.len() - 1;
                } else {
                    self.lower_index = pos;
                    self.upper_index = pos + len;
                }
                self.resampler
                    .set_buffer(&self.audio_buffer[self.lower_index..=self.upper_index])
            }
        }
    }
}

/// An enum for storing the different modes of the granular manager and their associated metadata
pub enum GrainMode {
    /// Playback in order of grain ID, grains are read one at a time
    Sequence,
    /// Creates grains in range between 2 parameters (lower, upper) and cascades the times up to full length of range.
    ///
    /// Grains are read concurrently
    Cascade(usize, usize), //(lower, upper) index to cascade
    /// Creates grains with a specified length, variation and start position (length, variation, start_pos).
    ///
    /// Grain variation places grains with indices up to half the variation length before and after in position.
    /// Variation is a multiple of the grain length
    ///
    /// Grains are read concurrently
    Cloud(usize, f32, usize), //(grain length, variation, start_pos)
}

/// A struct used to orchestrate and manage multiple grain objects as well as synchronize their playback.
/// ## Attributes:
/// * `id_manager`: An instance of `IdManager` used to assign an index to each grain, usually done in sequence
///
/// * `grains`: A vector of `Grain` objects being orchestrated.
///
/// * `grain_index`: A index (usize) used to determine which grain is currently being played back in sequence mode
///
/// * `sample_index`: The sample index being used to synchronize grain playback, used for multiple grains in some modes
///
/// * `grain_count`: The number of grains stored in the manager object.
///     Used in initialization, as the index mod value for grains
///
/// * `mode`: A grain mode variant, determines how to populate grains and how to read the next sample
///
/// * `makeup_gain`: The output makeup gain of the system,
///     short grains can cause reduced volume and therefore can be compensated
///
pub struct GrainManager {
    id_manager: IdManager,
    grains: Vec<Grain>,
    grain_index: usize,
    sample_index: usize,
    grain_count: usize,
    mode: GrainMode,
    makeup_gain: f32,
    global_pitch: i8,
    env: ADSREnvelope,
}

impl Default for GrainManager {
    /// The default construction of GrainManager
    fn default() -> Self {
        Self {
            id_manager: IdManager::new(),
            grains: Vec::new(),
            grain_index: 0,
            sample_index: 0,
            grain_count: 0,
            mode: GrainMode::Sequence,
            makeup_gain: 3.0,
            global_pitch: 1,
            env: ADSREnvelope::new(2.5, 1.0, 0.75, 2.0),
        }
    }
}

impl GrainManager {
    /// Constructor that creates a new GrainManager with specified mode
    pub fn new(mode: GrainMode) -> Self {
        Self {
            mode,
            ..Default::default()
        }
    }

    /// Function to populate the grains buffer with a number of grains, all from the same audio buffer,
    ///     by a specified mode, including that modes individual metadata
    ///
    /// Pushes grains to the grains vector with increasing indices assigned by the instances IdManager
    pub fn populate_grains(
        &mut self,
        grain_count: usize,
        audio_buffer: &'static Vec<i16>,
        mode: GrainMode,
    ) {
        self.env.setup();
        self.grains = (0..grain_count)
            .map(|_| Grain::new(audio_buffer, self.id_manager.get_next_id(), grain_count, 0))
            .collect();

        match mode {
            GrainMode::Sequence => {
                let grain_len = audio_buffer.len() / grain_count;

                (0..grain_count).for_each(|index| {
                    let grain = &mut self.grains[index];
                    grain.set_lower_index(index * grain_len);
                    grain.set_upper_index((index + 1) * grain_len);
                    grain.update_smoother();
                    grain.lock_playback();
                });
            }
            GrainMode::Cloud(grain_len, variation, start_index) => {
                let mut rng = thread_rng();
                (0..grain_count).for_each(|index| {
                    let variation_depth: f32 = rng.gen();
                    let grain = &mut self.grains[index];
                    let lower = start_index.saturating_sub(
                        (variation * 0.5 * variation_depth * (grain_len as f32)) as usize,
                    );
                    let mut upper = start_index.saturating_add(
                        grain_len
                            + (variation * 0.5 * variation_depth * (grain_len as f32)) as usize,
                    );
                    if upper > grain.audio_buffer.len() {
                        upper = grain.audio_buffer.len();
                    }

                    let octave: i8 = *[-1, 0, 1].choose(&mut rng).unwrap();
                    grain.set_lower_index(lower);
                    grain.set_upper_index(upper);
                    grain.set_looping(true);
                    grain.set_reverse(rng.gen_bool(0.25));
                    grain.set_pitch(12 * octave);
                    grain.update_smoother();
                    grain.lock_playback();
                });
            }
            GrainMode::Cascade(lower, upper) => {
                (0..grain_count).for_each(|index| {
                    let grain_len = (upper - lower) / grain_count;
                    let grain = &mut self.grains[index];
                    grain.set_lower_index((index * grain_len) + lower);
                    grain.set_upper_index(upper);
                    grain.set_looping(true);
                    grain.update_smoother();
                    grain.lock_playback();
                });
            }
        }
        self.grain_count = self.grains.len();
    }

    /// Get the grain as specified by the current grains `next_id` field, potentially the same grain
    pub fn read_next_grain(&mut self) -> &mut Grain {
        let grain = &mut self.grains[self.grain_index];
        self.grain_index = grain.next_id;

        grain
    }

    /// Setter for the global pitch shift in semitones, relative to the original pitch of the sample.
    pub fn set_global_pitch(&mut self, pitch: i8) {
        self.global_pitch = pitch;
        for grain in self.grains.iter_mut() {
            let grain_pitch = grain.resampler.get_pitch_freq();
            grain.set_pitch_freq(grain_pitch as f32 * semitone_to_hz_ratio(self.global_pitch))
        }
    }

    /// Setter for managers makeup gain
    pub fn set_makeup_gain(&mut self, gain: f32) {
        self.makeup_gain = gain;
    }

    /// Get the next sample from the current grain or grains.
    /// In sequence mode, returns the next sample from the current grain.
    ///
    /// In cascade mode, returns the next sample from all grains at once, mixed by average
    ///
    /// In cloud mode, returns the next sample from all grains at once, mixed by average
    ///
    /// All samples are multiplied by makeup gain.
    pub fn get_next_sample(&mut self) -> i16 {
        let value = match self.mode {
            GrainMode::Sequence => {
                if self.sample_index < (self.grains[self.grain_index].len() - 1) {
                    let value = self.grains[self.grain_index].get_next_sample(true);
                    self.sample_index += 1;
                    value
                } else {
                    self.grains[self.grain_index].set_sample_index(0);
                    self.read_next_grain();
                    self.sample_index = 0;

                    let value = self.grains[self.grain_index].get_next_sample(true);
                    self.sample_index += 1;
                    (value as f32 * self.makeup_gain) as i16
                }
            }
            GrainMode::Cloud(_, _, _) => {
                let mut output: i16 = 0;
                for grain in self.grains.iter_mut() {
                    output += grain.get_next_sample(true) / self.grain_count as i16;
                }
                (output as f32 * self.makeup_gain) as i16
            }
            GrainMode::Cascade(_, _) => {
                let mut output: i16 = 0;
                for grain in self.grains.iter_mut() {
                    output += grain.get_next_sample(true) / self.grain_count as i16;
                }
                (output as f32 * self.makeup_gain) as i16
            }
        };
        (value as f32 * self.env.get_next_sample()) as i16
    }

    /// Triggers the gate of the instances envelope with an on off boolean
    pub fn gate_trigger(&mut self, on_off: bool) {
        self.env.trigger_gate(on_off);
    }

    /// Wrapping setter for the instances envelope method of the same name
    pub fn set_attack(&mut self, attack_time: f32) {
        self.env.set_attack(attack_time)
    }

    /// Wrapping setter for the instances envelope method of the same name
    pub fn set_decay(&mut self, decay_time: f32) {
        self.env.set_decay(decay_time)
    }

    /// Wrapping setter for the instances envelope method of the same name
    pub fn set_sustain(&mut self, sustain_level: f32) {
        self.env.set_sustain(sustain_level)
    }

    /// Wrapping setter for the instances envelope method of the same name
    pub fn set_release(&mut self, release_time: f32) {
        self.env.set_release(release_time)
    }
}

#[cfg(test)]
mod tests {
    use crate::delay_line::StereoDelay;
    use crate::grain::{Grain, GrainManager, GrainMode};
    use crate::lfo::{LFOMode, MMLFO};
    use crate::multi_channel::MultiDelayLine;
    use crate::samples::{IntSamples, PhonicMode, Samples};
    use crate::smoothers::NoSmoother;
    use crate::{distribute_exponential, load_wav, write_wav};
    use ndarray::arr1;
    use once_cell::sync::Lazy;

    #[test]
    fn test_init() {
        static AUDIO_BUFFER: Lazy<Vec<i16>> = Lazy::new(|| load_wav("tests/amen_br.wav").unwrap());
        let _ = Grain::new(&AUDIO_BUFFER, 0, 1, 0);
    }

    #[test]
    fn test_get() {
        static AUDIO_BUFFER: Lazy<Vec<i16>> = Lazy::new(|| load_wav("tests/amen_br.wav").unwrap());

        let mut grain = Grain::new(&AUDIO_BUFFER, 0, 1, 0);
        for _ in 0..(44100 * 5) {
            grain.get_next_sample(false);
        }
    }

    #[test]
    fn test_set() {
        static AUDIO_BUFFER: Lazy<Vec<i16>> = Lazy::new(|| load_wav("tests/amen_br.wav").unwrap());

        let mut grain = Grain::new(&AUDIO_BUFFER, 0, 1, 0);

        grain.set_reverse(true);

        grain.set_smoothing(NoSmoother::new())
    }

    fn get_left(v: Vec<i16>) -> Vec<i16> {
        (0..(v.len() / 2)).map(|index| v[index * 2]).collect()
    }

    fn get_right(v: Vec<i16>) -> Vec<i16> {
        (0..(v.len() / 2)).map(|index| v[(index * 2) + 1]).collect()
    }

    #[test]
    #[ignore]
    fn generate_grain_with_manager() {
        static LEFT_AUDIO_BUFFER: Lazy<Vec<i16>> =
            Lazy::new(|| get_left(load_wav("tests/handpan.wav").unwrap()));
        static RIGHT_AUDIO_BUFFER: Lazy<Vec<i16>> =
            Lazy::new(|| get_right(load_wav("tests/handpan.wav").unwrap()));

        let grain_len: usize = LEFT_AUDIO_BUFFER.len() / 2056;
        let initial_grain_pos = 1;

        let mut manager_left = GrainManager::new(GrainMode::Cloud(0, 0.0, 0));
        manager_left.populate_grains(
            6,
            &LEFT_AUDIO_BUFFER,
            GrainMode::Cloud(grain_len * 16, 64.0, initial_grain_pos),
        );

        let mut manager_right = GrainManager::new(GrainMode::Cloud(0, 0.0, 0));
        manager_right.populate_grains(
            6,
            &RIGHT_AUDIO_BUFFER,
            GrainMode::Cloud(
                grain_len * 8,
                32.0,
                (1.5 * initial_grain_pos as f32) as usize,
            ),
        );

        let mut out_left: Vec<i16> = Vec::new();
        let mut out_right: Vec<i16> = Vec::new();

        let mut lfo = MMLFO::new(false, LFOMode::Sine);
        let mut lfo_2 = MMLFO::new(false, LFOMode::Triangle);
        lfo_2.set_frequency_hz(0.6125);
        lfo.set_frequency_hz(0.125);

        for _ in 0..(44100 * 15) {
            let mod_value_left = ((lfo.get_next_value() - 0.5) * grain_len as f32 * 256.0) as usize;
            let mod_value_right =
                ((0.5 - lfo_2.get_next_value()) * grain_len as f32 * 256.0) as usize;

            for grain in manager_left.grains.iter_mut() {
                grain.set_pos(initial_grain_pos + mod_value_left);
            }

            for grain in manager_right.grains.iter_mut() {
                grain.set_pos(initial_grain_pos + mod_value_right);
            }

            out_left.push(manager_left.get_next_sample());
            out_right.push(manager_right.get_next_sample());
        }

        let mut manager_left = GrainManager::new(GrainMode::Cloud(0, 0.0, 0));
        manager_left.populate_grains(
            64,
            &LEFT_AUDIO_BUFFER,
            GrainMode::Cloud(
                grain_len * 32,
                2.0,
                (0.5 * initial_grain_pos as f32) as usize,
            ),
        );

        let mut manager_right = GrainManager::new(GrainMode::Cloud(0, 0.0, 0));
        manager_right.populate_grains(
            64,
            &RIGHT_AUDIO_BUFFER,
            GrainMode::Cloud(grain_len * 6, 40.0, 0),
        );

        for _ in 0..(44100 * 2) {
            out_left.push(0);
            out_right.push(0);
        }

        for _ in 0..(44100 * 15) {
            let mod_value_left =
                ((lfo_2.get_next_value() - 0.5) * grain_len as f32 * 256.0) as usize;
            let mod_value_right =
                ((0.5 - lfo.get_next_value()) * grain_len as f32 * 256.0) as usize;

            for grain in manager_left.grains.iter_mut() {
                grain.set_pos(initial_grain_pos + mod_value_left);
            }

            for grain in manager_right.grains.iter_mut() {
                grain.set_pos(initial_grain_pos + mod_value_right);
            }

            out_left.push(manager_left.get_next_sample());
            out_right.push(manager_right.get_next_sample());
            manager_left.set_makeup_gain(2.0 * (lfo_2.get_next_value()));
            manager_right.set_makeup_gain(2.0 * (1.0 - lfo_2.get_next_value()))
        }

        let combined_samples = IntSamples::from_stereo(&out_left, &out_right);

        let mut delay = StereoDelay::new(44100.0, 0.249, 0.253, 0.25, 0.25);

        let mut out_stereo: Vec<i16> = Vec::new();

        for (left, right) in combined_samples.get_frames() {
            let (l, r) = delay.process(left as f32, right as f32, true, false);
            out_stereo.push(l as i16);
            out_stereo.push(r as i16);
        }

        let mut out_final: Vec<i16> = Vec::new();

        let mut multi = MultiDelayLine::new(distribute_exponential(8, 0.15), 0.8, 0.5, 8, 44100);

        for sample in out_stereo {
            out_final.push(
                multi
                    .process_with_feedback(arr1(&[sample as f32 / 4.0; 8]), true)
                    .sum() as i16,
            );
        }

        write_wav(
            "tests/debug/granular_cloud_octaves_second_test.wav",
            out_final,
            PhonicMode::Stereo,
        );
    }

    #[test]
    fn test_octaves() {
        static LEFT_AUDIO_BUFFER: Lazy<Vec<i16>> =
            Lazy::new(|| get_left(load_wav("tests/handpan.wav").unwrap()));
        static RIGHT_AUDIO_BUFFER: Lazy<Vec<i16>> =
            Lazy::new(|| get_right(load_wav("tests/handpan.wav").unwrap()));

        let mut manager_left = GrainManager::new(GrainMode::Cloud(0, 0.0, 0));
        manager_left.populate_grains(
            6,
            &LEFT_AUDIO_BUFFER,
            GrainMode::Cloud(LEFT_AUDIO_BUFFER.len() / 256, 4.0, 44100),
        );

        let mut manager_right = GrainManager::new(GrainMode::Cloud(0, 0.0, 0));
        manager_right.populate_grains(
            6,
            &RIGHT_AUDIO_BUFFER,
            GrainMode::Cloud(LEFT_AUDIO_BUFFER.len() / 256, 4.0, 44100),
        );

        let mut out_left: Vec<i16> = Vec::new();
        let mut out_right: Vec<i16> = Vec::new();

        for _ in 0..(44100 * 2) {
            out_left.push(manager_left.get_next_sample());
            out_right.push(manager_right.get_next_sample());
        }
        manager_left.set_global_pitch(4);
        manager_right.set_global_pitch(4);
        for _ in 0..(44100 * 2) {
            out_left.push(manager_left.get_next_sample());
            out_right.push(manager_right.get_next_sample());
        }
        manager_left.set_global_pitch(7);
        manager_right.set_global_pitch(7);
        for _ in 0..(44100 * 2) {
            out_left.push(manager_left.get_next_sample());
            out_right.push(manager_right.get_next_sample());
        }
        manager_left.set_global_pitch(11);
        manager_right.set_global_pitch(11);
        for _ in 0..(44100 * 2) {
            out_left.push(manager_left.get_next_sample());
            out_right.push(manager_right.get_next_sample());
        }
        manager_left.set_global_pitch(12);
        manager_right.set_global_pitch(12);
        for _ in 0..(44100 * 2) {
            out_left.push(manager_left.get_next_sample());
            out_right.push(manager_right.get_next_sample());
        }

        let combined = IntSamples::from_stereo(&out_left, &out_right);

        let mut out: Vec<i16> = Vec::new();

        for (l, r) in combined.get_frames() {
            out.push(l);
            out.push(r);
        }

        write_wav(
            "tests/debug/granular_pitch_test_3.wav",
            out,
            PhonicMode::Stereo,
        )
    }

    #[test]
    fn test_adsr() {
        static LEFT_AUDIO_BUFFER: Lazy<Vec<i16>> =
            Lazy::new(|| get_left(load_wav("tests/handpan.wav").unwrap()));
        static RIGHT_AUDIO_BUFFER: Lazy<Vec<i16>> =
            Lazy::new(|| get_right(load_wav("tests/handpan.wav").unwrap()));

        let mut manager_left = GrainManager::new(GrainMode::Cloud(0, 0.0, 0));
        manager_left.populate_grains(
            32,
            &LEFT_AUDIO_BUFFER,
            GrainMode::Cloud(LEFT_AUDIO_BUFFER.len() / 256, 4.0, 44100),
        );
        manager_left.set_makeup_gain(6.0);

        let mut manager_right = GrainManager::new(GrainMode::Cloud(0, 0.0, 0));
        manager_right.populate_grains(
            32,
            &RIGHT_AUDIO_BUFFER,
            GrainMode::Cloud(LEFT_AUDIO_BUFFER.len() / 256, 4.0, 44100),
        );
        manager_left.set_makeup_gain(6.0);

        let mut out_left: Vec<i16> = Vec::new();
        let mut out_right: Vec<i16> = Vec::new();

        for _ in 0..4 {
            manager_left.gate_trigger(true);
            manager_right.gate_trigger(true);
            for _ in 0..(44100 * 3) {
                out_left.push(manager_left.get_next_sample());
                out_right.push(manager_right.get_next_sample());
            }
            manager_left.gate_trigger(false);
            manager_right.gate_trigger(false);
            for _ in 0..(44100 * 3) {
                out_left.push(manager_left.get_next_sample());
                out_right.push(manager_right.get_next_sample());
            }
        }

        let combined = IntSamples::from_stereo(&out_left, &out_right);
        let mut out: Vec<i16> = Vec::new();

        for (l, r) in combined.get_frames() {
            out.push(l);
            out.push(r);
        }

        write_wav("tests/debug/grain_adsr_demo.wav", out, PhonicMode::Stereo);
    }

    #[test]
    fn test_chord() {
        static LEFT_AUDIO_BUFFER: Lazy<Vec<i16>> =
            Lazy::new(|| get_left(load_wav("tests/kalimba.wav").unwrap()));
        static RIGHT_AUDIO_BUFFER: Lazy<Vec<i16>> =
            Lazy::new(|| get_right(load_wav("tests/kalimba.wav").unwrap()));

        let mut root = GrainManager::new(GrainMode::Cloud(0, 0.0, 0));
        root.populate_grains(
            32,
            &LEFT_AUDIO_BUFFER,
            GrainMode::Cloud(LEFT_AUDIO_BUFFER.len() / 16, 16.0, 0),
        );
        let mut third = GrainManager::new(GrainMode::Cloud(0, 0.0, 0));
        third.populate_grains(
            32,
            &RIGHT_AUDIO_BUFFER,
            GrainMode::Cloud(LEFT_AUDIO_BUFFER.len() / 16, 16.0, 44100),
        );
        third.set_global_pitch(4);
        let mut fifth = GrainManager::new(GrainMode::Cloud(0, 0.0, 0));
        fifth.populate_grains(
            32,
            &LEFT_AUDIO_BUFFER,
            GrainMode::Cloud(LEFT_AUDIO_BUFFER.len() / 16, 16.0, 0),
        );
        fifth.set_global_pitch(7);
        let mut seventh = GrainManager::new(GrainMode::Cloud(0, 0.0, 0));
        seventh.populate_grains(
            32,
            &RIGHT_AUDIO_BUFFER,
            GrainMode::Cloud(LEFT_AUDIO_BUFFER.len() / 16, 16.0, 44100),
        );
        seventh.set_global_pitch(11);

        root.gate_trigger(true);
        let mut out: Vec<i16> = Vec::new();
        for i in 0..(44100 * 15) {
            if i == 44100 {
                third.gate_trigger(true);
            }
            if i == 88200 {
                fifth.gate_trigger(true);
            }
            if i == 176400 {
                seventh.gate_trigger(true);
            }
            if i == (44100 * 12) {
                root.gate_trigger(false);
                third.gate_trigger(false);
                fifth.gate_trigger(false);
                seventh.gate_trigger(false);
            }
            out.push(
                (root.get_next_sample() / 4
                    + third.get_next_sample() / 4
                    + fifth.get_next_sample() / 4
                    + seventh.get_next_sample() / 4)
                    * 3,
            )
        }
        write_wav(
            "tests/debug/grain_chord_maj7_2.wav",
            out,
            PhonicMode::Stereo,
        );
    }
}
