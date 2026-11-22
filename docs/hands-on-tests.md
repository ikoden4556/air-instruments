# Hands-on tests

Checks only the Player can do (playing on the phone, listening), saved for the testing pass after the initial build. Each line names the issue it came from. Tick a box once it is done; note what you changed.

Most of these need the engine (#8) and a shell wired up before they can be tried.

## Filter (#1)
- [ ] Fingertips look steady when a Hand is still, and keep up with a fast swipe (no visible lag).

## Pinch (#2)
- [ ] Pinching starts and stops when you expect, for a near Hand and a far Hand.
- [ ] A half-open Pinch doesn't flicker on and off.

## Strings (#3)
- [ ] Left and right Hands are mapped the right way round (the shell's job, untested in core).
- [ ] The 8%-of-screen-width minimum feels right: short Stretches are Discarded, deliberate ones Freeze.
- [ ] A Hand briefly leaving the camera (under 150 ms) keeps the Stretch.

## Crossing (#4)
- [ ] One swipe across a String gives exactly one Pluck; a Strum Plucks each String in order.
- [ ] A finger hovering on a String doesn't re-Pluck it (60 ms jitter guard).

## Prediction (#5)
- [ ] Plucks feel on time with the 60 ms lookahead; no Plucks for near misses.
- [ ] The 1 screen width/s speed cutoff: slow approaches still Pluck on the real crossing.
- [ ] Watch `false_plucks()` while playing; tune lookahead/cutoff if it climbs.

## Pitch (#6)
- [ ] String length → Base note feels playable: 1 Key step per 0.1 screen width, longest String = root in octave 3.
- [ ] 5 Frets per String is easy to aim at.

## Synth (#7)
- [ ] It sounds like a warm steel-string guitar.
- [ ] Ring time (3 s) and the Velocity brightness curve feel right.
- [ ] A loud Strum with many Voices doesn't sound squashed by the limiter.
