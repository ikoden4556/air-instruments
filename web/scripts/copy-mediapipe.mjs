// Serves MediaPipe's wasm files from the app itself, so it works offline.
// Only the non-module build: FilesetResolver.forVisionTasks(base) without useModule loads these.
import { copyFileSync, mkdirSync, rmSync } from 'node:fs';

const from = 'node_modules/@mediapipe/tasks-vision/wasm';
rmSync('public/mediapipe', { recursive: true, force: true });
mkdirSync('public/mediapipe', { recursive: true });
for (const file of ['vision_wasm_internal.js', 'vision_wasm_internal.wasm']) {
  copyFileSync(`${from}/${file}`, `public/mediapipe/${file}`);
}
