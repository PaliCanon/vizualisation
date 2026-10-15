//! The page-facing API: a `GraphView` the page drives (see the crate docs).

use crate::camera::Orbit;
use crate::check;
use crate::gpu::{Gpu, LineVertex, PointInstance, Uniforms};
use glam::{Vec2, Vec3};
use wasm_bindgen::prelude::*;

/// How points look.
struct Style {
    background: [f64; 3],
    light: bool, // light theme: flat discs, normal blending
    size: f32,
    bright: f32,
    point_size: f32, // world size of a point
}

/// A graph on a canvas.
#[wasm_bindgen]
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent dirty flags, one per GPU buffer to refresh; a bitset would hide them"
)]
pub struct GraphView {
    gpu: Gpu,
    orbit: Orbit,
    positions: Vec<Vec3>,
    points: Vec<PointInstance>,
    segments: Vec<u32>,       // pairs of point indices
    segment_colors: Vec<f32>, // rgba per segment
    style: Style,
    dim: bool,
    css: Vec2,
    dpr: f32,
    drag: Option<Vec2>,
    points_dirty: bool,
    lines_dirty: bool,
    redraw: bool, // something else changed (style, size, highlight): draw the next frame
}

/// Create a view on `canvas` (CSS size `width`×`height`, device pixel ratio `dpr`) for points
/// at `positions` (x, y, z per point).
///
/// # Errors
/// When the GPU, the canvas surface or a graphics adapter cannot be set up.
#[wasm_bindgen(js_name = createGraphView)]
pub async fn create_graph_view(
    canvas: web_sys::HtmlCanvasElement,
    width: f32,
    height: f32,
    dpr: f32,
    positions: Vec<f32>,
) -> Result<GraphView, JsError> {
    let dpr = dpr.min(2.0);
    let (w, h) = (check::px(width * dpr), check::px(height * dpr));
    canvas.set_width(w.max(1));
    canvas.set_height(h.max(1));
    let gpu = Gpu::new(canvas, w, h).await.map_err(|e| JsError::new(&e))?;
    let positions: Vec<Vec3> = positions
        .as_chunks::<3>()
        .0
        .iter()
        .map(|p| Vec3::from_array(*p))
        .collect();
    let (center, radius) = frame_of(&positions);
    let eye = center + Vec3::new(0.0, radius * 0.3, radius * 1.6);
    let points = positions.iter().map(|p| default_point(*p)).collect();
    Ok(GraphView {
        gpu,
        orbit: Orbit::new(center, eye, radius),
        positions,
        points,
        segments: vec![],
        segment_colors: vec![],
        style: Style {
            background: [0.0; 3],
            light: false,
            size: 1.6,
            bright: 1.3,
            point_size: 7.0,
        },
        dim: false,
        css: Vec2::new(width, height),
        dpr,
        drag: None,
        points_dirty: true,
        lines_dirty: true,
        redraw: true,
    })
}

/// A point before the page sets its look.
fn default_point(p: Vec3) -> PointInstance {
    PointInstance {
        pos: p.to_array(),
        size: 7.0,
        color: [1.0, 0.7, 0.2],
        hl: 0.0,
    }
}

/// A robust framing of `ps`: median centre, 90th-percentile radius.
fn frame_of(ps: &[Vec3]) -> (Vec3, f32) {
    if ps.is_empty() {
        return (Vec3::ZERO, 1.0);
    }
    let median = |k: usize| {
        let mut xs: Vec<f32> = ps.iter().map(|p| p[k]).collect();
        xs.sort_by(f32::total_cmp);
        xs[xs.len() / 2]
    };
    let c = Vec3::new(median(0), median(1), median(2));
    let mut d: Vec<f32> = ps.iter().map(|p| p.distance(c)).collect();
    d.sort_by(f32::total_cmp);
    (c, d[check::p90_index(d.len())].max(1.0))
}

#[wasm_bindgen]
impl GraphView {
    /// `"BrowserWebGpu"` or `"Gl"`.
    #[wasm_bindgen(getter)]
    pub fn backend(&self) -> String {
        self.gpu.backend.clone()
    }

    /// GPU time of the last measured frame, ms; NaN without timestamp queries (benchmarks).
    #[wasm_bindgen(getter, js_name = gpuMs)]
    pub fn gpu_ms(&self) -> f64 {
        self.gpu.timer.as_ref().map_or(f64::NAN, |t| t.last_ms)
    }

