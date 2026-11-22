# Motion-to-sound latency targets: under 90 ms felt on native, Pluck prediction required

Real phones set a floor on Motion-to-sound latency, and the phase 2 spike measured it on the Oppo Find X9 Pro (branch `spike/latency`, `spike/RESULTS.md`): the camera pipeline alone costs ~45–55 ms before the app sees a frame, GPU hand tracking ~22–31 ms, and shared-mode AAudio ~29 ms (exclusive mode is refused to third-party apps). No camera or audio setting moved the raw total below ~105 ms, so the original "under 60 ms" goal is not reachable by tuning.

Pluck prediction closes much of the gap: a Pluck fires when the filtered Plucker path will cross a String within a lookahead window, instead of after the crossing is seen. With the front camera at 60 fps and a 60 ms lookahead, 16 of 18 crossings were predicted and the felt latency averaged ~91 ms (~86 ms for predicted Plucks).

Targets, with wired audio or the phone speaker (Bluetooth adds 150 ms or more and is warned about, not supported):

- Native Android: under 110 ms raw, **under 90 ms felt**, aiming for 60–70 ms felt on fast strums once prediction is smoothed.
- Web app: provisional under 150 ms felt; set properly after phase 4 measures it end to end.

Prediction is therefore part of the core, not a later polish, and its cost (a FALSE Pluck when the Plucker stops just short of a String) is a tuned trade-off. Every change on the frame → Pluck → sound path is judged against felt latency, measured on the Oppo Find X9 Pro.
