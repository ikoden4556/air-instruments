// The AudioWorklet that runs the Rust synth on the audio thread.
import { BlockRenderer, type SynthMessage } from './block-renderer';

// AudioWorkletGlobalScope isn't in TypeScript's DOM lib.
declare const sampleRate: number;
declare function registerProcessor(name: string, ctor: unknown): void;
declare class AudioWorkletProcessor {
  readonly port: MessagePort;
}

class SynthProcessor extends AudioWorkletProcessor {
  private readonly renderer = new BlockRenderer(sampleRate);

  constructor() {
    super();
    this.port.onmessage = (e: MessageEvent<SynthMessage>) => this.renderer.handle(e.data);
  }

  process(_inputs: Float32Array[][], outputs: Float32Array[][]): boolean {
    this.renderer.fill(outputs[0]);
    return true;
  }
}

registerProcessor('synth', SynthProcessor);
