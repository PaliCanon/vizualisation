// Synthetic benchmark of the two 3D engines, no corpus: `points` random points in a ball,
// `links` random segments, optionally every point moving every frame (`animate=1`, what a
// running layout does). window.bench: stats(), reset(), pick(x, y).
import * as THREE from "three";
import { MapEngine } from "./map/engine";
import type { Graph } from "./map/data";
import type { MapTheme } from "./map/theme";
import init, { createGraphView } from "../engine/pkg/map_engine.js";

const q = new URLSearchParams(location.search);
const N = Number(q.get("points") ?? 10000);
const M = Number(q.get("links") ?? 10000);
const animate = q.get("animate") === "1";
const canvas = document.getElementById("map") as HTMLCanvasElement;

// Deterministic data: the same for both engines.
let seed = 1;
const rand = () => ((seed = (seed * 16807) % 2147483647) - 1) / 2147483646;
const pos = new Float32Array(N * 3);
for (let i = 0; i < N; i++) {
  const r = 800 * Math.cbrt(rand()), t = rand() * 2 * Math.PI, z = rand() * 2 - 1, s = Math.sqrt(1 - z * z);
  pos.set([r * s * Math.cos(t), r * s * Math.sin(t), r * z], i * 3);
}
const ends = new Uint32Array(M * 2);
for (let k = 0; k < M * 2; k++) ends[k] = Math.floor(rand() * N);

const cpu: number[] = [], gpu: number[] = [], intervals: number[] = [];
const pct = (xs: number[], p: number) => { const s = [...xs].sort((a, b) => a - b); return +(s[Math.floor(s.length * p)] ?? 0).toFixed(3); };
let last = performance.now();
const tick = (now: number) => { intervals.push(now - last); last = now; requestAnimationFrame(tick); };
requestAnimationFrame(tick);

// Every point moves a little, every frame.
const moved = new Float32Array(N * 3);
const wobble = (now: number) => {
  const k = Math.sin(now / 300) * 5;
  for (let i = 0; i < N * 3; i++) moved[i] = pos[i]! + k * ((i % 3) - 1);
  return moved;
};

async function rust() {
  await init();
  const r = canvas.getBoundingClientRect();
  const view = await createGraphView(canvas, r.width, r.height, devicePixelRatio, pos);
  view.setStyle(0.125, 0.15, 0.18, false);
  view.setTiming(true);
  const colors = new Float32Array(N * 3).fill(0.6);
  view.setPoints(colors, new Float32Array(N).fill(7));
  view.setSegments(ends, new Float32Array(M * 4).fill(0.3));
  let lastGpu = NaN;
  const loop = (now: number) => {
    const a = performance.now();
    if (animate) view.setPositions(wobble(now));
    view.frame(now);
    cpu.push(performance.now() - a);
    const g = view.gpuMs;
    if (!Number.isNaN(g) && g !== lastGpu) gpu.push((lastGpu = g));
    requestAnimationFrame(loop);
  };
  requestAnimationFrame(loop);
  return { backend: view.backend, pick: (x: number, y: number) => view.pick(x, y) };
}

function three() {
  const nodes = Array.from({ length: N }, (_, i) => ["", "", "", "", 0, 0, 0, pos[i * 3]!, pos[i * 3 + 1]!, pos[i * 3 + 2]!]);
  const edges: number[] = [];
  for (let k = 0; k < M; k++) edges.push(ends[k * 2]!, ends[k * 2 + 1]!, 0);
  const graph = { types: ["full"], langs: [{ iso: "x", name: "x" }], pitakas: [], collections: [], nodes, edges } as unknown as Graph;
  const theme: MapTheme = { light: false, background: "#20262e", gold: "#999999", groups: ["#999999"], types: { full: "#ffffff" } };
  const engine = new MapEngine(canvas, graph, theme, { onSelect: () => {}, onHover: () => {} });
  const e = engine as unknown as {
    frame: (now: number) => void; pickAt: (x: number, y: number) => number; renderer: THREE.WebGLRenderer;
    positions: Float32Array; nodeGeom: THREE.BufferGeometry; refreshEdgePositions: () => void;
  };
  const frame = e.frame.bind(engine);
  e.frame = (now) => {
    const a = performance.now();
    if (animate) {
      // what stepLayout does with new positions
      e.positions.set(wobble(now));
      e.nodeGeom.attributes.position!.needsUpdate = true;
      e.nodeGeom.computeBoundingSphere();
      e.refreshEdgePositions();
    }
    frame(now);
    cpu.push(performance.now() - a);
  };
  timeThree(e.renderer);
  const r = canvas.getBoundingClientRect();
  return { backend: "three.js WebGL2", pick: (x: number, y: number) => e.pickAt(x + r.left, y + r.top) };
}

function timeThree(renderer: THREE.WebGLRenderer) {
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
const api = q.get("engine") === "three" ? three() : await rust();
const setup = performance.now() - t0;
declare global { interface Window { bench: any } } // eslint-disable-line @typescript-eslint/no-explicit-any
window.bench = {
  ...api,
  reset: () => { cpu.length = 0; gpu.length = 0; intervals.length = 0; },
  stats: () => ({
    backend: api.backend, setup_ms: +setup.toFixed(1),
    fps: +(1000 / (intervals.reduce((a, b) => a + b, 0) / Math.max(intervals.length, 1))).toFixed(0),
    frame_p95: pct(intervals, 0.95), cpu_p50: pct(cpu, 0.5), cpu_p95: pct(cpu, 0.95),
    gpu_n: gpu.length, gpu_p50: pct(gpu, 0.5), gpu_p95: pct(gpu, 0.95),
    heap_mb: +(((performance as unknown as { memory?: { usedJSHeapSize: number } }).memory?.usedJSHeapSize ?? 0) / 1e6).toFixed(0),
  }),
};
