// The map on the Rust engine (engine/, wgpu: WebGPU, else WebGL2). The engine only draws (points,
// segments, camera, picking); what to draw is decided here: colours by collection, visibility,
// link types, selection, the live force layout of the vectorial space.
import createGraph from 'ngraph.graph';
import createLayout, { type Layout } from 'ngraph.forcelayout';
import init, { createGraphView, type GraphView } from '../../engine/pkg/map_engine.js';
import { TYPE_STYLE, type Graph } from './data';
import type { MapTheme } from './theme';

/** What the map tells the page: a text chosen (-1: nothing), a text under the pointer. */
export interface EngineEvents {
  onSelect: (i: number) => void; // -1: nothing
  onHover: (i: number, x: number, y: number) => void;
}

const POINT_SIZE = 7;
const LINKS = 1.5; // link opacity factor
const LAYOUT_STEPS = 400;
const median = (xs: number[]) => xs.sort((x, y) => x - y)[Math.floor(xs.length / 2)]!;

type Rgb = [number, number, number];
const hex = (h: string): Rgb => {
  const n = parseInt(h.replace('#', '').slice(0, 6), 16);
  return [(n >> 16) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
};
// Colours are given to the engine linear (sRGB decoded).
const toLinear = (c: number) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
const linear = (h: string): Rgb => hex(h).map(toLinear) as Rgb;
const mix = (a: Rgb, b: Rgb, t: number): Rgb => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];

export class RustMapEngine {
  N: number;
  adj: [number, number][][] = [];
  private sims = new Map<number, number>();
  private colorOn: boolean[];
  private groupOn: boolean[];
  private typeOn: boolean[];
  private hideAlone = false;
  private available: boolean[] | null = null;
  private selected = -1;
  private positions: Float32Array;
  private sizes: Float32Array;
  private radius: number; // the scene's (90th percentile), for framing
  private layout: Layout<ReturnType<typeof createGraph>> | null = null;
  private layoutSteps = LAYOUT_STEPS;
  private simRange: [number, number] = [0, 1];
  private raf = 0;
  private disposed = false; // React may still call a setter in the commit that disposes it
  private cleanup: (() => void)[] = [];

  /** Load the engine (once per page) and set it up on `canvas`. */
  static async create(canvas: HTMLCanvasElement, g: Graph, theme: MapTheme, events: EngineEvents) {
    await init();
    const r = canvas.getBoundingClientRect();
    const pos = new Float32Array(g.nodes.flatMap((n) => [n[7], n[8], n[9]]));
    const view = await createGraphView(canvas, r.width, r.height, devicePixelRatio, pos);
    return new RustMapEngine(canvas, view, g, pos, theme, events);
  }

  private constructor(
    private canvas: HTMLCanvasElement,
    private view: GraphView,
    private g: Graph,
    positions: Float32Array,
    private theme: MapTheme,
    private events: EngineEvents,
  ) {
    this.N = 0;
    this.positions = positions;
    this.sizes = new Float32Array(0);
    this.colorOn = this.groupOn = this.typeOn = [];
    this.radius = 1;
    this.reset(g, positions);
    this.setTheme(theme);
    this.bindPointer();
    this.bindKeys();
    const ro = new ResizeObserver(() => {
      const b = canvas.getBoundingClientRect();
      if (b.width && b.height) view.resize(b.width, b.height, devicePixelRatio);
    });
    ro.observe(canvas);
    this.cleanup.push(() => ro.disconnect());
    const loop = (now: number) => {
      this.stepLayout();
      view.frame(now);
      this.raf = requestAnimationFrame(loop);
    };
    this.raf = requestAnimationFrame(loop);
  }

  /** Another view (graph) on the same engine: no new GPU setup; the camera eases to frame it. */
  setGraph(g: Graph, theme: MapTheme) {
    if (this.disposed) return;
    const pos = new Float32Array(g.nodes.flatMap((n) => [n[7], n[8], n[9]]));
    this.view.setGraph(pos, performance.now());
    this.reset(g, pos);
    this.setTheme(theme);
  }

