//! `wgpu` + `glyphon` implementation of the Kai renderer.
//!
//! Quads are instanced rounded rectangles evaluated with a signed distance
//! field (`quad.wgsl`): radius, border, blur and a per-instance clip rect.
//! Text uses one glyphon `TextRenderer` per text batch, all sharing one
//! atlas, so quads and text interleave in painter's order inside a single
//! render pass.

use std::ops::Range;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glyphon::{
    Cache, Color as TextColor, Resolution, SwashCache, TextArea, TextAtlas, TextBounds,
    TextRenderer, Viewport,
};
use wgpu::{CurrentSurfaceTexture, SurfaceTexture};
use winit::window::Window;

use super::text::TextStore;
use super::{Batch, ClippedQuad, Color, DrawList, resolve};
use crate::geometry::Rect;

const MIN_INSTANCE_CAPACITY: usize = 256;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct QuadInstance {
    /// x, y, width, height in physical pixels.
    pub rect: [f32; 4],
    pub fill: [f32; 4],
    pub border: [f32; 4],
    /// left, top, right, bottom in physical pixels.
    pub clip: [f32; 4],
    /// radius, border width, blur (physical pixels), unused.
    pub params: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
struct Uniforms {
    surface: [f32; 2],
    _pad: [f32; 2],
}

/// Capacity (in instances) for a buffer that must hold `needed` instances.
pub fn grown_capacity(current: usize, needed: usize) -> usize {
    if needed <= current {
        current
    } else {
        needed.next_power_of_two().max(MIN_INSTANCE_CAPACITY)
    }
}

/// Convert logical quads to physical instances for a `width` × `height`
/// surface.
pub fn build_instances(
    quads: &[ClippedQuad],
    scale: f32,
    width: u32,
    height: u32,
    linear: bool,
) -> Vec<QuadInstance> {
    let color = |color: Color| {
        if linear {
            color.to_linear()
        } else {
            color.to_srgb()
        }
    };
    quads
        .iter()
        .map(|ClippedQuad { quad, clip }| {
            let clip = clip.map_or([0.0, 0.0, width as f32, height as f32], |clip| {
                [
                    clip.x * scale,
                    clip.y * scale,
                    (clip.x + clip.width) * scale,
                    (clip.y + clip.height) * scale,
                ]
            });
            QuadInstance {
                rect: [
                    quad.rect.x * scale,
                    quad.rect.y * scale,
                    quad.rect.width * scale,
                    quad.rect.height * scale,
                ],
                fill: color(quad.fill),
                border: color(quad.border),
                clip,
                params: [
                    quad.radius * scale,
                    quad.border_width * scale,
                    quad.blur * scale,
                    0.0,
                ],
            }
        })
        .collect()
}

fn text_bounds(rect: Option<Rect>, scale: f32, width: u32, height: u32) -> TextBounds {
    match rect {
        Some(rect) => TextBounds {
            left: (rect.x * scale).floor() as i32,
            top: (rect.y * scale).floor() as i32,
            right: ((rect.x + rect.width) * scale).ceil() as i32,
            bottom: ((rect.y + rect.height) * scale).ceil() as i32,
        },
        None => TextBounds {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
        },
    }
}

enum Step {
    Quads(Range<u32>),
    Text(usize),
}

enum Target {
    Surface {
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
    },
    /// Renders into textures created per frame and read back to memory.
    Offscreen,
}

pub struct WgpuRenderer {
    _instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    target: Target,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    linear_target: bool,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    text_renderers: Vec<TextRenderer>,
}

async fn request_device(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'static>>,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), String> {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: surface,
            force_fallback_adapter: false,
            ..Default::default()
        })
        .await
        .map_err(|error| format!("No se encontró un adaptador GPU compatible: {error}"))?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("Kai Desktop device"),
            ..Default::default()
        })
        .await
        .map_err(|error| format!("No se pudo crear el dispositivo GPU: {error}"))?;
    Ok((adapter, device, queue))
}

