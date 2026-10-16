//! `EngineView`: the map engine (`map_engine`, task 004) driven from Rust in the browser. The
//! port of `src/map/rust-engine.ts` — what [`draw`](crate::map::draw) did not take as pure
//! functions: the canvas, the frame loop, the pointer, wheel, pinch and arrow keys, picking,
//! the live layout of the vectorial views — and of the engine part of `src/App.tsx`: the
//! engine's lifecycle (made once, then given each view's graph), the effects pushing the
//! page's state into it, `data-engine` on `<html>`, the no-GPU message.
//!
//! **Inputs**: one [`EngineInput`], a signal per piece of state. **Outputs**: callbacks,
//! `on_hover`, `on_select`, `on_ready`, `on_no_gpu` (see [`EngineView`]). The map screen
//! (task 006, M5) wires both.
//!
//! The server renders the empty container (the canvas, no message), so hydration matches; the
//! engine starts in the browser only (`hydrate` on wasm32), in an effect, once the canvas, a
//! graph and a theme are there — tried once ([`Start`]). Every decision that needs no canvas
//! is a pure function below, tested on the host; the browser part only reads the DOM and calls
//! the engine.
//!
//! **The frame loop sleeps; everything that changes the picture wakes it.** The page's loop
//! drew every frame for ever; this one draws on demand:
//!
//! - *Sleep*: after a frame, [`keep_looping`] asks for another while the frame was busy (the
//!   engine drew — `GraphView::frame` is true while the camera eases, flies or the keys move
//!   it — or the live layout still steps); after [`IDLE_FRAMES`] still frames in a row it
//!   stops requesting animation frames. A frame the surface did not give (`frame` false with
//!   a redraw pending) is retried by those idle frames.
//! - *Wake*: `wake` restarts it (idle count back to 0) and is called by every path that hands
//!   the engine something new — the input effect when `sync` pushed anything (its [`Plan`]
//!   not the default: a buffer, the style, a camera move, a relayout), the engine's creation,
//!   each pointer press, move, release, wheel and arrow key, and a resize with an area. A new
//!   path that changes what the engine draws must call `wake`, or its change waits for the
//!   next input: the picture is not lost, only late.
//! - *Stop*: the component's cleanup marks the loop disposed; a frame already requested
//!   finds it so and requests no other.

use std::sync::Arc;

use leptos::prelude::*;
use leptos_i18n::td_string;

use crate::catalogue::strings;
use crate::map::draw::{Links, Scene};
use crate::map::model::Graph;
use crate::map::theme::MapTheme;

/// Live layout steps after a threshold change (rust-engine.ts line 18, `LAYOUT_STEPS`).
pub const LAYOUT_STEPS: u32 = 400;

/// Milliseconds of layout steps per frame at most (rust-engine.ts line 390).
pub const LAYOUT_BUDGET_MS: f64 = 14.0;

/// A press released within this many CSS pixels of where it started is a click
/// (rust-engine.ts line 331).
pub const CLICK_SLOP: f64 = 4.0;

/// Frames drawn with nothing moving before the loop sleeps (render on demand; the page's
/// loop never slept). They also retry a frame the surface did not give (`frame` false with
/// a redraw pending).
pub const IDLE_FRAMES: u32 = 30;

/// A vectorial view's links (M2's [`Links`]) at the threshold they were cut at: the
/// threshold decides whether the points are laid out again.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LinksAt {
    /// The similarity threshold of `links`.
    pub threshold: f64,
    /// The links at it ([`crate::map::draw::links_at`]).
    pub links: Links,
}

/// What [`EngineView`] draws: one signal per piece of the page's state (App.tsx lines
/// 263–325, the effects that pushed it into `RustMapEngine`). The masks are per group of
/// [`Graph::langs`] (`colored`, `visible`), per link type of [`Graph::types`] (`types`), per
/// node (`available`); a mask shorter than its list reads as false past its end.
#[derive(Clone, Copy, Debug)]
pub struct EngineInput {
    /// The view's graph; nothing is drawn before it. A new `Arc` is a new view: the same
    /// engine takes it, the camera eases to it (rust-engine.ts `setGraph`).
    pub graph: Signal<Option<Arc<Graph>>>,
    /// The colours ([`MapTheme::new`] of this graph); nothing is drawn before it.
    pub theme: Signal<Option<MapTheme>>,
    /// A vectorial view's links at its threshold (`setLinks`); `None`: the graph's own links
    /// (and always on a view without `sim`). A threshold other than the one the points were
    /// laid out for (the view's `sim.default` at first) lays them out again, live, unless the
    /// view is `fixed`.
    pub links: Signal<Option<Arc<LinksAt>>>,
    /// Per group: drawn in its own colour (`setColored`).
    pub colored: Signal<Vec<bool>>,
    /// Per group: shown (`setVisible`, the legend's hidden collections negated).
    pub visible: Signal<Vec<bool>>,
    /// Per link type: shown (`setTypes`).
    pub types: Signal<Vec<bool>>,
    /// Per node: in the texts' language; `None`: every text, Pāli (`setAvailable`).
    pub available: Signal<Option<Vec<bool>>>,
    /// Hide the texts without a link (`setHideAlone`; a view with `sim` only).
    pub hide_alone: Signal<bool>,
    /// The open text's node (`select`): highlighted with its neighbours, flown to.
    pub selected: Signal<Option<usize>>,
}

/// The text under the pointer (rust-engine.ts `onHover(i, x, y)`, `i` ≥ 0).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hover {
    /// Its node.
    pub index: usize,
    /// The pointer, CSS pixels from the viewport's left (`clientX`): where the tooltip goes.
    pub x: f64,
    /// The same from the top (`clientY`).
    pub y: f64,
}

// ---------- Input: pointer, wheel, pinch, keys ----------

/// A pointer event's position on the canvas, CSS pixels from its top left (rust-engine.ts
/// lines 288–291, `at`): the client position less the canvas' bounding box origin.
#[allow(
    clippy::cast_possible_truncation,
    reason = "CSS pixels on a canvas fit f32, the engine's unit"
)]
pub fn canvas_point(client: (f64, f64), origin: (f64, f64)) -> (f32, f32) {
    ((client.0 - origin.0) as f32, (client.1 - origin.1) as f32)
}

/// The wheel delta a pinch from a spread of `before` to `after` pixels makes (rust-engine.ts
/// line 317: `100 * Math.log(pinch / s) / 0.0513`, the wheel units of camera.rs); `None`
/// when either spread is 0 (the first move of a pinch).
#[allow(
    clippy::cast_possible_truncation,
    reason = "a wheel delta fits f32, the engine's unit"
)]
pub fn pinch_wheel(before: f64, after: f64) -> Option<f32> {
    (before != 0.0 && after != 0.0).then(|| (100.0 * (before / after).ln() / 0.0513) as f32)
}

/// The node `GraphView::pick` returned, `None` for -1 (empty map).
pub fn picked(i: i32) -> Option<usize> {
    usize::try_from(i).ok()
}

/// Whether a canvas of this CSS size can be drawn to: the `ResizeObserver` resized only then
/// (rust-engine.ts line 79, `if (b.width && b.height)`).
pub fn has_area(width: f64, height: f64) -> bool {
    width != 0.0 && height != 0.0
}

/// What a pointer pressed on the canvas does (rust-engine.ts lines 305–312).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerDown {
    /// The first pointer: start rotating (`view.pointerDown(at)`).
    Rotate,
    /// A second one: stop rotating, start a pinch (`view.pointerUp()`).
    Pinch,
}

/// What a pointer moving over the canvas does (rust-engine.ts lines 313–326).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PointerMove {
    /// Two pointers or more: zoom by this wheel delta, if any.
    Pinch(Option<f32>),
    /// Otherwise: `view.pointerMove(at)`; then, when `hover`, pick the point under it and
    /// say so (not on touch, not while pressed).
    Drag {
        /// Pick and report the hovered point.
        hover: bool,
    },
}

