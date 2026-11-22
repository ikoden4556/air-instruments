// WebGL2 drawing: every element is a glowing capsule (a segment with round ends; a point
// is a zero-length one), drawn in pixels with additive blending over the mirrored video.
import type { PluckEvent, RenderData, Segment } from './core';
import type { Point, TrackedHand } from './frame';
import { VIBRATION_MS, bowedString, burst, fretTicks, nearestOnSegment, type Spark, stepSparks, vibration } from './scene';

type Color = [number, number, number, number];

const STRING: Color = [0.35, 0.95, 1.0, 1.0];
const STRETCH: Color = [1.0, 1.0, 1.0, 0.45];
const TICK: Color = [0.85, 1.0, 1.0, 1.0];
const ANCHOR: Color = [1.0, 0.85, 0.35, 1.0];
const PLUCKER: Color = [1.0, 0.4, 0.95, 1.0];
const SKELETON: Color = [1.0, 1.0, 1.0, 0.12];
const SPARK: Color = [1.0, 0.9, 0.6, 1.0];

const VERTEX = `#version 300 es
in vec2 pos;
in vec4 seg;
in float radius;
in vec4 color;
uniform vec2 size;
out vec2 vPos;
flat out vec4 vSeg;
flat out float vRadius;
flat out vec4 vColor;
void main() {
  vPos = pos; vSeg = seg; vRadius = radius; vColor = color;
  vec2 clip = pos / size * 2.0 - 1.0;
  gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
}`;

const FRAGMENT = `#version 300 es
precision highp float;
in vec2 vPos;
flat in vec4 vSeg;
flat in float vRadius;
flat in vec4 vColor;
out vec4 outColor;
void main() {
  vec2 pa = vPos - vSeg.xy, ba = vSeg.zw - vSeg.xy;
  float h = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-6), 0.0, 1.0);
  float d = length(pa - ba * h) / vRadius;
  float core = smoothstep(0.3, 0.15, d);
  float glow = 0.55 * exp(-5.0 * d * d);
  float i = (core + glow) * smoothstep(1.0, 0.85, d) * vColor.a;
  outColor = vec4(vColor.rgb * i, i);
}`;

/** Floats per vertex: pos 2, seg 4, radius 1, color 4. */
const STRIDE = 11;

export class GlRenderer {
  private readonly gl: WebGL2RenderingContext;
  private readonly sizeUniform: WebGLUniformLocation;
  private readonly buffer: WebGLBuffer;
  private data = new Float32Array(STRIDE * 6 * 512);
  private count = 0;
  private sparks: Spark[] = [];
  /** When each String (by its ends) was last Plucked, and how hard. */
  private plucked = new Map<string, { atMs: number; velocity: number }>();
  private lastMs = 0;

  static create(canvas: HTMLCanvasElement): GlRenderer | null {
    const gl = canvas.getContext('webgl2', { premultipliedAlpha: true, antialias: false });
    return gl ? new GlRenderer(gl) : null;
  }

