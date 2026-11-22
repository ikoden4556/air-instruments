import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { BlockRenderer } from './block-renderer';

const module = new WebAssembly.Module(readFileSync(new URL('../public/air.wasm', import.meta.url)));
const peak = (c: Float32Array) => c.reduce((m, v) => Math.max(m, Math.abs(v)), 0);
const stereo = () => [new Float32Array(128), new Float32Array(128)];

it('renders silence until the core module arrives, and ignores Plucks before it', () => {
  const r = new BlockRenderer(48000);
  r.handle({ type: 'plucks', plucks: new Float32Array([220, 1]) });
  const out = stereo();
  out[0].fill(0.5);
  r.fill(out);
  expect(peak(out[0])).toBe(0);
  r.handle({ type: 'module', module });
  r.fill(out);
  expect(peak(out[0])).toBe(0);
});

it('a Pluck message sounds on every channel', () => {
  const r = new BlockRenderer(48000);
  r.handle({ type: 'module', module });
  r.handle({ type: 'plucks', plucks: new Float32Array([220, 1, 330, 0.5]) });
  const out = stereo();
  r.fill(out);
  expect(peak(out[0])).toBeGreaterThan(0.01);
  expect(out[1]).toEqual(out[0]);
});