/// The pointers pressed on the canvas (rust-engine.ts lines 299–339, `bindPointer`): one
/// rotates (a click picks), two pinch to zoom. Positions are client CSS pixels.
///
/// A gesture runs from the first press to the last release: once a second pointer joined it,
/// it is a pinch to its end, and no release in it is a click (the page measured the last
/// release against the last press, the second finger's: a pinch ending with the fingers
/// together picked a point).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pointers {
    down: Vec<(i32, (f64, f64))>,
    /// Where the gesture's first pointer pressed.
    start: (f64, f64),
    /// A second pointer joined the gesture.
    pinched: bool,
    pinch: f64,
}

impl Pointers {
    /// Pointer `id` pressed at `at` (`pointerdown`).
    pub fn down(&mut self, id: i32, at: (f64, f64)) -> PointerDown {
        self.set(id, at);
        if self.down.len() == 1 {
            // A new gesture.
            self.start = at;
            self.pinched = false;
            PointerDown::Rotate
        } else {
            self.pinched = true;
            self.pinch = self.spread();
            PointerDown::Pinch
        }
    }

    /// Pointer `id` moved to `at` (`pointermove`); `touch`: a touch pointer (no hover).
    pub fn moved(&mut self, id: i32, at: (f64, f64), touch: bool) -> PointerMove {
        if self.down.iter().any(|&(d, _)| d == id) {
            self.set(id, at);
        }
        if self.down.len() >= 2 {
            let s = self.spread();
            let delta = pinch_wheel(self.pinch, s);
            self.pinch = s;
            return PointerMove::Pinch(delta);
        }
        PointerMove::Drag {
            hover: !touch && self.down.is_empty(),
        }
    }

    /// Pointer `id` released at `at` (`pointerup`, `released`) or cancelled
    /// (`pointercancel`): whether it was a click, to pick and select (the one pointer of its
    /// gesture, never a pinch's, released within [`CLICK_SLOP`] of where it was pressed). The
    /// caller always stops rotating.
    pub fn up(&mut self, id: i32, at: (f64, f64), released: bool) -> bool {
        let was = self.down.len();
        self.down.retain(|&(d, _)| d != id);
        let (dx, dy) = (at.0 - self.start.0, at.1 - self.start.1);
        was == 1 && !self.pinched && released && dx.hypot(dy) <= CLICK_SLOP
    }

    /// Pointer `id` is at `at`: replaced in place, or added last (a `Map`'s order).
    fn set(&mut self, id: i32, at: (f64, f64)) {
        match self.down.iter_mut().find(|(d, _)| *d == id) {
            Some(entry) => entry.1 = at,
            None => self.down.push((id, at)),
        }
    }

    /// The distance between the first two pointers, 0 with fewer (rust-engine.ts line 304).
    fn spread(&self) -> f64 {
        match self.down.as_slice() {
            [(_, a), (_, b), ..] => (a.0 - b.0).hypot(a.1 - b.1),
            _ => 0.0,
        }
    }
}

/// Whether the focused element takes typing, so the arrow keys are its own (rust-engine.ts
/// lines 351–354): an input, a text area, a select, or anything content-editable.
pub fn is_typing(tag: &str, content_editable: bool) -> bool {
    content_editable || ["INPUT", "TEXTAREA", "SELECT"].contains(&tag)
}

/// The arrow keys held (rust-engine.ts lines 341–364, `bindKeys`): left/right slide, up/down
/// go forward or back, with Shift rise or sink. Each call returns `GraphView::set_keys`'s
/// `[x, y, z]`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "one flag per key held, as the page's set of held keys"
)]
pub struct Keys {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    shift: bool,
}

impl Keys {
    /// `keydown` of `key` (`KeyboardEvent.key`) with Shift `shift`, while the page is
    /// `typing` or not: whether to prevent its default (an arrow taken by the map), and the
    /// movement.
    pub fn down(&mut self, key: &str, shift: bool, typing: bool) -> (bool, [f32; 3]) {
        self.shift = shift;
        if !key.starts_with("Arrow") || typing {
            return (false, self.vector());
        }
        self.hold(key, true);
        (true, self.vector())
    }

    /// `keyup` of `key` with Shift `shift`: the movement.
    pub fn up(&mut self, key: &str, shift: bool) -> [f32; 3] {
        self.shift = shift;
        self.hold(key, false);
        self.vector()
    }

    /// The window lost the focus: nothing is held any more.
    pub fn blur(&mut self) -> [f32; 3] {
        self.hold("ArrowUp", false);
        self.hold("ArrowDown", false);
        self.hold("ArrowLeft", false);
        self.hold("ArrowRight", false);
        self.vector()
    }

    /// Mark an arrow key held or released (another key changes nothing).
    fn hold(&mut self, key: &str, on: bool) {
        match key {
            "ArrowUp" => self.up = on,
            "ArrowDown" => self.down = on,
            "ArrowLeft" => self.left = on,
            "ArrowRight" => self.right = on,
            _ => {}
        }
    }

    /// `[right - left, shift ? up - down : 0, shift ? 0 : up - down]` (line 349).
    fn vector(&self) -> [f32; 3] {
        let k = |on: bool| if on { 1.0 } else { 0.0 };
        let ud = k(self.up) - k(self.down);
        let x = k(self.right) - k(self.left);
        if self.shift {
            [x, ud, 0.0]
        } else {
            [x, 0.0, ud]
        }
    }
}

// ---------- The frame loop and the live layout ----------

/// After a frame: `Some(idle frames so far)` to ask for another, `None` to sleep until
/// something wakes the loop. `busy`: the frame drew, or the layout runs.
pub fn keep_looping(busy: bool, idle: u32) -> Option<u32> {
    if busy {
        return Some(0);
    }
    let idle = idle.saturating_add(1);
    (idle < IDLE_FRAMES).then_some(idle)
}

/// Whether to run another layout step this frame: [`LAYOUT_STEPS`] not reached and less than
/// [`LAYOUT_BUDGET_MS`] spent (rust-engine.ts line 390).
pub fn layout_continues(steps: u32, elapsed_ms: f64) -> bool {
    steps < LAYOUT_STEPS && elapsed_ms < LAYOUT_BUDGET_MS
}

/// The rest length of a link of similarity `sim` in a view whose similarities span `range`
/// (rust-engine.ts lines 376–379, `springTransform`): 17.5 for the most similar, 47.5 for
/// the least. A link without a similarity, or a range without width, is the least similar
/// (the page's `NaN` there; not reached on the data).
#[allow(
    clippy::cast_possible_truncation,
    reason = "a rest length fits f32, the layout's input"
)]
pub fn spring_length(sim: Option<f32>, range: [f64; 2]) -> f32 {
    let [lo, hi] = range;
    let t = match sim {
        Some(s) if hi > lo => ((f64::from(s) - lo) / (hi - lo)).clamp(0.0, 1.0),
        _ => 0.0,
    };
    (17.5 + (1.0 - t) * 30.0) as f32
}

/// The live layout's links from a scene: ends and rest lengths (rust-engine.ts lines
/// 369–372, the graph `startLayout` builds), for `map_engine::live_layout::Core::new`.
pub fn layout_links(scene: &Scene, range: [f64; 2]) -> (Vec<u32>, Vec<f32>) {
    let links = scene.edges.as_chunks::<3>().0;
    let mut ends = Vec::with_capacity(links.len() * 2);
    let mut lengths = Vec::with_capacity(links.len());
    for &[a, b, _] in links {
        ends.extend([a, b]);
        lengths.push(spring_length(scene.sim_of(a as usize, b as usize), range));
    }
    (ends, lengths)
}

/// Whether new links at `threshold` lay the points out again (App.tsx line 318): not on a
/// `fixed` view, and only for a threshold other than the one they were laid out for.
pub fn relayout(fixed: Option<bool>, threshold: f64, laid_out_at: f64) -> bool {
    fixed != Some(true) && threshold.total_cmp(&laid_out_at).is_ne()
}

// ---------- What to push into the engine ----------

/// One read of an [`EngineInput`] with its graph and theme there.
#[derive(Clone, Debug)]
pub struct Snapshot {
    /// [`EngineInput::graph`].
    pub graph: Arc<Graph>,
    /// [`EngineInput::theme`].
    pub theme: MapTheme,
    /// [`EngineInput::links`].
    pub links: Option<Arc<LinksAt>>,
    /// [`EngineInput::colored`].
    pub colored: Vec<bool>,
    /// [`EngineInput::visible`].
    pub visible: Vec<bool>,
    /// [`EngineInput::types`].
    pub types: Vec<bool>,
    /// [`EngineInput::available`].
    pub available: Option<Vec<bool>>,
    /// [`EngineInput::hide_alone`].
    pub hide_alone: bool,
    /// [`EngineInput::selected`].
    pub selected: Option<usize>,
}

