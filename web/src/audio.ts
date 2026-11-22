// Starts the synth worklet and returns how to send it Plucks.
import type { SynthMessage } from './block-renderer';
import type { PluckEvent } from './core';
import workletUrl from './synth-worklet.ts?worker&url';

/**
 * Call `createAudioContext` inside the tap handler (iOS only starts audio from a gesture),
 * then `startSynth` with it once the core module is compiled.
 */
export function createAudioContext(): AudioContext {
  const audio = new AudioContext({ latencyHint: 'interactive' });
  void audio.resume();
  return audio;
}

export async function startSynth(
  audio: AudioContext,
  core: WebAssembly.Module,
): Promise<(plucks: PluckEvent[]) => void> {
  await audio.audioWorklet.addModule(workletUrl);
  const node = new AudioWorkletNode(audio, 'synth', { numberOfInputs: 0, outputChannelCount: [2] });
  node.connect(audio.destination);
  const post = (m: SynthMessage) => node.port.postMessage(m);
  post({ type: 'module', module: core });
  // Shortcut: postMessage adds up to a block of jitter; a SharedArrayBuffer ring buffer can
  // replace it if measured latency demands (BLUEPRINT §2, Web shell).
  return (plucks) => {
    const pairs = new Float32Array(plucks.length * 2);
    plucks.forEach((p, i) => pairs.set([p.frequency, p.velocity], 2 * i));
    post({ type: 'plucks', plucks: pairs });
  };
}