    /// Show other points (`positions`, x, y, z per point; any count) on the same view: points
    /// get default colours and sizes, segments are cleared, and the camera eases to frame them.
    /// Much cheaper than a new view: the GPU device, pipelines and canvas stay.
    ///
    /// # Errors
    /// When `positions` is not a whole number of (x, y, z) triples.
    #[wasm_bindgen(js_name = setGraph)]
    pub fn set_graph(&mut self, positions: &[f32], now: f64) -> Result<(), JsError> {
        check::len("positions", positions.len(), positions.len() / 3, 3)
            .map_err(|e| JsError::new(&e))?;
        self.positions = positions
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| Vec3::from_array(*p))
            .collect();
        self.points = self.positions.iter().map(|p| default_point(*p)).collect();
        self.segments.clear();
        self.segment_colors.clear();
        self.dim = false;
        let (center, radius) = frame_of(&self.positions);
        self.orbit.set_scene(center, radius);
        self.orbit.fly_to(center, radius * 1.63, now);
        self.points_dirty = true;
        self.lines_dirty = true;
        Ok(())
    }

    /// Measure the GPU time of frames (benchmarks; off by default, it costs a read-back).
    #[wasm_bindgen(js_name = setTiming)]
    pub fn set_timing(&mut self, on: bool) {
        self.gpu.set_timing(on);
    }

    /// Theme: background colour (0–1), light or dark.
    #[wasm_bindgen(js_name = setStyle)]
    pub fn set_style(&mut self, r: f64, g: f64, b: f64, light: bool) {
        self.style.background = [r, g, b];
        self.style.light = light;
        self.style.bright = if light { 1.0 } else { 1.3 };
        self.redraw = true;
    }

    /// Colour (r, g, b per point) and size (0: hidden) of every point.
    ///
    /// # Errors
    /// When `colors` or `sizes` do not have one entry (three for colours) per point.
    #[wasm_bindgen(js_name = setPoints)]
    pub fn set_points(&mut self, colors: &[f32], sizes: &[f32]) -> Result<(), JsError> {
        let n = self.points.len();
        check::len("colours", colors.len(), n, 3)
            .and(check::len("sizes", sizes.len(), n, 1))
            .map_err(|e| JsError::new(&e))?;
        for (i, p) in self.points.iter_mut().enumerate() {
            p.color.copy_from_slice(&colors[i * 3..i * 3 + 3]);
            p.size = sizes[i] * self.style.point_size / 7.0;
        }
        self.points_dirty = true;
        Ok(())
    }

    /// Highlight: 2 selected, 1 neighbour, 0 other; `dim` fades the others.
    ///
    /// # Errors
    /// When `hl` does not have one entry per point.
    #[wasm_bindgen(js_name = setHighlight)]
    pub fn set_highlight(&mut self, hl: &[f32], dim: bool) -> Result<(), JsError> {
        check::len("highlight", hl.len(), self.points.len(), 1).map_err(|e| JsError::new(&e))?;
        for (p, h) in self.points.iter_mut().zip(hl) {
            p.hl = *h;
        }
        self.dim = dim;
        self.points_dirty = true;
        Ok(())
    }

    /// Segments: pairs of point indices, and r, g, b, a per segment.
    ///
    /// # Errors
    /// When the ends are not pairs, a colour is missing, or an end is not a point.
    #[wasm_bindgen(js_name = setSegments)]
    pub fn set_segments(&mut self, ends: Vec<u32>, colors: Vec<f32>) -> Result<(), JsError> {
        check::segments(&ends, &colors, self.points.len()).map_err(|e| JsError::new(&e))?;
        self.segments = ends;
        self.segment_colors = colors;
        self.lines_dirty = true;
        Ok(())
    }

    /// Move the points (a running layout).
    ///
    /// # Errors
    /// When `positions` does not have three values per point.
    #[wasm_bindgen(js_name = setPositions)]
    pub fn set_positions(&mut self, positions: &[f32]) -> Result<(), JsError> {
        check::len("positions", positions.len(), self.points.len(), 3)
            .map_err(|e| JsError::new(&e))?;
        for (i, p) in self.positions.iter_mut().enumerate() {
            *p = Vec3::from_slice(&positions[i * 3..i * 3 + 3]);
            self.points[i].pos = p.to_array();
        }
        self.points_dirty = true;
        self.lines_dirty = true;
        Ok(())
    }

    /// The canvas' new CSS size.
    pub fn resize(&mut self, width: f32, height: f32, dpr: f32) {
        self.css = Vec2::new(width, height);
        self.dpr = dpr.min(2.0);
        self.gpu
            .resize(check::px(width * self.dpr), check::px(height * self.dpr));
        self.redraw = true;
    }

    /// Pointer pressed at (x, y), CSS pixels from the canvas' top left; stops the auto-rotation.
    #[wasm_bindgen(js_name = pointerDown)]
    pub fn pointer_down(&mut self, x: f32, y: f32) {
        self.drag = Some(Vec2::new(x, y));
        self.orbit.auto_rotate = false; // like OrbitControls: touching the map stops the turn
    }

    /// Pointer moved: rotates while pressed.
    #[wasm_bindgen(js_name = pointerMove)]
    pub fn pointer_move(&mut self, x: f32, y: f32) {
        if let Some(from) = self.drag {
            self.orbit.rotate(x - from.x, y - from.y, self.css.y);
            self.drag = Some(Vec2::new(x, y));
        }
    }

    /// Pointer released.
    #[wasm_bindgen(js_name = pointerUp)]
    pub fn pointer_up(&mut self) {
        self.drag = None;
    }

    /// Keys held, each -1, 0 or 1: `x` right - left, `y` up - down, `z` forward - back. The
    /// camera travels, it does not turn.
    #[wasm_bindgen(js_name = setKeys)]
    pub fn set_keys(&mut self, x: f32, y: f32, z: f32) {
        self.orbit.set_keys(x, y, z);
    }

    /// Wheel: positive zooms out.
    pub fn wheel(&mut self, delta_y: f32) {
        self.orbit.wheel(delta_y);
    }

    /// The point at (x, y) CSS pixels, or -1.
    pub fn pick(&self, x: f32, y: f32) -> i32 {
        let aspect = self.css.x / self.css.y.max(1.0);
        let vp = self.orbit.projection(aspect) * self.orbit.view();
        let scale = self.css.y / 2.0;
        let pts = &self.points;
        let s = self.style.size;
        let radius = |i: usize, depth: f32| {
            if pts[i].size <= 0.0 {
                0.0
            } else {
                (pts[i].size * s * scale / depth).max(2.0 * s) / 2.0
            }
        };
        crate::pick::pick(Vec2::new(x, y), self.css, vp, &self.positions, radius)
            .map_or(-1, check::point_id)
    }

    /// Ease the camera to point `i`, `distance` away; `now` in ms.
    #[wasm_bindgen(js_name = flyTo)]
    pub fn fly_to(&mut self, i: usize, distance: f32, now: f64) {
        if let Some(p) = self.positions.get(i) {
            self.orbit.fly_to(*p, distance, now);
        }
    }

    /// Ease the camera to look at (x, y, z) from `distance` away; `now` in ms.
    #[wasm_bindgen(js_name = flyToPosition)]
    pub fn fly_to_position(&mut self, x: f32, y: f32, z: f32, distance: f32, now: f64) {
        self.orbit.fly_to(Vec3::new(x, y, z), distance, now);
    }

    /// Call on every animation frame (`now`: requestAnimationFrame's time, ms). Draws only when
    /// something changed — the camera moved, or the points, segments, style or size — so a still
    /// map costs the GPU nothing. Returns whether it drew.
    pub fn frame(&mut self, now: f64) -> bool {
        let moving = self.orbit.update(now);
        if !(moving || self.redraw || self.points_dirty || self.lines_dirty) {
            return false;
        }
        self.upload();
        let u = self.uniforms();
        let drawn = self.gpu.draw(&u, self.style.background, !self.style.light);
        self.redraw = !drawn; // no frame from the surface: try again next time
        drawn
    }
}

