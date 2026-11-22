import type { Point } from './frame';

/**
 * Maps screen-space points (0..1 of the camera image) to canvas pixels when the
 * video fills the canvas with `object-fit: cover` (scaled up, centred, edges cropped).
 */
export function coverMapping(
  videoWidth: number,
  videoHeight: number,
  canvasWidth: number,
  canvasHeight: number,
): (p: Point) => Point {
  const scale = Math.max(canvasWidth / videoWidth, canvasHeight / videoHeight);
  const offsetX = (canvasWidth - videoWidth * scale) / 2;
  const offsetY = (canvasHeight - videoHeight * scale) / 2;
  return (p) => ({ x: offsetX + p.x * videoWidth * scale, y: offsetY + p.y * videoHeight * scale });
}
