// PWA icon generator (v0.3 plan Task 13): renders the app icon —
// a rounded square in the accent colour with a white ascending
// line-chart mark — to PNG with no dependencies (a hand-rolled
// PNG encoder over node:zlib). `npm run icons` regenerates
// app/public/icons/*.png; the outputs are committed so builds do
// not need a rasteriser.
//
//   node scripts/make-icons.mjs

import { deflateSync } from 'node:zlib';
import { mkdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const OUT = fileURLToPath(new URL('../public/icons', import.meta.url));

const ACCENT = [0x0a, 0x53, 0xa5]; // --color-accent (light)
const WHITE = [0xff, 0xff, 0xff];
const RADIUS = 0.22; // corner radius, fraction of size
const LINE = 0.05; // chart stroke width, fraction of size
// Ascending chart polyline in unit space (y up); last point dips
// like an epi curve's tail.
const CHART = [
  [0.20, 0.30],
  [0.40, 0.55],
  [0.58, 0.44],
  [0.80, 0.72],
];

const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (const b of buf) c = CRC_TABLE[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}

/** Encodes an RGBA pixel buffer (rows of 4-byte pixels) as PNG. */
function encodePng(width, height, rgba) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // colour type RGBA
  const raw = Buffer.alloc(height * (1 + width * 4));
  for (let y = 0; y < height; y++) {
    const row = y * (1 + width * 4);
    raw[row] = 0; // filter: none
    rgba.copy(raw, row + 1, y * width * 4, (y + 1) * width * 4);
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

function distToSegment(px, py, x1, y1, x2, y2) {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const lenSq = dx * dx + dy * dy;
  const t = lenSq === 0 ? 0 : Math.max(0, Math.min(1, ((px - x1) * dx + (py - y1) * dy) / lenSq));
  const cx = x1 + t * dx;
  const cy = y1 + t * dy;
  return Math.hypot(px - cx, py - cy);
}

/** Renders the icon: `maskable` fills the whole canvas and keeps the
 *  mark inside the 80% safe zone; otherwise the square is rounded. */
function render(size, maskable) {
  const px = Buffer.alloc(size * size * 4);
  const radius = maskable ? 0 : RADIUS * size;
  const inset = maskable ? 0.9 : 1.0; // safe-zone scale
  const center = (v) => 0.5 + (v - 0.5) * inset;
  const pts = CHART.map(([x, y]) => [center(x) * size, (1 - center(y)) * size]);
  const stroke = LINE * size;
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const i = (y * size + x) * 4;
      const inside =
        maskable ||
        (x >= radius && x < size - radius) ||
        (y >= radius && y < size - radius) ||
        Math.hypot(
          Math.max(0, Math.min(size - radius, Math.max(radius, x)) - x),
          Math.max(0, Math.min(size - radius, Math.max(radius, y)) - y),
        ) <= radius;
      const [r, g, b, a] = inside ? [...ACCENT, 255] : [0, 0, 0, 0];
      px[i] = r;
      px[i + 1] = g;
      px[i + 2] = b;
      px[i + 3] = a;
      if (inside) {
        for (let s = 0; s + 1 < pts.length; s++) {
          const [x1, y1] = pts[s];
          const [x2, y2] = pts[s + 1];
          if (distToSegment(x + 0.5, y + 0.5, x1, y1, x2, y2) <= stroke) {
            px[i] = WHITE[0];
            px[i + 1] = WHITE[1];
            px[i + 2] = WHITE[2];
            break;
          }
        }
      }
    }
  }
  return encodePng(size, size, px);
}

mkdirSync(OUT, { recursive: true });
for (const [name, size, maskable] of [
  ['icon-192.png', 192, false],
  ['icon-512.png', 512, false],
  ['icon-maskable-192.png', 192, true],
  ['icon-maskable-512.png', 512, true],
]) {
  writeFileSync(`${OUT}/${name}`, render(size, maskable));
  console.log(`wrote public/icons/${name} (${size}x${size}${maskable ? ', maskable' : ''})`);
}
