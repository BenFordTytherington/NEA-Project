//!A module implementing an ADSR envelope and its associated functions.
use fast_math::exp;

/// A 4-stage Attack-Decay-Sustain-Release envelope, triggered by gate
/// # Attributes
/// * `current_index`: The current index being used to access the discrete samples of either AD or R stages
///
/// * `last_value`: The last value read out from the envelope, used in the seek function
///
/// * `attack_time`: The time in seconds of the attack stage
///
/// * `attack_curve`: The curve parameter of the attack stage. approaching 0 will give a linear curve,
///     above 0 will give an exponential curve, and below will be logarithmic
/// * `decay_time`: The time in seconds of the decay stage
///
/// * `decay_curve`: The curve parameter of the decay stage, same effect as in attack stage
///
/// * `sustain_level`: The level between 0 and 1 of the sustain (when gate is held but decay has finished)
///
/// * `release_time`: The time in seconds of the release stage
///
/// * `release_curve`: The curve parameter of the release stage, same effect as in attack stage
///
/// * `ad_discrete`: The discrete buffer of attack and decay samples stored
///
/// * `r_discrete`: The discrete buffer of release samples
///
/// * `finished_ad_stage`: A boolean determining how the interrupt behaviour should work, if the attack and decay stage finished
///
/// * `gate`: A gate boolean, used for triggering and sustain
pub struct ADSREnvelope {
    current_index: usize,
    last_value: f32,
    attack_time: f32,
    attack_curve: f32,
    decay_time: f32,
    decay_curve: f32,
    sustain_level: f32,
    release_time: f32,
    release_curve: f32,
    ad_discrete: Vec<f32>,
    r_discrete: Vec<f32>,
    finished_ad_stage: bool,
    gate: bool,
}

impl ADSREnvelope {
    /// The constructor for the ADSR envelope,
    /// given an attack time, decay time, sustain level and release time,
    /// all in seconds, and the sustain level between 0 and 1.
    pub fn new(attack_time: f32, decay_time: f32, sustain_level: f32, release_time: f32) -> Self {
        Self {
            current_index: 0,
            last_value: 0.0,
            attack_time,
            attack_curve: 3.0,
            decay_time,
            decay_curve: -3.0,
            sustain_level,
            release_time,
            release_curve: -5.0,
            ad_discrete: Vec::with_capacity((attack_time + decay_time) as usize * 44100),
            r_discrete: Vec::with_capacity(release_time as usize * 44100),
            finished_ad_stage: false,
            gate: false,
        }
    }

    /// Populates the discrete function vectors based off the parameters and the equations.
    /// Fills AD and R discrete buffers.
    pub fn setup(&mut self) {
        // populate attack buffer using equation (shown in doc)
        for i in 0..((self.attack_time * 44100.0) as usize) {
            let numerator_power = (self.attack_curve * (i as f32 / 44100.0)) / self.attack_time;
            let denominator_power = self.attack_curve;
            let numerator = (exp(numerator_power)) - 1.0;
            let denominator = (exp(denominator_power)) - 1.0;

            self.ad_discrete.push(numerator / denominator)
        }

        // populate decay buffer using equation (shown in doc)
        for i in 0..((self.decay_time * 44100.0) as usize) {
            let numerator_power =
                -1.0 * (self.decay_curve * (i as f32 / 44100.0)) / (self.decay_time);
            let denominator_power = -1.0 * self.decay_curve;
            let numerator = (exp(numerator_power)) - 1.0;
            let denominator = (exp(denominator_power)) - 1.0;
            let scalar = self.sustain_level - 1.0;
            self.ad_discrete
                .push(((numerator / denominator) * (scalar)) + 1.0)
        }

        // populate release buffer using equation (shown in doc)
        for i in 0..((self.release_time * 44100.0) as usize) {
            let numerator_power =
                -1.0 * (self.release_curve * (i as f32 / 44100.0)) / (self.release_time);
            let denominator_power = -1.0 * self.release_curve;
            let numerator = (exp(numerator_power)) - 1.0;
            let denominator = (exp(denominator_power)) - 1.0;
            self.r_discrete.push(
                ((numerator / denominator) * -1.0 * (self.sustain_level)) + self.sustain_level,
            )
        }
    }

    /// Returns the next sample from the ADSR envelope.
    /// If in AD stage, will return from ad_discrete.
    /// If not and gate is held, the sustain level is returned.
    /// Once released, the Release buffer is iterated.
    pub fn get_next_sample(&mut self) -> f32 {
        let mut value: f32;

        if self.gate {
            // attack - decay finished, give sustain sample
            if self.ad_discrete.len() == 0 || (self.current_index >= self.ad_discrete.len() - 1) {
                self.finished_ad_stage = true;
                value = self.sustain_level;
            }
            // attack - decay not finished, give attack / decay sample
            else {
                value = self.ad_discrete[self.current_index];
            }
        } else {
            if !self.finished_ad_stage {
                self.current_index = self.find_same_amp_release(self.last_value);
                self.finished_ad_stage = true;
            }

            // if release not finished, release sample
            if self.ad_discrete.len() == 0 || (self.current_index >= self.r_discrete.len() - 1) {
                value = 0.0;
            }
            // if release finished, 0.0
            else {
                value = self.r_discrete[self.current_index];
            }
        }

        // increment index (will be reset by sustain setter)
        self.current_index += 1;
        self.last_value = value;
        value
    }

    /// Trigger on or off the gate of the envelope, also resets the index.
    pub fn trigger_gate(&mut self, on_off: bool) {
        self.current_index = 0;
        if on_off {
            self.finished_ad_stage = false;
        }
        self.gate = on_off;
    }

    /// A binary search algorithm that finds a value of similar amplitude (accuracy of MAX_DELTA) in the release buffer
    /// Used to allow smooth interruption of Attack stage transition into Release if gate is released.
    fn find_same_amp_release(&self, amplitude: f32) -> usize {
        if amplitude > self.sustain_level {
            return 0;
        }

        let mut lb: usize = 0;
        let mut ub: usize = self.r_discrete.len();
        const MAX_DELTA: f32 = 0.01;
        let mut found_index = false;
        let mut mid = (ub + lb) / 2;

        while !found_index {
            mid = (ub + lb) / 2;
            if (self.r_discrete[mid] - amplitude).abs() <= MAX_DELTA {
                found_index = true;
            } else if self.r_discrete[mid] > amplitude {
                lb = mid;
            } else {
                ub = mid
            }
        }
        mid
    }
}

#[cfg(test)]
mod tests {
    use crate::envelope::ADSREnvelope;
    use crate::samples::PhonicMode;
    use crate::write_wav;
    use fast_math::exp;

    #[test]
    #[ignore]
    fn gen_env_example() {
        let mut env = ADSREnvelope::new(1.0, 1.0, 0.5, 1.0);
        env.setup();

        let mut out: Vec<i16> = Vec::new();

        // repeat 4 times
        for _ in 0..4 {
            // trigger gate
            env.trigger_gate(true);

            for _ in 0..((44100.0 * 1.75) as usize) {
                out.push((env.get_next_sample() * 2000.0) as i16);
            }

            env.trigger_gate(false);

            for _ in 0..((44100.0 * 1.0) as usize) {
                out.push((env.get_next_sample() * 2000.0) as i16);
            }
        }

        write_wav("tests/debug/env_adsr_2.wav", out, PhonicMode::Mono)
    }
}