  // Everything that belongs to one graph, back to its defaults.
  private reset(g: Graph, positions: Float32Array) {
    this.layout?.dispose();
    this.layout = null;
    this.layoutSteps = LAYOUT_STEPS;
    this.g = g;
    this.N = g.nodes.length;
    this.positions = positions;
    this.sizes = new Float32Array(this.N).fill(POINT_SIZE);
    this.colorOn = g.langs.map(() => false);
    this.groupOn = g.langs.map(() => true);
    this.typeOn = g.types.map(() => true);
    this.hideAlone = false;
    this.available = null;
    this.selected = -1;
    this.sims = new Map();
    this.simRange = g.sim ? [g.sim.min, g.sim.max] : [0, 1];
    const c = [0, 1, 2].map((k) => median(g.nodes.map((n) => n[7 + k] as number)));
    const d = g.nodes.map((n) => Math.hypot(n[7] - c[0]!, n[8] - c[1]!, n[9] - c[2]!)).sort((x, y) => x - y);
    this.radius = d[Math.floor(d.length * 0.9)] || 1;
    this.rebuildAdj();
    this.view.setHighlight(new Float32Array(this.N), false);
  }

  // ---------- Public setters ----------
  setTheme(theme: MapTheme) {
    if (this.disposed) return;
    this.theme = theme;
    const [r, g, b] = hex(theme.background);
    this.view.setStyle(r, g, b, theme.light);
    this.pushPoints();
    this.pushSegments();
  }

  setColored(on: boolean[]) {
    if (this.disposed) return;
    this.colorOn = on;
    this.pushPoints();
  }

  setVisible(on: boolean[]) {
    if (this.disposed) return;
    this.groupOn = on;
    this.applyVisibility();
  }

  setTypes(on: boolean[]) {
    if (this.disposed) return;
    this.typeOn = on;
    this.select(this.selected, false);
  }

  setAvailable(mask: boolean[] | null) {
    if (this.disposed) return;
    this.available = mask;
    this.applyVisibility();
  }

  setHideAlone(hide: boolean) {
    if (this.disposed) return;
    this.hideAlone = hide;
    this.applyVisibility();
    this.frameVisible();
  }

  setLinks(edges: number[], sims: Map<number, number>, relayout: boolean) {
    if (this.disposed) return;
    this.g.edges = edges;
    this.sims = sims;
    this.rebuildAdj();
    this.applyVisibility();
    if (relayout) this.startLayout();
  }

  simOf(a: number, b: number) {
    return this.sims.get(a < b ? a * this.N + b : b * this.N + a);
  }

  select(i: number, fly = true) {
    if (this.disposed) return;
    this.selected = i;
    const hl = new Float32Array(this.N);
    if (i >= 0) {
      hl[i] = 2;
      for (const [j, t] of this.adj[i]!) if (this.typeOn[t] && this.shown(j)) hl[j] = 1;
      if (fly) this.flyTo(i);
    }
    this.view.setHighlight(hl, i >= 0);
    this.pushSegments();
  }

  /** Re-centre the camera on the visible points (median centre, 90th-percentile radius). */
  frameVisible() {
    if (this.disposed) return;
    const vis = [...Array(this.N).keys()].filter((i) => this.sizes[i]! > 0);
    if (!vis.length) return;
    const p = this.positions;
    const c = [0, 1, 2].map((k) => median(vis.map((i) => p[i * 3 + k]!)));
    const d = vis.map((i) => Math.hypot(p[i * 3]! - c[0]!, p[i * 3 + 1]! - c[1]!, p[i * 3 + 2]! - c[2]!)).sort((x, y) => x - y);
    const r = Math.max(d[Math.floor(d.length * 0.9)] || 1, 50);
    this.view.flyToPosition(c[0]!, c[1]!, c[2]!, r * 2, performance.now());
  }

  /** The graphics API the engine got: "BrowserWebGpu" or "Gl" (WebGL2). */
  get backend(): string {
    return this.view.backend;
  }

  dispose() {
    if (this.disposed) return;
    this.disposed = true;
    cancelAnimationFrame(this.raf);
    this.cleanup.forEach((f) => f());
    this.layout?.dispose();
    this.view.free();
  }

  // ---------- What to draw ----------
  private shown(i: number) {
    return !!this.groupOn[this.g.nodes[i]![5]] && (!this.available || !!this.available[i]);
  }

  private rebuildAdj() {
    const e = this.g.edges;
    this.adj = Array.from({ length: this.N }, () => []);
    for (let i = 0; i < e.length; i += 3) {
      this.adj[e[i]!]!.push([e[i + 1]!, e[i + 2]!]);
      this.adj[e[i + 1]!]!.push([e[i]!, e[i + 2]!]);
    }
  }

