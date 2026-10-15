//! What the engine is given (`src/map/rust-engine.ts`): the engine only draws, the page
//! decides what. A [`Scene`] holds what `RustMapEngine` held between calls (links, the
//! legend's and the link types' switches, the texts' language mask, the selection), and
//! returns the buffers `map_engine::GraphView` takes: `set_points(colors, sizes)`,
//! `set_segments(ends, colors)`, `set_highlight(hl, dim)`; plus the camera's targets
//! (`fly_to`, `fly_to_position`).

use std::cmp::Ordering;
use std::collections::HashMap;

use map_formats::pairs::range_of;

use crate::map::model::{Graph, Pairs, SimInfo};
use crate::map::theme::MapTheme;

/// A shown point's size (rust-engine.ts line 16, `POINT_SIZE`).
pub const POINT_SIZE: f32 = 7.0;

/// The links' opacity factor (rust-engine.ts line 17, `LINKS`).
pub const LINKS: f64 = 1.5;

/// Each link type's colour on the dark page and opacity (data.ts lines 33–40, `TYPE_STYLE`);
/// the first, `full`, is the style of a type it does not list.
const TYPE_STYLE: [(&str, &str, f64); 6] = [
    ("full", "#ffffff", 0.22),
    ("sections", "#c7d2fe", 0.16),
    ("resembling", "#94a3b8", 0.1),
    ("mentions", "#64748b", 0.08),
    ("retells", "#64748b", 0.08),
    ("similar", "#c9a84c", 0.1),
];

/// A buffer value: the engine's buffers are single precision, as the page's `Float32Array`s.
#[allow(clippy::cast_possible_truncation)]
fn single(x: f64) -> f32 {
    x as f32
}

/// A node index as the engine's `u32` (a graph has far fewer nodes).
fn point(i: usize) -> u32 {
    u32::try_from(i).unwrap_or(u32::MAX)
}

/// Each node's links: `(other node, link type)`, both directions (rust-engine.ts line 33).
pub type Adjacency = Vec<Vec<(usize, u32)>>;

/// The adjacency of `n` nodes from flat `[a, b, type, …]` links (rust-engine.ts lines
/// 222–229, `rebuildAdj`).
///
/// # Panics
/// When a link names a node at or past `n` (the data files are checked at boot, task 004).
pub fn adjacency(n: usize, edges: &[u32]) -> Adjacency {
    let mut adj = vec![Vec::new(); n];
    for &[a, b, t] in edges.as_chunks::<3>().0 {
        adj[a as usize].push((b as usize, t));
        adj[b as usize].push((a as usize, t));
    }
    adj
}

/// A vectorial view's links at a threshold: flat `[a, b, 0, …]` (type 0, `similar`) and each
/// link's similarity.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Links {
    /// `[a, b, 0, a, b, 0, …]`.
    pub edges: Vec<u32>,
    /// `(a, b)` with `a < b` → similarity.
    pub sims: HashMap<(u32, u32), f32>,
}

/// The links of `pairs` whose similarity is at least `threshold` (data.ts lines 63–72,
/// `linksAt`, over `Pairs::links_at`). The comparison is in double precision as the page's
/// (`pairs.sim[k] < th`, a `Float32Array` value against a JS number).
pub fn links_at(pairs: &Pairs, sim: &SimInfo, threshold: f64) -> Links {
    // `Pairs::links_at` compares in single precision: ask it for a threshold at or below
    // the page's, then keep what the page keeps.
    let mut below = single(threshold);
    if f64::from(below) > threshold {
        below = below.next_down();
    }
    let mut links = Links::default();
    for link in pairs.links_at(range_of(sim), below) {
        if f64::from(link.sim) < threshold {
            continue;
        }
        let (a, b) = (u32::from(link.a), u32::from(link.b));
        links.edges.extend([a, b, 0]);
        links.sims.insert((a, b), link.sim);
    }
    links
}

/// `#rrggbb` as r, g, b in 0–1 (rust-engine.ts lines 22–25, `hex`); an unparsable colour
/// is black (the page got `NaN`).
pub fn rgb(hex: &str) -> [f64; 3] {
    // `parseInt(h.replace('#', '').slice(0, 6), 16)`: the leading hex digits of the six.
    let digits: String = hex
        .replacen('#', "", 1)
        .chars()
        .take(6)
        .take_while(char::is_ascii_hexdigit)
        .collect();
    let n = u32::from_str_radix(&digits, 16).unwrap_or(0);
    [
        f64::from(n >> 16) / 255.0,
        f64::from((n >> 8) & 255) / 255.0,
        f64::from(n & 255) / 255.0,
    ]
}

/// `#rrggbb` decoded from sRGB to linear (rust-engine.ts lines 27–28): the points' colours.
pub fn linear(hex: &str) -> [f64; 3] {
    rgb(hex).map(|c| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    })
}

