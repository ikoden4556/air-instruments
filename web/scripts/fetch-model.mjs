// Downloads MediaPipe's hand landmark model to public/ once, so the app serves it itself.
import { existsSync, writeFileSync } from 'node:fs';

const MODEL_URL =
  'https://storage.googleapis.com/mediapipe-models/hand_landmarker/hand_landmarker/float16/latest/hand_landmarker.task';
const out = 'public/hand_landmarker.task';

if (!existsSync(out)) {
  const res = await fetch(MODEL_URL);
  if (!res.ok) throw new Error(`model download failed: ${res.status}`);
  writeFileSync(out, Buffer.from(await res.arrayBuffer()));
}