/// Which inputs differ from the ones the engine was last given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "one flag per input, as the page had one effect per input"
)]
pub struct Changed {
    /// Another graph (another `Arc`), or the first.
    pub graph: bool,
    /// The theme.
    pub theme: bool,
    /// The coloured groups.
    pub colored: bool,
    /// The shown groups.
    pub visible: bool,
    /// The shown link types.
    pub types: bool,
    /// The texts' language mask.
    pub available: bool,
    /// The links.
    pub links: bool,
    /// Hide the texts without a link.
    pub hide_alone: bool,
    /// The selection.
    pub selected: bool,
}

/// What `next` changes from `prev` (`None`: the engine was never given an input).
pub fn changed(prev: Option<&Snapshot>, next: &Snapshot) -> Changed {
    let Some(prev) = prev.filter(|p| Arc::ptr_eq(&p.graph, &next.graph)) else {
        return Changed {
            graph: true,
            ..Changed::default()
        };
    };
    let links = match (&prev.links, &next.links) {
        (Some(a), Some(b)) => !Arc::ptr_eq(a, b) && a != b,
        (a, b) => a.is_some() != b.is_some(),
    };
    Changed {
        graph: false,
        theme: prev.theme != next.theme,
        colored: prev.colored != next.colored,
        visible: prev.visible != next.visible,
        types: prev.types != next.types,
        available: prev.available != next.available,
        links,
        hide_alone: prev.hide_alone != next.hide_alone,
        selected: prev.selected != next.selected,
    }
}

/// What to push into the engine for a [`Changed`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "one flag per buffer or camera move, as GraphView has one setter each"
)]
pub struct Plan {
    /// A new graph: `set_graph` (unless the engine was just made with it) and the scene back
    /// to its defaults (rust-engine.ts `reset`).
    pub reset: bool,
    /// `set_style` (the background, light or dark).
    pub style: bool,
    /// The scene's links from [`EngineInput::links`] (a view with `sim`), and a relayout if
    /// [`relayout`] says so.
    pub links: bool,
    /// Recompute the points' sizes (shown or not).
    pub sizes: bool,
    /// `set_points` (colours and sizes).
    pub points: bool,
    /// `set_highlight`.
    pub highlight: bool,
    /// `set_segments`.
    pub segments: bool,
    /// Fly to the selected text, if any.
    pub fly: bool,
    /// Frame the shown points (`frameVisible`).
    pub frame_visible: bool,
}

/// What the page's effects pushed for these changes (App.tsx lines 263–325 over
/// rust-engine.ts's setters), `sim`: the view has `sim` (links and hide-alone apply there
/// only). A new graph is everything, as every effect ran again once the engine drew it.
pub fn plan(changed: Changed, sim: bool) -> Plan {
    if changed.graph {
        return Plan {
            reset: true,
            style: true,
            links: sim,
            sizes: true,
            points: true,
            highlight: true,
            segments: true,
            fly: true,
            frame_visible: sim,
        };
    }
    let mut p = Plan::default();
    if changed.theme {
        p.style = true;
        p.points = true;
        p.segments = true;
    }
    if changed.colored {
        p.points = true;
    }
    let links = sim && changed.links;
    let hide_alone = sim && changed.hide_alone;
    if changed.visible || changed.available || links || hide_alone {
        p.sizes = true;
        p.points = true;
        p.highlight = true;
        p.segments = true;
    }
    if links || hide_alone {
        p.links |= links;
        p.frame_visible = true;
    }
    if changed.types || changed.selected {
        p.highlight = true;
        p.segments = true;
    }
    p.fly = changed.selected;
    p
}

/// Whether every link of `edges` (flat `[a, b, type, …]`) names a node of a graph of `n`:
/// links cut for another view are refused, not a panic of the adjacency.
pub fn links_fit(edges: &[u32], n: usize) -> bool {
    edges
        .as_chunks::<3>()
        .0
        .iter()
        .all(|&[a, b, _]| (a as usize) < n && (b as usize) < n)
}

/// The links a scene of `n` nodes takes from `at` (`setLinks`): `at`'s when they fit; links
/// cut for another view are refused, and the scene then has none — not the previous ones,
/// which the legend would no longer count. Returns whether they fit.
pub fn take_links(scene: &mut Scene, at: &LinksAt, n: usize) -> bool {
    let fits = links_fit(&at.links.edges, n);
    scene.set_links(if fits {
        at.links.clone()
    } else {
        Links::default()
    });
    fits
}

/// The engine's creation (rust-engine.ts `create`): where it stands. It is tried once: a
/// browser without WebGPU or WebGL2 keeps the no-GPU message, not one more GPU setup per
/// input change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Start {
    /// Not tried yet.
    #[default]
    Never,
    /// The GPU is being set up.
    Creating,
    /// The engine draws.
    Made,
    /// Neither WebGPU nor WebGL2.
    Failed,
}

impl Start {
    /// Whether to try now (never tried); the attempt is then under way.
    pub fn begin(&mut self) -> bool {
        if *self != Start::Never {
            return false;
        }
        *self = Start::Creating;
        true
    }

    /// The attempt ended, the engine made (`ok`) or not: either way, never tried again.
    pub fn end(&mut self, ok: bool) {
        *self = if ok { Start::Made } else { Start::Failed };
    }
}

// ---------- The component ----------

/// What the component tells the page.
#[derive(Clone, Copy)]
#[cfg_attr(
    not(all(feature = "browser", target_arch = "wasm32")),
    allow(dead_code, reason = "read by the browser part only")
)]
struct Outputs {
    hover: Option<Callback<Option<Hover>>>,
    select: Option<Callback<Option<usize>>>,
    ready: Option<Callback<String>>,
    no_gpu: Option<Callback<()>>,
}

/// The map, drawn by the engine on a canvas filling the nearest positioned ancestor, with
/// the no-GPU message over it when neither WebGPU nor WebGL2 is there (task 003). The
/// server renders the canvas alone; the browser starts the engine once `input` has a graph
/// and a theme, and keeps it for every later graph (App.tsx lines 210–242).
///
/// Input on the canvas (rust-engine.ts lines 298–364): one pointer rotates, a click (released
/// within [`CLICK_SLOP`] px) picks; two pinch to zoom; the wheel zooms; the arrow keys travel
/// (Shift: up and down), unless a field has the focus. The loop draws on demand: it sleeps
/// after [`IDLE_FRAMES`] still frames and any input or change wakes it.
#[component]
pub fn EngineView(
    /// What to draw.
    input: EngineInput,
    /// The text under the pointer, `None` when the pointer is over the empty map, presses,
    /// or leaves the canvas (rust-engine.ts `onHover`; not on touch).
    #[prop(optional, into)]
    on_hover: Option<Callback<Option<Hover>>>,
    /// A click on the map: the text clicked, `None` on the empty map — deselect
    /// (rust-engine.ts `onSelect`).
    #[prop(optional, into)]
    on_select: Option<Callback<Option<usize>>>,
    /// The engine draws: its graphics API, `"BrowserWebGpu"` or `"Gl"` (WebGL2), also set as
    /// `data-engine` on `<html>`.
    #[prop(optional, into)]
    on_ready: Option<Callback<String>>,
    /// The engine could not start (neither WebGPU nor WebGL2): the component shows the
    /// message; the page may hide what needs the map.
    #[prop(optional, into)]
    on_no_gpu: Option<Callback<()>>,
) -> impl IntoView {
    let canvas = NodeRef::<leptos::html::Canvas>::new();
    let no_gpu = RwSignal::new(false);
    let outputs = Outputs {
        hover: on_hover,
        select: on_select,
        ready: on_ready,
        no_gpu: on_no_gpu,
    };
    #[cfg(all(feature = "browser", target_arch = "wasm32"))]
    browser::start(canvas, input, outputs, no_gpu);
    #[cfg(not(all(feature = "browser", target_arch = "wasm32")))]
    let _ = (input, outputs);
    let t = strings();
    view! {
        <div class="absolute inset-0">
            <canvas node_ref=canvas class="absolute inset-0 block size-full"></canvas>
            <Show when=move || no_gpu.get()>
                <div role="alert" class="absolute inset-0 grid place-items-center p-6">
                    <div class="max-w-md text-center">
                        <p class="font-serif text-xl italic">
                            {move || td_string!(t.locale(), no_gpu_title)}
                        </p>
                        <p class="mt-3 text-sm text-muted">
                            {move || td_string!(t.locale(), no_gpu_body)}
                        </p>
                    </div>
                </div>
            </Show>
        </div>
    }
}

