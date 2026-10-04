// The 3D map: Three.js scene, camera, picking and (vectorial space) live force layout.
// No UI here: React drives it through the setters and hears back through callbacks.
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import createGraph from 'ngraph.graph';
import createLayout, { type Layout } from 'ngraph.forcelayout';
import { TYPE_STYLE, type Graph } from './data';
import type { MapTheme } from './theme';

export interface EngineEvents {
  onSelect: (i: number) => void; // -1: nothing
  onHover: (i: number, x: number, y: number) => void;
}

const POINT_SIZE = 7; // every point the same size
const VIEW = { size: 1.6, bright: 1.3, links: 1.5 };
const LAYOUT_STEPS = 400;
const median = (xs: number[]) => xs.sort((x, y) => x - y)[Math.floor(xs.length / 2)]!;
const ease = (t: number) => 1 - Math.pow(1 - t, 3);

export class MapEngine {
  readonly N: number;
  adj: [number, number][][] = [];
  private sims = new Map<number, number>();
  private colorOn: boolean[];
  private groupOn: boolean[];
  private typeOn: boolean[];
  private hideAlone = false;
  private available: boolean[] | null = null; // null: every text (English)
  private selected = -1;

  private renderer: THREE.WebGLRenderer;
  private scene = new THREE.Scene();
  private camera: THREE.PerspectiveCamera;
  private controls: OrbitControls;
  private positions: Float32Array;
  private colors: Float32Array;
  private sizes: Float32Array;
  private highlight: Float32Array;
  private groupColor: THREE.Color[];
  // Theme: points start gold; once a group is coloured, the gold ones fade towards the
  // background so the colours stand out. Light themes draw links normally (additive
  // blending would vanish on a light page).
  private theme: MapTheme;
  private baseColor = new THREE.Color();
  private baseDim = new THREE.Color();
  private nodeGeom = new THREE.BufferGeometry();
  private nodeMat: THREE.ShaderMaterial;
  private points: THREE.Points;
  private edgeObjs: THREE.LineSegments[] = [];
  private edgeEnds: Int32Array[] = [];
  private selLines: THREE.LineSegments | null = null;
  private sphere: { center: THREE.Vector3; radius: number };
  private flight: { from: THREE.Vector3; to: THREE.Vector3; camFrom: THREE.Vector3; camTo: THREE.Vector3; t0: number } | null = null;
  private layout: Layout<ReturnType<typeof createGraph>> | null = null;
  private layoutSteps = LAYOUT_STEPS;
  private simRange: [number, number] = [0, 1];
  private raycaster = new THREE.Raycaster();
  private resizeObserver: ResizeObserver;
  private cleanup: (() => void)[] = [];