/// `a` moved `t` of the way to `b` (rust-engine.ts line 29, `mix`).
fn mix(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * t)
}

/// A link type's colour and opacity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TypeStyle {
    /// `#rrggbb`.
    pub color: &'static str,
    /// 0–1, before [`LINKS`].
    pub opacity: f64,
}

/// The style of link type `t` of `graph` on `theme` (rust-engine.ts lines 250–254,
/// `typeStyle`, over data.ts lines 33–40 `TYPE_STYLE`): the theme's colour for the type,
/// else the type's own (an unknown type is drawn as `full`); the opacity 2.2 times
/// stronger on a light page.
pub fn type_style(graph: &Graph, theme: &MapTheme, t: u32) -> TypeStyle {
    let name = graph.types.get(t as usize).map_or("", String::as_str);
    let (_, color, opacity) = TYPE_STYLE
        .iter()
        .find(|(type_name, ..)| *type_name == name)
        .unwrap_or(&TYPE_STYLE[0]);
    TypeStyle {
        color: theme.type_color(name).unwrap_or(color),
        opacity: opacity * if theme.light { 2.2 } else { 1.0 },
    }
}

/// The segments' buffers for `GraphView::set_segments`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Segments {
    /// Pairs of point indices.
    pub ends: Vec<u32>,
    /// r, g, b, a per segment.
    pub colors: Vec<f32>,
}

/// The highlight for `GraphView::set_highlight`.
#[derive(Clone, Debug, PartialEq)]
pub struct Highlight {
    /// Per point: 2 selected, 1 neighbour, 0 other.
    pub hl: Vec<f32>,
    /// Fade the others (a text is selected).
    pub dim: bool,
}

/// What the map draws of one graph, besides positions and theme: `RustMapEngine`'s state
/// (rust-engine.ts lines 31–49). Every method takes the graph it was made for.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    /// The links, flat `[a, b, type, …]`: the graph's, or a vectorial view's at its threshold.
    pub edges: Vec<u32>,
    /// A vectorial view's similarities, `(a, b)` with `a < b`.
    pub sims: HashMap<(u32, u32), f32>,
    /// [`adjacency`] of `edges`.
    pub adj: Adjacency,
    /// Per group: drawn in its own colour.
    pub colored: Vec<bool>,
    /// Per group: shown.
    pub group_on: Vec<bool>,
    /// Per link type: shown.
    pub type_on: Vec<bool>,
    /// Per node: in the texts' language (`None`: Pāli, every text).
    pub available: Option<Vec<bool>>,
    /// Hide the texts without a link (the vectorial view's switch).
    pub hide_alone: bool,
    /// The selected node.
    pub selected: Option<usize>,
}

impl Scene {
    /// A graph's scene with its defaults (rust-engine.ts lines 101–122, `reset`): its own
    /// links, nothing coloured, every group and link type shown, every text, nothing selected.
    pub fn new(graph: &Graph) -> Scene {
        Scene {
            edges: graph.edges.clone(),
            sims: HashMap::new(),
            adj: adjacency(graph.nodes.len(), &graph.edges),
            colored: vec![false; graph.langs.len()],
            group_on: vec![true; graph.langs.len()],
            type_on: vec![true; graph.types.len()],
            available: None,
            hide_alone: false,
            selected: None,
        }
    }

    /// New links (a vectorial view's threshold moved; rust-engine.ts lines 165–172,
    /// `setLinks`): the adjacency follows.
    pub fn set_links(&mut self, links: Links) {
        self.adj = adjacency(self.adj.len(), &links.edges);
        self.edges = links.edges;
        self.sims = links.sims;
    }

    /// The similarity of the link `a`–`b`, either order (rust-engine.ts lines 174–176).
    pub fn sim_of(&self, a: usize, b: usize) -> Option<f32> {
        let key = (u32::try_from(a.min(b)).ok()?, u32::try_from(a.max(b)).ok()?);
        self.sims.get(&key).copied()
    }

    /// Whether links of type `t` are shown (`!!this.typeOn[t]`).
    fn type_shown(&self, t: u32) -> bool {
        self.type_on.get(t as usize).copied().unwrap_or(false)
    }

    /// Whether the texts' language has node `i`.
    fn in_language(&self, i: usize) -> bool {
        self.available
            .as_ref()
            .is_none_or(|on| on.get(i).copied().unwrap_or(false))
    }

    /// Whether node `i`'s group is shown and the texts' language has it (rust-engine.ts lines
    /// 218–220, `shown`).
    pub fn shown(&self, graph: &Graph, i: usize) -> bool {
        let group = graph.nodes[i].group as usize;
        self.group_on.get(group).copied().unwrap_or(false) && self.in_language(i)
    }

