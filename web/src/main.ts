// The web shell: camera → MediaPipe → the Rust Engine, once per camera frame.
import { FilesetResolver, HandLandmarker } from '@mediapipe/tasks-vision';
import { createAudioContext, startSynth } from './audio';
import { Engine, type PluckEvent } from './core';
import { drawDebug } from './draw2d';
import { toScreenHands } from './frame';
import { coverMapping } from './view';

const BASE = import.meta.env.BASE_URL;
const video = document.querySelector('video')!;
const canvas = document.getElementById('scene') as HTMLCanvasElement;
const start = document.getElementById('start') as HTMLButtonElement;
const status = document.getElementById('status')!;

/** Called with each frame's Plucks; the audio and drawing code hook in here. */
export const pluckListeners: ((plucks: PluckEvent[]) => void)[] = [];

start.onclick = () => {
  start.remove();
  const audio = createAudioContext();
  if (import.meta.env.DEV) Object.assign(window, { airAudio: audio });
  run(audio).catch((e: unknown) => {
    status.textContent = `Could not start: ${e instanceof Error ? e.message : String(e)}`;
    console.error(e);
  });
};

async function run(audio: AudioContext): Promise<void> {
  status.textContent = 'Starting sound, camera and hand tracking…';
  const core = await WebAssembly.compileStreaming(fetch(`${BASE}air.wasm`));
  pluckListeners.push(await startSynth(audio, core));
  const stream = await navigator.mediaDevices.getUserMedia({
    audio: false,
    video: { facingMode: 'user', width: { ideal: 640 }, height: { ideal: 480 }, frameRate: { ideal: 60 } },
  });
  video.srcObject = stream;
  await video.play();
  const files = await FilesetResolver.forVisionTasks(`${BASE}mediapipe`);
  const landmarker = await HandLandmarker.createFromOptions(files, {
    baseOptions: { modelAssetPath: `${BASE}hand_landmarker.task`, delegate: 'GPU' },
    runningMode: 'VIDEO',
    numHands: 2,
  });
  const engine = new Engine(core, video.videoHeight / video.videoWidth, 0);
  const ctx = canvas.getContext('2d')!;
  status.textContent = '';

  const onFrame = (now: number, meta: VideoFrameCallbackMetadata) => {
    // Capture time is on the camera timeline where the browser gives it (Chrome); else callback time.
    const captured = meta.captureTime ?? now;
    const result = landmarker.detectForVideo(video, now);
    const hands = toScreenHands(result.landmarks, result.handedness);
    const plucks = engine.update(hands, captured);
    if (plucks.length) for (const listen of pluckListeners) listen(plucks);

    const dpr = window.devicePixelRatio || 1;
    const [w, h] = [Math.round(canvas.clientWidth * dpr), Math.round(canvas.clientHeight * dpr)];
    if (canvas.width !== w || canvas.height !== h) [canvas.width, canvas.height] = [w, h];
    drawDebug(ctx, engine.render(), coverMapping(video.videoWidth, video.videoHeight, w, h));
    video.requestVideoFrameCallback(onFrame);
  };
  video.requestVideoFrameCallback(onFrame);
}