impl WgpuRenderer {
    pub async fn new(window: Arc<Window>) -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let size = window.inner_size();
        let surface = instance
            .create_surface(window)
            .map_err(|error| format!("No se pudo crear la superficie GPU: {error}"))?;
        let (adapter, device, queue) = request_device(&instance, Some(&surface)).await?;
        let info = adapter.get_info();
        eprintln!(
            "Adaptador GPU: {} ({:?}, {:?})",
            info.name, info.backend, info.device_type
        );
        let config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or_else(|| {
                "El adaptador no ofrece configuración para esta superficie".to_owned()
            })?;
        surface.configure(&device, &config);
        let (format, width, height) = (config.format, config.width, config.height);
        Ok(Self::with_device(
            instance,
            device,
            queue,
            Target::Surface { surface, config },
            format,
            width,
            height,
        ))
    }

    /// Renderer without a window, for snapshots of `width` × `height`
    /// physical pixels in RGBA8 sRGB.
    pub async fn new_offscreen(width: u32, height: u32) -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (_, device, queue) = request_device(&instance, None).await?;
        Ok(Self::with_device(
            instance,
            device,
            queue,
            Target::Offscreen,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            width.max(1),
            height.max(1),
        ))
    }

    fn with_device(
        instance: wgpu::Instance,
        device: wgpu::Device,
        queue: wgpu::Queue,
        target: Target,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Kai SDF quad shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../quad.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Kai SDF quad pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<QuadInstance>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x4,
                        1 => Float32x4,
                        2 => Float32x4,
                        3 => Float32x4,
                        4 => Float32x4
                    ],
                })],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Kai quad uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Kai quad bind group"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        let instance_buffer = create_instance_buffer(&device, MIN_INSTANCE_CAPACITY);
        let cache = Cache::new(&device);
        let viewport = Viewport::new(&device, &cache);
        let atlas = TextAtlas::new(&device, &queue, &cache, format);
        Self {
            _instance: instance,
            device,
            queue,
            target,
            format,
            width,
            height,
            pipeline,
            uniforms,
            bind_group,
            instance_buffer,
            instance_capacity: MIN_INSTANCE_CAPACITY,
            linear_target: format.is_srgb(),
            swash_cache: SwashCache::new(),
            viewport,
            atlas,
            text_renderers: Vec::new(),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.width = width;
        self.height = height;
        if let Target::Surface { surface, config } = &mut self.target {
            config.width = width;
            config.height = height;
            surface.configure(&self.device, config);
        }
    }

    /// Upload instances and shape text for one frame.
    fn prepare(
        &mut self,
        list: &DrawList,
        text: &mut TextStore,
        scale: f32,
    ) -> Result<Vec<Step>, String> {
        let (width, height) = (self.width, self.height);
        let batches = resolve(list);
        let mut instances = Vec::new();
        let mut steps = Vec::with_capacity(batches.len());
        let mut text_batches = Vec::new();
        for batch in &batches {
            match batch {
                Batch::Quads(quads) => {
                    let start = instances.len() as u32;
                    instances.extend(build_instances(
                        quads,
                        scale,
                        width,
                        height,
                        self.linear_target,
                    ));
                    steps.push(Step::Quads(start..instances.len() as u32));
                }
                Batch::Texts(texts) => {
                    steps.push(Step::Text(text_batches.len()));
                    text_batches.push(texts);
                }
            }
        }
        self.ensure_instance_capacity(instances.len());
        if !instances.is_empty() {
            self.queue
                .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));
        }
        self.queue.write_buffer(
            &self.uniforms,
            0,
            bytemuck::bytes_of(&Uniforms {
                surface: [width as f32, height as f32],
                _pad: [0.0; 2],
            }),
        );

        self.viewport
            .update(&self.queue, Resolution { width, height });
        while self.text_renderers.len() < text_batches.len() {
            self.text_renderers.push(TextRenderer::new(
                &mut self.atlas,
                &self.device,
                wgpu::MultisampleState::default(),
                None,
            ));
        }
        let (font_system, buffers) = text.parts();
        for (renderer, texts) in self.text_renderers.iter_mut().zip(&text_batches) {
            let areas = texts.iter().filter_map(|draw| {
                let buffer = buffers.get(draw.id)?;
                let Color { r, g, b, a } = draw.color;
                Some(TextArea {
                    buffer,
                    left: draw.origin.x * scale,
                    top: draw.origin.y * scale,
                    scale,
                    bounds: text_bounds(draw.clip, scale, width, height),
                    default_color: TextColor::rgba(r, g, b, a),
                    custom_glyphs: &[],
                })
            });
            renderer
                .prepare(
                    &self.device,
                    &self.queue,
                    font_system,
                    &mut self.atlas,
                    &self.viewport,
                    areas,
                    &mut self.swash_cache,
                )
                .map_err(|error| format!("No se pudo preparar el texto: {error}"))?;
        }
        Ok(steps)
    }

    fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        clear: Color,
        steps: Vec<Step>,
    ) -> Result<(), String> {
        let [r, g, b, a] = if self.linear_target {
            clear.to_linear()
        } else {
            clear.to_srgb()
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Kai Desktop quads and text"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: f64::from(r),
                        g: f64::from(g),
                        b: f64::from(b),
                        a: f64::from(a),
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        for step in steps {
            match step {
                Step::Quads(range) => {
                    pass.set_pipeline(&self.pipeline);
                    pass.set_bind_group(0, &self.bind_group, &[]);
                    pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
                    pass.draw(0..6, range);
                }
                Step::Text(index) => self.text_renderers[index]
                    .render(&self.atlas, &self.viewport, &mut pass)
                    .map_err(|error| format!("No se pudo dibujar el texto: {error}"))?,
            }
        }
        Ok(())
    }

    /// Present one frame on the window surface. Returns `Ok(false)` when the
    /// frame was skipped and should be requested again (lost/outdated
    /// surface).
    pub fn render(
        &mut self,
        list: &DrawList,
        text: &mut TextStore,
        scale: f32,
    ) -> Result<bool, String> {
        let steps = self.prepare(list, text, scale)?;
        let Target::Surface { surface, config } = &self.target else {
            return Err("El renderer no tiene superficie de ventana".to_owned());
        };
        let (frame, suboptimal): (SurfaceTexture, bool) = match surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) => (frame, false),
            CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            CurrentSurfaceTexture::Lost | CurrentSurfaceTexture::Outdated => {
                surface.configure(&self.device, config);
                return Ok(false);
            }
            CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => return Ok(true),
            CurrentSurfaceTexture::Validation => {
                return Err("Error de validación al adquirir el frame GPU".to_owned());
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Kai Desktop frame encoder"),
            });
        self.encode(&mut encoder, &view, list.clear, steps)?;
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        self.atlas.trim();
        if suboptimal {
            surface.configure(&self.device, config);
            return Ok(false);
        }
        Ok(true)
    }

    /// Render one frame offscreen and return its pixels as tightly packed
    /// RGBA8 (sRGB), row by row from the top.
    pub fn render_to_rgba(
        &mut self,
        list: &DrawList,
        text: &mut TextStore,
        scale: f32,
    ) -> Result<Vec<u8>, String> {
        let steps = self.prepare(list, text, scale)?;
        let (width, height) = (self.width, self.height);
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Kai snapshot"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let padded_row = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Kai snapshot readback"),
            size: u64::from(padded_row) * u64::from(height),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Kai snapshot encoder"),
            });
        self.encode(&mut encoder, &view, list.clear, steps)?;
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        readback.map_async(wgpu::MapMode::Read, .., |_| {});
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .map_err(|error| format!("No se pudo leer el snapshot: {error}"))?;
        let mapped = readback
            .get_mapped_range(..)
            .map_err(|error| format!("No se pudo mapear el snapshot: {error:?}"))?;
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in mapped.chunks(padded_row as usize) {
            pixels.extend_from_slice(&row[..(width * 4) as usize]);
        }
        drop(mapped);
        readback.unmap();
        self.atlas.trim();
        Ok(pixels)
    }

    fn ensure_instance_capacity(&mut self, needed: usize) {
        let capacity = grown_capacity(self.instance_capacity, needed);
        if capacity != self.instance_capacity {
            self.instance_buffer = create_instance_buffer(&self.device, capacity);
            self.instance_capacity = capacity;
        }
    }
}

