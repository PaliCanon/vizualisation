// Builds public/graph.json from SuttaCentral's sc-data (data/sc-data).
// Nodes: texts. Edges: parallels (deduplicated, strongest type kept).
// Also precomputes a 3D force layout so the browser doesn't have to.
import fs from 'node:fs';
import path from 'node:path';
import createGraph from 'ngraph.graph';
import createLayout from 'ngraph.forcelayout';
import { GROUPS, loadCanon } from './lib/canon.mjs';

const SC = 'data/sc-data';
const readJson = (p) => JSON.parse(fs.readFileSync(path.join(SC, p), 'utf8'));
const walkFiles = (dir) =>
  fs.readdirSync(dir, { withFileTypes: true }).flatMap((e) =>
    e.isDirectory() ? walkFiles(path.join(dir, e.name)) : [path.join(dir, e.name)]);

// ---- Edge types, strongest first
const TYPES = ['full', 'sections', 'resembling', 'mentions', 'retells'];

// ---- Hierarchy: child -> parent, from the super tree and every per-collection tree
const parent = new Map();
const collectionRoots = new Set();
function walkTree(node, par) {
  if (typeof node === 'string') { if (par && !parent.has(node)) parent.set(node, par); return; }
  if (Array.isArray(node)) { node.forEach((c) => walkTree(c, par)); return; }
  for (const [k, v] of Object.entries(node)) {
    if (par && !parent.has(k)) parent.set(k, par);
    walkTree(v, k);
  }
}
walkTree(readJson('structure/tree/super-tree.json'), null);
for (const f of walkFiles(path.join(SC, 'structure/tree'))) {
  if (f.endsWith('super-tree.json')) continue;
  const t = JSON.parse(fs.readFileSync(f, 'utf8'));
  Object.keys(t).forEach((k) => collectionRoots.add(k));
  walkTree(t, null);
}

const langOf = new Map();
const LANGS = [];
for (const l of readJson('structure/super_root_lang.json')) {
  LANGS.push({ iso: l.root_lang_iso, name: l.root_lang_name });
  l.contains.forEach((c) => langOf.set(c, LANGS.length - 1));
}
LANGS.push({ iso: '?', name: 'Other' });
const OTHER_LANG = LANGS.length - 1;

const PITAKAS = ['sutta', 'vinaya', 'abhidhamma'];

// ---- Names / acronyms
const acronyms = new Map();
for (const x of readJson('structure/text_extra_info.json')) if (x.acronym) acronyms.set(x.uid, x.acronym);
const collNames = new Map();
for (const x of readJson('misc/uid_expansion.json')) collNames.set(x.uid, x);
for (const x of readJson('structure/super_extra_info.json')) if (x.acronym && !collNames.has(x.uid)) collNames.set(x.uid, { acronym: x.acronym });

function loadNames(dir) {
  const m = new Map();
  if (!fs.existsSync(dir)) return m;
  for (const f of walkFiles(dir)) {
    if (!f.endsWith('.json')) continue;
    for (const [k, v] of Object.entries(JSON.parse(fs.readFileSync(f, 'utf8')))) {
      const uid = k.split(':')[1]?.replace(/^[\d.]+?\./, '');
      if (uid && v.trim()) m.set(uid, v.trim());
    }
  }
  return m;
}
const enNames = loadNames(path.join(SC, 'sc_bilara_data/translation/en/sujato/name'));
const rootNames = loadNames(path.join(SC, 'sc_bilara_data/root'));

// ---- Resolve a uid's place in the hierarchy
function ancestors(uid) {
  // Try the uid itself, then progressively shorter forms (ranges, sub-numbers).
  let cur = uid;
  const tried = [uid, uid.replace(/-\d+$/, ''), uid.replace(/\.[\d-]+$/, ''), uid.replace(/[\d.-]+$/, '')];
  for (const c of tried) if (parent.has(c) || collectionRoots.has(c)) { cur = c; break; }
  const chain = [cur];
  while (parent.has(cur)) { cur = parent.get(cur); chain.push(cur); }
  return chain;
}
function classify(uid) {
  const chain = ancestors(uid);
  const prefixLang = LANGS.findIndex((l) => uid.startsWith(`${l.iso}-`));
  const lang = chain.map((a) => langOf.get(a)).find((x) => x !== undefined)
    ?? (prefixLang >= 0 ? prefixLang : OTHER_LANG);
  const pitaka = PITAKAS.indexOf(chain[chain.length - 1]);
  const collection = chain.find((a) => collectionRoots.has(a)) ?? chain[chain.length > 1 ? 1 : 0];
  return { lang, pitaka, collection };
}
function acronymOf(uid, collection) {
  if (acronyms.has(uid)) return acronyms.get(uid);
  const c = collNames.get(collection);
  if (c?.acronym && uid.startsWith(collection)) return `${c.acronym} ${uid.slice(collection.length).replace(/^-/, '')}`.trim();
  return uid;
}

