# Rust core with thin platform shells

All instrument logic (landmark filtering, Pinch detection, Stretch and Freeze, Pluck detection, pitch, and the synth) lives in one Rust crate. The web app (TypeScript) and Android app (Kotlin) only handle camera, hand tracking, audio output and drawing. Rust was chosen over C++ because it compiles cleanly to both WebAssembly and Android, and it catches memory bugs at compile time on the real-time audio thread. It was chosen over Flutter, React Native or Kotlin Multiplatform because those put a bridge or a garbage collector on the frame → Pluck → sound path.

## Considered Options

- **C++ core**: same speed, but Emscripten (C++'s WebAssembly compiler) is much rougher than Rust's wasm-bindgen, and there are no compile-time memory-safety checks.
- **Flutter / React Native**: every frame and landmark result crosses a bridge; the audio would still need native code.
- **Kotlin Multiplatform**: the audio callback would run in a garbage-collected runtime.
