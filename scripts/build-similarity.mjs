// Builds a "Similarité" map: the same force-directed view as the "Liens" map, but the links
// are similarity links (cosine in any vector space) instead of SuttaCentral's parallels.
//   $OUT.json   nodes, positioned for the default threshold
//   $OUT.bin    every pair >= the lowest threshold, so the page can change the threshold and
//               re-run the layout live: per pair, a u16, b u16, round(t * 65535) u16 where
//               t = (sim - lo) / (hi - lo) and [lo, hi] is stored in the JSON (sim.range)
//
//   npm run build-sim    (vectors: ../pali-embedder/out/pali-lsa-kept-512.*)
//
// Without MIN_SIM / DEFAULT_SIM / MAX_SIM the thresholds come from the number of links per
// text (cosines are not on the same scale in every space): ~25, ~6 and ~0.5 per text.
import fs from 'node:fs';
import path from 'node:path';
import createGraph from 'ngraph.graph';
import createLayout from 'ngraph.forcelayout';
import { GROUPS, loadCanon, loadVectors } from './lib/canon.mjs';

const { VEC_BIN, VEC_DOCS, OUT = 'public/graph-sim' } = process.env;
const DIM = Number(process.env.DIM ?? 1024);
if (!VEC_BIN || !VEC_DOCS) throw new Error('set VEC_BIN and VEC_DOCS');

const { uids, vectors: V } = loadVectors(VEC_BIN, VEC_DOCS, DIM);
const N = V.length;
if (N > 65535) throw new Error('pair file stores node indices as u16');
const dot = (x, y) => { let s = 0; for (let k = 0; k < DIM; k++) s += x[k] * y[k]; return s; };

// ---- Thresholds: given, or from the number of links per text
const per = { min: 25, def: 6, max: 0.5 };
let lo = process.env.MIN_SIM ? Number(process.env.MIN_SIM) : undefined;
if (lo === undefined) {
  // Estimate the cosine above which ~25 links per text remain, from a random sample of pairs.
  let seed = 1; const rnd = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
  const sample = [];
  for (let k = 0; k < 400_000; k++) { const a = Math.floor(rnd() * N), b = Math.floor(rnd() * N); if (a !== b) sample.push(dot(V[a], V[b])); }
  sample.sort((x, y) => y - x);
  const share = (per.min * N / 2) / (N * (N - 1) / 2);
  lo = sample[Math.floor(share * sample.length)];
}

// ---- Every pair above the lowest threshold (N² / 2 dot products)
const t0 = Date.now();
const simPairs = [];
for (let i = 0; i < N; i++) {
  for (let j = i + 1; j < N; j++) { const s = dot(V[i], V[j]); if (s >= lo) simPairs.push([i, j, s]); }
}
simPairs.sort((x, y) => y[2] - x[2]);
const atCount = (links) => simPairs[Math.min(simPairs.length - 1, Math.round(links * N / 2))][2];
const round2 = (x) => Math.round(x * 100) / 100;
const hi = process.env.MAX_SIM ? Number(process.env.MAX_SIM) : round2(atCount(per.max));
const def = process.env.DEFAULT_SIM ? Number(process.env.DEFAULT_SIM) : round2(atCount(per.def));
lo = process.env.MIN_SIM ? lo : Math.ceil(lo * 100) / 100;
const kept = simPairs.filter(([, , s]) => s >= lo);
console.log(`${N} texts; ${kept.length} pairs >= ${lo} in ${((Date.now() - t0) / 1000).toFixed(1)}s; ` +
  `thresholds ${lo} / ${def} / ${hi} -> ${kept.length} / ${kept.filter((p) => p[2] >= def).length} / ${kept.filter((p) => p[2] >= hi).length} links`);

const buf = Buffer.alloc(kept.length * 6);
kept.forEach(([a, b, s], k) => {
  buf.writeUInt16LE(a, k * 6); buf.writeUInt16LE(b, k * 6 + 2);
  buf.writeUInt16LE(Math.round(Math.min(Math.max((s - lo) / (hi - lo), 0), 1) * 65535), k * 6 + 4);
});
fs.writeFileSync(`${OUT}.bin`, buf);

// ---- Force layout on the default threshold's links (springs mirrored in src/main.ts)
const g = createGraph();
for (let i = 0; i < N; i++) g.addNode(i);
for (const [a, b, s] of kept) if (s >= def) g.addLink(a, b, { t: Math.min((s - lo) / (hi - lo), 1) });
const layout = createLayout(g, {
  dimensions: 3, springLength: 30, springCoefficient: 0.0008, gravity: -2.5, theta: 0.9, dragCoefficient: 0.04, timeStep: 20,
  springTransform: (link, spring) => {
    // t = 1 (most similar) -> short, tight spring; t = 0 -> long, loose.
    spring.length = 17.5 + (1 - link.data.t) * 30;
    spring.weight = 0.6 + 0.4 * link.data.t;
  },
});
const STEPS = Number(process.env.STEPS ?? 600);
for (let i = 0; i < STEPS; i++) {
  layout.step();
  // Same gentle pull to the origin as the page, so isolated texts stay near.
  g.forEachNode((n) => { const p = layout.getBody(n.id).pos; p.x *= 0.99; p.y *= 0.99; p.z *= 0.99; });
  if (i % 200 === 0) console.log(`layout ${i}/${STEPS}`);
}

const describe = loadCanon();
const nodes = uids.map((uid, i) => {
  const m = describe(uid);
  const p = layout.getNodePosition(i);
  return [uid, m.acronym, m.en, m.root, m.group, m.group, 0, Math.round(p.x), Math.round(p.y), Math.round(p.z)];
});

fs.writeFileSync(`${OUT}.json`, JSON.stringify({
  legendTitle: 'Collections',
  types: ['similar'],
  sim: { url: path.basename(`${OUT}.bin`), min: lo, max: hi, default: def, range: [lo, hi] },
  langs: GROUPS.map(({ iso, name }) => ({ iso, name })),
  colors: GROUPS.map((x) => x.color),
  pitakas: ['sutta'],
  collections: GROUPS.map(({ iso, name }) => ({ uid: iso, name })),
  nodes,
  edges: [], // built in the page from the pair file
}));
console.log(`wrote ${OUT}.json (${(fs.statSync(`${OUT}.json`).size / 1e6).toFixed(1)} MB), ${OUT}.bin (${(buf.length / 1e6).toFixed(1)} MB)`);
