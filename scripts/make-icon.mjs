// Renders the Kestrel app icon with signed-distance-field anti-aliasing and
// writes a 1024x1024 PNG. Run `node scripts/make-icon.mjs`.
import { deflateSync } from "node:zlib";
import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const S = 1024;
const SS = 2; // supersampling grid per axis
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");

// ---------- helpers ----------
const clamp01 = (v) => (v < 0 ? 0 : v > 1 ? 1 : v);
const smooth = (edge0, edge1, x) => {
  const t = clamp01((x - edge0) / (edge1 - edge0));
  return t * t * (3 - 2 * t);
};
const mix = (a, b, t) => a + (b - a) * t;
const hex = (h) => [
  parseInt(h.slice(1, 3), 16),
  parseInt(h.slice(3, 5), 16),
  parseInt(h.slice(5, 7), 16),
];

// Superellipse (squircle) signed distance, approximated.
function squircleDist(px, py, half, n) {
  const ax = Math.abs(px) / half;
  const ay = Math.abs(py) / half;
  const v = Math.pow(Math.pow(ax, n) + Math.pow(ay, n), 1 / n);
  return (v - 1) * half;
}

function sdCircle(px, py, cx, cy, r) {
  return Math.hypot(px - cx, py - cy) - r;
}

function sdSegment(px, py, ax, ay, bx, by) {
  const vx = bx - ax;
  const vy = by - ay;
  const wx = px - ax;
  const wy = py - ay;
  const t = clamp01((wx * vx + wy * vy) / (vx * vx + vy * vy));
  return Math.hypot(wx - vx * t, wy - vy * t);
}

// ---------- design ----------
const cx = S / 2;
const cy = S / 2;
const half = S * 0.5;
const cornerN = 4.4;

const topColor = hex("#A7C2FF");
const midColor = hex("#3E63F6");
const botColor = hex("#131F8E");

// magnifier geometry
const lensCx = S * 0.442;
const lensCy = S * 0.428;
const lensR = S * 0.176;
const stroke = S * 0.070;
const handleStart = [S * 0.567, S * 0.556];
const handleEnd = [S * 0.716, S * 0.706];
const handleW = S * 0.052;

// glass interior of the lens (inset from the ring's inner edge)
const glassR = lensR - stroke / 2 - S * 0.008;
// light direction (from top-left), normalised
const Lx = -0.7071;
const Ly = -0.7071;

// 4-point sparkle = union of two thin rhombi (horizontal + vertical spikes)
const sparkX = S * 0.492;
const sparkY = S * 0.376;
const sparkR = S * 0.042; // spike half-length
const sparkW = S * 0.012; // spike half-width
function sdSparkle(px, py) {
  const x = Math.abs(px - sparkX);
  const y = Math.abs(py - sparkY);
  const rhombus = (a, b) => (x / a + y / b - 1) / Math.hypot(1 / a, 1 / b);
  return Math.min(rhombus(sparkR, sparkW), rhombus(sparkW, sparkR));
}

function background(px, py) {
  // Squircle mask
  const d = squircleDist(px - cx, py - cy, half * 0.995, cornerN);
  const mask = 1 - smooth(-0.8, 0.8, d);
  if (mask <= 0) return [0, 0, 0, 0];

  // Diagonal gradient, deeper range than before for more depth
  const t = clamp01((px / S) * 0.35 + (py / S) * 0.75);
  let c;
  if (t < 0.52) {
    const k = t / 0.52;
    c = [
      mix(topColor[0], midColor[0], k),
      mix(topColor[1], midColor[1], k),
      mix(topColor[2], midColor[2], k),
    ];
  } else {
    const k = (t - 0.52) / 0.48;
    c = [
      mix(midColor[0], botColor[0], k),
      mix(midColor[1], botColor[1], k),
      mix(midColor[2], botColor[2], k),
    ];
  }

  // Soft top-left sheen (light source)
  const sheen = 1 - smooth(0, S * 0.85, Math.hypot(px - S * 0.30, py - S * 0.16));
  const s = sheen * 0.18;
  c = [c[0] + (255 - c[0]) * s, c[1] + (255 - c[1]) * s, c[2] + (255 - c[2]) * s];

  // Radial glow behind the lens: the light the glass would catch
  const gd = Math.hypot(px - lensCx, py - lensCy);
  const glow = Math.exp(-(gd * gd) / (2 * (S * 0.30) ** 2)) * 0.10;
  c = [c[0] + (255 - c[0]) * glow, c[1] + (255 - c[1]) * glow, c[2] + (255 - c[2]) * glow];

  // Bottom vignette for depth
  const vig = smooth(S * 0.45, S * 1.05, py) * 0.20;
  c = [c[0] * (1 - vig), c[1] * (1 - vig), c[2] * (1 - vig)];

  // Inner rim light along the top edge, inner shade along the bottom:
  // a whisper of bevel that makes the tile feel glazed.
  const depth = -d; // >0 inside, grows inward
  const band = smooth(S * 0.002, S * 0.010, depth) * (1 - smooth(S * 0.012, S * 0.030, depth));
  const topness = clamp01(0.5 - ((py - cy) / S) * 2.2);
  const rimHi = band * topness * 0.14;
  c = [c[0] + (255 - c[0]) * rimHi, c[1] + (255 - c[1]) * rimHi, c[2] + (255 - c[2]) * rimHi];
  const botness = clamp01(0.5 + ((py - cy) / S) * 2.2);
  const rimLo = band * botness * 0.16;
  c = [c[0] * (1 - rimLo), c[1] * (1 - rimLo), c[2] * (1 - rimLo)];

  return [c[0], c[1], c[2], 255 * mask];
}