  private applyVisibility() {
    for (let i = 0; i < this.N; i++) {
      const on = this.shown(i) && !(this.hideAlone && this.adj[i]!.length === 0);
      this.sizes[i] = on ? POINT_SIZE : 0;
    }
    this.pushPoints();
    this.select(this.selected, false);
  }

  // Points start gold; once a collection is coloured, the gold ones fade towards the background.
  private pushPoints() {
    const gold = linear(this.theme.gold);
    const base = this.colorOn.some(Boolean) ? mix(gold, linear(this.theme.background), 0.7) : gold;
    const groups = this.theme.groups.map(linear);
    const colors = new Float32Array(this.N * 3);
    this.g.nodes.forEach((n, i) => colors.set(this.colorOn[n[5]] ? groups[n[5]]! : base, i * 3));
    this.view.setPoints(colors, this.sizes);
  }

  private typeStyle(t: number) {
    const type = this.g.types[t]!;
    const style = TYPE_STYLE[type] ?? TYPE_STYLE.full!;
    return { color: this.theme.types[type] ?? style.color, opacity: style.opacity * (this.theme.light ? 2.2 : 1) };
  }

  // Every shown link in gold (dimmed while a text is selected), then the selected text's links
  // in their type's colour.
  private pushSegments() {
    const e = this.g.edges;
    const ends: number[] = [];
    const cols: number[] = [];
    const gold = hex(this.theme.gold);
    const dim = this.selected >= 0 ? 0.25 : 1;
    for (let k = 0; k < e.length; k += 3) {
      const [a, b, t] = [e[k]!, e[k + 1]!, e[k + 2]!];
      if (!this.typeOn[t] || !this.shown(a) || !this.shown(b)) continue;
      ends.push(a, b);
      cols.push(...gold, Math.min(this.typeStyle(t).opacity * LINKS * dim, 1));
    }
    const i = this.selected;
    for (const [j, t] of i >= 0 ? this.adj[i]! : []) {
      if (!this.typeOn[t] || !this.shown(j)) continue;
      ends.push(i, j);
      cols.push(...hex(this.typeStyle(t).color), 0.85);
    }
    this.view.setSegments(new Uint32Array(ends), new Float32Array(cols));
  }

  // Frame the text together with its neighbours.
  private flyTo(i: number) {
    const p = this.positions;
    let r = 60;
    for (const [j] of this.adj[i]!) r = Math.max(r, Math.hypot(p[i * 3]! - p[j * 3]!, p[i * 3 + 1]! - p[j * 3 + 1]!, p[i * 3 + 2]! - p[j * 3 + 2]!));
    this.view.flyTo(i, Math.min(r * 2.2, this.radius * 1.5), performance.now());
  }

  // ---------- Input ----------
  private at(e: PointerEvent | WheelEvent) {
    const b = this.canvas.getBoundingClientRect();
    return [e.clientX - b.left, e.clientY - b.top] as const;
  }

  private listen<K extends keyof WindowEventMap>(target: EventTarget, type: K, f: (e: WindowEventMap[K]) => void, opts?: AddEventListenerOptions) {
    target.addEventListener(type, f as EventListener, opts);
    this.cleanup.push(() => target.removeEventListener(type, f as EventListener, opts));
  }

