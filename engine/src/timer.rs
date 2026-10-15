//! GPU time of the render pass, from timestamp queries, when the device has them (a measure for
//! benchmarks; nothing depends on it). One measure in flight at a time: start one, read it back
//! a few frames later, start the next.

use std::cell::Cell;
use std::rc::Rc;

const IDLE: u8 = 0;
const PENDING: u8 = 1;
const READY: u8 = 2;

/// Timestamp queries and their read-back.
pub struct Timer {
    queries: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
    state: Rc<Cell<u8>>,
    period_ns: f64,
    /// Last measured GPU time of a frame's pass, ms (NaN until the first one).
    pub last_ms: f64,
}

impl Timer {
    /// None when the device has no timestamp queries.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Timer> {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        let buffer = |label, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: 16,
                usage,
                mapped_at_creation: false,
            })
        };
        Some(Timer {
            queries: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("timer"),
                ty: wgpu::QueryType::Timestamp,
                count: 2,
            }),
            resolve: buffer(
                "timer resolve",
                wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            ),
            readback: buffer(
                "timer readback",
                wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            ),
            state: Rc::new(Cell::new(IDLE)),
            period_ns: f64::from(queue.get_timestamp_period()),
            last_ms: f64::NAN,
        })
    }

    /// Read a finished measure; true if this frame should start a new one.
    pub fn poll(&mut self) -> bool {
        if self.state.get() == READY {
            if let Ok(data) = self.readback.slice(..).get_mapped_range() {
                let t = |k: usize| bytemuck::pod_read_unaligned::<u64>(&data[k * 8..k * 8 + 8]);
                #[allow(
                    clippy::cast_precision_loss,
                    reason = "a tick difference over a frame is far below 2^52"
                )]
                let ticks = t(1).saturating_sub(t(0)) as f64;
                self.last_ms = ticks * self.period_ns / 1e6;
            }
            self.readback.unmap();
            self.state.set(IDLE);
        }
        self.state.get() == IDLE
    }

    /// The pass' timestamp writes.
    pub fn writes(&self) -> wgpu::RenderPassTimestampWrites<'_> {
        wgpu::RenderPassTimestampWrites {
            query_set: &self.queries,
            beginning_of_pass_write_index: Some(0),
            end_of_pass_write_index: Some(1),
        }
    }

    /// After the pass: copy the timestamps out.
    pub fn resolve(&self, enc: &mut wgpu::CommandEncoder) {
        enc.resolve_query_set(&self.queries, 0..2, &self.resolve, 0);
        enc.copy_buffer_to_buffer(&self.resolve, 0, &self.readback, 0, 16);
    }

    /// After the submit: map the read-back; it is read on a later `poll`.
    pub fn read(&self) {
        self.state.set(PENDING);
        let state = self.state.clone();
        self.readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| {
                state.set(if r.is_ok() { READY } else { IDLE });
            });
    }
}
