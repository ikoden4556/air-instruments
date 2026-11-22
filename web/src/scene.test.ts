import { describe, expect, it } from 'vitest';
import {
  SPARK_LIFE,
  VIBRATION_MS,
  bowedString,
  burst,
  fretTicks,
  nearestOnSegment,
  stepSparks,
  vibration,
} from './scene';

describe('fretTicks', () => {
  it('splits a String into 5 equal Frets: 4 inner ticks', () => {
    const ticks = fretTicks({ x: 0, y: 0 }, { x: 100, y: 50 });
    expect(ticks).toEqual([
      { x: 20, y: 10 },
      { x: 40, y: 20 },
      { x: 60, y: 30 },
      { x: 80, y: 40 },
    ]);
  });
});

describe('vibration', () => {
  it('starts still, wobbles, and settles within VIBRATION_MS', () => {
    expect(vibration(0, 1, 10)).toBeCloseTo(0);
    const early = Math.max(...Array.from({ length: 50 }, (_, i) => Math.abs(vibration(i, 1, 10))));
    expect(early).toBeGreaterThan(5);
    expect(vibration(VIBRATION_MS, 1, 10)).toBe(0);
    expect(vibration(-5, 1, 10)).toBe(0);
  });

  it('a softer Pluck wobbles less and never past maxPixels', () => {
    for (let t = 0; t < VIBRATION_MS; t += 7) {
      expect(Math.abs(vibration(t, 0.3, 10))).toBeLessThanOrEqual(Math.abs(vibration(t, 1, 10)) + 1e-9);
      expect(Math.abs(vibration(t, 1, 10))).toBeLessThanOrEqual(10);
    }
  });
});

describe('bowedString', () => {
  it('keeps both ends and bows the middle sideways by the offset', () => {
    const pts = bowedString({ x: 0, y: 0 }, { x: 100, y: 0 }, 8, 4);
    expect(pts[0]).toEqual({ x: 0, y: 0 });
    expect(pts[4].x).toBeCloseTo(100);
    expect(pts[4].y).toBeCloseTo(0);
    expect(pts[2].x).toBeCloseTo(50);
    expect(Math.abs(pts[2].y)).toBeCloseTo(8);
  });
});

describe('nearestOnSegment', () => {
  it('projects onto the segment and clamps to its ends', () => {
    const [a, b] = [{ x: 0, y: 0 }, { x: 10, y: 0 }];
    expect(nearestOnSegment(a, b, { x: 4, y: 3 })).toEqual({ x: 4, y: 0 });
    expect(nearestOnSegment(a, b, { x: -5, y: 1 })).toEqual({ x: 0, y: 0 });
    expect(nearestOnSegment(a, b, { x: 50, y: 1 })).toEqual({ x: 10, y: 0 });
  });
});

describe('sparks', () => {
  it('fly out from the Pluck, slow down and burn out after SPARK_LIFE', () => {
    let sparks = burst({ x: 50, y: 50 }, 10, 100, () => 0.5);
    expect(sparks).toHaveLength(10);
    const moved = stepSparks(sparks, 0.1);
    expect(Math.hypot(moved[0].x - 50, moved[0].y - 50)).toBeGreaterThan(0);
    expect(Math.hypot(moved[0].vx, moved[0].vy)).toBeLessThan(Math.hypot(sparks[0].vx, sparks[0].vy));
    for (let t = 0; t < SPARK_LIFE + 0.05; t += 0.05) sparks = stepSparks(sparks, 0.05);
    expect(sparks).toHaveLength(0);
  });
});
