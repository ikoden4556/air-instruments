import { describe, expect, it } from 'vitest';
import { HAND_FLOATS, LANDMARKS, toScreenHands, writeHands } from './frame';

const points = (x: number, y: number) => Array.from({ length: LANDMARKS }, () => ({ x, y }));

describe('toScreenHands', () => {
  it('mirrors x like the preview and keeps y', () => {
    const [hand] = toScreenHands([points(0.2, 0.7)], [[{ categoryName: 'Left' }]]);
    expect(hand.landmarks[8].x).toBeCloseTo(0.8);
    expect(hand.landmarks[8].y).toBeCloseTo(0.7);
  });

  it("swaps MediaPipe's labels to the Player's own hands", () => {
    const hands = toScreenHands(
      [points(0.2, 0.5), points(0.8, 0.5)],
      [[{ categoryName: 'Left' }], [{ categoryName: 'Right' }]],
    );
    expect(hands.map((h) => h.handedness)).toEqual(['Right', 'Left']);
  });
});

describe('writeHands', () => {
  it('writes handedness then x, y per landmark, at most 2 Hands', () => {
    const buf = new Float32Array(2 * HAND_FLOATS);
    const hands = [
      { handedness: 'Left' as const, landmarks: points(0.25, 0.5) },
      { handedness: 'Right' as const, landmarks: points(0.75, 0.5) },
      { handedness: 'Right' as const, landmarks: points(0.5, 0.5) },
    ];
    expect(writeHands(hands, buf)).toBe(2);
    expect([buf[0], buf[1], buf[2]]).toEqual([0, 0.25, 0.5]);
    expect([buf[HAND_FLOATS], buf[HAND_FLOATS + 1]]).toEqual([1, 0.75]);
  });
});
