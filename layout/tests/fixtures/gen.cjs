// Generates small.json: ngraph.forcelayout 3.3.1's own trajectory on a small two-cluster graph,
// with the map's parameters and similarity springs, the 0.99 pull to the origin after each step.
// The Rust port (tests/ngraph_parity.rs) is compared against it.
//
//   NGRAPH=/path/to/node_modules node tests/fixtures/gen.cjs > tests/fixtures/small.json
//
// ngraph and Node leave the repository with task 008; the committed small.json stays.
const root = process.env.NGRAPH || `${__dirname}/../../../node_modules`;
const createGraph = require(`${root}/ngraph.graph`);
const createLayout = require(`${root}/ngraph.forcelayout`);

const lcg = (seed) => () => (seed = (seed * 16807) % 2147483647) / 2147483647;
const N = 300;
const SNAPSHOTS = [1, 5, 20, 400];

function build(r) {
  const links = [];
  const seen = new Set();
  for (let c = 0; c < 2; c++) {
    for (let k = 0; k < 400; k++) {
      const a = c * 150 + Math.floor(r() * 150);
      const b = c * 150 + Math.floor(r() * 150);
      if (a === b || seen.has(`${a},${b}`) || seen.has(`${b},${a}`)) continue;
      seen.add(`${a},${b}`);
      const s = r(); // a similarity in [0, 1]
      links.push([a, b, 17.5 + (1 - s) * 30]);
    }
  }
  return links;
}

function run(links, start, withWeight) {
  const g = createGraph();
  for (let i = 0; i < N; i++) g.addNode(i);
  links.forEach(([a, b, length], i) => g.addLink(a, b, { length, i }));
  const layout = createLayout(g, {
    dimensions: 3, springLength: 30, springCoefficient: 0.0008, gravity: -2.5, theta: 0.9, dragCoefficient: 0.04, timeStep: 20,
    springTransform: (link, spring) => {
      spring.length = link.data.length;
      if (withWeight) spring.weight = 0.6 + 0.4 * Math.random(); // set by the callers, never read by ngraph
    },
  });
  start.forEach(([x, y, z], i) => layout.setNodePosition(i, x, y, z));
  const out = {};
  for (let step = 1; step <= SNAPSHOTS[SNAPSHOTS.length - 1]; step++) {
    layout.step();
    g.forEachNode((n) => { const q = layout.getBody(n.id).pos; q.x *= 0.99; q.y *= 0.99; q.z *= 0.99; });
    if (SNAPSHOTS.includes(step)) out[step] = [...Array(N).keys()].map((i) => { const q = layout.getBody(i).pos; return [q.x, q.y, q.z]; });
  }
  return out;
}

const r = lcg(7);
const links = build(r);
const p = lcg(11);
const start = [...Array(N).keys()].map((i) => [(p() - 0.5) * 100 + (i < 150 ? -60 : 60), (p() - 0.5) * 100, (p() - 0.5) * 100]);
const plain = run(links, start, false);
const weighted = run(links, start, true);
console.error('weight ignored by ngraph:', JSON.stringify(plain) === JSON.stringify(weighted));
console.log(JSON.stringify({
  note: 'ngraph.forcelayout 3.3.1, map parameters, 0.99 pull after each step; see gen.cjs',
  params: { springLength: 30, springCoefficient: 0.0008, gravity: -2.5, theta: 0.9, dragCoefficient: 0.04, timeStep: 20, centring: 0.99 },
  start, links, snapshots: plain,
}));