  constructor(private canvas: HTMLCanvasElement, private g: Graph, theme: MapTheme, private events: EngineEvents) {
    this.N = g.nodes.length;
    const N = this.N;
    this.colorOn = g.langs.map(() => false);
    this.groupOn = g.langs.map(() => true);
    this.typeOn = g.types.map(() => true);
    this.theme = theme;
    this.groupColor = [];
    this.applyThemeColors();
    if (g.sim) this.simRange = [g.sim.min, g.sim.max];

    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
    this.renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
    const { width, height } = this.box();
    this.renderer.setSize(width, height, false);
    this.camera = new THREE.PerspectiveCamera(55, width / height, 1, 30000);
    this.controls = new OrbitControls(this.camera, canvas);
    this.controls.enableDamping = true;
    this.controls.autoRotate = true;
    this.controls.autoRotateSpeed = 0.25;
    this.controls.addEventListener('start', () => (this.controls.autoRotate = false));

    // Positions, and a robust framing: median centre, 90th-percentile radius.
    this.positions = new Float32Array(N * 3);
    g.nodes.forEach((n, i) => this.positions.set([n[7], n[8], n[9]], i * 3));
    const center = new THREE.Vector3(...[7, 8, 9].map((k) => median(g.nodes.map((n) => n[k] as number))));
    const dists = g.nodes.map((n) => center.distanceTo(new THREE.Vector3(n[7], n[8], n[9]))).sort((x, y) => x - y);
    this.sphere = { center, radius: dists[Math.floor(dists.length * 0.9)] || 1 };
    this.controls.target.copy(center);
    this.camera.position.copy(center).add(new THREE.Vector3(0, this.sphere.radius * 0.3, this.sphere.radius * 1.6));

    // Nodes: one Points draw call, custom shader.
    this.colors = new Float32Array(N * 3);
    this.sizes = new Float32Array(N).fill(POINT_SIZE);
    this.highlight = new Float32Array(N);
    this.fillColors();
    this.nodeGeom.setAttribute('position', new THREE.BufferAttribute(this.positions, 3));
    this.nodeGeom.setAttribute('color', new THREE.BufferAttribute(this.colors, 3));
    this.nodeGeom.setAttribute('size', new THREE.BufferAttribute(this.sizes, 1));
    this.nodeGeom.setAttribute('hl', new THREE.BufferAttribute(this.highlight, 1));
    const pr = this.renderer.getPixelRatio();
    this.nodeMat = new THREE.ShaderMaterial({
      uniforms: {
        uDim: { value: 0 },
        uScale: { value: (height / 2) * pr },
        uMinSize: { value: 2 * pr },
        uSize: { value: VIEW.size },
        uBright: { value: theme.light ? 1 : VIEW.bright },
        uFlat: { value: theme.light ? 1 : 0 }, // light themes: plain discs, no glowing core
      },
      vertexShader: /* glsl */ `
        attribute float size;
        attribute vec3 color;
        attribute float hl;
        uniform float uDim;
        uniform float uScale;
        uniform float uMinSize;
        uniform float uSize;
        varying vec3 vColor;
        varying float vAlpha;
        void main() {
          vec4 mv = modelViewMatrix * vec4(position, 1.0);
          float boost = hl > 1.5 ? 2.2 : 1.0;
          gl_PointSize = max(size * uSize * boost * uScale / -mv.z, uMinSize * uSize);
          vColor = hl > 1.5 ? vec3(1.0) * 0.4 + color * 0.8 : color;
          vAlpha = (uDim > 0.5 && hl < 0.5) ? 0.12 : 1.0;
          gl_Position = projectionMatrix * mv;
          // Hidden nodes: GPUs clamp gl_PointSize to >= 1px, so clip them out instead.
          if (size <= 0.0) gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
        }`,
      fragmentShader: /* glsl */ `
        uniform float uBright;
        uniform float uFlat;
        varying vec3 vColor;
        varying float vAlpha;
        void main() {
          float d = length(gl_PointCoord - 0.5);
          if (d > 0.5) discard;
          float core = smoothstep(0.5, 0.0, d);
          vec3 glow = vColor * (0.55 + core * 0.9) * uBright;
          gl_FragColor = vec4(mix(glow, vColor, uFlat), vAlpha * smoothstep(0.5, 0.4 + 0.05 * uFlat, d));
        }`,
      transparent: true,
      depthWrite: false,
    });
    this.points = new THREE.Points(this.nodeGeom, this.nodeMat);
    this.scene.add(this.points);

    this.rebuildAdj();
    this.buildEdges();
    this.bindPointer();
    this.resizeObserver = new ResizeObserver(() => this.resize());
    this.resizeObserver.observe(canvas);
    this.renderer.setAnimationLoop((now) => this.frame(now));
  }

  // ---------- Public setters ----------
  setTheme(theme: MapTheme) {
    this.theme = theme;
    this.applyThemeColors();
    this.nodeMat.uniforms.uBright!.value = theme.light ? 1 : VIEW.bright;
    this.nodeMat.uniforms.uFlat!.value = theme.light ? 1 : 0;
    this.fillColors();
    this.nodeGeom.attributes.color!.needsUpdate = true;
    this.buildEdges();
    if (this.selected >= 0) this.drawSelLines(this.selected);
  }

  setColored(on: boolean[]) {
    this.colorOn = on;
    this.fillColors();
    this.nodeGeom.attributes.color!.needsUpdate = true;
  }

  setVisible(on: boolean[]) {
    this.groupOn = on;
    this.applyNodeVisibility();
    this.buildEdges();
    if (this.selected >= 0) this.select(this.selected, false);
  }

  setTypes(on: boolean[]) {
    this.typeOn = on;
    this.edgeObjs.forEach((o, t) => (o.visible = !!on[t]));
    if (this.selected >= 0) this.select(this.selected, false);
  }

