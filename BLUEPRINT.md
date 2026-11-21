# air-instruments — Blueprint

An air string instrument played with bare hands in front of a phone camera. Pinch with both hands to stretch a glowing string between them, release to freeze it in the air, swipe an index fingertip across it to pluck. Fully offline, no recording, no uploads.

Status: **design agreed, nothing built.** Building starts only after final confirmation.

---

## 1. Decisions

| # | Decision | Choice |
|---|---|---|
| Platforms | Where it runs | **Web app** (all browsers, the iPhone path) + **native Android** (the lowest-latency version). No native iOS. |
| Camera | Setup | Front camera, landscape, phone on a stand 0.5–1 m away, mirrored full-screen preview |
| Interaction | How you play | Exactly the reference video: two-hand pinch → stretch → release freezes the string → index-fingertip swipe plucks |
| Pluckers | What can pluck | Index fingertips of both hands, only while that hand is not pinching |
| Strings | Limits | Flat 2D screen space, max 12, oldest auto-removed, on-screen **Undo** and **Clear** |
| Pitch | Rule | String length sets the base note, the tick marks along the string step it up; snapped to **major pentatonic**, key selectable |
| Sound | Source | Synthesised Karplus-Strong, one voice: warm steel-string guitar. No audio files. |
| Visuals | Drawing | Glowing strings with additive blending, vibration, spark particles, faint hand skeleton |
| Haptics | — | **None** (the phone is on a stand) |
| Hints | On-screen | "No hands detected", "Too dark", "Bluetooth audio: expect lag" |
| Privacy | — | No internet permission (Android); the web app is an offline PWA after the first visit |
| Distribution | — | Personal sideload (Android) and GitHub Pages (web). No app stores. |
| Repo | — | Public GitHub repo `air-instruments`, hosted on GitHub Pages |
| Min Android | — | Android 10 (API 29) |

### Latency targets (motion → sound)

| Stage | Native Android | Web (iPhone 11 Safari) |
|---|---|---|
| Camera frame (60 / 30 fps) + readout | ~20 ms | ~35 ms |
| Hand tracking (GPU) | ~10–20 ms | ~20–35 ms |
| Crossing check + synth trigger | < 1 ms | < 2 ms |
| Audio output (wired or speaker) | ~10–20 ms | ~20–40 ms |
| **Target** | **< 60 ms** | **< 100 ms** |

Fingertip **prediction** (extrapolating the filtered fingertip path by the measured pipeline delay) hides part of the camera delay. Bluetooth audio adds 150 ms or more, so the app warns about it.

---

## 2. Tech stack

### Shared core: **Rust**
Rust and C++ run at the same speed. Rust was chosen because it compiles cleanly to WebAssembly *and* Android, catches memory bugs at compile time (which matters on the real-time audio thread), and has `cargo test` built in.

The core is pure logic with no platform code:
- `filter`: 1€ jitter filter for landmarks
- `pinch`: per-hand pinch detection with hysteresis (start below 0.22 × hand size, release above 0.32)
- `strings`: string model (stretch, freeze, max 12, undo, clear)
- `crossing`: swept segment intersection of the fingertip path against each string, plus a short per-string debounce
- `pitch`: length + position → pentatonic note → frequency
- `synth`: Karplus-Strong voices with a voice pool, velocity from fingertip speed, a soft limiter, no allocations on the audio thread
- `engine`: takes hand landmarks per frame and returns pluck events and render data

### Web shell: **TypeScript + Vite**
| Part | Library or API |
|---|---|
| Hand tracking | `@mediapipe/tasks-vision` (HandLandmarker, GPU delegate, VIDEO mode) |
| Core | Rust → WebAssembly via `wasm-pack` |
| Audio | `AudioWorklet` running the Rust synth compiled to WebAssembly (the compiled module is passed to the worklet by message) |
| Drawing | WebGL2 canvas over a `<video>` element |
| Offline | PWA: manifest + service worker, with the hand model cached |
| Audio unlock | "Tap to start" screen, because iOS only starts audio after a user gesture |