fn create_instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Kai quad instances"),
        size: (capacity * std::mem::size_of::<QuadInstance>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

#[cfg(test)]
mod tests {
    use super::{MIN_INSTANCE_CAPACITY, build_instances, grown_capacity, text_bounds};
    use crate::geometry::Rect;
    use crate::render::{ClippedQuad, Color, Quad};

    #[test]
    fn instance_capacity_grows_to_power_of_two_and_never_shrinks() {
        assert_eq!(grown_capacity(256, 10), 256);
        assert_eq!(grown_capacity(256, 256), 256);
        assert_eq!(grown_capacity(256, 257), 512);
        assert_eq!(grown_capacity(512, 3000), 4096);
        assert_eq!(grown_capacity(0, 1), MIN_INSTANCE_CAPACITY);
    }

    #[test]
    fn quads_scale_to_physical_pixels_with_clip_and_color_space() {
        let quad = Quad {
            rect: Rect {
                x: 50.0,
                y: 25.0,
                width: 100.0,
                height: 50.0,
            },
            fill: Color::rgb(255, 128, 0),
            border: Color::rgb(0, 0, 0),
            border_width: 1.0,
            radius: 6.0,
            blur: 0.0,
        };
        let quads = [
            ClippedQuad { quad, clip: None },
            ClippedQuad {
                quad,
                clip: Some(Rect {
                    x: 10.0,
                    y: 20.0,
                    width: 30.0,
                    height: 40.0,
                }),
            },
        ];
        let linear = build_instances(&quads, 2.0, 800, 600, true);
        assert_eq!(linear[0].rect, [100.0, 50.0, 200.0, 100.0]);
        assert_eq!(linear[0].params, [12.0, 2.0, 0.0, 0.0]);
        assert_eq!(linear[0].clip, [0.0, 0.0, 800.0, 600.0]);
        assert_eq!(linear[1].clip, [20.0, 40.0, 80.0, 120.0]);
        assert!((linear[0].fill[1] - 0.2158).abs() < 0.001);

        let srgb = build_instances(&quads, 1.0, 800, 600, false);
        assert!((srgb[0].fill[1] - 128.0 / 255.0).abs() < 0.0001);
    }

    #[test]
    fn text_bounds_scale_clip_or_cover_surface() {
        let bounds = text_bounds(
            Some(Rect {
                x: 10.25,
                y: 5.0,
                width: 20.0,
                height: 10.0,
            }),
            2.0,
            800,
            600,
        );
        assert_eq!(
            (bounds.left, bounds.top, bounds.right, bounds.bottom),
            (20, 10, 61, 30)
        );
        let full = text_bounds(None, 2.0, 800, 600);
        assert_eq!((full.right, full.bottom), (800, 600));
    }
}
