// Prototype page for the Rust engine (engine/): a map drawn either by the Rust engine or by the
// three.js one, with a readout and hooks (window.bench) to compare them.
//   ?engine=three        the three.js engine (default: Rust)
//   ?engine=none         no engine, nothing drawn: what the browser alone gives (baseline)
//   ?graph=sim&th=0.52   the vectorial space at a threshold (default: SuttaCentral)
import * as THREE from "three";
import "./styles.css";
import { MapEngine } from "./map/engine";
import { linksAt, loadGraph, loadPairs, TYPE_STYLE, type Graph } from "./map/data";
import { mapTheme } from "./map/theme";
import init, { createGraphView, type GraphView } from "../engine/pkg/map_engine.js";

const params = new URLSearchParams(location.search);
const canvas = document.getElementById("map") as HTMLCanvasElement;
const hud = document.getElementById("hud")!;
const useThree = params.get("engine") === "three";
const info: Record<string, string> = {};
const show = () => (hud.textContent = Object.entries(info).map(([k, v]) => `${k}: ${v}`).join("\n"));

// What a benchmark reads: time spent in the engine's frame (CPU), frame intervals.
const cpu: number[] = [];
const gpu: number[] = []; // GPU time of a frame's drawing, ms (timer queries; one sample every few frames)
const intervals: number[] = [];
let drawn = 0; // frames the engine actually drew
let drawnAt = 0;
declare global { interface Window { bench: any } } // eslint-disable-line @typescript-eslint/no-explicit-any

