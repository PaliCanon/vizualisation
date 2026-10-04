// Metadata for any text of the Pāli canon, from SuttaCentral's sc-data (data/sc-data):
// acronym, English and Pāli titles, and a display group (DN, MN, SN, AN, KN, Vinaya,
// Abhidhamma). Used by the vector maps, whose texts come from different sources.
import fs from 'node:fs';
import path from 'node:path';

const SC = 'data/sc-data';

export const GROUPS = [
  { iso: 'dn', name: 'Dīgha Nikāya', color: '#ffffff' }, // not gold: gold is the page's default colour
  { iso: 'mn', name: 'Majjhima Nikāya', color: '#ff6f8e' },
  { iso: 'sn', name: 'Saṃyutta Nikāya', color: '#3ee0c3' },
  { iso: 'an', name: 'Aṅguttara Nikāya', color: '#8b9cff' },
  { iso: 'kn', name: 'Khuddaka Nikāya', color: '#c792ff' },
  { iso: 'vinaya', name: 'Vinaya', color: '#ff9b5c' },
  { iso: 'abhidhamma', name: 'Abhidhamma', color: '#a6e86f' },
];

const walk = (dir) => fs.readdirSync(dir, { withFileTypes: true })
  .flatMap((e) => (e.isDirectory() ? walk(path.join(dir, e.name)) : [path.join(dir, e.name)]));
const readJson = (p) => JSON.parse(fs.readFileSync(path.join(SC, p), 'utf8'));

export function loadCanon() {
  // child -> parent over the super tree and every collection tree
  const parent = new Map();
  const roots = new Set();
  const visit = (node, par) => {
    if (typeof node === 'string') { if (par && !parent.has(node)) parent.set(node, par); return; }
    if (Array.isArray(node)) { node.forEach((c) => visit(c, par)); return; }
    for (const [k, v] of Object.entries(node)) { if (par && !parent.has(k)) parent.set(k, par); visit(v, k); }
  };
  visit(readJson('structure/tree/super-tree.json'), null);
  for (const f of walk(path.join(SC, 'structure/tree'))) {
    if (f.endsWith('super-tree.json')) continue;
    const t = JSON.parse(fs.readFileSync(f, 'utf8'));
    Object.keys(t).forEach((k) => roots.add(k));
    visit(t, null);
  }

  const names = (dir) => {
    const m = new Map();
    for (const f of walk(path.join(SC, dir))) {
      for (const [k, v] of Object.entries(JSON.parse(fs.readFileSync(f, 'utf8')))) {
        const uid = k.split(':')[1]?.replace(/^[\d.]+?\./, '');
        if (uid && v.trim()) m.set(uid, v.trim());
      }
    }
    return m;
  };
  const en = names('sc_bilara_data/translation/en/sujato/name');
  const root = names('sc_bilara_data/root');
  const acronyms = new Map(readJson('structure/text_extra_info.json').filter((x) => x.acronym).map((x) => [x.uid, x.acronym]));
  const collAcronym = new Map(readJson('misc/uid_expansion.json').map((x) => [x.uid, x.acronym]));
  for (const x of readJson('structure/super_extra_info.json')) if (x.acronym && !collAcronym.has(x.uid)) collAcronym.set(x.uid, x.acronym);

  // A uid's ancestors, trying shorter forms for ranges and sub-numbers (an1.1-10 -> an1 ...).
  const chain = (uid) => {
    let cur = uid;
    for (const c of [uid, uid.replace(/-\d+$/, ''), uid.replace(/\.[\d-]+$/, ''), uid.replace(/[\d.-]+$/, '')]) {
      if (parent.has(c) || roots.has(c)) { cur = c; break; }
    }
    const out = [cur];
    while (parent.has(cur)) { cur = parent.get(cur); out.push(cur); }
    return out;
  };
  const groupIdx = new Map(GROUPS.map((g, i) => [g.iso, i]));

  return function describe(uid) {
    const c = chain(uid);
    const basket = c[c.length - 1];
    const collection = c.find((a) => roots.has(a)) ?? c[0];
    let group = groupIdx.get(basket);
    if (basket === 'sutta') group = groupIdx.get(collection) ?? groupIdx.get('kn');
    if (group === undefined) group = groupIdx.get(uid.match(/^[a-z]+/)?.[0]) ?? groupIdx.get('kn');
    let acronym = acronyms.get(uid);
    if (!acronym) {
      const a = collAcronym.get(collection);
      acronym = a && uid.startsWith(collection) ? `${a} ${uid.slice(collection.length).replace(/^-/, '')}`.trim() : uid;
    }
    return { acronym, en: en.get(uid) ?? '', root: root.get(uid) ?? '', group };
  };
}

// Read a vector dump (per chunk: doc u32, chunk u32, dim × f32, little-endian) and its
// docs JSON ({ "<doc>": { "id": uid, ... } }). One text = mean of its chunks, L2-normalised.
export function loadVectors(binPath, docsPath, dim) {
  const buf = fs.readFileSync(binPath);
  const rec = 8 + dim * 4;
  if (buf.length % rec) throw new Error(`${binPath}: size is not a multiple of ${rec} (dim ${dim}?)`);
  const sums = new Map();
  for (let o = 0; o < buf.length; o += rec) {
    const d = buf.readUInt32LE(o);
    if (!sums.has(d)) sums.set(d, new Float64Array(dim));
    const s = sums.get(d);
    for (let k = 0; k < dim; k++) s[k] += buf.readFloatLE(o + 8 + k * 4);
  }
  const docs = JSON.parse(fs.readFileSync(docsPath, 'utf8'));
  const ids = [...sums.keys()].sort((a, b) => a - b);
  const uids = [], vectors = [];
  for (const d of ids) {
    const s = sums.get(d);
    let n = 0; for (let k = 0; k < dim; k++) n += s[k] * s[k];
    n = Math.sqrt(n);
    if (n < 1e-9) continue; // empty text
    uids.push(docs[d].id);
    vectors.push(Float32Array.from(s, (x) => x / n));
  }
  return { uids, vectors };
}