    /// Each point's size: [`POINT_SIZE`] when shown, 0 when not or alone while alone texts
    /// are hidden (rust-engine.ts lines 231–235, `applyVisibility`).
    pub fn sizes(&self, graph: &Graph) -> Vec<f32> {
        (0..graph.nodes.len())
            .map(|i| {
                let alone = self.adj.get(i).is_none_or(Vec::is_empty);
                if self.shown(graph, i) && !(self.hide_alone && alone) {
                    POINT_SIZE
                } else {
                    0.0
                }
            })
            .collect()
    }

    /// Each point's linear r, g, b (rust-engine.ts lines 240–248, `pushPoints`): gold; once
    /// a collection is coloured, its texts take its colour and the others fade 70 % towards
    /// the background.
    pub fn point_colors(&self, graph: &Graph, theme: &MapTheme) -> Vec<f32> {
        let gold = linear(&theme.gold);
        let base = if self.colored.iter().any(|&on| on) {
            mix(gold, linear(&theme.background), 0.7)
        } else {
            gold
        };
        let groups: Vec<[f64; 3]> = theme.groups.iter().map(|hex| linear(hex)).collect();
        let mut colors = Vec::with_capacity(graph.nodes.len() * 3);
        for node in &graph.nodes {
            let group = node.group as usize;
            let color = if self.colored.get(group).copied().unwrap_or(false) {
                groups.get(group).copied().unwrap_or(base)
            } else {
                base
            };
            colors.extend(color.map(single));
        }
        colors
    }

    /// The segments (rust-engine.ts lines 256–277, `pushSegments`): every link of a shown
    /// type between two shown texts in gold, at its type's opacity (a quarter of it while a
    /// text is selected); then the selected text's links to shown texts in their type's
    /// colour. Colours are sRGB here, as the page gave them.
    pub fn segments(&self, graph: &Graph, theme: &MapTheme) -> Segments {
        let [r, g, b] = rgb(&theme.gold).map(single);
        let dim = if self.selected.is_some() { 0.25 } else { 1.0 };
        let mut segments = Segments::default();
        for &[a, b_end, t] in self.edges.as_chunks::<3>().0 {
            if !self.type_shown(t)
                || !self.shown(graph, a as usize)
                || !self.shown(graph, b_end as usize)
            {
                continue;
            }
            segments.ends.extend([a, b_end]);
            let alpha = (type_style(graph, theme, t).opacity * LINKS * dim).min(1.0);
            segments.colors.extend([r, g, b, single(alpha)]);
        }
        if let Some(i) = self.selected {
            for &(j, t) in self.adj.get(i).map_or(&[][..], Vec::as_slice) {
                if !self.type_shown(t) || !self.shown(graph, j) {
                    continue;
                }
                segments.ends.extend([point(i), point(j)]);
                let [r, g, b] = rgb(type_style(graph, theme, t).color).map(single);
                segments.colors.extend([r, g, b, 0.85]);
            }
        }
        segments
    }

    /// The highlight (rust-engine.ts lines 178–189, `select`): the selected text 2, its
    /// shown neighbours by a shown link type 1.
    pub fn highlight(&self, graph: &Graph) -> Highlight {
        let mut hl = vec![0.0; graph.nodes.len()];
        if let Some(i) = self.selected.filter(|&i| i < hl.len()) {
            hl[i] = 2.0;
            for &(j, t) in self.adj.get(i).map_or(&[][..], Vec::as_slice) {
                if self.type_shown(t) && self.shown(graph, j) {
                    hl[j] = 1.0;
                }
            }
        }
        Highlight {
            hl,
            dim: self.selected.is_some(),
        }
    }

    /// Node `i`'s neighbours the texts' language has (App.tsx line 537).
    pub fn neighbours(&self, i: usize) -> Vec<(usize, u32)> {
        self.adj.get(i).map_or_else(Vec::new, |links| {
            links
                .iter()
                .copied()
                .filter(|&(j, _)| self.in_language(j))
                .collect()
        })
    }

    /// Node `i`'s neighbours (as [`Scene::neighbours`]) by link type, each list most similar
    /// first, then by group, then by acronym in natural order (`MN 2` before `MN 10`):
    /// TextPanel.tsx lines 31–32 and 83.
    pub fn neighbours_by_type(&self, graph: &Graph, i: usize) -> Vec<Vec<usize>> {
        let mut by_type: Vec<Vec<usize>> = vec![Vec::new(); graph.types.len()];
        for (j, t) in self.neighbours(i) {
            if let Some(list) = by_type.get_mut(t as usize) {
                list.push(j);
            }
        }
        let sim = |j: usize| self.sim_of(i, j).unwrap_or(0.0);
        for list in &mut by_type {
            list.sort_by(|&a, &b| {
                let (na, nb) = (&graph.nodes[a], &graph.nodes[b]);
                sim(b)
                    .total_cmp(&sim(a))
                    .then(na.group.cmp(&nb.group))
                    .then_with(|| natural_cmp(&na.acronym, &nb.acronym))
            });
        }
        by_type
    }