  // Only these texts are shown (e.g. those translated into the interface language).
  setAvailable(mask: boolean[] | null) {
    this.available = mask;
    this.applyNodeVisibility();
    this.buildEdges();
    if (this.selected >= 0) this.select(this.selected, false);
  }

  private shown(i: number) {
    return !!this.groupOn[this.g.nodes[i]![5]] && (!this.available || !!this.available[i]);
  }

  setHideAlone(hide: boolean) {
    this.hideAlone = hide;
    this.applyNodeVisibility();
    this.frameVisible();
  }

  // Vectorial space: new links for a new threshold; relayout re-runs the force layout live.
  setLinks(edges: number[], sims: Map<number, number>, relayout: boolean) {
    this.g.edges = edges;
    this.sims = sims;
    this.rebuildAdj();
    this.applyNodeVisibility();
    this.buildEdges();
    if (this.selected >= 0) this.select(this.selected, false);
    if (relayout) this.startLayout();
  }

  simOf(a: number, b: number) {
    return this.sims.get(a < b ? a * this.N + b : b * this.N + a);
  }

  select(i: number, fly = true) {
    this.selected = i;
    this.highlight.fill(0);
    this.clearSelLines();
    if (i >= 0) {
      this.highlight[i] = 2;
      this.drawSelLines(i);
      if (fly) this.flyTo(i);
    }
    this.nodeGeom.attributes.hl!.needsUpdate = true;
    this.applyDim();
  }

  // Re-centre the camera on the visible nodes (median centre, 90th-percentile radius).
  frameVisible() {
    const vis = [...Array(this.N).keys()].filter((i) => this.sizes[i]! > 0);
    if (!vis.length) return;
    const p = this.positions;
    const c = new THREE.Vector3(...[0, 1, 2].map((k) => median(vis.map((i) => p[i * 3 + k]!))));
    const d = vis.map((i) => c.distanceTo(new THREE.Vector3(p[i * 3], p[i * 3 + 1], p[i * 3 + 2]))).sort((x, y) => x - y);
    const r = Math.max(d[Math.floor(d.length * 0.9)] || 1, 50);
    const dir = this.camera.position.clone().sub(this.controls.target).normalize();
    this.flight = { from: this.controls.target.clone(), to: c, camFrom: this.camera.position.clone(), camTo: c.clone().add(dir.multiplyScalar(r * 2)), t0: performance.now() };
  }

  dispose() {
    this.renderer.setAnimationLoop(null);
    this.resizeObserver.disconnect();
    this.cleanup.forEach((f) => f());
    this.layout?.dispose();
    this.controls.dispose();
    this.scene.traverse((o) => {
      if (o instanceof THREE.Points || o instanceof THREE.LineSegments) { o.geometry.dispose(); (o.material as THREE.Material).dispose(); }
    });
    this.renderer.dispose();
  }

  // ---------- Internals ----------
  private box() {
    return this.canvas.getBoundingClientRect();
  }

  private resize() {
    const { width, height } = this.box();
    if (!width || !height) return;
    this.camera.aspect = width / height;
    this.camera.updateProjectionMatrix();
    this.renderer.setSize(width, height, false);
    this.nodeMat.uniforms.uScale!.value = (height / 2) * this.renderer.getPixelRatio();
  }

  private applyThemeColors() {
    const bg = new THREE.Color(this.theme.background);
    this.scene.background = bg;
    this.baseColor.set(this.theme.gold);
    this.baseDim.copy(this.baseColor).lerp(bg, 0.7);
    this.groupColor = this.theme.groups.map((c) => new THREE.Color(c));
  }

  private typeStyle(t: number) {
    const type = this.g.types[t]!;
    const style = TYPE_STYLE[type] ?? TYPE_STYLE.full!;
    // Normal blending on light themes: lines need more opacity to read.
    return { color: this.theme.types[type] ?? style.color, opacity: style.opacity * (this.theme.light ? 2.2 : 1) };
  }

  private rebuildAdj() {
    const e = this.g.edges;
    this.adj = Array.from({ length: this.N }, () => []);
    for (let i = 0; i < e.length; i += 3) {
      this.adj[e[i]!]!.push([e[i + 1]!, e[i + 2]!]);
      this.adj[e[i + 1]!]!.push([e[i]!, e[i + 2]!]);
    }
  }