`shortcut: pluck events reach the worklet via postMessage (adds ~1 frame of jitter), upgrade to a SharedArrayBuffer ring buffer if measured latency demands it — needs cross-origin isolation, which GitHub Pages can't set without a service-worker workaround.`

### Android shell: **Kotlin**
| Part | Library or API |
|---|---|
| Camera | CameraX + Camera2Interop, 60 fps requested, falls back to 30 |
| Hand tracking | `com.google.mediapipe:tasks-vision`, GPU delegate, LIVE_STREAM mode |
| Core | Rust → `.so` via `cargo-ndk`, called over JNI |
| Audio | AAudio, called from Rust: low-latency performance mode, exclusive sharing mode |
| Drawing | OpenGL ES 3 on a transparent surface over the camera preview |

---

## 3. Project layout

```text
air-instruments/
├── GLOSSARY.md                 # Domain terms: Anchor, String, Pluck, Voice, …
├── docs/adr/                   # One short record per stack decision
├── core/                       # Rust crate: pure logic + synth, cargo test
├── bindings/
│   ├── wasm/                   # wasm-bindgen wrapper → web/
│   └── android/                # JNI wrapper + AAudio stream → android/
├── web/                        # Vite + TypeScript app (PWA)
├── android/                    # Gradle project, Kotlin shell
├── spike/                      # Throwaway latency prototype (deleted after phase 2)
└── .github/workflows/pages.yml # Build web/ + wasm, deploy to GitHub Pages
```

---

## 4. Workflow

| Phase | What happens | Done when |
|---|---|---|
| **0. Setup** | Install with winget: rustup (+ `wasm32-unknown-unknown` and Android targets, `wasm-pack`, `cargo-ndk`), Node LTS, Android Studio (you click through its SDK/NDK wizard), GitHub CLI (you run `gh auth login`). `git init`, create the public GitHub repo. **You run `codegraph init`.** | All tools report versions; the repo is on GitHub |
| **1. Domain model** (`domain-modeling`) | Write `GLOSSARY.md` and ADRs for: Rust core, web + native Android, synthesised audio, the latency targets | You've read and agreed both |
| **2. Prototype spike** (`prototype`) | Throwaway camera → hand tracking → crossing → beep, with timestamps at each stage. A web version on all 3 phones, a native Android version on the Redmi K50 and Oppo Find X9 Pro. Check the real front-camera fps per phone. | Measured latency table per phone; the architecture is confirmed or changed; `spike/` deleted |
| **3. Core** (`tdd`) | Each core module built test-first with `cargo test` on the PC | All core behaviour covered by tests |
| **4. Web app** | Wire the core into the web shell. Develop over self-signed HTTPS on your Wi-Fi (`https://<pc-ip>:5173`), so the phones reload on each save | Playable on all 3 phones from the dev server |
| **5. Deploy** | GitHub Actions builds and publishes to GitHub Pages; PWA install + offline check on the iPhone | Works from the public URL, including offline |
| **6. Native Android** | Kotlin shell + the same core via JNI + AAudio + OpenGL ES | Under 60 ms measured on both Android phones |
| **7. Tuning** | Prediction amount, pinch thresholds, guitar timbre, on-screen hints | Feels right to you |

### Measuring latency
- **In-app:** timestamps at camera frame, hand result, pluck event and audio callback.
- **End-to-end check:** film the phone with another phone's slow-motion camera (240 fps) and count frames from the finger crossing the string to the speaker sound.

---

## 5. Test phones

| Phone | Role |
|---|---|
| iPhone 11 (A13) | Web app only (Safari) |
| Redmi K50 (Dimensity 8100) | Low-end floor: web + native |
| Oppo Find X9 Pro | High end: web + native |

---

## 6. Later (not in v1)
- More voices (banjo, harp, koto): about 10 lines of synth settings each
- "Fist = clear" hand sign
- Native iOS app reusing the Rust core (needs a Mac or a cloud Mac + Apple Developer account)
- App store release