/// The browser side: the engine, the DOM events, the frame loop.
#[cfg(all(feature = "browser", target_arch = "wasm32"))]
mod browser {
    use std::sync::Arc;

    use leptos::html::Canvas;
    use leptos::prelude::*;
    use leptos::task::spawn_local;
    use map_engine::GraphView;
    use map_engine::live_layout::Core;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::{JsCast, JsError, JsValue};
    use web_sys::{
        AddEventListenerOptions, Event, EventTarget, HtmlCanvasElement, HtmlElement, KeyboardEvent,
        PointerEvent, ResizeObserver, WheelEvent,
    };

    use super::{
        EngineInput, Hover, Keys, LAYOUT_STEPS, Outputs, Plan, PointerDown, PointerMove, Pointers,
        Snapshot, Start, canvas_point, changed, has_area, is_typing, keep_looping,
        layout_continues, layout_links, picked, plan, relayout, take_links,
    };
    use crate::map::draw::{self, POINT_SIZE, Scene};
    use crate::map::model::Graph;

    /// The engine and what it was given: `RustMapEngine`'s fields (rust-engine.ts lines
    /// 31–49), the scene being M2's.
    struct Live {
        view: GraphView,
        graph: Arc<Graph>,
        scene: Scene,
        positions: Vec<f32>,
        sizes: Vec<f32>,
        radius: f32,
        /// The running layout and its steps so far.
        layout: Option<(Core, u32)>,
        /// The threshold the positions are for.
        laid_out_at: f64,
        applied: Option<Snapshot>,
    }

    impl Live {
        /// A view just made for `graph`'s positions.
        fn new(view: GraphView, graph: &Arc<Graph>) -> Live {
            let mut live = Live {
                view,
                graph: Arc::clone(graph),
                scene: Scene::new(graph),
                positions: draw::positions(graph),
                sizes: Vec::new(),
                radius: 1.0,
                layout: None,
                laid_out_at: 0.0,
                applied: None,
            };
            live.reset(graph);
            live
        }

        /// Everything that belongs to one graph, back to its defaults (rust-engine.ts lines
        /// 100–122, `reset`).
        fn reset(&mut self, graph: &Arc<Graph>) {
            let n = graph.nodes.len();
            self.graph = Arc::clone(graph);
            self.scene = Scene::new(graph);
            self.sizes = vec![POINT_SIZE; n];
            self.radius = draw::scene_radius(&self.positions);
            self.layout = None;
            self.laid_out_at = graph.sim.as_ref().map_or(0.0, |s| s.default);
            report(self.view.set_highlight(&vec![0.0; n], false));
        }

        /// Push what changed since the last input; returns whether anything did.
        fn sync(&mut self, want: &Snapshot, now: f64) -> bool {
            let sim = want.graph.sim.as_ref();
            let p = plan(changed(self.applied.as_ref(), want), sim.is_some());
            if p == Plan::default() {
                return false;
            }
            if p.reset {
                if !Arc::ptr_eq(&self.graph, &want.graph) {
                    // Another view on the same engine: no new GPU setup, the camera eases
                    // to the new points (rust-engine.ts lines 91–98, `setGraph`).
                    self.positions = draw::positions(&want.graph);
                    report(self.view.set_graph(&self.positions, now));
                }
                self.reset(&want.graph);
            }
            let relaid = self.take_input(want, p.links);
            self.push(p, want, now);
            if relaid {
                self.start_layout(sim.map_or([0.0, 1.0], |s| [s.min, s.max]));
            }
            if p.frame_visible {
                self.frame_visible(now);
            }
            self.applied = Some(want.clone());
            true
        }

        /// The input into the scene (the setters' first lines); with `links`, the links too,
        /// and whether they lay the points out again.
        fn take_input(&mut self, want: &Snapshot, links: bool) -> bool {
            let graph = &*want.graph;
            let n = graph.nodes.len();
            let scene = &mut self.scene;
            scene.colored.clone_from(&want.colored);
            scene.group_on.clone_from(&want.visible);
            scene.type_on.clone_from(&want.types);
            scene.available.clone_from(&want.available);
            scene.hide_alone = graph.sim.is_some() && want.hide_alone;
            scene.selected = want.selected.filter(|&i| i < n);
            let mut relaid = false;
            if links && let Some(at) = &want.links {
                if take_links(scene, at, n) {
                    relaid = relayout(graph.fixed, at.threshold, self.laid_out_at);
                    self.laid_out_at = at.threshold;
                } else {
                    warn("map engine: links for another view, refused: none drawn");
                }
            }
            relaid
        }

        /// The buffers and the camera moves of `p`.
        fn push(&mut self, p: Plan, want: &Snapshot, now: f64) {
            let graph = &*want.graph;
            if p.style {
                let [red, green, blue] = draw::rgb(&want.theme.background);
                self.view.set_style(red, green, blue, want.theme.light);
            }
            if p.sizes {
                self.sizes = self.scene.sizes(graph);
            }
            if p.points {
                let colors = self.scene.point_colors(graph, &want.theme);
                report(self.view.set_points(&colors, &self.sizes));
            }
            if p.highlight {
                let h = self.scene.highlight(graph);
                report(self.view.set_highlight(&h.hl, h.dim));
            }
            if p.segments {
                let s = self.scene.segments(graph, &want.theme);
                report(self.view.set_segments(s.ends, s.colors));
            }
            if p.fly
                && let Some(i) = self.scene.selected
            {
                let d = draw::fly_distance(&self.positions, &self.scene.adj, i, self.radius);
                self.view.fly_to(i, d, now);
            }
        }

        /// Lay the points out again from where they are (rust-engine.ts lines 367–385,
        /// `startLayout`).
        fn start_layout(&mut self, range: [f64; 2]) {
            let (ends, lengths) = layout_links(&self.scene, range);
            match Core::new(&self.positions, &ends, &lengths) {
                Ok(core) => self.layout = Some((core, 0)),
                Err(e) => warn(&format!("map engine: no layout: {e}")),
            }
        }

        /// Run the layout for this frame's budget and hand the positions to the view
        /// (rust-engine.ts lines 387–407, `stepLayout`); returns whether it still runs.
        fn step_layout(&mut self, clock: impl Fn() -> f64) -> bool {
            let Some((core, steps)) = self.layout.as_mut() else {
                return false;
            };
            let t0 = clock();
            while layout_continues(*steps, clock() - t0) {
                core.step(1);
                *steps += 1;
            }
            let done = *steps >= LAYOUT_STEPS;
            self.positions = core.positions();
            report(self.view.set_positions(&self.positions));
            if done {
                self.layout = None;
                self.frame_visible(clock());
                if let Some(body) = document().body() {
                    let _ = body.set_attribute("data-layout", "done");
                }
            }
            !done
        }

        /// Ease the camera to the shown points (rust-engine.ts lines 191–201).
        fn frame_visible(&mut self, now: f64) {
            if let Some(f) = draw::frame_visible(&self.positions, &self.sizes) {
                let [x, y, z] = f.center;
                self.view.fly_to_position(x, y, z, f.distance, now);
            }
        }
    }

    /// The frame loop's state.
    #[derive(Default)]
    struct Loop {
        running: bool,
        idle: u32,
        start: Start,
        disposed: bool,
    }

    /// An event listener, kept to be removed.
    struct Listener {
        target: EventTarget,
        kind: &'static str,
        f: Closure<dyn FnMut(Event)>,
    }

    /// What the component bound in the page.
    #[derive(Default)]
    struct Bound {
        listeners: Vec<Listener>,
        resize: Option<(ResizeObserver, Closure<dyn FnMut()>)>,
    }