  private constructor(gl: WebGL2RenderingContext) {
    this.gl = gl;
    const program = gl.createProgram()!;
    for (const [type, src] of [
      [gl.VERTEX_SHADER, VERTEX],
      [gl.FRAGMENT_SHADER, FRAGMENT],
    ] as const) {
      const s = gl.createShader(type)!;
      gl.shaderSource(s, src);
      gl.compileShader(s);
      if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s) ?? 'shader');
      gl.attachShader(program, s);
    }
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program) ?? 'link');
    gl.useProgram(program);
    this.sizeUniform = gl.getUniformLocation(program, 'size')!;
    this.buffer = gl.createBuffer()!;
    gl.bindVertexArray(gl.createVertexArray());
    gl.bindBuffer(gl.ARRAY_BUFFER, this.buffer);
    let offset = 0;
    for (const [name, n] of [
      ['pos', 2],
      ['seg', 4],
      ['radius', 1],
      ['color', 4],
    ] as const) {
      const loc = gl.getAttribLocation(program, name);
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, n, gl.FLOAT, false, STRIDE * 4, offset * 4);
      offset += n;
    }
    // Light adds up over the video, but overlapping pieces of one glow don't double up:
    // MAX blending keeps the brightest, so bowed Strings don't get beads at the joints.
    gl.enable(gl.BLEND);
    gl.blendEquation(gl.MAX);
  }

  /** Starts a String vibrating and throws sparks where the nearest Plucker crossed it. */
  pluck(plucks: PluckEvent[], r: RenderData, map: (p: Point) => Point, nowMs: number): void {
    for (const p of plucks) {
      const s = r.strings[p.string];
      if (!s) continue;
      this.plucked.set(key(s), { atMs: nowMs, velocity: p.velocity });
      // The Plucker closest to the String crossed it: spark at its nearest point on the String.
      let near: Point | null = null;
      let best = Infinity;
      for (const q of r.pluckers) {
        if (!q) continue;
        const n = nearestOnSegment(s[0], s[1], q);
        const d = Math.hypot(q.x - n.x, q.y - n.y);
        if (d < best) [best, near] = [d, n];
      }
      if (near) this.sparks.push(...burst(map(near), 10, 500 * this.scale()));
    }
  }

  draw(r: RenderData, hands: TrackedHand[], connections: { start: number; end: number }[], map: (p: Point) => Point, nowMs: number): void {
    const gl = this.gl;
    const { width, height } = gl.canvas as HTMLCanvasElement;
    const u = this.scale();
    const dt = this.lastMs ? Math.min(0.1, (nowMs - this.lastMs) / 1000) : 0;
    this.lastMs = nowMs;
    this.sparks = stepSparks(this.sparks, dt);
    this.count = 0;

    for (const h of hands) {
      for (const c of connections) this.capsule(map(h.landmarks[c.start]), map(h.landmarks[c.end]), 5 * u, SKELETON);
    }
    const live = new Set<string>();
    for (const s of r.strings) {
      const k = key(s);
      live.add(k);
      const [a, b] = [map(s[0]), map(s[1])];
      const hit = this.plucked.get(k);
      const offset = hit ? vibration(nowMs - hit.atMs, hit.velocity, 14 * u) : 0;
      const pts = offset ? bowedString(a, b, offset) : [a, b];
      for (let i = 1; i < pts.length; i++) this.capsule(pts[i - 1], pts[i], 11 * u, STRING);
      for (const t of fretTicks(a, b)) this.capsule(t, t, 20 * u, TICK);
    }
    for (const [k, hit] of this.plucked) if (!live.has(k) || nowMs - hit.atMs > VIBRATION_MS) this.plucked.delete(k);
    if (r.stretch) this.capsule(map(r.stretch[0]), map(r.stretch[1]), 7 * u, STRETCH);
    for (const a of r.anchors) if (a) this.capsule(map(a), map(a), 22 * u, ANCHOR);
    for (const p of r.pluckers) if (p) this.capsule(map(p), map(p), 18 * u, PLUCKER);
    for (const s of this.sparks) {
      const c: Color = [SPARK[0], SPARK[1], SPARK[2], s.life * 2];
      this.capsule({ x: s.x, y: s.y }, { x: s.x - s.vx * 0.02, y: s.y - s.vy * 0.02 }, 6 * u, c);
    }

    gl.viewport(0, 0, width, height);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.uniform2f(this.sizeUniform, width, height);
    gl.bufferData(gl.ARRAY_BUFFER, this.data.subarray(0, this.count * STRIDE), gl.STREAM_DRAW);
    gl.drawArrays(gl.TRIANGLES, 0, this.count);
  }

  /** Pixels per "unit", so sizes look the same on any screen. */
  private scale(): number {
    const c = this.gl.canvas as HTMLCanvasElement;
    return Math.min(c.width, c.height) / 720;
  }

  /** Adds a capsule from a to b as two triangles covering it and its glow. */
  private capsule(a: Point, b: Point, radius: number, color: Color): void {
    const len = Math.hypot(b.x - a.x, b.y - a.y);
    const [dx, dy] = len > 1e-6 ? [(b.x - a.x) / len, (b.y - a.y) / len] : [1, 0];
    const [ax, ay] = [a.x - dx * radius, a.y - dy * radius];
    const [bx, by] = [b.x + dx * radius, b.y + dy * radius];
    const [nx, ny] = [-dy * radius, dx * radius];
    const corners = [
      [ax + nx, ay + ny],
      [ax - nx, ay - ny],
      [bx + nx, by + ny],
      [bx + nx, by + ny],
      [ax - nx, ay - ny],
      [bx - nx, by - ny],
    ];
    if ((this.count + 6) * STRIDE > this.data.length) {
      const bigger = new Float32Array(this.data.length * 2);
      bigger.set(this.data);
      this.data = bigger;
    }
    for (const [x, y] of corners) {
      this.data.set([x, y, a.x, a.y, b.x, b.y, radius, ...color], this.count * STRIDE);
      this.count++;
    }
  }
}

function key(s: Segment): string {
  return `${s[0].x.toFixed(4)},${s[0].y.toFixed(4)},${s[1].x.toFixed(4)},${s[1].y.toFixed(4)}`;
}