// ---- Parallels
const raw = readJson('relationship/new_parallels.json');
const norm = (s) => s.replace(/#.*/, '').replace(/^~/, '').trim();
const isUid = (s) => /^[a-z][a-z0-9.\-]*$/.test(s);

const ids = new Map();
const nodeList = [];
const idOf = (uid) => {
  if (!ids.has(uid)) { ids.set(uid, nodeList.length); nodeList.push(uid); }
  return ids.get(uid);
};
// ONLY_LANG=pli keeps only texts in that root language (and links between them).
const ONLY_LANG = process.env.ONLY_LANG;
const onlyLangIdx = ONLY_LANG ? LANGS.findIndex((l) => l.iso === ONLY_LANG) : -1;
if (ONLY_LANG && onlyLangIdx < 0) throw new Error(`unknown language ${ONLY_LANG}`);
const langCache = new Map();
const keep = (uid) => {
  if (onlyLangIdx < 0) return true;
  if (!langCache.has(uid)) langCache.set(uid, classify(uid).lang === onlyLangIdx);
  return langCache.get(uid);
};

const pairs = new Map(); // "a,b" -> type index
function addEdge(a, tgt, t) {
  const b = norm(tgt);
  if (!isUid(b) || a === b || !keep(a) || !keep(b)) return;
  const ia = idOf(a), ib = idOf(b);
  const key = ia < ib ? `${ia},${ib}` : `${ib},${ia}`;
  if (!pairs.has(key) || pairs.get(key) > t) pairs.set(key, t);
}
for (const [src, rels] of Object.entries(raw)) {
  const a = norm(src);
  if (!isUid(a)) continue;
  for (const [type, targets] of Object.entries(rels)) {
    if (type === 'sections') {
      // { "an1.43#1.1": { full: [...], mentions: [...] } } — a passage-level parallel.
      for (const sub of Object.values(targets))
        for (const [subType, subTargets] of Object.entries(sub)) {
          const st = TYPES.indexOf(subType);
          const t = st === 0 || st === 2 ? TYPES.indexOf('sections') : st;
          if (t >= 0) subTargets.forEach((tgt) => addEdge(a, tgt, t));
        }
      continue;
    }
    const t = TYPES.indexOf(type);
    if (t >= 0) targets.forEach((tgt) => addEdge(a, tgt, t));
  }
}
console.log(`nodes ${nodeList.length}, edges ${pairs.size}`);
const typeCounts = TYPES.map((_, i) => [...pairs.values()].filter((t) => t === i).length);
console.log('edges per type', Object.fromEntries(TYPES.map((t, i) => [t, typeCounts[i]])));

// ---- Layout (3D force-directed)
const g = createGraph();
nodeList.forEach((_, i) => g.addNode(i));
for (const [key, t] of pairs) {
  const [a, b] = key.split(',').map(Number);
  g.addLink(a, b, { t });
}
const layout = createLayout(g, {
  dimensions: 3,
  springLength: 30,
  springCoefficient: 0.0008,
  gravity: -2.5,
  theta: 0.9,
  dragCoefficient: 0.04,
  timeStep: 20,
  springTransform: (link, spring) => {
    // Full parallels pull tighter than loose resemblances/mentions.
    spring.length = [20, 30, 45, 60, 60][link.data.t];
    spring.weight = [1, 0.7, 0.4, 0.3, 0.3][link.data.t];
  },
});
const STEPS = Number(process.env.STEPS ?? 600);
const t0 = Date.now();
for (let i = 0; i < STEPS; i++) {
  layout.step();
  if (i % 100 === 0) console.log(`layout step ${i}/${STEPS} (${((Date.now() - t0) / 1000).toFixed(1)}s)`);
}

// ---- Pull disconnected components towards the centre (keeps each one's shape).
// Without this, small isolated clusters drift far away and the main mass looks tiny.
const pos = nodeList.map((_, i) => ({ ...layout.getNodePosition(i) }));
const comp = new Int32Array(nodeList.length).fill(-1);
const nbrs = nodeList.map(() => []);
for (const key of pairs.keys()) { const [a, b] = key.split(',').map(Number); nbrs[a].push(b); nbrs[b].push(a); }
let nComp = 0;
for (let s = 0; s < nodeList.length; s++) {
  if (comp[s] >= 0) continue;
  const stack = [s]; comp[s] = nComp;
  while (stack.length) for (const v of nbrs[stack.pop()]) if (comp[v] < 0) { comp[v] = nComp; stack.push(v); }
  nComp++;
}
const cent = Array.from({ length: nComp }, () => ({ x: 0, y: 0, z: 0, n: 0 }));
pos.forEach((p, i) => { const c = cent[comp[i]]; c.x += p.x; c.y += p.y; c.z += p.z; c.n++; });
cent.forEach((c) => { c.x /= c.n; c.y /= c.n; c.z /= c.n; });
const largest = cent.reduce((m, c, i) => (c.n > cent[m].n ? i : m), 0);
const origin = cent[largest];
const R = Math.max(...pos.filter((_, i) => comp[i] === largest).map((p) => Math.hypot(p.x - origin.x, p.y - origin.y, p.z - origin.z)));
pos.forEach((p, i) => {
  const c = cent[comp[i]];
  if (comp[i] === largest) return;
  const d = Math.hypot(c.x - origin.x, c.y - origin.y, c.z - origin.z) || 1;
  const target = R * 0.6 + Math.sqrt(d) * 8; // ring just around the main mass
  const k = target / d;
  p.x += (origin.x + (c.x - origin.x) * k) - c.x;
  p.y += (origin.y + (c.y - origin.y) * k) - c.y;
  p.z += (origin.z + (c.z - origin.z) * k) - c.z;
});
console.log(`components: ${nComp}, largest: ${cent[largest].n} nodes`);

// ---- Output
const collections = [];
const collIdx = new Map();
// One language only (ONLY_LANG): colour groups are the collections (DN, MN, SN, AN, KN,
// Vinaya, Abhidhamma) instead of languages, as in the vector maps.
const describe = ONLY_LANG ? loadCanon() : null;
const nodes = nodeList.map((uid, i) => {
  const { lang: langIdx, pitaka, collection } = classify(uid);
  const lang = describe ? describe(uid).group : langIdx;
  if (!collIdx.has(collection)) {
    collIdx.set(collection, collections.length);
    collections.push({ uid: collection, name: collNames.get(collection)?.name ?? enNames.get(collection) ?? rootNames.get(collection) ?? collection });
  }
  const p = pos[i];
  return [
    uid,
    acronymOf(uid, collection),
    enNames.get(uid) ?? '',
    rootNames.get(uid) ?? '',
    collIdx.get(collection),
    lang,
    pitaka,
    Math.round(p.x), Math.round(p.y), Math.round(p.z),
  ];
});
const edges = [];
for (const [key, t] of pairs) { const [a, b] = key.split(','); edges.push(+a, +b, t); }

const out = describe
  ? { legendTitle: 'Collections', types: TYPES, langs: GROUPS.map(({ iso, name }) => ({ iso, name })),
    colors: GROUPS.map((x) => x.color), pitakas: PITAKAS, collections, nodes, edges }
  : { types: TYPES, langs: LANGS, pitakas: PITAKAS, collections, nodes, edges };
fs.mkdirSync('public', { recursive: true });
fs.writeFileSync('public/graph.json', JSON.stringify(out));
const unknown = describe ? [] : nodes.filter((n) => n[5] === OTHER_LANG).map((n) => n[0]);
console.log(`unknown lang: ${unknown.length}`, unknown.slice(0, 20));
console.log(`untitled: ${nodes.filter((n) => !n[2] && !n[3]).length}`);
console.log(`wrote public/graph.json (${(fs.statSync('public/graph.json').size / 1e6).toFixed(1)} MB)`);
