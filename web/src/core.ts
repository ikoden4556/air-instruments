// Typed wrappers over the Rust core's plain wasm exports (bindings/wasm).
// The module has no imports, so it is instantiated synchronously from a compiled Module,
// once for the Engine (main thread) and once for the Synth (AudioWorklet).

import { HAND_FLOATS, type Point, type TrackedHand, writeHands } from './frame';

interface Exports {
  memory: WebAssembly.Memory;
  engine_new(aspect: number, keyRoot: number): number;
  engine_hands(e: number): number;
  engine_update(e: number, timeMs: number, handCount: number): number;
  engine_plucks(e: number): number;
  engine_render(e: number): number;
  engine_undo(e: number): void;
  engine_clear(e: number): void;
  engine_set_key(e: number, keyRoot: number): void;
  synth_new(sampleRate: number): number;
  synth_pluck(s: number, frequency: number, velocity: number): void;
  synth_render(s: number, frames: number): number;
}

const PLUCK_NUMBERS = 5;
const RENDER_HEADER = 13;

/** One Pluck, ready for the synth. */
export interface PluckEvent {
  string: number;
  frequency: number;
  /** How hard, 0..1. */
  velocity: number;
  /** When the Plucker crossed (or is predicted to), in ms on the frame clock. */
  timeMs: number;
  predicted: boolean;
}

export type Segment = [Point, Point];

export interface RenderData {
  stretch: Segment | null;
  /** [left, right] while Pinching. */
  anchors: (Point | null)[];
  /** [left, right] while open. */
  pluckers: (Point | null)[];
  strings: Segment[];
}

function instantiate(module: WebAssembly.Module): Exports {
  return new WebAssembly.Instance(module, {}).exports as unknown as Exports;
}

export class Engine {
  private readonly x: Exports;
  private readonly e: number;

  /** `aspect` is the camera image's height / width; `keyRoot` 0..11 (C = 0). */
  constructor(module: WebAssembly.Module, aspect: number, keyRoot: number) {
    this.x = instantiate(module);
    this.e = this.x.engine_new(aspect, keyRoot);
  }

  /** Feeds one camera frame; returns its Plucks in the order they happen. */
  update(hands: TrackedHand[], timeMs: number): PluckEvent[] {
    const input = new Float32Array(this.x.memory.buffer, this.x.engine_hands(this.e), 2 * HAND_FLOATS);
    const count = this.x.engine_update(this.e, timeMs, writeHands(hands, input));
    const out = new Float64Array(this.x.memory.buffer, this.x.engine_plucks(this.e), count * PLUCK_NUMBERS);
    const plucks: PluckEvent[] = [];
    for (let i = 0; i < out.length; i += PLUCK_NUMBERS) {
      plucks.push({
        string: out[i],
        frequency: out[i + 1],
        velocity: out[i + 2],
        timeMs: out[i + 3],
        predicted: out[i + 4] === 1,
      });
    }
    return plucks;
  }

  render(): RenderData {
    const ptr = this.x.engine_render(this.e);
    const head = new Float32Array(this.x.memory.buffer, ptr, RENDER_HEADER);
    const count = head[RENDER_HEADER - 1];
    const r = new Float32Array(this.x.memory.buffer, ptr, RENDER_HEADER + 4 * count);
    const point = (i: number): Point | null => (Number.isNaN(r[i]) ? null : { x: r[i], y: r[i + 1] });
    const segment = (i: number): Segment | null => {
      const [a, b] = [point(i), point(i + 2)];
      return a && b ? [a, b] : null;
    };
    const strings: Segment[] = [];
    for (let s = 0; s < count; s++) strings.push(segment(RENDER_HEADER + 4 * s)!);
    return {
      stretch: segment(0),
      anchors: [point(4), point(6)],
      pluckers: [point(8), point(10)],
      strings,
    };
  }

  undo(): void {
    this.x.engine_undo(this.e);
  }

  clear(): void {
    this.x.engine_clear(this.e);
  }

  setKey(keyRoot: number): void {
    this.x.engine_set_key(this.e, keyRoot);
  }
}

export class Synth {
  private readonly x: Exports;
  private readonly s: number;

  constructor(module: WebAssembly.Module, sampleRate: number) {
    this.x = instantiate(module);
    this.s = this.x.synth_new(sampleRate);
  }

  pluck(frequency: number, velocity: number): void {
    this.x.synth_pluck(this.s, frequency, velocity);
  }

  /** The next `frames` mono samples (a view into wasm memory: copy before the next call). */
  render(frames: number): Float32Array {
    return new Float32Array(this.x.memory.buffer, this.x.synth_render(this.s, frames), frames);
  }
}
