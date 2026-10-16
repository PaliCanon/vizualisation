// Data files in public/, built by scripts/build-graph.mjs (SuttaCentral) and
// scripts/build-similarity.mjs (vectorial space).

export type NodeRow = [uid: string, acronym: string, en: string, root: string, coll: number, group: number, pitaka: number, x: number, y: number, z: number];

export interface Graph {
  legendTitle?: string;
  colors?: string[]; // one colour per group
  types: string[];
  langs: { iso: string; name: string }[]; // the colour groups (collections)
  pitakas: string[];
  collections: { uid: string; name: string }[];
  nodes: NodeRow[];
  edges: number[]; // flat [a, b, type, a, b, type, ...]
  // Vectorial space: links come from a binary list of pairs, filtered by an adjustable
  // threshold. Pairs store t = (sim - range[0]) / (range[1] - range[0]) as u16.
  sim?: { url: string; min: number; max: number; default: number; range?: [number, number] };
  fixed?: boolean; // positions are the data (a 3D reduction): no live layout when the threshold moves
}

export type MapName = 'liens' | 'sim';
// Labels and hints are translated: views.label_<value>, views.<value>.
export const MAPS: { value: MapName; file: string }[] = [
  { value: 'sim', file: 'graph-sim.json' },
  { value: 'liens', file: 'graph.json' },
];

export const TYPE_STYLE: Record<string, { color: string; opacity: number }> = {
  full: { color: '#ffffff', opacity: 0.22 },
  sections: { color: '#c7d2fe', opacity: 0.16 },
  resembling: { color: '#94a3b8', opacity: 0.1 },
  mentions: { color: '#64748b', opacity: 0.08 },
  retells: { color: '#64748b', opacity: 0.08 },
  similar: { color: '#c9a84c', opacity: 0.1 },
};

export async function loadGraph(name: MapName): Promise<Graph> {
  const file = MAPS.find((m) => m.value === name)!.file;
  return fetch(`${import.meta.env.BASE_URL}${file}`).then((r) => r.json());
}

export interface Pairs { a: Uint16Array; b: Uint16Array; sim: Float32Array }

export async function loadPairs(sim: NonNullable<Graph['sim']>): Promise<Pairs> {
  const buf = new DataView(await fetch(`${import.meta.env.BASE_URL}${sim.url}`).then((r) => r.arrayBuffer()));
  const n = buf.byteLength / 6;
  const [lo, hi] = sim.range ?? [0, 1];
  const p: Pairs = { a: new Uint16Array(n), b: new Uint16Array(n), sim: new Float32Array(n) };
  for (let k = 0; k < n; k++) {
    p.a[k] = buf.getUint16(k * 6, true);
    p.b[k] = buf.getUint16(k * 6 + 2, true);
    p.sim[k] = lo + (buf.getUint16(k * 6 + 4, true) / 65535) * (hi - lo);
  }
  return p;
}

// Links of the vectorial space at a threshold, with each link's similarity (key a * N + b, a < b).
export function linksAt(pairs: Pairs, th: number, N: number) {
  const edges: number[] = [];
  const sims = new Map<number, number>();
  for (let k = 0; k < pairs.sim.length; k++) {
    if (pairs.sim[k]! < th) continue;
    edges.push(pairs.a[k]!, pairs.b[k]!, 0);
    sims.set(pairs.a[k]! * N + pairs.b[k]!, pairs.sim[k]!);
  }
  return { edges, sims };
}

// Texts translated on SuttaCentral, per language: title and translator (index into translators).
export interface LangTranslations { titles: Record<string, string>; by: Record<string, number>; translators: string[] }
export type Translations = Record<string, LangTranslations>;
export async function loadTranslations(): Promise<Translations> {
  return fetch(`${import.meta.env.BASE_URL}translations.json`).then((r) => r.json());
}

// SuttaCentral's summaries of the texts in a language ({ uid: blurb }); {} when there are none.
export async function loadBlurbs(lang: string): Promise<Record<string, string>> {
  const r = await fetch(`${import.meta.env.BASE_URL}blurbs-${lang}.json`);
  return r.ok && r.headers.get("content-type")?.includes("json") ? r.json() : {};
}

export const normalize = (s: string) => s.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase().replace(/\s+/g, '');