function meter() {
  let last = performance.now();
  const tick = (now: number) => {
    intervals.push(now - last);
    last = now;
    if (intervals.length % 60 === 0) {
      const xs = intervals.slice(-60).sort((a, b) => a - b);
      info.fps = `${(1000 / (xs.reduce((a, b) => a + b) / xs.length)).toFixed(0)} (p95 ${xs[57]!.toFixed(1)} ms)`;
      // frames the engine drew over the same 60 (0 when the map is still: it draws on change only)
      if (!useThree) info.drawn = `${drawn - drawnAt} / 60`;
      drawnAt = drawn;
      show();
    }
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}

const pct = (xs: number[], p: number) => { const s = [...xs].sort((a, b) => a - b); return s[Math.floor(s.length * p)] ?? 0; };
const stats = () => ({
  frames: intervals.length,
  drawn,
  fps: +(1000 / (intervals.reduce((a, b) => a + b, 0) / intervals.length)).toFixed(1),
  frame_p50: +pct(intervals, 0.5).toFixed(2),
  frame_p95: +pct(intervals, 0.95).toFixed(2),
  cpu_p50: +pct(cpu, 0.5).toFixed(3),
  cpu_p95: +pct(cpu, 0.95).toFixed(3),
  gpu_n: gpu.length,
  gpu_p50: +pct(gpu, 0.5).toFixed(3),
  gpu_p95: +pct(gpu, 0.95).toFixed(3),
});

const linear = (hex: string) => new THREE.Color(hex).toArray() as [number, number, number]; // what three's point shader gets
const srgb = (hex: string) => { const c = new THREE.Color(hex); c.convertLinearToSRGB(); return c.toArray(); }; // what three's line materials show

function adjacency(graph: Graph) {
  const adj: [number, number][][] = Array.from({ length: graph.nodes.length }, () => []);
  const e = graph.edges;
  for (let k = 0; k < e.length; k += 3) { adj[e[k]!]!.push([e[k + 1]!, e[k + 2]!]); adj[e[k + 1]!]!.push([e[k]!, e[k + 2]!]); }
  return adj;
}

async function rust(graph: Graph, t0: number) {
  await init();
  info.wasm = `${(performance.now() - t0).toFixed(0)} ms`;
  const r = canvas.getBoundingClientRect();
  const pos = new Float32Array(graph.nodes.flatMap((n) => [n[7], n[8], n[9]]));
  const view = await createGraphView(canvas, r.width, r.height, devicePixelRatio, pos);
  info.engine = `rust (${view.backend})`;
  view.setTiming(true); // test page: measure GPU time
  const theme = mapTheme("dark", graph);
  const [br, bg, bb] = srgb(theme.background);
  view.setStyle(br, bg, bb, theme.light);
  const N = graph.nodes.length;
  const gold = linear(theme.gold);
  const colors = new Float32Array(N * 3);
  for (let i = 0; i < N; i++) colors.set(gold, i * 3);
  view.setPoints(colors, new Float32Array(N).fill(7));
  const adj = adjacency(graph);
  const e = graph.edges;
  const base = srgb(theme.gold);
  const opacity = graph.types.map((t) => TYPE_STYLE[t]!.opacity * 1.5);
  const segments = (sel: number) => {
    const extra = sel >= 0 ? adj[sel]!.length : 0;
    const ends = new Uint32Array((e.length / 3 + extra) * 2);
    const cols = new Float32Array((e.length / 3 + extra) * 4);
    const dim = sel >= 0 ? 0.25 : 1;
    for (let k = 0, s = 0; k < e.length; k += 3, s++) {
      ends[s * 2] = e[k]!; ends[s * 2 + 1] = e[k + 1]!;
      cols.set(base, s * 4); cols[s * 4 + 3] = Math.min(opacity[e[k + 2]!]! * dim, 1);
    }
    let s = e.length / 3;
    for (const [j, t] of sel >= 0 ? adj[sel]! : []) {
      ends[s * 2] = sel; ends[s * 2 + 1] = j;
      cols.set(srgb(theme.types[graph.types[t]!]!), s * 4); cols[s * 4 + 3] = 0.85;
      s++;
    }
    view.setSegments(ends, cols);
  };
  segments(-1);
  const select = (i: number) => {
    const hl = new Float32Array(N);
    if (i >= 0) { hl[i] = 2; for (const [j] of adj[i]!) hl[j] = 1; }
    view.setHighlight(hl, i >= 0);
    segments(i);
    if (i < 0) return;
    const p = (k: number) => new THREE.Vector3(pos[k * 3], pos[k * 3 + 1], pos[k * 3 + 2]);
    let d = 60;
    for (const [j] of adj[i]!) d = Math.max(d, p(i).distanceTo(p(j)));
    view.flyTo(i, Math.min(d * 2.2, 1500), performance.now());
    info.selected = `${graph.nodes[i]![1]} (${adj[i]!.length} links)`;
    show();
  };
  bind(view, select);
  let lastGpu = NaN;
  const loop = (now: number) => {
    const a = performance.now();
    if (view.frame(now)) drawn++;
    cpu.push(performance.now() - a);
    const g = view.gpuMs;
    if (!Number.isNaN(g) && g !== lastGpu) gpu.push((lastGpu = g));
    requestAnimationFrame(loop);
  };
  requestAnimationFrame(loop);
  new ResizeObserver(() => { const b = canvas.getBoundingClientRect(); view.resize(b.width, b.height, devicePixelRatio); }).observe(canvas);
  return { select: (i: number) => { const a = performance.now(); select(i); return performance.now() - a; }, pick: (x: number, y: number) => view.pick(x, y) };
}

function bind(view: GraphView, select: (i: number) => void) {
  let down = { x: 0, y: 0 };
  const at = (ev: PointerEvent | WheelEvent) => { const b = canvas.getBoundingClientRect(); return [ev.clientX - b.left, ev.clientY - b.top] as const; };
  canvas.addEventListener("pointerdown", (ev) => { canvas.setPointerCapture(ev.pointerId); down = { x: ev.clientX, y: ev.clientY }; view.pointerDown(...at(ev)); });
  canvas.addEventListener("pointermove", (ev) => {
    view.pointerMove(...at(ev));
    canvas.style.cursor = view.pick(...at(ev)) >= 0 ? "pointer" : "";
  });
  canvas.addEventListener("pointerup", (ev) => {
    view.pointerUp();
    if (Math.hypot(ev.clientX - down.x, ev.clientY - down.y) <= 4) select(view.pick(...at(ev)));
  });
  // Arrow keys travel: left/right slide, up/down go forward or back, with Shift rise or sink.
  const held = new Set<string>();
  let shift = false;
  const k = (key: string) => +held.has(key);
  const keys = () => {
    const ud = k("ArrowUp") - k("ArrowDown");
    view.setKeys(k("ArrowRight") - k("ArrowLeft"), shift ? ud : 0, shift ? 0 : ud);
  };
  addEventListener("keydown", (ev) => { shift = ev.shiftKey; if (!ev.key.startsWith("Arrow")) return keys(); ev.preventDefault(); held.add(ev.key); keys(); });
  addEventListener("keyup", (ev) => { shift = ev.shiftKey; held.delete(ev.key); keys(); });
  addEventListener("blur", () => { held.clear(); keys(); });
  canvas.addEventListener("wheel", (ev) => { ev.preventDefault(); view.wheel(ev.deltaY); }, { passive: false });
}

function three(graph: Graph, sims: Map<number, number>) {
  const engine = new MapEngine(canvas, graph, mapTheme("dark", graph), {
    onSelect: (i) => { engine.select(i); info.selected = i >= 0 ? graph.nodes[i]![1] : "-"; show(); },
    onHover: () => {},
  });
  if (sims.size) engine.setLinks(graph.edges, sims, false);
  info.engine = "three.js";
  // Time the engine's frame (private, reached for the benchmark only).
  const e = engine as unknown as { frame: (now: number) => void; pickAt: (x: number, y: number) => number };
  const frame = e.frame.bind(engine);
  e.frame = (now) => { const a = performance.now(); frame(now); cpu.push(performance.now() - a); };
  timeThree(engine);
  const r = canvas.getBoundingClientRect();
  return {
    select: (i: number) => { const a = performance.now(); engine.select(i); return performance.now() - a; },
    pick: (x: number, y: number) => e.pickAt(x + r.left, y + r.top),
  };
}

// GPU time of three's render calls, with WebGL2 timer queries (one in flight at a time).
function timeThree(engine: MapEngine) {
  const renderer = (engine as unknown as { renderer: THREE.WebGLRenderer }).renderer;
  const gl = renderer.getContext() as WebGL2RenderingContext;
  const ext = gl.getExtension("EXT_disjoint_timer_query_webgl2");
  if (!ext) return;
  const render = renderer.render.bind(renderer);
  let pending: WebGLQuery | null = null;
  renderer.render = (scene, camera) => {
    if (pending && gl.getQueryParameter(pending, gl.QUERY_RESULT_AVAILABLE)) {
      if (!gl.getParameter(ext.GPU_DISJOINT_EXT)) gpu.push(gl.getQueryParameter(pending, gl.QUERY_RESULT) / 1e6);
      gl.deleteQuery(pending);
      pending = null;
    }
    if (pending) return render(scene, camera);
    pending = gl.createQuery()!;
    gl.beginQuery(ext.TIME_ELAPSED_EXT, pending);
    render(scene, camera);
    gl.endQuery(ext.TIME_ELAPSED_EXT);
  };
}

const t0 = performance.now();
const graph = await loadGraph(params.get("graph") === "sim" ? "sim" : "liens");
let sims = new Map<number, number>();
if (graph.sim) {
  const l = linksAt(await loadPairs(graph.sim), Number(params.get("th") ?? graph.sim.default), graph.nodes.length);
  graph.edges = l.edges;
  sims = l.sims;
}
info.points = `${graph.nodes.length}, links ${graph.edges.length / 3}`;
const t1 = performance.now();
const none = params.get("engine") === "none";
if (none) info.engine = "none (baseline: nothing drawn)";
const api = none ? { select: () => 0, pick: () => -1 } : useThree ? three(graph, sims) : await rust(graph, t1);
info.ready = `${(performance.now() - t1).toFixed(0)} ms (+ data ${(t1 - t0).toFixed(0)} ms)`;
window.bench = { ...api, reset: () => { drawn = 0; drawnAt = 0; cpu.length = 0; gpu.length = 0; intervals.length = 0; }, stats };
show();
meter();