    /// The component's state, every part a handle (`Copy`, and dead once it is disposed).
    #[derive(Clone, Copy)]
    struct Handles {
        live: StoredValue<Option<Live>, LocalStorage>,
        looping: StoredValue<Loop, LocalStorage>,
        pointers: StoredValue<Pointers, LocalStorage>,
        keys: StoredValue<Keys, LocalStorage>,
        bound: StoredValue<Bound, LocalStorage>,
        out: Outputs,
        no_gpu: RwSignal<bool>,
    }

    /// Start the engine once there is something to draw, keep it in step with `input`, stop
    /// it with the component (App.tsx lines 210–242 and 262–325).
    pub(super) fn start(
        canvas: NodeRef<Canvas>,
        input: EngineInput,
        out: Outputs,
        no_gpu: RwSignal<bool>,
    ) {
        let h = Handles {
            live: StoredValue::new_local(None),
            looping: StoredValue::new_local(Loop::default()),
            pointers: StoredValue::new_local(Pointers::default()),
            keys: StoredValue::new_local(Keys::default()),
            bound: StoredValue::new_local(Bound::default()),
            out,
            no_gpu,
        };
        Effect::new(move |_| {
            let want = read(input);
            let (Some(el), Some(want)) = (canvas.get(), want) else {
                return;
            };
            let synced = h
                .live
                .try_update_value(|live| live.as_mut().map(|l| l.sync(&want, now())));
            match synced {
                Some(Some(true)) => wake(h),
                Some(None) => create(h, el, want, input),
                _ => {}
            }
        });
        on_cleanup(move || stop(h));
    }

    /// The input, tracked; `None` before a graph and a theme.
    fn read(input: EngineInput) -> Option<Snapshot> {
        let graph = input.graph.get();
        let theme = input.theme.get();
        Some(Snapshot {
            graph: graph?,
            theme: theme?,
            links: input.links.get(),
            colored: input.colored.get(),
            visible: input.visible.get(),
            types: input.types.get(),
            available: input.available.get(),
            hide_alone: input.hide_alone.get(),
            selected: input.selected.get(),
        })
    }

    /// Make the engine on `el` for `want` (rust-engine.ts lines 51–89, `create`): bind the
    /// input, say which API draws, start the loop; or say there is no GPU.
    fn create(h: Handles, el: HtmlCanvasElement, want: Snapshot, input: EngineInput) {
        // Once: a failed start keeps the no-GPU message (not one GPU setup per input).
        if h.looping.try_update_value(|l| l.start.begin()) != Some(true) {
            return;
        }
        let rect = el.get_bounding_client_rect();
        let positions = draw::positions(&want.graph);
        spawn_local(async move {
            #[allow(clippy::cast_possible_truncation, reason = "CSS pixels fit f32")]
            let made = map_engine::create_graph_view(
                el.clone(),
                rect.width() as f32,
                rect.height() as f32,
                dpr(),
                positions,
            )
            .await;
            let ok = made.is_ok();
            if h.looping.try_update_value(|l| l.start.end(ok)).is_none() {
                return; // the component is gone; the view is dropped
            }
            match made {
                Ok(view) => ready(h, &el, view, want, input),
                Err(e) => {
                    web_sys::console::error_2(
                        &JsValue::from_str("map engine unavailable"),
                        &JsValue::from(e),
                    );
                    let _ = h.no_gpu.try_set(true);
                    if let Some(f) = h.out.no_gpu {
                        f.run(());
                    }
                }
            }
        });
    }

    /// The engine is made: give it the latest input, bind the input, say which API draws,
    /// start the loop.
    fn ready(
        h: Handles,
        el: &HtmlCanvasElement,
        view: GraphView,
        want: Snapshot,
        input: EngineInput,
    ) {
        let backend = view.backend();
        let mut live = Live::new(view, &want.graph);
        // The input may have moved while the GPU was set up.
        let latest = untrack(|| read(input)).unwrap_or(want);
        live.sync(&latest, now());
        if h.live.try_update_value(|l| *l = Some(live)).is_none() {
            return;
        }
        if let Some(html) = document().document_element() {
            // Which API draws: visible in the dev tools (App.tsx line 233).
            let _ = html.set_attribute("data-engine", &backend);
        }
        bind(h, el);
        wake(h);
        if let Some(f) = h.out.ready {
            f.run(backend);
        }
    }

    /// Run `f` on the engine, if it is there.
    fn with_live<R>(h: Handles, f: impl FnOnce(&mut Live) -> R) -> Option<R> {
        h.live.try_update_value(|l| l.as_mut().map(f)).flatten()
    }

    /// Draw frames again (any input or change), unless the loop runs.
    fn wake(h: Handles) {
        let start = h.looping.try_update_value(|l| {
            l.idle = 0;
            let start = !l.running && !l.disposed;
            l.running |= start;
            start
        });
        if start == Some(true) {
            request_animation_frame(move || tick(h));
        }
    }

    /// One frame: the layout's steps, then the engine's frame (rust-engine.ts lines 83–88);
    /// another one unless the map has been still for [`super::IDLE_FRAMES`] frames.
    fn tick(h: Handles) {
        let busy = with_live(h, |l| {
            let running = l.step_layout(now);
            let drew = l.view.frame(now());
            running || drew
        });
        let again = h.looping.try_update_value(|l| {
            let next = busy
                .and_then(|busy| keep_looping(busy, l.idle))
                .filter(|_| !l.disposed);
            l.idle = next.unwrap_or(0);
            l.running = next.is_some();
            l.running
        });
        if again == Some(true) {
            request_animation_frame(move || tick(h));
        }
    }

    /// Stop the loop, unbind, free the GPU (rust-engine.ts lines 208–215, `dispose`): the
    /// engine view dropped releases it at once (map-engine: `device.destroy()` under WebGPU,
    /// the WebGL2 context lost), and `<html>` no longer names an API drawing.
    fn stop(h: Handles) {
        h.looping.try_update_value(|l| l.disposed = true);
        h.bound.try_update_value(|b| {
            for l in b.listeners.drain(..) {
                let _ = l
                    .target
                    .remove_event_listener_with_callback(l.kind, l.f.as_ref().unchecked_ref());
            }
            if let Some((observer, _)) = b.resize.take() {
                observer.disconnect();
            }
        });
        h.live.try_update_value(|l| *l = None);
        if let Some(html) = document().document_element() {
            let _ = html.remove_attribute("data-engine");
        }
    }

    /// Listen to `kind` on `target` until the component goes (rust-engine.ts `listen`);
    /// `passive: Some(false)` lets the handler prevent the default (the wheel).
    fn listen(
        h: Handles,
        target: &EventTarget,
        kind: &'static str,
        passive: Option<bool>,
        f: impl FnMut(Event) + 'static,
    ) {
        let f = Closure::<dyn FnMut(Event)>::new(f);
        let added = match passive {
            Some(passive) => {
                let options = AddEventListenerOptions::new();
                options.set_passive(passive);
                target.add_event_listener_with_callback_and_add_event_listener_options(
                    kind,
                    f.as_ref().unchecked_ref(),
                    &options,
                )
            }
            None => target.add_event_listener_with_callback(kind, f.as_ref().unchecked_ref()),
        };
        if added.is_ok() {
            h.bound.update_value(|b| {
                b.listeners.push(Listener {
                    target: target.clone(),
                    kind,
                    f,
                });
            });
        }
    }

    /// The pointer on the canvas, CSS pixels from its top left.
    fn at(el: &HtmlCanvasElement, client: (f64, f64)) -> (f32, f32) {
        let b = el.get_bounding_client_rect();
        canvas_point(client, (b.left(), b.top()))
    }

    /// The event's client position.
    fn client(e: &PointerEvent) -> (f64, f64) {
        (f64::from(e.client_x()), f64::from(e.client_y()))
    }

    /// Say what is under the pointer.
    fn hover(h: Handles, what: Option<Hover>) {
        if let Some(f) = h.out.hover {
            f.run(what);
        }
    }

