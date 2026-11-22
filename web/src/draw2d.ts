// Plain debug drawing of the render data; the WebGL2 drawing replaces it.
import type { RenderData } from './core';
import type { Point } from './frame';

export function drawDebug(ctx: CanvasRenderingContext2D, r: RenderData, map: (p: Point) => Point): void {
  const { width, height } = ctx.canvas;
  ctx.clearRect(0, 0, width, height);
  ctx.lineCap = 'round';
  const line = (a: Point, b: Point, color: string, w: number) => {
    const [p, q] = [map(a), map(b)];
    ctx.strokeStyle = color;
    ctx.lineWidth = w;
    ctx.beginPath();
    ctx.moveTo(p.x, p.y);
    ctx.lineTo(q.x, q.y);
    ctx.stroke();
  };
  const dot = (a: Point, color: string, radius: number) => {
    const p = map(a);
    ctx.fillStyle = color;
    ctx.beginPath();
    ctx.arc(p.x, p.y, radius, 0, 2 * Math.PI);
    ctx.fill();
  };
  for (const [a, b] of r.strings) line(a, b, '#8ff', 4);
  if (r.stretch) line(r.stretch[0], r.stretch[1], 'rgba(255,255,255,0.5)', 2);
  for (const a of r.anchors) if (a) dot(a, '#ff8', 10);
  for (const p of r.pluckers) if (p) dot(p, '#f6f', 8);
}
