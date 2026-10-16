//! The GPU side: device and canvas surface (WebGPU, else WebGL2), the two pipelines (points,
//! lines) and their buffers, one frame's draw.

use crate::check;
use crate::timer::Timer;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

const SAMPLES: u32 = 4; // MSAA, like three.js's `antialias: true`

/// The shaders' uniform block (see shaders.wgsl).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Uniforms {
    pub view_proj: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub viewport: [f32; 2],
    pub scale: f32,
    pub min_size: f32,
    pub size: f32,
    pub bright: f32,
    pub flat_disc: f32,
    pub dim: f32,
}

/// One point, as the vertex shader reads it.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct PointInstance {
    pub pos: [f32; 3],
    pub size: f32,
    pub color: [f32; 3],
    pub hl: f32,
}

/// One end of a line segment.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct LineVertex {
    pub pos: [f32; 3],
    pub color: [f32; 4],
}

/// Everything on the GPU.
pub struct Gpu {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    msaa: wgpu::TextureView,
    uniforms: wgpu::Buffer,
    bind: wgpu::BindGroup,
    points_pipe: wgpu::RenderPipeline,
    lines_add: wgpu::RenderPipeline,  // dark themes: additive
    lines_over: wgpu::RenderPipeline, // light themes: normal blending
    points: wgpu::Buffer,
    n_points: u32,
    lines: Option<(wgpu::Buffer, u32)>,
    pub backend: String,
    pub timer: Option<Timer>,
    /// Releases the GPU once everything above is dropped (fields drop in order): last.
    _release: Release,
}

/// What a dropped [`Gpu`] gives back at once, rather than when the browser collects it:
/// under WebGPU the device, destroyed (wgpu's drop does not destroy it); under WebGL2 the
/// canvas' context, lost (`WEBGL_lose_context`). And its place in `crate::live_views`.
struct Release {
    /// The device, under WebGPU only.
    webgpu: Option<wgpu::Device>,
    /// The canvas whose WebGL2 context is lost, under GL.
    canvas: web_sys::HtmlCanvasElement,
    /// Counted as a live view until dropped.
    _counted: crate::Counted,
}

impl Drop for Release {
    fn drop(&mut self) {
        if let Some(device) = self.webgpu.take() {
            device.destroy();
        } else {
            lose_webgl_context(&self.canvas);
        }
    }
}

/// Loses the canvas' WebGL2 context now (its GPU memory goes back), if it has one.
fn lose_webgl_context(canvas: &web_sys::HtmlCanvasElement) {
    use wasm_bindgen::JsCast;
    let Ok(Some(context)) = canvas.get_context("webgl2") else {
        return;
    };
    let Ok(context) = context.dyn_into::<web_sys::WebGl2RenderingContext>() else {
        return;
    };
    if let Ok(Some(extension)) = context.get_extension("WEBGL_lose_context") {
        extension
            .unchecked_into::<web_sys::WebglLoseContext>()
            .lose_context();
    }
}

const ALPHA: wgpu::BlendState = wgpu::BlendState::ALPHA_BLENDING;
const ADD: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent::OVER,
};

