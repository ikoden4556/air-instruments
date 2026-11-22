//! The synth: Karplus-Strong Voices with the v1 Instrument, a warm steel-string guitar.

/// Every Voice, made up front so that Plucking and rendering never allocate.
pub struct Synth {
    sample_rate: f32,
    voices: Vec<Voice>,
    /// The Voice the next Pluck takes: always the oldest.
    next: usize,
    noise: u32,
}

impl Synth {
    pub fn new(sample_rate: f32) -> Self {
        let capacity = (sample_rate / LOWEST_FREQUENCY) as usize + 2;
        Self {
            sample_rate,
            voices: (0..VOICES).map(|_| Voice::new(capacity)).collect(),
            next: 0,
            noise: 0x9E37_79B9,
        }
    }

    /// Starts a Voice at `frequency` Hz; `velocity` in 0..1 sets loudness and brightness.
    pub fn pluck(&mut self, frequency: f32, velocity: f32) {
        let voice = &mut self.voices[self.next];
        self.next = (self.next + 1) % VOICES;
        let frequency = frequency.max(LOWEST_FREQUENCY);
        // The loop's averaging filter adds half a sample of delay.
        voice.delay = self.sample_rate / frequency - 0.5;
        voice.decay = 10f32.powf(-3.0 / (RING_SECONDS * frequency));
        voice.write = 0;
        voice.previous = 0.0;
        voice.active = true;
        voice.buffer.fill(0.0);
        // A softer Pluck excites the loop with duller noise.
        let brightness = 0.15 + 0.7 * velocity.clamp(0.0, 1.0);
        let mut smoothed = 0.0;
        for sample in &mut voice.buffer[..voice.delay as usize + 2] {
            self.noise ^= self.noise << 13;
            self.noise ^= self.noise >> 17;
            self.noise ^= self.noise << 5;
            let white = self.noise as f32 / u32::MAX as f32 * 2.0 - 1.0;
            smoothed += (white - smoothed) * brightness;
            *sample = smoothed * velocity;
        }
        voice.write = voice.delay as usize + 2;
    }

    /// Fills `out` with the next mono samples of every Voice, through the soft limiter.
    pub fn render(&mut self, out: &mut [f32]) {
        for sample in out.iter_mut() {
            let mix: f32 = self.voices.iter_mut().filter(|v| v.active).map(Voice::step).sum();
            *sample = (mix * MIX_GAIN).tanh();
        }
    }
}

/// One sounding, fading instance of a Pluck: a Karplus-Strong delay loop.
struct Voice {
    buffer: Vec<f32>,
    write: usize,
    /// Delay of the loop's read tap, in samples (fractional).
    delay: f32,
    /// Loop gain per pass, so the Voice fades over `RING_SECONDS`.
    decay: f32,
    previous: f32,
    active: bool,
}

impl Voice {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0; capacity],
            write: 0,
            delay: 1.0,
            decay: 0.0,
            previous: 0.0,
            active: false,
        }
    }

    /// Advances one sample and returns it.
    fn step(&mut self) -> f32 {
        let len = self.buffer.len();
        let back = self.delay.floor();
        let frac = self.delay - back;
        let i = (self.write + len - back as usize) % len;
        let j = (i + len - 1) % len;
        let out = self.buffer[i] * (1.0 - frac) + self.buffer[j] * frac;
        self.buffer[self.write] = 0.5 * (out + self.previous) * self.decay;
        self.previous = out;
        self.write = (self.write + 1) % len;
        out
    }
}

/// Voices in the pool: a Strum across all 12 Strings, with some ringing over.
const VOICES: usize = 16;

/// The lowest frequency a Voice can play; sets each Voice's buffer size.
const LOWEST_FREQUENCY: f32 = 40.0;

/// Level of the mix going into the soft limiter (tanh), which keeps output inside ±1.0.
const MIX_GAIN: f32 = 0.5;

