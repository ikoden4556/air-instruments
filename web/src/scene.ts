// Pure geometry for the drawing: Fret ticks, String vibration, sparks. No WebGL here.
import type { Point } from './frame';

export const FRETS = 5;

/** Where the Fret boundaries sit along a String from `a` to `b` (the inner ticks). */
export function fretTicks(a: Point, b: Point, frets = FRETS): Point[] {
  return Array.from({ length: frets - 1 }, (_, i) => lerp(a, b, (i + 1) / frets));
}

/** How long a Plucked String visibly vibrates. */
export const VIBRATION_MS = 900;

/**
 * Sideways displacement (in the String's own pixels) of a Plucked String's middle,
 * `ageMs` after its Pluck: a fast wobble that fades out, 0 at the Pluck and after it settles.
 */
export function vibration(ageMs: number, velocity: number, maxPixels: number): number {
  if (ageMs < 0 || ageMs >= VIBRATION_MS) return 0;
  const fade = Math.exp((-4 * ageMs) / VIBRATION_MS) * (1 - ageMs / VIBRATION_MS);
  return maxPixels * velocity * fade * Math.sin((2 * Math.PI * ageMs) / 45);
}

/** Points along a String bowed sideways by `offset` pixels at its middle (ends stay put). */
export function bowedString(a: Point, b: Point, offset: number, pieces = 16): Point[] {
  const len = Math.hypot(b.x - a.x, b.y - a.y) || 1;
  const nx = -(b.y - a.y) / len;
  const ny = (b.x - a.x) / len;
  return Array.from({ length: pieces + 1 }, (_, i) => {
    const t = i / pieces;
    const bow = offset * Math.sin(Math.PI * t);
    const p = lerp(a, b, t);
    return { x: p.x + nx * bow, y: p.y + ny * bow };
  });
}

/** The point on segment a–b nearest to p. */
export function nearestOnSegment(a: Point, b: Point, p: Point): Point {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const lenSq = dx * dx + dy * dy;
  const t = lenSq ? Math.min(1, Math.max(0, ((p.x - a.x) * dx + (p.y - a.y) * dy) / lenSq)) : 0;
  return lerp(a, b, t);
}

export interface Spark {
  x: number;
  y: number;
  vx: number;
  vy: number;
  /** Seconds left. */
  life: number;
}

export const SPARK_LIFE = 0.5;

/** A burst of sparks at `at`, flying out in every direction at up to `speed` pixels/s. */
export function burst(at: Point, count: number, speed: number, random: () => number = Math.random): Spark[] {
  return Array.from({ length: count }, () => {
    const angle = random() * 2 * Math.PI;
    const s = speed * (0.4 + 0.6 * random());
    return { x: at.x, y: at.y, vx: Math.cos(angle) * s, vy: Math.sin(angle) * s, life: SPARK_LIFE };
  });
}

/** Moves the sparks `dt` seconds on, slowing them, and drops the ones that burned out. */
export function stepSparks(sparks: Spark[], dt: number): Spark[] {
  const drag = Math.exp(-4 * dt);
  return sparks
    .map((s) => ({ x: s.x + s.vx * dt, y: s.y + s.vy * dt, vx: s.vx * drag, vy: s.vy * drag, life: s.life - dt }))
    .filter((s) => s.life > 0);
}

function lerp(a: Point, b: Point, t: number): Point {
  return { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t };
}
