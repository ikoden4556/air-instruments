// What the synth AudioWorklet does each block, kept free of worklet globals so it can be tested.
import { Synth } from './core';

/** Messages the main thread posts to the synth worklet. */
export type SynthMessage =
  | { type: 'module'; module: WebAssembly.Module }
  /** Frequency and Velocity pairs, one per Pluck. */
  | { type: 'plucks'; plucks: Float32Array };

export class BlockRenderer {
  private synth: Synth | null = null;

  constructor(private readonly sampleRate: number) {}

  handle(message: SynthMessage): void {
    if (message.type === 'module') {
      this.synth = new Synth(message.module, this.sampleRate);
    } else if (this.synth) {
      const p = message.plucks;
      for (let i = 0; i + 1 < p.length; i += 2) this.synth.pluck(p[i], p[i + 1]);
    }
  }

  /** Fills every channel of the output with the next block; silence until the module arrives. */
  fill(channels: Float32Array[]): void {
    if (!channels.length) return;
    if (!this.synth) {
      for (const c of channels) c.fill(0);
      return;
    }
    const block = this.synth.render(channels[0].length);
    for (const c of channels) c.set(block);
  }
}