impl GraphView {
    fn upload(&mut self) {
        if self.points_dirty {
            self.gpu.set_points(&self.points);
            self.points_dirty = false;
        }
        if self.lines_dirty {
            let verts: Vec<LineVertex> = self
                .segments
                .as_chunks::<2>()
                .0
                .iter()
                .zip(self.segment_colors.as_chunks::<4>().0)
                .flat_map(|(ab, &color)| ab.iter().map(move |&k| (k, color)))
                .map(|(k, color)| LineVertex {
                    pos: self.positions[k as usize].to_array(),
                    color,
                })
                .collect();
            self.gpu.set_lines(&verts);
            self.lines_dirty = false;
        }
    }

    fn uniforms(&self) -> Uniforms {
        let px = self.css * self.dpr;
        let view = self.orbit.view();
        Uniforms {
            view_proj: (self.orbit.projection(px.x / px.y.max(1.0)) * view).to_cols_array_2d(),
            view: view.to_cols_array_2d(),
            viewport: px.to_array(),
            scale: px.y / 2.0,
            min_size: 2.0 * self.dpr,
            size: self.style.size,
            bright: self.style.bright,
            flat_disc: if self.style.light { 1.0 } else { 0.0 },
            dim: if self.dim { 1.0 } else { 0.0 },
        }
    }
}
