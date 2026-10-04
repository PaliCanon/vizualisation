// Points: one camera-facing quad per point (WebGPU has no point size), a disc with a glowing
// core on dark themes, a plain disc on light ones. Same maths as the three.js shader it
// replaces (suttasmap src/map/engine.ts). Lines: plain coloured segments.

struct Uniforms {
    view_proj: mat4x4f,
    view: mat4x4f,
    viewport: vec2f,  // physical pixels
    scale: f32,       // viewport height / 2
    min_size: f32,    // physical pixels
    size: f32,        // global size factor
    bright: f32,
    flat_disc: f32,   // 1: light themes, no glow
    dim: f32,         // 1: a point is selected, the others fade
};
@group(0) @binding(0) var<uniform> u: Uniforms;

struct Point {
    @location(0) pos: vec3f,
    @location(1) size: f32,  // 0: hidden
    @location(2) color: vec3f,
    @location(3) hl: f32,    // 2: selected, 1: neighbour of the selected, 0: other
};

struct PointOut {
    @builtin(position) clip: vec4f,
    @location(0) uv: vec2f,
    @location(1) color: vec3f,
    @location(2) alpha: f32,
};

@vertex
fn point_vs(@builtin(vertex_index) vi: u32, p: Point) -> PointOut {
    var corners = array<vec2f, 6>(
        vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0),
        vec2f(-1.0, -1.0), vec2f(1.0, 1.0), vec2f(-1.0, 1.0),
    );
    let c = corners[vi];
    let mv = u.view * vec4f(p.pos, 1.0);
    let boost = select(1.0, 2.2, p.hl > 1.5);
    let px = max(p.size * u.size * boost * u.scale / -mv.z, u.min_size * u.size);
    var clip = u.view_proj * vec4f(p.pos, 1.0);
    clip = vec4f(clip.xy + c * px / u.viewport * clip.w, clip.zw);
    if (p.size <= 0.0) {
        clip = vec4f(2.0, 2.0, 2.0, 1.0); // hidden: outside the clip volume
    }
    var o: PointOut;
    o.clip = clip;
    o.uv = c * 0.5;
    o.color = select(p.color, vec3f(0.4) + p.color * 0.8, p.hl > 1.5);
    o.alpha = select(1.0, 0.12, u.dim > 0.5 && p.hl < 0.5);
    return o;
}

@fragment
fn point_fs(o: PointOut) -> @location(0) vec4f {
    let d = length(o.uv);
    if (d > 0.5) {
        discard;
    }
    let core = 1.0 - smoothstep(0.0, 0.5, d);
    let glow = o.color * (0.55 + core * 0.9) * u.bright;
    let edge = 1.0 - smoothstep(0.4 + 0.05 * u.flat_disc, 0.5, d);
    return vec4f(mix(glow, o.color, u.flat_disc), o.alpha * edge);
}

struct LineOut {
    @builtin(position) clip: vec4f,
    @location(0) color: vec4f,
};

@vertex
fn line_vs(@location(0) pos: vec3f, @location(1) color: vec4f) -> LineOut {
    var o: LineOut;
    o.clip = u.view_proj * vec4f(pos, 1.0);
    o.color = color;
    return o;
}

@fragment
fn line_fs(o: LineOut) -> @location(0) vec4f {
    return o.color;
}