    /// The number of links per type (App.tsx lines 349–354, `linkCounts`).
    pub fn link_counts(&self, graph: &Graph) -> Vec<usize> {
        let mut counts = vec![0; graph.types.len()];
        for &[_, _, t] in self.edges.as_chunks::<3>().0 {
            if let Some(count) = counts.get_mut(t as usize) {
                *count += 1;
            }
        }
        counts
    }

    /// The number of texts without a link (App.tsx lines 355–357, `alone`).
    pub fn alone(&self) -> usize {
        self.adj.iter().filter(|links| links.is_empty()).count()
    }
}

/// `a` and `b` in natural order, as `localeCompare(…, { numeric: true })`: runs of digits
/// by value, the rest ignoring case; case breaks a tie, lower case first.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    tokens(a).cmp(&tokens(b)).then_with(|| {
        let case = |s: &str| s.chars().map(|c| (c.is_uppercase(), c)).collect::<Vec<_>>();
        case(a).cmp(&case(b))
    })
}

/// A piece of a string in natural order: separators before numbers before letters, as the
/// root collation orders them.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Token {
    /// Space or punctuation.
    Separator(char),
    /// A run of digits: its significant digits' count, then the digits.
    Number(usize, String),
    /// A letter, lower-cased.
    Letter(char),
}

/// `s` as [`Token`]s.
fn tokens(s: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            let mut digits = String::from(c);
            while let Some(d) = chars.next_if(char::is_ascii_digit) {
                digits.push(d);
            }
            let significant = digits.trim_start_matches('0').to_owned();
            out.push(Token::Number(significant.len(), significant));
        } else if c.is_alphanumeric() {
            out.push(Token::Letter(c.to_lowercase().next().unwrap_or(c)));
        } else {
            out.push(Token::Separator(c));
        }
    }
    out
}

/// The nodes' positions, x, y, z per node (rust-engine.ts line 55).
pub fn positions(graph: &Graph) -> Vec<f32> {
    graph
        .nodes
        .iter()
        .flat_map(|node| [node.x, node.y, node.z].map(single))
        .collect()
}

/// The points `which` of `positions`: their median centre and the 90th percentile of their
/// distances to it (`d[Math.floor(d.length * 0.9)]`); `None` without points.
fn centre_and_reach(
    positions: &[f32],
    which: impl Iterator<Item = usize>,
) -> Option<([f64; 3], f64)> {
    let points: Vec<[f64; 3]> = which
        .filter_map(|i| positions.get(i * 3..i * 3 + 3))
        .map(|p| [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])])
        .collect();
    if points.is_empty() {
        return None;
    }
    // The page's `median`: the upper middle of the sorted values.
    let median = |k: usize| {
        let mut xs: Vec<f64> = points.iter().map(|p| p[k]).collect();
        xs.sort_by(f64::total_cmp);
        xs[xs.len() / 2]
    };
    let c = [median(0), median(1), median(2)];
    let mut d: Vec<f64> = points.iter().map(|p| distance(*p, c)).collect();
    d.sort_by(f64::total_cmp);
    Some((c, d[d.len() * 9 / 10]))
}

/// The distance between two points (`Math.hypot` of the differences).
fn distance(p: [f64; 3], q: [f64; 3]) -> f64 {
    (p[0] - q[0]).hypot(p[1] - q[1]).hypot(p[2] - q[2])
}

/// `x`, or `default` when it is 0 or not a number (the page's `x || default`).
fn or(x: f64, default: f64) -> f64 {
    if x == 0.0 || x.is_nan() { default } else { x }
}

/// Where the camera looks and from how far.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Framing {
    /// The point looked at.
    pub center: [f32; 3],
    /// The distance from it.
    pub distance: f32,
}

/// The scene's radius, for framing a text: the 90th percentile of the distances to the
/// median centre, 1 when it is 0 (rust-engine.ts lines 117–119).
pub fn scene_radius(positions: &[f32]) -> f32 {
    let reach = centre_and_reach(positions, 0..positions.len() / 3).map_or(0.0, |(_, r)| r);
    single(or(reach, 1.0))
}

/// The framing of the shown points (`sizes` above 0): median centre, twice the 90th
/// percentile radius, at least 50; `None` when nothing is shown (rust-engine.ts lines
/// 192–201, `frameVisible`).
pub fn frame_visible(positions: &[f32], sizes: &[f32]) -> Option<Framing> {
    let shown = (0..sizes.len()).filter(|&i| sizes[i] > 0.0);
    let (c, reach) = centre_and_reach(positions, shown)?;
    Some(Framing {
        center: c.map(single),
        distance: single(or(reach, 1.0).max(50.0) * 2.0),
    })
}

