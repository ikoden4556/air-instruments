// From MediaPipe's result to the Hands the core expects: screen space, mirrored like the preview.

export const LANDMARKS = 21;
/** Floats per Hand in the core's input buffer: handedness, then x, y per landmark. */
export const HAND_FLOATS = 1 + 2 * LANDMARKS;

export interface Point {
  x: number;
  y: number;
}

export type Handedness = 'Left' | 'Right';

/** One Hand in screen space (x, y in 0..1, mirrored like the preview the Player sees). */
export interface TrackedHand {
  handedness: Handedness;
  landmarks: Point[];
}

/**
 * MediaPipe sees the raw (unmirrored) camera image but labels hands as if it were
 * mirrored, so each label names the Player's other Hand. Mirroring x and swapping
 * the label gives the Player's own left and right.
 */
export function toScreenHands(
  landmarks: Point[][],
  handedness: { categoryName: string }[][],
): TrackedHand[] {
  return landmarks.map((points, i) => ({
    handedness: handedness[i]?.[0]?.categoryName === 'Left' ? 'Right' : 'Left',
    landmarks: points.map((p) => ({ x: 1 - p.x, y: p.y })),
  }));
}

/** Writes up to 2 Hands into the core's input buffer; returns how many it wrote. */
export function writeHands(hands: TrackedHand[], buf: Float32Array): number {
  const count = Math.min(hands.length, 2);
  for (let h = 0; h < count; h++) {
    const at = h * HAND_FLOATS;
    buf[at] = hands[h].handedness === 'Left' ? 0 : 1;
    hands[h].landmarks.forEach((p, i) => {
      buf[at + 1 + 2 * i] = p.x;
      buf[at + 2 + 2 * i] = p.y;
    });
  }
  return count;
}
