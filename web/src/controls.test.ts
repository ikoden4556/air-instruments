import { expect, it } from 'vitest';
import { KEY_NAMES, apply, type Controlled } from './controls';

function fakeEngine() {
  const calls: string[] = [];
  const engine: Controlled = {
    undo: () => calls.push('undo'),
    clear: () => calls.push('clear'),
    setKey: (root) => calls.push(`key ${root}`),
  };
  return { engine, calls };
}

it('Undo, Clear and Key reach the Engine', () => {
  const { engine, calls } = fakeEngine();
  apply({ type: 'undo' }, engine);
  apply({ type: 'clear' }, engine);
  apply({ type: 'key', root: 7 }, engine);
  expect(calls).toEqual(['undo', 'clear', 'key 7']);
});

it('a Key root outside 0..11 wraps round the octave', () => {
  const { engine, calls } = fakeEngine();
  apply({ type: 'key', root: 14 }, engine);
  apply({ type: 'key', root: -1 }, engine);
  expect(calls).toEqual(['key 2', 'key 11']);
});

it('names all 12 roots, starting from C', () => {
  expect(KEY_NAMES).toHaveLength(12);
  expect([KEY_NAMES[0], KEY_NAMES[2], KEY_NAMES[9]]).toEqual(['C', 'D', 'A']);
});