  private fillColors() {
    const base = this.colorOn.some(Boolean) ? this.baseDim : this.baseColor;
    this.g.nodes.forEach((n, i) => (this.colorOn[n[5]] ? this.groupColor[n[5]]! : base).toArray(this.colors, i * 3));
  }

  private applyNodeVisibility() {
    for (let i = 0; i < this.N; i++) {
      const shown = this.shown(i) && !(this.hideAlone && this.adj[i]!.length === 0);
      this.sizes[i] = shown ? POINT_SIZE : 0;
    }
    this.nodeGeom.attributes.size!.needsUpdate = true;
  }

  private buildEdges() {
    this.edgeObjs.forEach((o) => { this.scene.remove(o); o.geometry.dispose(); (o.material as THREE.Material).dispose(); });
    this.edgeObjs = [];
    const e = this.g.edges;
    this.g.types.forEach((_, t) => {
      const ends: number[] = [];
      for (let i = 0; i < e.length; i += 3) {
        if (e[i + 2] !== t) continue;
        const a = e[i]!, b = e[i + 1]!;
        if (!this.shown(a) || !this.shown(b)) continue;
        ends.push(a, b);
      }
      // Links stay out of the colour code: always gold.
      const col = new Float32Array(ends.length * 3);
      for (let v = 0; v < ends.length; v++) this.baseColor.toArray(col, v * 3);
      const geom = new THREE.BufferGeometry();
      geom.setAttribute('position', new THREE.BufferAttribute(new Float32Array(ends.length * 3), 3));
      geom.setAttribute('color', new THREE.BufferAttribute(col, 3));
      this.edgeEnds[t] = Int32Array.from(ends);
      const obj = new THREE.LineSegments(geom, new THREE.LineBasicMaterial({
        vertexColors: true, transparent: true, opacity: this.typeStyle(t).opacity,
        blending: this.theme.light ? THREE.NormalBlending : THREE.AdditiveBlending, depthWrite: false,
      }));
      obj.visible = !!this.typeOn[t];
      obj.renderOrder = -1;
      this.edgeObjs[t] = obj;
      this.scene.add(obj);
    });
    this.refreshEdgePositions();
    this.applyDim();
  }

  private refreshEdgePositions() {
    const p = this.positions;
    this.edgeObjs.forEach((o, t) => {
      const attr = o.geometry.attributes.position as THREE.BufferAttribute;
      const arr = attr.array as Float32Array;
      this.edgeEnds[t]!.forEach((n, v) => { arr[v * 3] = p[n * 3]!; arr[v * 3 + 1] = p[n * 3 + 1]!; arr[v * 3 + 2] = p[n * 3 + 2]!; });
      attr.needsUpdate = true;
      o.geometry.computeBoundingSphere();
    });
  }

  private applyDim() {
    const dim = this.selected >= 0;
    this.nodeMat.uniforms.uDim!.value = dim ? 1 : 0;
    this.edgeObjs.forEach((o, t) => ((o.material as THREE.LineBasicMaterial).opacity = Math.min(this.typeStyle(t).opacity * VIEW.links * (dim ? 0.25 : 1), 1)));
  }

  private clearSelLines() {
    if (!this.selLines) return;
    this.scene.remove(this.selLines);
    this.selLines.geometry.dispose();
    (this.selLines.material as THREE.Material).dispose();
    this.selLines = null;
  }

