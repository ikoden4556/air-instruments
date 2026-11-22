// Runs the real Rust core (public/air.wasm, built by `npm run wasm`) from TypeScript.
import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { Engine, type PluckEvent, Synth } from './core';
import { LANDMARKS, type Handedness, type TrackedHand } from './frame';

const module = new WebAssembly.Module(readFileSync(new URL('../public/air.wasm', import.meta.url)));
const FRAME_MS = 1000 / 60;

/** A Hand 0.1 wide with its index fingertip at (x, y), Pinching or open. */
function hand(handedness: Handedness, x: number, y: number, pinching: boolean): TrackedHand {
  const landmarks = Array.from({ length: LANDMARKS }, () => ({ x, y: y + 0.15 }));
  landmarks[0] = { x, y: y + 0.2 };
  landmarks[9] = { x, y: y + 0.1 };
  landmarks[8] = { x, y };
  landmarks[4] = { x: pinching ? x : x + 0.08, y };
  return { handedness, landmarks };
}

function play(engine: Engine, frames: TrackedHand[][]): PluckEvent[] {
  return frames.flatMap((hands, i) => engine.update(hands, i * FRAME_MS));
}

const times = (n: number, hands: TrackedHand[]) => Array.from({ length: n }, () => hands);

it('a String built with two Pinches and swiped through the middle Plucks once at D4', () => {
  const engine = new Engine(module, 1, 0);
  const plucks = play(engine, [
    ...times(10, [hand('Left', 0.2, 0.5, true), hand('Right', 0.8, 0.5, true)]),
    ...times(10, [hand('Left', 0.2, 0.5, false), hand('Right', 0.8, 0.5, false)]),
    ...times(5, []),
    ...Array.from({ length: 11 }, (_, i) => [hand('Right', 0.5, 0.3 + 0.04 * i, false)]),
  ]);
  expect(plucks).toHaveLength(1);
  expect(plucks[0].frequency).toBeCloseTo(293.66, 1);
  expect(plucks[0].string).toBe(0);
  const r = engine.render();
  expect(r.strings).toHaveLength(1);
  expect(r.stretch).toBeNull();
  expect(r.pluckers[1]).not.toBeNull();
  engine.clear();
  expect(engine.render().strings).toHaveLength(0);
});

it('render data shows the Stretch and Anchors while both Hands Pinch', () => {
  const engine = new Engine(module, 1, 0);
  play(engine, times(5, [hand('Left', 0.2, 0.5, true), hand('Right', 0.8, 0.5, true)]));
  const r = engine.render();
  expect(r.stretch?.[0].x).toBeCloseTo(0.2);
  expect(r.stretch?.[1].x).toBeCloseTo(0.8);
  expect(r.anchors.every((a) => a !== null)).toBe(true);
  expect(r.pluckers).toEqual([null, null]);
});

it('the Synth sounds after a Pluck', () => {
  const synth = new Synth(module, 48000);
  synth.pluck(220, 1);
  const block = synth.render(128);
  expect(block).toHaveLength(128);
  expect(Math.max(...block.map(Math.abs))).toBeGreaterThan(0.01);
});
