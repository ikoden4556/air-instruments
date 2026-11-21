// PROTOTYPE: throwaway phase-2 latency spike. Answers: how fast is camera -> hand tracking -> sound on each phone?
import { FilesetResolver, HandLandmarker } from '@mediapipe/tasks-vision';

const video = document.querySelector('video')!;
const canvas = document.querySelector('canvas')!;
const ctx = canvas.getContext('2d')!;
const hud = document.getElementById('hud')!;
const flash = document.getElementById('flash')!;

const DELEGATE = location.search.includes('cpu') ? 'CPU' : 'GPU'; // ?cpu compares against the GPU path
const STRING_X = 0.5; // one test string: a vertical line down the middle of the (mirrored) image
let audio: AudioContext;
let landmarker: HandLandmarker;
const prevX = new Map<string, number>(); // index fingertip x per hand ("Left"/"Right")
let lastPluck = 0;
let cam = '';
let callbacks = 0;
let fps = 0;
let lastPresented = 0;
let dropped = 0;
let usesCaptureTime = false;
const infer: number[] = [];
const pipe: number[] = [];
let plucks = 0;

document.getElementById('start')!.onclick = async (e) => {
  (e.target as HTMLElement).remove();
  hud.textContent = 'Starting camera + model...';
  audio = new AudioContext({ latencyHint: 'interactive' });
  await audio.resume(); // iOS only allows audio after a tap
  const stream = await navigator.mediaDevices.getUserMedia({
    audio: false,
    video: { facingMode: 'user', width: { ideal: 640 }, height: { ideal: 480 }, frameRate: { ideal: 60 } },
  });
  video.srcObject = stream;
  await video.play();
  const s = stream.getVideoTracks()[0].getSettings();
  cam = `${s.width}x${s.height} @ ${s.frameRate ?? '?'} fps requested-60`;
  const files = await FilesetResolver.forVisionTasks('/wasm');
  landmarker = await HandLandmarker.createFromOptions(files, {
    baseOptions: { modelAssetPath: '/hand_landmarker.task', delegate: DELEGATE },
    runningMode: 'VIDEO',
    numHands: 2,
  });
  setInterval(() => { fps = callbacks * 2; callbacks = 0; render(); }, 500);
  video.requestVideoFrameCallback(onFrame);
};

function beep() {
  const t = audio.currentTime;
  const o = audio.createOscillator();
  const g = audio.createGain();
  o.frequency.value = 880;
  g.gain.setValueAtTime(0.6, t);
  g.gain.exponentialRampToValueAtTime(0.001, t + 0.12);
  o.connect(g).connect(audio.destination);
  o.start(t);
  o.stop(t + 0.13);
  // white flash so a slow-motion recording can line up the trigger moment with the sound
  flash.style.opacity = '1';
  setTimeout(() => (flash.style.opacity = '0'), 50);
}

function onFrame(now: number, meta: VideoFrameCallbackMetadata) {
  callbacks++;
  if (lastPresented) dropped += Math.max(0, meta.presentedFrames - lastPresented - 1);
  lastPresented = meta.presentedFrames;
  // captureTime exists for camera frames on Chrome; Safari lacks it, so fall back to callback time (under-reports)
  usesCaptureTime = meta.captureTime !== undefined;
  const captured = meta.captureTime ?? now;

  const t0 = performance.now();
  const res = landmarker.detectForVideo(video, t0);
  infer.push(performance.now() - t0);

  canvas.width = video.videoWidth;
  canvas.height = video.videoHeight;
  ctx.strokeStyle = '#0ff';
  ctx.lineWidth = 4;
  ctx.beginPath();
  ctx.moveTo(canvas.width * STRING_X, 0);
  ctx.lineTo(canvas.width * STRING_X, canvas.height);
  ctx.stroke();

  const seen = new Set<string>();
  res.landmarks.forEach((lm, i) => {
    const hand = res.handedness[i][0].categoryName;
    seen.add(hand);
    const x = lm[8].x; // canvas is mirrored by CSS, so raw x is fine for the crossing test
    ctx.fillStyle = '#f0f';
    ctx.beginPath();
    ctx.arc(x * canvas.width, lm[8].y * canvas.height, 10, 0, 7);
    ctx.fill();
    const p = prevX.get(hand);
    if (p !== undefined && (p - STRING_X) * (x - STRING_X) < 0 && now - lastPluck > 60) {
      lastPluck = now;
      plucks++;
      beep();
      pipe.push(performance.now() - captured);
    }
    prevX.set(hand, x);
  });
  for (const k of [...prevX.keys()]) if (!seen.has(k)) prevX.delete(k);

  video.requestVideoFrameCallback(onFrame);
}

const stat = (a: number[]) => {
  if (!a.length) return 'n/a';
  const r = a.slice(-60).sort((x, y) => x - y);
  return `avg ${(r.reduce((s, v) => s + v, 0) / r.length).toFixed(1)}  p95 ${r[Math.floor(r.length * 0.95)].toFixed(1)} ms`;
};

function render() {
  const outMs = ((audio as any).outputLatency ?? 0) * 1000;
  const baseMs = audio.baseLatency * 1000;
  const lastPipe = pipe.at(-1);
  const total = lastPipe === undefined ? 'n/a' : `${(lastPipe + baseMs + outMs).toFixed(0)} ms`;
  hud.textContent =
`PROTOTYPE latency spike  ${navigator.userAgent.match(/\(([^)]+)\)/)?.[1] ?? ''}
camera        ${cam}
frames/s      ${fps}   dropped ${dropped}
inference     ${stat(infer)}  [${DELEGATE}]
frame->beep   ${stat(pipe)}  ${usesCaptureTime ? '(from capture)' : '(from callback: UNDER-reports)'}
audio         base ${baseMs.toFixed(1)}  output ${outMs ? outMs.toFixed(1) : 'n/a'} ms
EST TOTAL     ${total}  (last pluck, excludes sensor exposure)
plucks        ${plucks}
Swipe an index fingertip across the cyan line.`;
}