/// Time for a Voice to fade by 60 dB, before the loop filter's extra damping.
const RING_SECONDS: f32 = 3.0;

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    /// `seconds` of output after one Pluck.
    fn pluck_once(frequency: f32, velocity: f32, seconds: f32) -> Vec<f32> {
        let mut synth = Synth::new(RATE);
        synth.pluck(frequency, velocity);
        let mut out = vec![0.0; (seconds * RATE) as usize];
        synth.render(&mut out);
        out
    }

    /// The repeating frequency of `signal`, from its autocorrelation peak near `guess` Hz.
    fn measured_frequency(signal: &[f32], guess: f32) -> f32 {
        let period = RATE / guess;
        let corr = |lag: usize| -> f32 {
            signal.iter().zip(&signal[lag..]).map(|(a, b)| a * b).sum()
        };
        let lo = (period * 0.8) as usize;
        let hi = (period * 1.2) as usize;
        let best = (lo..=hi).max_by(|&a, &b| corr(a).total_cmp(&corr(b))).unwrap();
        // Parabolic fit through the peak for a sub-sample period.
        let (l, c, r) = (corr(best - 1), corr(best), corr(best + 1));
        let offset = 0.5 * (l - r) / (l - 2.0 * c + r);
        RATE / (best as f32 + offset)
    }

    #[test]
    fn a_voice_sounds_at_its_frequency() {
        for hz in [110.0, 220.0, 440.0, 1318.5] {
            let out = pluck_once(hz, 1.0, 0.3);
            let got = measured_frequency(&out[2_000..12_000], hz);
            assert!((got / hz - 1.0).abs() < 0.01, "asked {hz} Hz, got {got} Hz");
        }
    }

    fn peak(signal: &[f32]) -> f32 {
        signal.iter().fold(0.0, |m, s| m.max(s.abs()))
    }

    #[test]
    fn a_voice_decays_to_silence() {
        for hz in [110.0, 1318.5] {
            let out = pluck_once(hz, 1.0, 8.0);
            let start = peak(&out[..4_800]);
            let end = peak(&out[out.len() - 4_800..]);
            assert!(start > 0.1, "{hz} Hz starts at {start}");
            assert!(end < 0.001 * start, "{hz} Hz still at {end} after 8 s");
        }
    }

    fn rms(signal: &[f32]) -> f32 {
        (signal.iter().map(|s| s * s).sum::<f32>() / signal.len() as f32).sqrt()
    }

    /// How much of the signal is high frequencies: sample-to-sample change against level.
    fn brightness(signal: &[f32]) -> f32 {
        let change: Vec<f32> = signal.windows(2).map(|w| w[1] - w[0]).collect();
        rms(&change) / rms(signal)
    }

    #[test]
    fn a_harder_pluck_is_louder() {
        let hard = rms(&pluck_once(220.0, 1.0, 0.2));
        let soft = rms(&pluck_once(220.0, 0.3, 0.2));
        assert!(hard > 2.0 * soft, "hard {hard}, soft {soft}");
    }

    #[test]
    fn a_full_pool_steals_the_oldest_voice() {
        let mut synth = Synth::new(RATE);
        synth.pluck(220.0, 1.0);
        // One more Pluck than the pool holds, all silent: the loud first Voice must go.
        for _ in 0..VOICES {
            synth.pluck(330.0, 0.0);
        }
        let mut out = vec![0.0; 4_800];
        synth.render(&mut out);
        assert!(peak(&out) < 1e-6, "the oldest Voice still sounds: {}", peak(&out));
    }

    #[test]
    fn a_pool_with_room_keeps_every_voice() {
        let mut synth = Synth::new(RATE);
        synth.pluck(220.0, 1.0);
        for _ in 1..VOICES {
            synth.pluck(330.0, 0.0);
        }
        let mut out = vec![0.0; 4_800];
        synth.render(&mut out);
        assert!(peak(&out) > 0.1);
    }

    #[test]
    fn output_never_exceeds_full_scale_with_every_voice_at_full_velocity() {
        let mut synth = Synth::new(RATE);
        for _ in 0..VOICES {
            synth.pluck(110.0, 1.0);
        }
        let mut out = vec![0.0; 48_000];
        synth.render(&mut out);
        assert!(peak(&out) <= 1.0, "peak {}", peak(&out));
    }

    #[test]
    fn a_harder_pluck_is_brighter() {
        let hard = brightness(&pluck_once(220.0, 1.0, 0.05));
        let soft = brightness(&pluck_once(220.0, 0.3, 0.05));
        assert!(hard > 1.5 * soft, "hard {hard}, soft {soft}");
    }
}