// Glass fill over the lens interior: translucent white, denser toward the
// light. Returns alpha 0..1 (independent of the white ring).
function glassAlpha(px, py) {
  const dg = sdCircle(px, py, lensCx, lensCy, glassR);
  const inside = 1 - smooth(-0.7, 0.7, dg);
  if (inside <= 0) return 0;
  // gradient across the glass, along the light direction
  const nx = (px - lensCx) / glassR;
  const ny = (py - lensCy) / glassR;
  const towardLight = clamp01(0.5 - 0.5 * (nx * Lx + ny * Ly)); // 1 at top-left rim
  return inside * (0.10 + 0.16 * towardLight);
}

// Crescent highlight hugging the inner top-left edge of the glass.
function glassEdgeHi(px, py) {
  const dg = sdCircle(px, py, lensCx, lensCy, glassR);
  const depth = -dg;
  if (depth <= 0) return 0;
  const band = smooth(0, S * 0.004, depth) * (1 - smooth(S * 0.006, S * 0.024, depth));
  if (band <= 0) return 0;
  const nx = (px - lensCx) / glassR;
  const ny = (py - lensCy) / glassR;
  const facing = clamp01(nx * Lx + ny * Ly); // normals pointing at the light
  return band * Math.pow(facing, 1.6) * 0.55;
}

function glyph(px, py) {
  // 1 = fully inside the white glyph, 0 = outside (with AA)
  let cov = 0;

  // lens ring
  const dRing = Math.abs(sdCircle(px, py, lensCx, lensCy, lensR)) - stroke / 2;
  cov = Math.max(cov, 1 - smooth(-0.7, 0.7, dRing));

  // handle
  const dHandle =
    sdSegment(px, py, handleStart[0], handleStart[1], handleEnd[0], handleEnd[1]) - handleW / 2;
  cov = Math.max(cov, 1 - smooth(-0.7, 0.7, dHandle));

  // sparkle inside the glass
  const dSpark = sdSparkle(px, py);
  cov = Math.max(cov, 1 - smooth(-0.7, 0.7, dSpark));

  return cov;
}

// ---------- render ----------
const buf = new Uint8Array(S * S * 4);
for (let y = 0; y < S; y++) {
  for (let x = 0; x < S; x++) {
    let r = 0,
      g = 0,
      b = 0,
      a = 0;
    for (let sy = 0; sy < SS; sy++) {
      for (let sx = 0; sx < SS; sx++) {
        const px = x + (sx + 0.5) / SS;
        const py = y + (sy + 0.5) / SS;

        const [br, bg, bb, ba] = background(px, py);
        const gv = glyph(px, py);
        const ga2 = glassAlpha(px, py);
        const gh = glassEdgeHi(px, py);

        // glyph shadow (slightly offset, low alpha)
        const gs = glyph(px - S * 0.004, py - S * 0.010);
        const shadowA = gs * 0.16;

        let cr = br;
        let cg = bg;
        let cb = bb;
        // glass fill (frosted, gradient toward the light)
        cr = cr * (1 - ga2) + (255 * 0.92 + br * 0.08) * ga2;
        cg = cg * (1 - ga2) + (255 * 0.94 + bg * 0.06) * ga2;
        cb = cb * (1 - ga2) + 255 * ga2;
        // crescent highlight on the glass rim
        cr = cr * (1 - gh) + 255 * gh;
        cg = cg * (1 - gh) + 255 * gh;
        cb = cb * (1 - gh) + 255 * gh;
        // composite shadow
        cr = cr * (1 - shadowA) + 12 * shadowA;
        cg = cg * (1 - shadowA) + 16 * shadowA;
        cb = cb * (1 - shadowA) + 60 * shadowA;
        // composite white glyph
        const ga = gv * 0.985;
        cr = cr * (1 - ga) + 255 * ga;
        cg = cg * (1 - ga) + 255 * ga;
        cb = cb * (1 - ga) + 255 * ga;

        const alpha = Math.max(ba / 255, Math.max(gv, gs, ga2));
        r += cr;
        g += cg;
        b += cb;
        a += alpha * 255;
      }
    }
    const n = SS * SS;
    const i = (y * S + x) * 4;
    buf[i] = Math.round(r / n);
    buf[i + 1] = Math.round(g / n);
    buf[i + 2] = Math.round(b / n);
    buf[i + 3] = Math.round(a / n);
  }
}

// ---------- PNG encoding ----------
function crc32(bytes) {
  let c;
  const table = crc32.table || (crc32.table = (() => {
    const t = new Int32Array(256);
    for (let n = 0; n < 256; n++) {
      c = n;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      t[n] = c;
    }
    return t;
  })());
  let crc = -1;
  for (let i = 0; i < bytes.length; i++) crc = (crc >>> 8) ^ table[(crc ^ bytes[i]) & 0xff];
  return (crc ^ -1) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const typeBuf = Buffer.from(type, "ascii");
  const body = Buffer.concat([typeBuf, data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(S, 0);
ihdr.writeUInt32BE(S, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // RGBA

// filter type 0 per scanline
const raw = Buffer.alloc(S * (S * 4 + 1));
for (let y = 0; y < S; y++) {
  raw[y * (S * 4 + 1)] = 0;
  Buffer.from(buf.buffer, y * S * 4, S * 4).copy(raw, y * (S * 4 + 1) + 1);
}

const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", deflateSync(raw, { level: 9 })),
  chunk("IEND", Buffer.alloc(0)),
]);

const out = resolve(root, "icon-1024.png");
writeFileSync(out, png);
console.log(`[kestrel] icon written: ${out} (${(png.length / 1024).toFixed(1)} KB)`);
