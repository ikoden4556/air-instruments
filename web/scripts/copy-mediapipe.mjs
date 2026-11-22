// Serves MediaPipe's wasm files from the app itself, so it works offline.
import { cpSync } from 'node:fs';

cpSync('node_modules/@mediapipe/tasks-vision/wasm', 'public/mediapipe', { recursive: true });