    /// The canvas' and the window's listeners, and the resize observer (rust-engine.ts lines
    /// 75–82, `bindPointer`, `bindKeys`).
    #[allow(
        clippy::too_many_lines,
        reason = "one listener per event, as the page bound them"
    )]
    fn bind(h: Handles, el: &HtmlCanvasElement) {
        let target: &EventTarget = el.as_ref();
        let c = el.clone();
        listen(h, target, "pointerdown", None, move |e| {
            let e: &PointerEvent = e.unchecked_ref();
            let _ = c.set_pointer_capture(e.pointer_id());
            let action = h
                .pointers
                .try_update_value(|p| p.down(e.pointer_id(), client(e)));
            hover(h, None);
            let pt = at(&c, client(e));
            with_live(h, |l| match action {
                Some(PointerDown::Rotate) => l.view.pointer_down(pt.0, pt.1),
                _ => l.view.pointer_up(),
            });
            wake(h);
        });
        let c = el.clone();
        listen(h, target, "pointermove", None, move |e| {
            let e: &PointerEvent = e.unchecked_ref();
            let touch = e.pointer_type() == "touch";
            let action = h
                .pointers
                .try_update_value(|p| p.moved(e.pointer_id(), client(e), touch));
            let pt = at(&c, client(e));
            match action {
                Some(PointerMove::Pinch(delta)) => {
                    if let Some(d) = delta {
                        with_live(h, |l| l.view.wheel(d));
                    }
                }
                Some(PointerMove::Drag { hover: true }) => {
                    let under = with_live(h, |l| {
                        l.view.pointer_move(pt.0, pt.1);
                        l.view.pick(pt.0, pt.1)
                    })
                    .and_then(picked);
                    let cursor = if under.is_some() { "pointer" } else { "" };
                    let _ = HtmlElement::style(&c).set_property("cursor", cursor);
                    let (cx, cy) = client(e);
                    hover(
                        h,
                        under.map(|index| Hover {
                            index,
                            x: cx,
                            y: cy,
                        }),
                    );
                }
                Some(PointerMove::Drag { hover: false }) => {
                    with_live(h, |l| l.view.pointer_move(pt.0, pt.1));
                }
                None => return,
            }
            wake(h);
        });
        for kind in ["pointerup", "pointercancel"] {
            let c = el.clone();
            listen(h, target, kind, None, move |e| {
                let e: &PointerEvent = e.unchecked_ref();
                let released = e.type_() == "pointerup";
                let click = h
                    .pointers
                    .try_update_value(|p| p.up(e.pointer_id(), client(e), released));
                with_live(h, |l| l.view.pointer_up());
                if click == Some(true) {
                    let pt = at(&c, client(e));
                    let under = with_live(h, |l| l.view.pick(pt.0, pt.1));
                    if let (Some(under), Some(f)) = (under, h.out.select) {
                        f.run(picked(under));
                    }
                }
                wake(h);
            });
        }
        listen(h, target, "pointerleave", None, move |_| hover(h, None));
        listen(h, target, "wheel", Some(false), move |e| {
            e.prevent_default();
            let e: &WheelEvent = e.unchecked_ref();
            #[allow(clippy::cast_possible_truncation, reason = "a wheel delta fits f32")]
            let d = e.delta_y() as f32;
            with_live(h, |l| l.view.wheel(d));
            wake(h);
        });

        let win: EventTarget = window().into();
        let keys = move |v: [f32; 3]| {
            with_live(h, |l| l.view.set_keys(v[0], v[1], v[2]));
            wake(h);
        };
        listen(h, &win, "keydown", None, move |e| {
            let e: &KeyboardEvent = e.unchecked_ref();
            let typing = typing();
            let out = h
                .keys
                .try_update_value(|k| k.down(&e.key(), e.shift_key(), typing));
            if let Some((prevent, v)) = out {
                if prevent {
                    e.prevent_default();
                }
                keys(v);
            }
        });
        listen(h, &win, "keyup", None, move |e| {
            let e: &KeyboardEvent = e.unchecked_ref();
            if let Some(v) = h.keys.try_update_value(|k| k.up(&e.key(), e.shift_key())) {
                keys(v);
            }
        });
        listen(h, &win, "blur", None, move |_| {
            if let Some(v) = h.keys.try_update_value(Keys::blur) {
                keys(v);
            }
        });

        let c = el.clone();
        let resized = Closure::<dyn FnMut()>::new(move || {
            let b = c.get_bounding_client_rect();
            if has_area(b.width(), b.height()) {
                #[allow(clippy::cast_possible_truncation, reason = "CSS pixels fit f32")]
                let (w, ht) = (b.width() as f32, b.height() as f32);
                with_live(h, |l| l.view.resize(w, ht, dpr()));
                wake(h);
            }
        });
        if let Ok(observer) = ResizeObserver::new(resized.as_ref().unchecked_ref()) {
            observer.observe(el);
            h.bound
                .update_value(|b| b.resize = Some((observer, resized)));
        }
    }

    /// Whether the focused element takes typing (rust-engine.ts lines 351–354).
    fn typing() -> bool {
        document().active_element().is_some_and(|a| {
            let editable = a
                .dyn_ref::<HtmlElement>()
                .is_some_and(HtmlElement::is_content_editable);
            is_typing(&a.tag_name(), editable)
        })
    }

    /// `performance.now()`, ms.
    fn now() -> f64 {
        window().performance().map_or(0.0, |p| p.now())
    }

    /// The device pixel ratio.
    #[allow(clippy::cast_possible_truncation, reason = "a pixel ratio fits f32")]
    fn dpr() -> f32 {
        window().device_pixel_ratio() as f32
    }

    /// Log an engine call that refused its input.
    fn report(r: Result<(), JsError>) {
        if let Err(e) = r {
            web_sys::console::error_2(&JsValue::from_str("map engine"), &JsValue::from(e));
        }
    }

    /// Log a warning.
    fn warn(message: &str) {
        web_sys::console::warn_1(&JsValue::from_str(message));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::draw::{Links, Scene};
    use crate::map::fixture;
    use std::collections::HashMap;

    // ---------- Input ----------

    #[test]
    fn a_pointer_is_placed_from_the_canvas_top_left() {
        // rust-engine.ts line 290: `[e.clientX - b.left, e.clientY - b.top]`.
        assert_eq!(canvas_point((130.5, 90.0), (30.5, 40.0)), (100.0, 50.0));
        assert_eq!(canvas_point((10.0, 10.0), (0.0, 0.0)), (10.0, 10.0));
    }

    #[test]
    fn a_pinch_is_a_wheel_delta_in_camera_units() {
        // rust-engine.ts line 317: `(100 * Math.log(pinch / s)) / 0.0513`.
        let out = pinch_wheel(200.0, 100.0).unwrap();
        assert!((out - (100.0 * 2f32.ln() / 0.0513)).abs() < 1e-3, "{out}");
        assert!(out > 0.0, "fingers closing zoom out (positive is out)");
        assert!(pinch_wheel(100.0, 200.0).unwrap() < 0.0, "opening zooms in");
        assert_eq!(pinch_wheel(100.0, 100.0), Some(0.0));
        // `if (pinch && s)`: no delta on the first move of a pinch or a zero spread.
        assert_eq!(pinch_wheel(0.0, 100.0), None);
        assert_eq!(pinch_wheel(100.0, 0.0), None);
    }

    #[test]
    fn a_pick_of_minus_one_is_the_empty_map() {
        assert_eq!(picked(-1), None);
        assert_eq!(picked(0), Some(0));
        assert_eq!(picked(42), Some(42));
    }

    #[test]
    fn a_canvas_without_area_is_not_resized() {
        assert!(has_area(800.0, 600.0));
        assert!(!has_area(0.0, 600.0));
        assert!(!has_area(800.0, 0.0));
    }

    #[test]
    fn one_pointer_rotates_and_a_still_release_is_a_click() {
        let mut p = Pointers::default();
        assert_eq!(p.down(1, (10.0, 10.0)), PointerDown::Rotate);
        // Pressed: no hover (line 322, `|| down.size`).
        assert_eq!(
            p.moved(1, (12.0, 12.0), false),
            PointerMove::Drag { hover: false }
        );
        // Within 4 px of the press (line 331): a click.
        assert!(p.up(1, (12.5, 12.0), true), "3.2 px from the press");
        // Released, a mouse moving hovers; a touch does not.
        assert_eq!(
            p.moved(1, (50.0, 50.0), false),
            PointerMove::Drag { hover: true }
        );
        assert_eq!(
            p.moved(1, (50.0, 50.0), true),
            PointerMove::Drag { hover: false }
        );
    }

    #[test]
    fn a_drag_or_a_cancel_is_not_a_click() {
        let mut p = Pointers::default();
        p.down(1, (10.0, 10.0));
        assert!(!p.up(1, (15.0, 10.0), true), "5 px is a drag");
        p.down(1, (10.0, 10.0));
        assert!(!p.up(1, (10.0, 10.0), false), "pointercancel never picks");
        p.down(1, (10.0, 10.0));
        assert!(p.up(1, (14.0, 10.0), true), "4 px is still a click");
    }

    #[test]
    fn two_pointers_pinch_and_their_release_is_no_click() {
        let mut p = Pointers::default();
        p.down(1, (0.0, 0.0));
        assert_eq!(p.down(2, (200.0, 0.0)), PointerDown::Pinch);
        // The spread goes from 200 to 100: zoom out by the pinch's wheel delta.
        assert_eq!(
            p.moved(2, (100.0, 0.0), true),
            PointerMove::Pinch(pinch_wheel(200.0, 100.0))
        );
        // Then from 100 to 100: no zoom.
        assert_eq!(
            p.moved(2, (100.0, 0.0), true),
            PointerMove::Pinch(Some(0.0))
        );
        assert!(!p.up(2, (100.0, 0.0), true), "two pointers were down");
        // One left: it rotates again, and its release is not a click either (it moved
        // nowhere, but `start` is the second pointer's press).
        assert_eq!(
            p.moved(1, (0.0, 0.0), true),
            PointerMove::Drag { hover: false }
        );
        assert!(!p.up(1, (0.0, 0.0), true));
    }

    /// 006 review R3: `start` was the last press, the second finger's; a pinch ending with
    /// the fingers together then picked a point.
    #[test]
    fn a_pinch_that_ends_never_selects() {
        let mut p = Pointers::default();
        p.down(1, (0.0, 0.0));
        p.down(2, (100.0, 0.0));
        // The fingers come together, near where the second pressed.
        p.moved(1, (98.0, 0.0), true);
        assert!(!p.up(2, (100.0, 0.0), true), "two pointers were down");
        assert!(!p.up(1, (99.0, 0.0), true), "the last finger of a pinch");
        // The first finger lifted first: the second's release is no click either.
        p.down(1, (0.0, 0.0));
        p.down(2, (100.0, 0.0));
        assert!(!p.up(1, (0.0, 0.0), true));
        assert!(!p.up(2, (100.0, 0.0), true), "released where it pressed");
        // The next press is a new gesture: a still release is a click again.
        p.down(1, (50.0, 50.0));
        assert!(p.up(1, (51.0, 50.0), true));
    }

    #[test]
    fn a_pinch_starting_from_one_spot_waits_for_a_spread() {
        let mut p = Pointers::default();
        p.down(1, (5.0, 5.0));
        p.down(2, (5.0, 5.0));
        // Spread 0 at the press: the first move gives no delta (line 317, `pinch && s`).
        assert_eq!(p.moved(2, (105.0, 5.0), true), PointerMove::Pinch(None));
        assert_eq!(p.spread(), 100.0);
    }

    #[test]
    fn a_move_of_a_pointer_not_pressed_does_not_count_as_one() {
        let mut p = Pointers::default();
        p.down(1, (0.0, 0.0));
        p.down(2, (100.0, 0.0));
        // Line 314: only the pressed pointers are tracked.
        assert_eq!(
            p.moved(3, (500.0, 0.0), false),
            PointerMove::Pinch(Some(0.0))
        );
    }

    #[test]
    fn typing_is_an_input_a_text_area_a_select_or_an_editable() {
        for tag in ["INPUT", "TEXTAREA", "SELECT"] {
            assert!(is_typing(tag, false), "{tag}");
        }
        assert!(is_typing("DIV", true));
        assert!(!is_typing("BODY", false));
        assert!(!is_typing("BUTTON", false));
    }

    #[test]
    fn arrows_travel_and_shift_turns_forward_into_up() {
        let mut k = Keys::default();
        // rust-engine.ts line 349: `setKeys(right - left, shift ? ud : 0, shift ? 0 : ud)`.
        assert_eq!(k.down("ArrowUp", false, false), (true, [0.0, 0.0, 1.0]));
        assert_eq!(k.down("ArrowLeft", false, false), (true, [-1.0, 0.0, 1.0]));
        assert_eq!(k.down("Shift", true, false), (false, [-1.0, 1.0, 0.0]));
        assert_eq!(k.up("ArrowLeft", true), [0.0, 1.0, 0.0]);
        assert_eq!(k.up("Shift", false), [0.0, 0.0, 1.0]);
        assert_eq!(k.down("ArrowDown", false, false), (true, [0.0, 0.0, 0.0]));
        assert_eq!(k.down("ArrowRight", false, false), (true, [1.0, 0.0, 0.0]));
        assert_eq!(k.blur(), [0.0; 3]);
    }

    #[test]
    fn arrows_while_typing_are_the_fields_own() {
        let mut k = Keys::default();
        // Line 357: not taken (no preventDefault, not held); the movement is pushed anyway.
        assert_eq!(k.down("ArrowUp", false, true), (false, [0.0; 3]));
        assert_eq!(k.down("a", false, false), (false, [0.0; 3]));
        assert_eq!(k.up("ArrowUp", false), [0.0; 3]);
    }

    // ---------- The loop and the layout ----------

    #[test]
    fn the_loop_sleeps_after_idle_frames_and_a_busy_frame_resets_it() {
        assert_eq!(keep_looping(true, 0), Some(0));
        assert_eq!(keep_looping(true, IDLE_FRAMES - 1), Some(0));
        assert_eq!(keep_looping(false, 0), Some(1));
        assert_eq!(keep_looping(false, IDLE_FRAMES - 2), Some(IDLE_FRAMES - 1));
        assert_eq!(keep_looping(false, IDLE_FRAMES - 1), None);
        // Past the count (a frame already requested when it slept): asleep, and a busy frame
        // still wakes it.
        assert_eq!(keep_looping(false, IDLE_FRAMES), None);
        assert_eq!(keep_looping(false, u32::MAX), None);
        assert_eq!(keep_looping(true, IDLE_FRAMES), Some(0));
    }

    #[test]
    fn the_layout_steps_within_its_budget_and_its_count() {
        assert!(layout_continues(0, 0.0));
        assert!(layout_continues(LAYOUT_STEPS - 1, 13.9));
        assert!(!layout_continues(LAYOUT_STEPS, 0.0));
        assert!(!layout_continues(10, LAYOUT_BUDGET_MS));
    }

    #[test]
    fn a_spring_is_shorter_the_more_similar_its_texts() {
        // rust-engine.ts line 378: `17.5 + (1 - t) * 30`, t the similarity in the range.
        let range = [0.29, 0.95];
        assert!((spring_length(Some(0.95), range) - 17.5).abs() < 1e-4);
        assert!((spring_length(Some(0.29), range) - 47.5).abs() < 1e-4);
        assert!((spring_length(Some(0.62), range) - 32.5).abs() < 1e-3);
        // Clamped to the range (line 377).
        assert!((spring_length(Some(1.0), range) - 17.5).abs() < 1e-4);
        assert!((spring_length(Some(0.1), range) - 47.5).abs() < 1e-4);
        // No similarity, or no width: the least similar, never NaN.
        assert!((spring_length(None, range) - 47.5).abs() < 1e-4);
        assert!((spring_length(Some(0.5), [0.5, 0.5]) - 47.5).abs() < 1e-4);
    }

    #[test]
    fn the_layout_springs_every_link_of_the_scene() {
        let g = fixture();
        let mut scene = Scene::new(&g);
        let mut sims = HashMap::new();
        sims.insert((0, 1), 0.95);
        scene.set_links(Links {
            edges: vec![0, 1, 0, 2, 3, 0],
            sims,
        });
        let (ends, lengths) = layout_links(&scene, [0.29, 0.95]);
        assert_eq!(ends, [0, 1, 2, 3]);
        assert_eq!(lengths.len(), 2);
        assert!((lengths[0] - 17.5).abs() < 1e-4);
        assert!((lengths[1] - 47.5).abs() < 1e-4, "2–3 has no similarity");
    }

    #[test]
    fn new_links_relayout_only_at_another_threshold_on_a_free_view() {
        assert!(relayout(None, 0.6, 0.56));
        assert!(relayout(Some(false), 0.6, 0.56));
        assert!(!relayout(None, 0.56, 0.56), "the positions are for it");
        assert!(!relayout(Some(true), 0.6, 0.56), "fixed: a projection");
    }

    #[test]
    fn links_for_another_view_do_not_fit() {
        assert!(links_fit(&[0, 4, 0, 3, 1, 1], 5));
        assert!(!links_fit(&[0, 5, 0], 5));
        assert!(!links_fit(&[5, 0, 0], 5));
        assert!(links_fit(&[], 0));
    }

    /// 006 review R3: a refused payload left the previous links in the scene, drawn, while
    /// the legend counted the new ones.
    #[test]
    fn refused_links_clear_the_scenes_old_ones() {
        let g = fixture();
        let mut scene = Scene::new(&g);
        let fit = LinksAt {
            threshold: 0.6,
            links: Links {
                edges: vec![0, 1, 0],
                sims: HashMap::new(),
            },
        };
        assert!(take_links(&mut scene, &fit, 5));
        assert_eq!(scene.edges, [0, 1, 0]);
        let other_view = LinksAt {
            threshold: 0.7,
            links: Links {
                edges: vec![0, 9, 0],
                sims: HashMap::new(),
            },
        };
        assert!(!take_links(&mut scene, &other_view, 5));
        assert!(scene.edges.is_empty(), "{:?}", scene.edges);
        assert_eq!(scene.link_counts(&g).iter().sum::<usize>(), 0);
        let segments = scene.segments(&g, &MapTheme::new(false, &g));
        assert_eq!(segments.ends, Vec::<u32>::new());
    }

    /// 006 review R3: a failed creation went back to "not tried", so every input change
    /// tried again.
    #[test]
    fn a_failed_start_is_not_retried() {
        let mut start = Start::default();
        assert!(start.begin());
        assert!(!start.begin(), "one attempt at a time");
        start.end(false);
        assert_eq!(start, Start::Failed);
        assert!(!start.begin(), "no GPU: never again");
        let mut made = Start::default();
        assert!(made.begin());
        made.end(true);
        assert_eq!(made, Start::Made);
        assert!(!made.begin());
    }

    // ---------- What to push ----------

    fn snapshot(graph: &Arc<Graph>) -> Snapshot {
        Snapshot {
            graph: Arc::clone(graph),
            theme: MapTheme::new(false, graph),
            links: None,
            colored: vec![false; 3],
            visible: vec![true; 3],
            types: vec![true; 2],
            available: None,
            hide_alone: false,
            selected: None,
        }
    }

    #[test]
    fn the_first_input_and_a_new_graph_change_everything() {
        let g = Arc::new(fixture());
        let s = snapshot(&g);
        let all = Changed {
            graph: true,
            ..Changed::default()
        };
        assert_eq!(changed(None, &s), all);
        // Another Arc, even of an equal graph, is another view.
        let other = snapshot(&Arc::new(fixture()));
        assert_eq!(changed(Some(&s), &other), all);
        assert_eq!(changed(Some(&s), &s.clone()), Changed::default());
    }

    #[test]
    fn each_input_is_its_own_change() {
        let graph = Arc::new(fixture());
        let before = snapshot(&graph);
        let mut after = before.clone();
        after.theme = MapTheme::new(true, &graph);
        after.colored[0] = true;
        after.visible[1] = false;
        after.types[0] = false;
        after.available = Some(vec![true; 5]);
        after.links = Some(Arc::new(LinksAt::default()));
        after.hide_alone = true;
        after.selected = Some(1);
        assert_eq!(
            changed(Some(&before), &after),
            Changed {
                graph: false,
                theme: true,
                colored: true,
                visible: true,
                types: true,
                available: true,
                links: true,
                hide_alone: true,
                selected: true,
            }
        );
        // Equal links in another Arc are no change (the page compared values).
        let mut one = before.clone();
        one.links = Some(Arc::new(LinksAt::default()));
        let mut two = before;
        two.links = Some(Arc::new(LinksAt::default()));
        assert!(!changed(Some(&one), &two).links);
    }

    #[test]
    fn a_new_graph_pushes_everything_and_frames_a_vectorial_view() {
        let c = Changed {
            graph: true,
            ..Changed::default()
        };
        let parallels = plan(c, false);
        assert_eq!(
            parallels,
            Plan {
                reset: true,
                style: true,
                links: false,
                sizes: true,
                points: true,
                highlight: true,
                segments: true,
                fly: true,
                frame_visible: false,
            }
        );
        // App.tsx lines 315–325 ran for a view with `sim` only.
        assert_eq!(
            plan(c, true),
            Plan {
                links: true,
                frame_visible: true,
                ..parallels
            }
        );
    }

    #[test]
    fn theme_colours_and_visibility_push_what_their_setters_did() {
        let only = |f: fn(&mut Changed)| {
            let mut c = Changed::default();
            f(&mut c);
            c
        };
        // setTheme (lines 125–132): style, points, segments.
        assert_eq!(
            plan(only(|c| c.theme = true), false),
            Plan {
                style: true,
                points: true,
                segments: true,
                ..Plan::default()
            }
        );
        // setColored (134–138): points.
        assert_eq!(
            plan(only(|c| c.colored = true), false),
            Plan {
                points: true,
                ..Plan::default()
            }
        );
        // setVisible, setAvailable (140–156): applyVisibility (231–238).
        let visibility = Plan {
            sizes: true,
            points: true,
            highlight: true,
            segments: true,
            ..Plan::default()
        };
        assert_eq!(plan(only(|c| c.visible = true), false), visibility);
        assert_eq!(plan(only(|c| c.available = true), false), visibility);
    }

    #[test]
    fn types_selection_links_and_hide_alone_push_what_their_setters_did() {
        let only = |f: fn(&mut Changed)| {
            let mut c = Changed::default();
            f(&mut c);
            c
        };
        let visibility = Plan {
            sizes: true,
            points: true,
            highlight: true,
            segments: true,
            ..Plan::default()
        };
        // setTypes (146–150): select(selected, false).
        assert_eq!(
            plan(only(|c| c.types = true), false),
            Plan {
                highlight: true,
                segments: true,
                ..Plan::default()
            }
        );
        // select(selected, true) (178–189).
        assert_eq!(
            plan(only(|c| c.selected = true), false),
            Plan {
                highlight: true,
                segments: true,
                fly: true,
                ..Plan::default()
            }
        );
        // setLinks (165–172) then setHideAlone's effect (App.tsx 323–325): frameVisible.
        assert_eq!(
            plan(only(|c| c.links = true), true),
            Plan {
                links: true,
                frame_visible: true,
                ..visibility
            }
        );
        assert_eq!(
            plan(only(|c| c.hide_alone = true), true),
            Plan {
                frame_visible: true,
                ..visibility
            }
        );
        // Neither applies to a view without `sim`.
        assert_eq!(plan(only(|c| c.links = true), false), Plan::default());
        assert_eq!(plan(only(|c| c.hide_alone = true), false), Plan::default());
        assert_eq!(plan(Changed::default(), true), Plan::default());
    }

    /// Written after the component; red by rendering the message from the start
    /// (`RwSignal::new(true)`) or dropping the canvas.
    #[cfg(feature = "ssr")]
    #[test]
    fn the_server_renders_the_canvas_alone() {
        let html = Owner::new().with(|| {
            let input = EngineInput {
                graph: Signal::stored(None),
                theme: Signal::stored(None),
                links: Signal::stored(None),
                colored: Signal::stored(Vec::new()),
                visible: Signal::stored(Vec::new()),
                types: Signal::stored(Vec::new()),
                available: Signal::stored(None),
                hide_alone: Signal::stored(false),
                selected: Signal::stored(None),
            };
            view! { <EngineView input=input /> }.to_html()
        });
        assert!(
            html.contains(r#"<canvas class="absolute inset-0 block size-full">"#),
            "{html}"
        );
        assert!(
            !html.contains("role=\"alert\""),
            "no message before the engine tried: {html}"
        );
    }
}