/// The distance to fly to node `i` from, framing it with its neighbours: 2.2 times its
/// farthest neighbour (at least 60), at most 1.5 times the scene's `radius`
/// (rust-engine.ts lines 280–285, `flyTo`).
pub fn fly_distance(positions: &[f32], adj: &Adjacency, i: usize, radius: f32) -> f32 {
    let at = |k: usize| {
        positions.get(k * 3..k * 3 + 3).map_or([0.0; 3], |p| {
            [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])]
        })
    };
    let mut r: f64 = 60.0;
    for &(j, _) in adj.get(i).map_or(&[][..], Vec::as_slice) {
        r = r.max(distance(at(i), at(j)));
    }
    single((r * 2.2).min(f64::from(radius) * 1.5))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{fixture, public_file};

    fn close(a: &[f32], b: &[f32]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-6)
    }

    #[test]
    fn the_adjacency_lists_each_link_both_ways() {
        // rust-engine.ts:226-227 `this.adj[e[i]].push([e[i + 1], e[i + 2]]); this.adj[e[i + 1]].push([e[i], e[i + 2]]);`
        let g = fixture();
        let adj = adjacency(g.nodes.len(), &g.edges);
        assert_eq!(adj[0], [(1, 0)]);
        assert_eq!(adj[1], [(0, 0), (2, 1), (3, 0)]);
        assert_eq!(adj[2], [(1, 1)]);
        assert_eq!(adj[4], Vec::<(usize, u32)>::new());
    }

    #[test]
    fn links_at_keeps_pairs_at_or_above_the_threshold() {
        // data.ts:67-69 `if (pairs.sim[k]! < th) continue; edges.push(a, b, 0); sims.set(a * N + b, sim)`
        let sim = SimInfo {
            url: "graph-sim.bin".to_owned(),
            min: 0.0,
            max: 1.0,
            default: 0.5,
            range: Some([0.0, 1.0]),
            sha256: None,
        };
        let pairs = Pairs::from_similarities(&[(0, 1, 0.9), (1, 2, 0.3), (2, 4, 0.6)], [0.0, 1.0]);
        let links = links_at(&pairs, &sim, 0.5);
        assert_eq!(links.edges, [0, 1, 0, 2, 4, 0]);
        assert_eq!(links.sims.len(), 2);
        assert!((links.sims[&(0, 1)] - 0.9).abs() < 1e-4);
        assert_eq!(links_at(&pairs, &sim, 0.95).edges, Vec::<u32>::new());
    }

    #[test]
    fn links_at_compares_in_double_precision() {
        // The page compares a single-precision similarity with a double threshold: a link is
        // kept at its own value and dropped just above it, however close.
        let sim = SimInfo {
            url: String::new(),
            min: 0.0,
            max: 1.0,
            default: 0.5,
            range: None,
            sha256: None,
        };
        let pairs = Pairs::from_similarities(&[(0, 1, 0.7)], [0.0, 1.0]);
        let s = f64::from(pairs.links_at([0.0, 1.0], 0.0)[0].sim);
        assert_eq!(
            links_at(&pairs, &sim, s).edges.len(),
            3,
            "at the value itself"
        );
        assert_eq!(
            links_at(&pairs, &sim, s + 1e-12).edges,
            Vec::<u32>::new(),
            "just above"
        );
    }

    #[test]
    fn hex_colours_decode_and_linearise() {
        // rust-engine.ts:23-24 `[(n >> 16) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255]`
        assert_eq!(rgb("#ff8000"), [1.0, 128.0 / 255.0, 0.0]);
        assert_eq!(rgb("ff8000"), [1.0, 128.0 / 255.0, 0.0]);
        assert_eq!(
            rgb("#ff8000cc"),
            [1.0, 128.0 / 255.0, 0.0],
            "alpha ignored (slice(0, 6))"
        );
        assert_eq!(rgb("nope"), [0.0, 0.0, 0.0]);
        // rust-engine.ts:27 `c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4`
        let l = linear("#ff8000");
        assert_eq!(l[0], 1.0);
        assert!((l[1] - ((128.0 / 255.0 + 0.055) / 1.055_f64).powf(2.4)).abs() < 1e-12);
        assert_eq!(linear("#010101")[0], 1.0 / 255.0 / 12.92);
    }

    #[test]
    fn a_type_is_drawn_in_the_themes_colour_and_stronger_on_light() {
        // rust-engine.ts:253 `{ color: this.theme.types[type] ?? style.color, opacity: style.opacity * (this.theme.light ? 2.2 : 1) }`
        let mut g = fixture();
        let dark = MapTheme::new(false, &g);
        let light = MapTheme::new(true, &g);
        assert_eq!(
            type_style(&g, &dark, 0),
            TypeStyle {
                color: "#ffffff",
                opacity: 0.22
            }
        );
        assert_eq!(
            type_style(&g, &dark, 1),
            TypeStyle {
                color: "#94a3b8",
                opacity: 0.1
            }
        );
        let l = type_style(&g, &light, 0);
        assert_eq!(l.color, "#2c1810");
        assert!((l.opacity - 0.22 * 2.2).abs() < 1e-12);
        // data.ts:252 `TYPE_STYLE[type] ?? TYPE_STYLE.full`
        g.types[1] = "unknown".to_owned();
        assert_eq!(type_style(&g, &light, 1).color, "#ffffff");
    }

    #[test]
    fn a_new_scene_shows_everything_and_selects_nothing() {
        // rust-engine.ts:109-114 colorOn false, groupOn true, typeOn true, hideAlone false, available null, selected -1
        let g = fixture();
        let s = Scene::new(&g);
        assert_eq!(s.edges, g.edges);
        assert_eq!(s.colored, [false; 3]);
        assert_eq!(s.group_on, [true; 3]);
        assert_eq!(s.type_on, [true; 2]);
        assert_eq!(
            (s.available.clone(), s.hide_alone, s.selected),
            (None, false, None)
        );
        assert_eq!(s.adj, adjacency(5, &g.edges));
    }

    #[test]
    fn new_links_rebuild_the_adjacency_and_similarities() {
        // rust-engine.ts:167-169 `this.g.edges = edges; this.sims = sims; this.rebuildAdj();`
        // rust-engine.ts:175 `this.sims.get(a < b ? a * this.N + b : b * this.N + a)`
        let g = fixture();
        let mut s = Scene::new(&g);
        s.set_links(Links {
            edges: vec![2, 4, 0],
            sims: [((2, 4), 0.8)].into(),
        });
        assert_eq!(s.adj[4], [(2, 0)]);
        assert_eq!(s.adj[1], Vec::<(usize, u32)>::new());
        assert_eq!(s.sim_of(4, 2), Some(0.8));
        assert_eq!(s.sim_of(2, 4), Some(0.8));
        assert_eq!(s.sim_of(0, 1), None);
        assert_eq!(s.alone(), 3);
    }

    #[test]
    fn shown_needs_the_group_and_the_language() {
        // rust-engine.ts:219 `!!this.groupOn[this.g.nodes[i]![5]] && (!this.available || !!this.available[i])`
        let g = fixture();
        let mut s = Scene::new(&g);
        assert!((0..5).all(|i| s.shown(&g, i)));
        s.group_on[1] = false;
        assert_eq!(
            (0..5).map(|i| s.shown(&g, i)).collect::<Vec<_>>(),
            [true, false, false, false, true]
        );
        s.available = Some(vec![false, true, true, true, true]);
        assert!(!s.shown(&g, 0));
    }

    #[test]
    fn sizes_hide_the_hidden_and_the_alone_when_asked() {
        // rust-engine.ts:233-234 `on = this.shown(i) && !(this.hideAlone && this.adj[i]!.length === 0); sizes[i] = on ? POINT_SIZE : 0`
        let g = fixture();
        let mut s = Scene::new(&g);
        assert_eq!(s.sizes(&g), [POINT_SIZE; 5]);
        s.hide_alone = true;
        assert_eq!(s.sizes(&g), [7.0, 7.0, 7.0, 7.0, 0.0]);
        s.group_on[0] = false;
        assert_eq!(s.sizes(&g), [0.0, 7.0, 7.0, 7.0, 0.0]);
    }

    #[test]
    fn points_are_gold_until_a_collection_is_coloured() {
        // rust-engine.ts:243 `base = this.colorOn.some(Boolean) ? mix(gold, linear(background), 0.7) : gold`
        // rust-engine.ts:246 `colors.set(this.colorOn[n[5]] ? groups[n[5]]! : base, i * 3)`
        let g = fixture();
        let theme = MapTheme::new(false, &g);
        let mut s = Scene::new(&g);
        let gold = linear(&theme.gold).map(single);
        let all = s.point_colors(&g, &theme);
        assert_eq!(all.len(), 15);
        assert!(all.chunks(3).all(|c| close(c, &gold)));
        s.colored[2] = true;
        let some = s.point_colors(&g, &theme);
        let bg = linear(&theme.background);
        let faded: Vec<f32> = linear(&theme.gold)
            .iter()
            .zip(bg)
            .map(|(a, b)| single(a + (b - a) * 0.7))
            .collect();
        assert!(close(&some[0..3], &faded));
        assert!(close(&some[12..15], &linear("#c792ff").map(single)));
    }

    #[test]
    fn segments_are_gold_then_the_selections_in_its_type_colour() {
        // rust-engine.ts:266-268 skip a hidden type or end; `cols.push(...gold, Math.min(opacity * LINKS * dim, 1))`
        // rust-engine.ts:271-274 the selected's links: `cols.push(...hex(this.typeStyle(t).color), 0.85)`
        let g = fixture();
        let theme = MapTheme::new(false, &g);
        let mut s = Scene::new(&g);
        let gold = rgb(&theme.gold).map(single);
        let seg = s.segments(&g, &theme);
        assert_eq!(seg.ends, [0, 1, 1, 2, 1, 3]);
        let a = |o: f64| single((o * LINKS).min(1.0));
        assert!(close(
            &seg.colors[0..4],
            &[gold[0], gold[1], gold[2], a(0.22)]
        ));
        assert!(close(
            &seg.colors[4..8],
            &[gold[0], gold[1], gold[2], a(0.1)]
        ));
        s.type_on[1] = false;
        assert_eq!(s.segments(&g, &theme).ends, [0, 1, 1, 3]);
        s.type_on[1] = true;
        s.group_on[0] = false;
        assert_eq!(s.segments(&g, &theme).ends, [1, 2, 1, 3]);
        s.group_on[0] = true;
        s.selected = Some(1);
        let sel = s.segments(&g, &theme);
        assert_eq!(sel.ends, [0, 1, 1, 2, 1, 3, 1, 0, 1, 2, 1, 3]);
        assert!(
            close(&sel.colors[3..4], &[single(0.22 * LINKS * 0.25)]),
            "dimmed"
        );
        assert!(
            close(&sel.colors[12..16], &[1.0, 1.0, 1.0, 0.85]),
            "full: white"
        );
        let resembling = rgb("#94a3b8").map(single);
        assert!(close(
            &sel.colors[16..20],
            &[resembling[0], resembling[1], resembling[2], 0.85]
        ));
    }

    #[test]
    fn the_highlight_marks_the_selection_and_its_shown_neighbours() {
        // rust-engine.ts:183-184 `hl[i] = 2; for (const [j, t] of this.adj[i]!) if (this.typeOn[t] && this.shown(j)) hl[j] = 1;`
        // rust-engine.ts:187 `this.view.setHighlight(hl, i >= 0)`
        let g = fixture();
        let mut s = Scene::new(&g);
        assert_eq!(
            s.highlight(&g),
            Highlight {
                hl: vec![0.0; 5],
                dim: false
            }
        );
        s.selected = Some(1);
        assert_eq!(
            s.highlight(&g),
            Highlight {
                hl: vec![1.0, 2.0, 1.0, 1.0, 0.0],
                dim: true
            }
        );
        s.type_on[1] = false;
        s.available = Some(vec![false, true, true, true, true]);
        assert_eq!(s.highlight(&g).hl, [0.0, 2.0, 0.0, 1.0, 0.0]);
    }

    #[test]
    fn neighbours_leave_out_texts_the_language_lacks() {
        // App.tsx:537 `(engine.adj[selected] ?? []).filter(([j]) => !available || available[j])`
        let g = fixture();
        let mut s = Scene::new(&g);
        assert_eq!(s.neighbours(1), [(0, 0), (2, 1), (3, 0)]);
        s.available = Some(vec![false, true, true, true, true]);
        assert_eq!(s.neighbours(1), [(2, 1), (3, 0)]);
    }

    #[test]
    fn neighbours_by_type_sort_by_similarity_group_then_acronym() {
        // TextPanel.tsx:83 `(simOf(index, b) ?? 0) - (simOf(index, a) ?? 0) || nodes[a][5] - nodes[b][5] || nodes[a][1].localeCompare(nodes[b][1], undefined, { numeric: true })`
        let g = fixture();
        let mut s = Scene::new(&g);
        s.set_links(Links {
            edges: vec![1, 0, 0, 1, 2, 0, 1, 3, 0],
            sims: HashMap::new(),
        });
        // No similarity: group (dn 0 before mn 1), then MN 2 before MN 10.
        assert_eq!(s.neighbours_by_type(&g, 1), [vec![0, 3, 2], vec![]]);
        s.sims = [((1, 2), 0.9), ((1, 3), 0.6)].into();
        assert_eq!(s.neighbours_by_type(&g, 1)[0], [2, 3, 0]);
    }

    #[test]
    fn natural_order_reads_numbers_as_numbers() {
        assert_eq!(natural_cmp("MN 2", "MN 10"), Ordering::Less);
        assert_eq!(natural_cmp("Snp 1.10", "Snp 1.9"), Ordering::Greater);
        assert_eq!(natural_cmp("an1", "AN 1"), natural_cmp("an1", "an 1"));
        assert_eq!(
            natural_cmp("mn", "MN"),
            Ordering::Less,
            "case breaks a tie, lower first"
        );
        assert_eq!(natural_cmp("DN 1", "DN 1"), Ordering::Equal);
    }

    #[test]
    fn link_counts_and_alone_texts() {
        // App.tsx:351 `for (let i = 2; i < graph.edges.length; i += 3) c[graph.edges[i]!]!++;`
        // App.tsx:355 `engine.adj.filter((a) => a.length === 0).length`
        let g = fixture();
        let s = Scene::new(&g);
        assert_eq!(s.link_counts(&g), [2, 1]);
        assert_eq!(s.alone(), 1);
    }

    #[test]
    fn positions_are_xyz_per_node() {
        // rust-engine.ts:55 `g.nodes.flatMap((n) => [n[7], n[8], n[9]])`
        let p = positions(&fixture());
        assert_eq!(p.len(), 15);
        assert_eq!(&p[6..9], [10.0, 30.0, 0.0]);
    }

    #[test]
    fn the_radius_is_the_90th_percentile_from_the_median() {
        // rust-engine.ts:117-119 median centre, `d[Math.floor(d.length * 0.9)] || 1`
        let p = positions(&fixture());
        // Centre (10, 0, 0); distances 0, 10, 30, 40, 210 → index 4.
        assert_eq!(scene_radius(&p), 210.0);
        assert_eq!(scene_radius(&[0.0; 6]), 1.0);
        assert_eq!(scene_radius(&[]), 1.0);
    }

    #[test]
    fn framing_covers_the_shown_points() {
        // rust-engine.ts:194-200 visible = size > 0; `r = Math.max(d[floor(len * 0.9)] || 1, 50)`; distance r * 2
        let p = positions(&fixture());
        assert_eq!(frame_visible(&p, &[0.0; 5]), None);
        assert_eq!(
            frame_visible(&p, &[7.0, 7.0, 7.0, 7.0, 0.0]),
            Some(Framing {
                center: [10.0, 0.0, 0.0],
                distance: 100.0
            })
        );
        assert_eq!(
            frame_visible(&p, &[7.0; 5]),
            Some(Framing {
                center: [10.0, 0.0, 0.0],
                distance: 420.0
            })
        );
    }

    #[test]
    fn flying_frames_the_text_and_its_neighbours() {
        // rust-engine.ts:282-284 `r = 60; r = max(r, |p_i - p_j|)`; `Math.min(r * 2.2, this.radius * 1.5)`
        let g = fixture();
        let p = positions(&g);
        let adj = adjacency(5, &g.edges);
        assert!((fly_distance(&p, &adj, 1, 1000.0) - 132.0).abs() < 1e-4);
        assert!((fly_distance(&p, &adj, 1, 50.0) - 75.0).abs() < 1e-4);
        assert!(
            (fly_distance(&p, &adj, 4, 1000.0) - 132.0).abs() < 1e-4,
            "alone: 60 * 2.2"
        );
    }

    /// The buffers of every committed view have the lengths the engine checks: three colours
    /// and one size per node, one highlight per node, two ends and four colours per segment.
    #[test]
    fn buffers_match_the_committed_data() {
        let pairs = Pairs::from_bytes(&public_file("graph-sim.bin")).expect("graph-sim.bin");
        for file in ["graph.json", "graph-sim.json"] {
            let g = Graph::from_json(&public_file(file)).expect(file);
            let n = g.nodes.len();
            let mut s = Scene::new(&g);
            if let Some(sim) = &g.sim {
                s.set_links(links_at(&pairs, sim, sim.default));
            }
            let links = s.edges.len() / 3;
            assert!(links > 0, "{file} has links");
            for light in [false, true] {
                let theme = MapTheme::new(light, &g);
                assert_eq!(s.point_colors(&g, &theme).len(), 3 * n, "{file}");
                let seg = s.segments(&g, &theme);
                assert_eq!(seg.ends.len(), 2 * links, "{file}");
                assert_eq!(seg.colors.len(), 4 * links, "{file}");
                assert!(seg.ends.iter().all(|&e| (e as usize) < n));
            }
            assert_eq!(s.sizes(&g).len(), n);
            assert_eq!(positions(&g).len(), 3 * n);
            let busiest = (0..n).max_by_key(|&i| s.adj[i].len()).expect("nodes");
            s.selected = Some(busiest);
            let hl = s.highlight(&g);
            assert_eq!(hl.hl.len(), n);
            assert_eq!(hl.hl.iter().filter(|&&h| h > 1.5).count(), 1);
            let seg = s.segments(&g, &MapTheme::new(false, &g));
            assert_eq!(seg.ends.len(), 2 * (links + s.adj[busiest].len()), "{file}");
            assert_eq!(s.link_counts(&g).iter().sum::<usize>(), links);
        }
    }
}