impl Gpu {
    /// Set up on `canvas`, `width`×`height` physical pixels.
    pub async fn new(
        canvas: web_sys::HtmlCanvasElement,
        width: u32,
        height: u32,
    ) -> Result<Gpu, String> {
        let instance = wgpu::util::new_instance_with_webgpu_detection(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        })
        .await;
        let kept = canvas.clone();
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| e.to_string())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())?;
        let backend = format!("{:?}", adapter.get_info().backend);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                required_features: adapter.features() & wgpu::Features::TIMESTAMP_QUERY,
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())?;
        let mut config = surface
            .get_default_config(&adapter, width.max(1), height.max(1))
            .ok_or("surface not supported")?;
        config.format = config.format.remove_srgb_suffix(); // colours are given as the shaders output them
        config.view_formats = vec![];
        surface.configure(&device, &config);
        let webgpu = adapter.get_info().backend == wgpu::Backend::BrowserWebGpu;
        let release = Release {
            webgpu: webgpu.then(|| device.clone()),
            canvas: kept,
            _counted: crate::Counted::new(),
        };
        Ok(Self::build(
            surface, device, queue, config, backend, release,
        ))
    }

    fn build(
        surface: wgpu::Surface<'static>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        config: wgpu::SurfaceConfiguration,
        backend: String,
        release: Release,
    ) -> Gpu {
        let (uniforms, layout, bind) = uniform_bind(&device);
        let pipes = Pipelines::new(&device, &layout, config.format);
        let msaa = msaa_view(&device, &config);
        let points = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("points"),
            size: 32,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Gpu {
            timer: None,
            surface,
            device,
            queue,
            config,
            msaa,
            uniforms,
            bind,
            points_pipe: pipes.points,
            lines_add: pipes.lines_add,
            lines_over: pipes.lines_over,
            points,
            n_points: 0,
            lines: None,
            backend,
            _release: release,
        }
    }

    /// Start or stop measuring frames' GPU time (when the device has timestamp queries).
    pub fn set_timing(&mut self, on: bool) {
        self.timer = if on {
            Timer::new(&self.device, &self.queue)
        } else {
            None
        };
    }

    /// New canvas size, physical pixels.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
        self.msaa = msaa_view(&self.device, &self.config);
    }

    /// Replace every point (same count or not).
    pub fn set_points(&mut self, pts: &[PointInstance]) {
        let bytes: &[u8] = bytemuck::cast_slice(pts);
        if self.points.size() < bytes.len() as u64 {
            self.points = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("points"),
                    contents: bytes,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
        } else {
            self.queue.write_buffer(&self.points, 0, bytes);
        }
        self.n_points = check::to_u32(pts.len());
    }

    /// Replace every line segment (2 vertices each).
    pub fn set_lines(&mut self, verts: &[LineVertex]) {
        self.lines = (!verts.is_empty()).then(|| {
            let buf = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("lines"),
                    contents: bytemuck::cast_slice(verts),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            (buf, check::to_u32(verts.len()))
        });
    }

    /// Draw one frame on `background`; `additive`: lines add up (dark themes). False when the
    /// surface had no frame to give (it is then reconfigured, and the next call draws).
    pub fn draw(&mut self, u: &Uniforms, background: [f64; 3], additive: bool) -> bool {
        self.queue
            .write_buffer(&self.uniforms, 0, bytemuck::bytes_of(u));
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return false;
            }
            _ => return false,
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let timing = self.timer.as_mut().is_some_and(Timer::poll);
        self.record_pass(&mut enc, &view, background, additive, timing);
        let timer = self.timer.as_ref().filter(|_| timing);
        if let Some(t) = timer {
            t.resolve(&mut enc);
        }
        self.queue.submit([enc.finish()]);
        if let Some(t) = timer {
            t.read();
        }
        self.queue.present(frame);
        true
    }

    /// Record the one render pass: clear to `background`, the lines, then the points.
    fn record_pass(
        &self,
        enc: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        background: [f64; 3],
        additive: bool,
        timing: bool,
    ) {
        let [r, g, b] = background;
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.msaa,
                resolve_target: Some(view),
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a: 1.0 }),
                    store: wgpu::StoreOp::Discard,
                },
            })],
            timestamp_writes: if timing {
                self.timer.as_ref().map(Timer::writes)
            } else {
                None
            },
            ..Default::default()
        });
        pass.set_bind_group(0, &self.bind, &[]);
        if let Some((buf, n)) = &self.lines {
            pass.set_pipeline(if additive {
                &self.lines_add
            } else {
                &self.lines_over
            });
            pass.set_vertex_buffer(0, buf.slice(..));
            pass.draw(0..*n, 0..1);
        }
        if self.n_points > 0 {
            pass.set_pipeline(&self.points_pipe);
            pass.set_vertex_buffer(0, self.points.slice(..));
            pass.draw(0..6, 0..self.n_points);
        }
    }
}

/// The uniform buffer, and the bind group (with its layout) that exposes it to both stages.
fn uniform_bind(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup) {
    let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("uniforms"),
        size: size_of::<Uniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniforms.as_entire_binding(),
        }],
    });
    (uniforms, layout, bind)
}

fn msaa_view(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("msaa"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: SAMPLES,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

struct Pipelines {
    points: wgpu::RenderPipeline,
    lines_add: wgpu::RenderPipeline,
    lines_over: wgpu::RenderPipeline,
}

impl Pipelines {
    fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> Pipelines {
        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders.wgsl"));
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(layout)],
            immediate_size: 0,
        });
        let point_attrs =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 2 => Float32x3, 3 => Float32];
        let line_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
        let points = pipeline(
            device,
            &pl,
            &shader,
            Stage {
                vs: "point_vs",
                fs: "point_fs",
                layout: wgpu::VertexBufferLayout {
                    array_stride: size_of::<PointInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &point_attrs,
                },
                topology: wgpu::PrimitiveTopology::TriangleList,
                blend: ALPHA,
                format,
            },
        );
        let line = |blend| {
            pipeline(
                device,
                &pl,
                &shader,
                Stage {
                    vs: "line_vs",
                    fs: "line_fs",
                    layout: wgpu::VertexBufferLayout {
                        array_stride: size_of::<LineVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &line_attrs,
                    },
                    topology: wgpu::PrimitiveTopology::LineList,
                    blend,
                    format,
                },
            )
        };
        Pipelines {
            points,
            lines_add: line(ADD),
            lines_over: line(ALPHA),
        }
    }
}

struct Stage<'a> {
    vs: &'a str,
    fs: &'a str,
    layout: wgpu::VertexBufferLayout<'a>,
    topology: wgpu::PrimitiveTopology,
    blend: wgpu::BlendState,
    format: wgpu::TextureFormat,
}

fn pipeline(
    device: &wgpu::Device,
    pl: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    s: Stage,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(s.vs),
        layout: Some(pl),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(s.vs),
            buffers: &[Some(s.layout)],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: s.topology,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: SAMPLES,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(s.fs),
            targets: &[Some(wgpu::ColorTargetState {
                format: s.format,
                blend: Some(s.blend),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview_mask: None,
        cache: None,
    })
}
