// Draws the app icon (glowing Strings on black) and writes it as PNGs to public/.
// Run once with `node scripts/make-icons.mjs`; the PNGs are committed.
import { writeFileSync } from 'node:fs';
import { deflateSync } from 'node:zlib';

const STRINGS = [
  [0.14, 0.34, 0.86, 0.28],
  [0.18, 0.52, 0.82, 0.5],
  [0.14, 0.7, 0.86, 0.74],
];

function distToSegment(px, py, [ax, ay, bx, by]) {
  const [dx, dy] = [bx - ax, by - ay];
  const t = Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)));
  return Math.hypot(px - (ax + t * dx), py - (ay + t * dy));
}

function draw(size) {
  const px = Buffer.alloc(size * (size * 4 + 1));
  for (let y = 0; y < size; y++) {
    px[y * (size * 4 + 1)] = 0; // filter: none
    for (let x = 0; x < size; x++) {
      const [u, v] = [(x + 0.5) / size, (y + 0.5) / size];
      let [r, g, b] = [0.02, 0.06, 0.08];
      for (const s of STRINGS) {
        const d = distToSegment(u, v, s) / 0.012;
        const i = Math.exp(-d * d) + 0.35 * Math.exp(-d * d * 0.08);
        [r, g, b] = [r + 0.35 * i, g + 0.95 * i, b + 1.0 * i];
      }
      const dot = Math.hypot(u - 0.56, v - 0.42) / 0.035;
      const k = Math.exp(-dot * dot);
      [r, g, b] = [r + 1.0 * k, g + 0.4 * k, b + 0.95 * k];
      const at = y * (size * 4 + 1) + 1 + x * 4;
      px[at] = Math.min(255, r * 255);
      px[at + 1] = Math.min(255, g * 255);
      px[at + 2] = Math.min(255, b * 255);
      px[at + 3] = 255;
    }
  }
  return png(size, size, px);
}

function crc32(buf) {
  let crc = 0xffffffff;
  for (const byte of buf) {
    crc ^= byte;
    for (let k = 0; k < 8; k++) crc = crc & 1 ? (crc >>> 1) ^ 0xedb88320 : crc >>> 1;
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}

function png(w, h, rows) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr.set([8, 6, 0, 0, 0], 8); // 8-bit RGBA
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(rows)),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

for (const [name, size] of [
  ['icon-192.png', 192],
  ['icon-512.png', 512],
  ['apple-touch-icon.png', 180],
]) {
  writeFileSync(`public/${name}`, draw(size));
}
