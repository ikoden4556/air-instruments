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
- [x] The public URL (https://ikoden4556.github.io/air-instruments/) plays on the Oppo (2026-11-22, Chrome): tracking, Strings, sparks, sound, no extra notes heard. **Clearly slower than the native app** (not measured: the web app has no latency readout yet). Page stayed portrait because the phone's auto-rotate was off; the Player says that's a phone setting, not an app problem.
- [ ] The public URL plays on the iPhone.
- [ ] Install as an app (Chrome "Install app"; Safari Share → Add to Home Screen); the icon and fullscreen landscape look right. Oppo 2026-11-22: installs, opens fullscreen landscape. **First open reported "completely black, camera did not load"; not reproduced** — on the second open (watched over Chrome remote debugging) Tap to start → camera 640x480, MediaPipe GPU running, no errors in 2 min. Open question to the Player: was "Tap to start" visible on the black screen? **Small bug: a black strip on the left edge** (front-camera cutout; needs `viewport-fit=cover`).
- [ ] Offline: open it with no internet at all and it starts and tracks hands. Not proven yet: the Oppo's airplane mode was on but Wi-Fi was connected. Retry with Wi-Fi off.

## Android audio (#33)
- [x] The AAudio stream opens low-latency, shared (exclusive refused), burst 256. (2026-11-22) Crackles under hard Strums not checked yet.
- [x] Audio output latency is near the spike's ~29 ms: 26–30 ms. (2026-11-22)

## Android shell (#34)
- [x] Install: `cd android; .\gradlew.bat installDebug` with the Oppo plugged in; it asks for the camera only and starts. (2026-11-22)
- [ ] **FAILS** — the front camera really runs at 60 fps. First 32 fps; with the AE range also set on both streams (fix on `fix-android-camera-fps`) 44 fps facing the ceiling, but **20–28 fps while playing** under one ceiling light (motion blur: long exposure). Retry in brighter light.
- [x] The drawing sits on your fingers: Anchors land on the pinching fingertips; hand skeleton follows the hands. (2026-11-22)
- [x] Pinch, Freeze and Pluck work end to end and Plucks sound. (2026-11-22) **But extra notes:** 39 FALSE Plucks in ~20 s of play with 12 Strings.

## Android drawing (#35)
- [x] The GL layer shows over the camera preview and lines up with your fingers. (2026-11-22)
- [x] Strings glow with Fret ticks and spark on Pluck. (2026-11-22) Vibration not looked at closely yet.
- [ ] Drawing doesn't lower the hand-tracking rate (GPU shared with MediaPipe; it redraws only per camera result).

## Android controls and latency (#36, and #11's Done-when)
- [x] Undo, Clear and Key (popup) work and are easy to hit on a stand. (2026-11-22: Undo removed only the newest String, Clear the rest; Key G raised the same String's note, button label updated)
- [ ] Tap "ms" for the readout; check camera ~60 fps and results/s, capture→result, audio out, audio mode.
- [x] The readout works (2026-11-22). First run, one ceiling light: camera 20–28 fps, capture→result 108–117 ms, FELT avg 133–153 ms (predicted ~110, real ~160), FALSE 39.
- [ ] **#11 Done-when: FELT under 90 ms on the Oppo** (not met yet: 133–153 ms in dim light) over ~20 fast swipes, plus a few deliberate near-misses (FALSE Plucks count). Note: predicted Plucks are measured against the *predicted* crossing time; cross-check once with a 240 fps slow-motion video (BLUEPRINT §4).
