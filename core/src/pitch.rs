//! Pitch: a String's length sets its Base note, its Frets step up the Key.

/// The major pentatonic Key every note snaps to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key {
    /// Root pitch class, 0..12 (C = 0).
    pub root: u8,
}

/// The note in Hz of `fret` on a String `length` screen widths long.
pub fn frequency(key: Key, length: f32, fret: usize) -> f32 {
    let step = base_step(length) + fret;
    let octave = (step / PENTATONIC.len()) as i32;
    let midi = LOWEST_MIDI + key.root as i32 + 12 * octave + PENTATONIC[step % PENTATONIC.len()];
    440.0 * 2f32.powf((midi - 69) as f32 / 12.0)
}

/// How many Key steps above the lowest note a String's Base note is:
/// one step per `LENGTH_PER_STEP` shorter than a full screen width.
fn base_step(length: f32) -> usize {
    ((1.0 - length.clamp(0.0, 1.0)) / LENGTH_PER_STEP).round() as usize
}

/// How much shorter, in screen widths, a String must be to raise its Base note one Key step.
const LENGTH_PER_STEP: f32 = 0.1;

/// Major pentatonic steps, in semitones above the root.
const PENTATONIC: [i32; 5] = [0, 2, 4, 7, 9];

/// MIDI number of C3: a full-width String's Base note in the Key of C.
const LOWEST_MIDI: i32 = 48;

#[cfg(test)]
mod tests {
    use super::*;

    const C: Key = Key { root: 0 };

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn a_full_width_string_in_c_plays_c3_at_its_first_fret() {
        assert!(close(frequency(C, 1.0, 0), 130.81));
    }

    #[test]
    fn frets_step_up_the_major_pentatonic_c3_d3_e3_g3_a3() {
        let hz: Vec<f32> = (0..5).map(|f| frequency(C, 1.0, f)).collect();
        for (got, want) in hz.iter().zip([130.81, 146.83, 164.81, 196.00, 220.00]) {
            assert!(close(*got, want), "{hz:?}");
        }
    }

    #[test]
    fn a_shorter_string_has_a_higher_base_note() {
        assert!(frequency(C, 0.5, 0) > frequency(C, 1.0, 0));
        assert!(frequency(C, 0.08, 0) > frequency(C, 0.5, 0));
        // Each 0.1 screen width shorter is one Key step up: 0.6 wide is 4 steps, A3.
        assert!(close(frequency(C, 0.6, 0), 220.00));
    }

    #[test]
    fn frets_follow_the_pentatonic_across_the_octave_boundary() {
        // Base note A3; the next Frets climb into octave 4.
        let hz: Vec<f32> = (0..5).map(|f| frequency(C, 0.6, f)).collect();
        for (got, want) in hz.iter().zip([220.00, 261.63, 293.66, 329.63, 392.00]) {
            assert!(close(*got, want), "{hz:?}");
        }
    }

    #[test]
    fn strings_longer_than_the_screen_width_play_the_lowest_base_note() {
        assert_eq!(frequency(C, 1.3, 0), frequency(C, 1.0, 0));
    }
}