  private drawSelLines(i: number) {
    this.clearSelLines();
    const p = this.positions;
    const pos: number[] = [];
    const col: number[] = [];
    for (const [j, t] of this.adj[i]!) {
      if (!this.typeOn[t] || !this.shown(j)) continue;
      this.highlight[j] = 1;
      const c = new THREE.Color(this.typeStyle(t).color);
      pos.push(p[i * 3]!, p[i * 3 + 1]!, p[i * 3 + 2]!, p[j * 3]!, p[j * 3 + 1]!, p[j * 3 + 2]!);
      col.push(c.r, c.g, c.b, c.r, c.g, c.b);
    }
    const geom = new THREE.BufferGeometry();
    geom.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3));
    geom.setAttribute('color', new THREE.Float32BufferAttribute(col, 3));
    this.selLines = new THREE.LineSegments(geom, new THREE.LineBasicMaterial({ vertexColors: true, transparent: true, opacity: 0.85, depthWrite: false }));
    this.scene.add(this.selLines);
  }

  private flyTo(i: number) {
    const p = this.positions;
    const to = new THREE.Vector3(p[i * 3], p[i * 3 + 1], p[i * 3 + 2]);
    // Frame the node together with its neighbours.
    let r = 60;
    for (const [j] of this.adj[i]!) r = Math.max(r, to.distanceTo(new THREE.Vector3(p[j * 3], p[j * 3 + 1], p[j * 3 + 2])));
    const dir = this.camera.position.clone().sub(this.controls.target).normalize();
    const camTo = to.clone().add(dir.multiplyScalar(Math.min(r * 2.2, this.sphere.radius * 1.5)));
    this.flight = { from: this.controls.target.clone(), to, camFrom: this.camera.position.clone(), camTo, t0: performance.now() };
    this.controls.autoRotate = false;
  }

  private pickAt(x: number, y: number) {
    const r = this.box();
    const mouse = new THREE.Vector2(((x - r.left) / r.width) * 2 - 1, -((y - r.top) / r.height) * 2 + 1);
    this.raycaster.setFromCamera(mouse, this.camera);
    this.raycaster.params.Points!.threshold = this.camera.position.distanceTo(this.controls.target) * 0.008;
    const hit = this.raycaster.intersectObject(this.points).find((h) => this.sizes[h.index!]! > 0);
    return hit ? hit.index! : -1;
  }

  private bindPointer() {
    const c = this.canvas;
    let downAt = { x: 0, y: 0 };
    const move = (e: PointerEvent) => {
      if (e.pointerType === 'touch') return; // no hover on touch screens: the tooltip would stick
      const i = this.pickAt(e.clientX, e.clientY);
      c.style.cursor = i >= 0 ? 'pointer' : '';
      this.events.onHover(i, e.clientX, e.clientY);
    };
    const down = (e: PointerEvent) => {
      downAt = { x: e.clientX, y: e.clientY };
      this.events.onHover(-1, 0, 0);
    };
    const up = (e: PointerEvent) => {
      if (Math.hypot(e.clientX - downAt.x, e.clientY - downAt.y) > 4) return; // was a drag
      this.events.onSelect(this.pickAt(e.clientX, e.clientY));
    };
    const leave = () => this.events.onHover(-1, 0, 0);
    c.addEventListener('pointermove', move);
    c.addEventListener('pointerdown', down);
    c.addEventListener('pointerup', up);
    c.addEventListener('pointerleave', leave);
    this.cleanup.push(() => {
      c.removeEventListener('pointermove', move);
      c.removeEventListener('pointerdown', down);
      c.removeEventListener('pointerup', up);
      c.removeEventListener('pointerleave', leave);
    });
  }

  // Same engine and springs as scripts/build-similarity.mjs, a few steps per frame.
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
      // Pull everything gently towards the origin, every step, so separate groups stay
      // compact around the main mass instead of being pushed out by the repulsion.
      for (let i = 0; i < this.N; i++) {
        const q = this.layout.getBody(i)!.pos as { x: number; y: number; z: number };
        q.x *= 0.99; q.y *= 0.99; q.z *= 0.99;
      }
    }
    for (let i = 0; i < this.N; i++) {
      const q = this.layout.getBody(i)!.pos as { x: number; y: number; z: number };
      this.positions[i * 3] = q.x; this.positions[i * 3 + 1] = q.y; this.positions[i * 3 + 2] = q.z;
    }
    this.nodeGeom.attributes.position!.needsUpdate = true;
    this.nodeGeom.computeBoundingSphere();
    this.refreshEdgePositions();
    if (this.selected >= 0) this.drawSelLines(this.selected);
    if (this.layoutSteps >= LAYOUT_STEPS) {
      this.frameVisible(); // frame the result
      document.body.dataset.layout = 'done';
    }
  }

  private frame(now: number) {
    if (this.flight) {
      const t = Math.min((now - this.flight.t0) / 1100, 1);
      const k = ease(t);
      this.controls.target.lerpVectors(this.flight.from, this.flight.to, k);
      this.camera.position.lerpVectors(this.flight.camFrom, this.flight.camTo, k);
      if (t >= 1) this.flight = null;
    }
    this.stepLayout();
    this.controls.update();
    this.renderer.render(this.scene, this.camera);
  }
}
