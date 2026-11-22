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

## Engine (#8)
- [ ] Jitter filter settings (min cutoff 1 Hz, beta 10) feel steady but not laggy on real Hands.
- [ ] Velocity: a 5 screen widths/s swipe is full strength; soft and hard Plucks both feel reachable.
- [ ] Right after letting go of a Stretch, that Hand's index fingertip sits on the String's end: check it doesn't Pluck by accident.
- [ ] MediaPipe's left/right labels match the Player's Hands (mirrored preview).

## Web shell (#23)
- [ ] On the Oppo: `adb reverse tcp:5173 tcp:5173`, `npm run dev` in `web/`, open https://localhost:5173, tap to start; the camera and hand tracking start (http://localhost also works over adb reverse).
- [ ] Strings, Stretch, Anchors and Pluckers in the debug drawing sit on your real fingers (mirroring, cover crop).
- [ ] Frames per second: hand tracking keeps up (spike: ~16–20 processed frames/s on Chrome).
- [ ] The swapped MediaPipe labels really give your own left and right (`frame.ts`).

## Web audio (#24)
- [ ] A Pluck sounds on the Oppo (Chrome) and the iPhone (Safari) right after "Tap to start".
- [ ] Safari accepts the compiled wasm module posted to the AudioWorklet (if not: send the bytes and compile in the worklet).
- [ ] Felt latency of the postMessage path; switch to a SharedArrayBuffer ring buffer only if it's too slow (ADR 0004 web target: under 150 ms felt).

## Web drawing (#25)
- [ ] Strings glow and are easy to see over the camera image; Fret ticks are clear enough to aim at.
- [ ] A Plucked String's vibration and sparks look right and land where the finger crossed.
- [ ] The faint hand skeleton helps rather than distracts.
- [ ] Drawing doesn't slow hand tracking (frames/s with and without WebGL; the 2D fallback is used only without WebGL2).

## Web controls (#26)
- [ ] Undo, Clear and Key are easy to hit with the phone on a stand, and don't get in the way of playing (top edge).
- [ ] Changing the Key changes the notes of existing Strings straight away.

## Deploy and offline (#10)
- [ ] The public URL (https://ikoden4556.github.io/air-instruments/) plays on the Oppo and the iPhone.
- [ ] Install as an app (Chrome "Install app"; Safari Share → Add to Home Screen); the icon and fullscreen landscape look right.
- [ ] Offline: after one visit, turn on airplane mode and open it again; it starts and tracks hands.

## Android audio (#33)
- [ ] The AAudio stream opens low-latency at 48 kHz with a 1-burst buffer (readout info: shared/exclusive, burst); no crackles while playing hard Strums.
- [ ] Audio output latency is near the spike's ~29 ms.
