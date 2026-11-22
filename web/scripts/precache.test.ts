import { expect, it } from 'vitest';
import { cacheVersion, precacheList, serviceWorker } from './precache.mjs';

it('pre-caches the page, the bundle and every public file, but not source maps or itself', () => {
  const files = precacheList(
    ['assets/index-abc.js', 'assets/index-abc.js.map', 'assets/synth-worklet-def.js'],
    ['air.wasm', 'hand_landmarker.task', 'mediapipe/vision_wasm_internal.wasm', 'manifest.webmanifest', 'sw.js'],
  );
  expect(files).toEqual([
    './',
    'air.wasm',
    'assets/index-abc.js',
    'assets/synth-worklet-def.js',
    'hand_landmarker.task',
    'index.html',
    'manifest.webmanifest',
    'mediapipe/vision_wasm_internal.wasm',
  ]);
});

it('the cache version changes when any content changes', () => {
  const a = cacheVersion(['one', 'two']);
  expect(cacheVersion(['one', 'two'])).toBe(a);
  expect(cacheVersion(['one', 'too'])).not.toBe(a);
});

it('the service worker lists its files and names its cache by version', () => {
  const sw = serviceWorker(['./', 'air.wasm'], 'v123');
  expect(sw).toContain(`const CACHE = 'air-v123';`);
  expect(sw).toContain('const FILES = ["./","air.wasm"];');
});