  // One pointer rotates (a click picks); two pinch to zoom. Hover shows the tooltip (not on touch).
  private bindPointer() {
    const c = this.canvas;
    const down = new Map<number, { x: number; y: number }>();
    let start = { x: 0, y: 0 };
    let pinch = 0;
    const spread = () => { const [a, b] = [...down.values()]; return a && b ? Math.hypot(a.x - b.x, a.y - b.y) : 0; };
    this.listen(c, 'pointerdown', (e) => {
      c.setPointerCapture(e.pointerId);
      down.set(e.pointerId, { x: e.clientX, y: e.clientY });
      start = { x: e.clientX, y: e.clientY };
      this.events.onHover(-1, 0, 0);
      if (down.size === 1) this.view.pointerDown(...this.at(e));
      else { this.view.pointerUp(); pinch = spread(); }
    });
    this.listen(c, 'pointermove', (e) => {
      if (down.has(e.pointerId)) down.set(e.pointerId, { x: e.clientX, y: e.clientY });
      if (down.size >= 2) {
        const s = spread();
        if (pinch && s) this.view.wheel((100 * Math.log(pinch / s)) / 0.0513); // wheel units, see camera.rs
        pinch = s;
        return;
      }
      this.view.pointerMove(...this.at(e));
      if (e.pointerType === 'touch' || down.size) return;
      const i = this.view.pick(...this.at(e));
      c.style.cursor = i >= 0 ? 'pointer' : '';
      this.events.onHover(i, e.clientX, e.clientY);
    });
    const up = (e: PointerEvent) => {
      const was = down.size;
      down.delete(e.pointerId);
      this.view.pointerUp();
      if (was === 1 && e.type === 'pointerup' && Math.hypot(e.clientX - start.x, e.clientY - start.y) <= 4) {
        this.events.onSelect(this.view.pick(...this.at(e)));
      }
    };
    this.listen(c, 'pointerup', up);
    this.listen(c, 'pointercancel', up);
    this.listen(c, 'pointerleave', () => this.events.onHover(-1, 0, 0));
    this.listen(c, 'wheel', (e) => { e.preventDefault(); this.view.wheel(e.deltaY); }, { passive: false });
  }

  // Arrow keys travel: left/right slide, up/down go forward or back, with Shift rise or sink.
  // Not while typing (the search box).
  private bindKeys() {
    const held = new Set<string>();
    let shift = false;
    const k = (key: string) => +held.has(key);
    const push = () => {
      const ud = k('ArrowUp') - k('ArrowDown');
      this.view.setKeys(k('ArrowRight') - k('ArrowLeft'), shift ? ud : 0, shift ? 0 : ud);
    };
    const typing = () => {
      const a = document.activeElement as HTMLElement | null;
      return !!a && (a.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(a.tagName));
    };
    this.listen(window, 'keydown', (e) => {
      shift = e.shiftKey;
      if (!e.key.startsWith('Arrow') || typing()) return push();
      e.preventDefault();
      held.add(e.key);
      push();
    });
    this.listen(window, 'keyup', (e) => { shift = e.shiftKey; held.delete(e.key); push(); });
    this.listen(window, 'blur', () => { held.clear(); push(); });
  }

  // ---------- Live layout (vectorial space) ----------
  private startLayout() {
    this.layout?.dispose();
    const graph = createGraph();
    for (let i = 0; i < this.N; i++) graph.addNode(i);
    const e = this.g.edges;
    for (let i = 0; i < e.length; i += 3) graph.addLink(e[i]!, e[i + 1]!, { s: this.simOf(e[i]!, e[i + 1]!) });
    const [lo, hi] = this.simRange;
    this.layout = createLayout(graph, {
      dimensions: 3, springLength: 30, springCoefficient: 0.0008, gravity: -2.5, theta: 0.9, dragCoefficient: 0.04, timeStep: 20,
      springTransform: (link: { data: { s: number } }, spring: { length: number; weight: number }) => {
        const t = Math.min(Math.max((link.data.s - lo) / (hi - lo), 0), 1);
        spring.length = 17.5 + (1 - t) * 30;
        spring.weight = 0.6 + 0.4 * t;
      },
    } as Parameters<typeof createLayout>[1]);
    const p = this.positions;
    for (let i = 0; i < this.N; i++) this.layout.setNodePosition(i, p[i * 3]!, p[i * 3 + 1]!, p[i * 3 + 2]!);
    this.layoutSteps = 0;
  }

  private stepLayout() {
    if (!this.layout || this.layoutSteps >= LAYOUT_STEPS) return;
    const t0 = performance.now();
    while (performance.now() - t0 < 14 && this.layoutSteps < LAYOUT_STEPS) {
      this.layout.step();
      this.layoutSteps++;
      for (let i = 0; i < this.N; i++) {
        const q = this.layout.getBody(i)!.pos as { x: number; y: number; z: number };
        q.x *= 0.99; q.y *= 0.99; q.z *= 0.99;
      }
    }
    for (let i = 0; i < this.N; i++) {
      const q = this.layout.getBody(i)!.pos as { x: number; y: number; z: number };
      this.positions[i * 3] = q.x; this.positions[i * 3 + 1] = q.y; this.positions[i * 3 + 2] = q.z;
    }
    this.view.setPositions(this.positions);
    if (this.layoutSteps >= LAYOUT_STEPS) {
      this.frameVisible();
      document.body.dataset.layout = 'done';
    }
  }
}
