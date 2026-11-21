# Latency spike results (PROTOTYPE branch only)

Question: how fast is camera → hand tracking → sound on each phone?

## Web spike (Chrome / Safari, MediaPipe tasks-vision 1.2.3, 640×480 front camera)

| Phone | Delegate | Camera | Frames processed | Inference avg / p95 | Frame→beep avg / p95 | Audio base + output | Est. total |
|---|---|---|---|---|---|---|---|
| Oppo Find X9 Pro (Chrome) | GPU run 1 | 60 fps | 16/s | 54 / 80 ms | 70 / 94 ms | 5 + 40 ms | 108 ms |
| Oppo Find X9 Pro (Chrome) | GPU run 2 | 60 fps | 20/s | 36 / 60 ms | 71 / 118 ms | 5 + 40 ms | 130 ms |
| Oppo Find X9 Pro (Brave) | GPU | 60 fps | 20/s | 41 / 70 ms | 73 / 93 ms | 5 + 40 ms | 129 ms |
| Oppo Find X9 Pro (Chrome) | CPU | 60 fps | 8/s | 134 / 142 ms | 180 / 240 ms | 5 + 40 ms | 228 ms |
| Redmi K50 | — | not yet measured | | | | | |
| iPhone 11 (Safari) | — | not yet measured | | | | | |

Findings:
- The GPU path is in use (3–4× faster than CPU). Chrome's WebGL path is still slow even on a flagship.
- Brave matches Chrome (both Chromium), so switching Android browsers doesn't help.
- Inference blocks the main thread, so a 60 fps camera collapses to 16–20 processed frames/s.
- The web app misses the 100 ms target on the fastest phone, so native Android is the low-latency path (ADR 0004).
- At CPU speed, the drawn fingertip visibly trails the real finger by ~230 ms.

Network note: phones could not reach the PC's dev server over Wi-Fi (Windows Firewall is off, so likely router client isolation). Android phones work over USB with `adb reverse tcp:5173 tcp:5173` and `https://localhost:5173`.
