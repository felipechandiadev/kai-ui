//! `wgpu` + `glyphon` implementation of the Kai renderer.
//!
//! Quads are instanced rounded rectangles evaluated with a signed distance
//! field (`quad.wgsl`): radius, border, blur and a per-instance clip rect.
//! Icons are quads in mask mode: their coverage comes from an `R8Unorm`
//! atlas of rasterized SVGs ([`MaskAtlas`]). Images are rounded rectangles
//! of `image.wgsl` sampling an RGBA atlas ([`ImageAtlas`], created with the
//! first image) with a linear filter. Text uses one glyphon
//! `TextRenderer` per text batch, all sharing one atlas, so quads and text
//! interleave in painter's order inside a single render pass.
//!
//! Colors are blended in gamma space, over sRGB-encoded values, as AppKit,
//! Core Animation and browsers do (`docs/dev/12-modelo-appkit.md` D8,
//! Paso 14): the surface and the snapshot target use a non-sRGB format and
//! glyphon runs in `ColorMode::Web`. `KAI_LINEAR_BLEND=1` (debug builds
//! only) brings back the sRGB formats and blending in linear light.
//!
//! A window background effect ([`crate::background`]) is evaluated by
//! `background.wgsl` into a texture at 1/8 of the surface resolution, only
//! when its parameters, time or size change, and stretched over the window
//! as the first draw of the frame. [`WgpuRenderer::redraw_background`]
//! presents a new frame of an animated effect reusing the prepared quads
//! and text of the previous one.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glyphon::{
    Cache, Color as TextColor, ColorMode, Resolution, SwashCache, TextArea, TextAtlas, TextBounds,
    TextRenderer, Viewport,
};
use wgpu::{CurrentSurfaceTexture, SurfaceTexture};
use winit::window::Window;

use super::icons::{ATLAS_SIZE, AtlasRect, MaskAtlas, MaskKey};
use super::images::{IMAGE_ATLAS_SIZE, ImageAtlas, ImageRegion};
use super::text::TextStore;
use super::{Batch, Bitmap, ClippedImage, ClippedQuad, Color, DrawList, Quad, resolve};
use crate::background::{AURORA_WHITEN, BackgroundKind, BackgroundLayer};
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
    /// radius, border width, blur (physical pixels), mask mode (0 or 1).
    pub params: [f32; 4],
    /// Mask region in the atlas: x, y, size in pixels, unused.
    pub uv: [f32; 4],
}

/// One image of `image.wgsl`, in physical pixels and atlas texels.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct ImageInstance {
    /// x, y, width, height in physical pixels.
    pub rect: [f32; 4],
    /// Region shown, in atlas texels: x, y, width, height.
    pub source: [f32; 4],
    /// left, top, right, bottom in physical pixels.
    pub clip: [f32; 4],
    /// radius (physical pixels), opacity, linear output (0/1), unused.
    pub params: [f32; 4],
    /// Whole atlas region of the image: x, y, width, height.
    pub region: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
struct Uniforms {
    surface: [f32; 2],
    _pad: [f32; 2],
}

/// Divisor of the surface size for the background effect texture.
pub const BACKGROUND_DOWNSCALE: u32 = 8;

/// `Params` of `background.wgsl`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct BackgroundUniforms {
    /// Window size in logical points.
    pub size: [f32; 2],
    pub time: f32,
    pub cycle: f32,
    pub base: [f32; 4],
    pub colors: [[f32; 4]; 4],
    /// Kind (1 mesh, 2 aurora, 3 glow), dark (0/1), blur, intensity.
    pub effect: [f32; 4],
    /// Effect texture size, surface size (physical pixels).
    pub target: [f32; 4],
    /// Linear output (0/1), dither amplitude, extra white over the aurora.
    pub flags: [f32; 4],
}

/// Size of the effect texture for a `width` × `height` surface.
pub fn background_texture_size(width: u32, height: u32) -> (u32, u32) {
    (
        width.div_ceil(BACKGROUND_DOWNSCALE).max(1),
        height.div_ceil(BACKGROUND_DOWNSCALE).max(1),
    )
}

/// Uniforms of `layer` on a `width` × `height` surface at `scale`.
pub fn background_uniforms(
    layer: &BackgroundLayer,
    width: u32,
    height: u32,
    scale: f32,
    linear: bool,
) -> BackgroundUniforms {
    let (texture_width, texture_height) = background_texture_size(width, height);
    let kind = match layer.kind {
        BackgroundKind::None | BackgroundKind::Mesh => 1.0,
        BackgroundKind::Aurora => 2.0,
        BackgroundKind::Glow => 3.0,
    };
    let scale = scale.max(f32::EPSILON);
    BackgroundUniforms {
        size: [width as f32 / scale, height as f32 / scale],
        time: layer.time,
        cycle: layer.cycle.max(0.1),
        base: layer.base.to_srgb(),
        colors: layer.colors.map(Color::to_srgb),
        effect: [
            kind,
            if layer.dark { 1.0 } else { 0.0 },
            layer.blur,
            layer.intensity,
        ],
        target: [
            texture_width as f32,
            texture_height as f32,
            width as f32,
            height as f32,
        ],
        flags: [
            if linear { 1.0 } else { 0.0 },
            1.0 / 255.0,
            if layer.dark { 0.0 } else { AURORA_WHITEN },
            0.0,
        ],
    }
}

/// Capacity (in instances) for a buffer that must hold `needed` instances.
pub fn grown_capacity(current: usize, needed: usize) -> usize {
    if needed <= current {
        current
    } else {
        needed.next_power_of_two().max(MIN_INSTANCE_CAPACITY)
    }
}

/// Atlas key of a masked quad at `scale`.
pub fn mask_key(quad: &Quad, scale: f32) -> Option<MaskKey> {
    let mask = quad.mask?;
    Some(MaskKey::new(
        mask.icon,
        quad.rect.width.min(quad.rect.height) * scale,
        mask.stroke_width,
    ))
}

/// Convert logical quads to physical instances for a `width` × `height`
/// surface. Masked quads snap to whole pixels so the mask maps 1:1; those
/// without a region in `masks` are skipped.
pub fn build_instances(
    quads: &[ClippedQuad],
    scale: f32,
    width: u32,
    height: u32,
    linear: bool,
    masks: &HashMap<MaskKey, AtlasRect>,
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
        .filter_map(|ClippedQuad { quad, clip }| {
            let clip = clip_pixels(*clip, scale, width, height);
            if let Some(key) = mask_key(quad, scale) {
                let region = masks.get(&key)?;
                let size = region.size as f32;
                let center_x = (quad.rect.x + quad.rect.width / 2.0) * scale;
                let center_y = (quad.rect.y + quad.rect.height / 2.0) * scale;
                return Some(QuadInstance {
                    rect: [
                        (center_x - size / 2.0).round(),
                        (center_y - size / 2.0).round(),
                        size,
                        size,
                    ],
                    fill: color(quad.fill),
                    border: [0.0; 4],
                    clip,
                    params: [0.0, 0.0, 0.0, 1.0],
                    uv: [region.x as f32, region.y as f32, size, 0.0],
                });
            }
            Some(QuadInstance {
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
                uv: [0.0; 4],
            })
        })
        .collect()
}

fn clip_pixels(clip: Option<Rect>, scale: f32, width: u32, height: u32) -> [f32; 4] {
    clip.map_or([0.0, 0.0, width as f32, height as f32], |clip| {
        [
            clip.x * scale,
            clip.y * scale,
            (clip.x + clip.width) * scale,
            (clip.y + clip.height) * scale,
        ]
    })
}

/// Convert logical images to physical instances for a `width` × `height`
/// surface; those without a region in `regions` are skipped.
pub fn build_image_instances(
    images: &[ClippedImage],
    scale: f32,
    width: u32,
    height: u32,
    linear: bool,
    regions: &HashMap<u64, ImageRegion>,
) -> Vec<ImageInstance> {
    images
        .iter()
        .filter_map(|ClippedImage { image, clip }| {
            let region = regions.get(&image.bitmap.id())?;
            let (region_width, region_height) = (region.width as f32, region.height as f32);
            Some(ImageInstance {
                rect: [
                    image.rect.x * scale,
                    image.rect.y * scale,
                    image.rect.width * scale,
                    image.rect.height * scale,
                ],
                source: [
                    region.x as f32 + image.source.x * region_width,
                    region.y as f32 + image.source.y * region_height,
                    image.source.width * region_width,
                    image.source.height * region_height,
                ],
                clip: clip_pixels(*clip, scale, width, height),
                params: [
                    image.radius * scale,
                    image.opacity.clamp(0.0, 1.0),
                    if linear { 1.0 } else { 0.0 },
                    0.0,
                ],
                region: [
                    region.x as f32,
                    region.y as f32,
                    region_width,
                    region_height,
                ],
            })
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
    Images(Range<u32>),
    Text(usize),
}

/// Pipeline of `image.wgsl`, its instances and the image atlas texture,
/// created with the first image.
struct ImagePass {
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    atlas: ImageAtlas,
    texture: Option<(wgpu::Texture, wgpu::BindGroup)>,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
}

impl ImagePass {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Kai image shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../image.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Kai image pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<ImageInstance>() as wgpu::BufferAddress,
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
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Kai image sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            pipeline,
            sampler,
            atlas: ImageAtlas::default(),
            texture: None,
            instance_buffer: create_buffer::<ImageInstance>(
                device,
                "Kai image instances",
                MIN_INSTANCE_CAPACITY,
            ),
            instance_capacity: MIN_INSTANCE_CAPACITY,
        }
    }

    /// Pack this frame's images, copy the new ones to the texture (made on
    /// first use) and upload `instances` built from their regions.
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        uniforms: &wgpu::Buffer,
        bitmaps: &[&Bitmap],
    ) -> HashMap<u64, ImageRegion> {
        if bitmaps.is_empty() {
            return HashMap::new();
        }
        let regions = self.atlas.prepare(bitmaps);
        let (texture, _) = self.texture.get_or_insert_with(|| {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Kai image atlas"),
                size: wgpu::Extent3d {
                    width: IMAGE_ATLAS_SIZE,
                    height: IMAGE_ATLAS_SIZE,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Kai image bind group"),
                layout: &self.pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniforms.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            (texture, bind_group)
        });
        for (region, pixels) in self.atlas.take_uploads() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: region.x,
                        y: region.y,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(region.width * 4),
                    rows_per_image: Some(region.height),
                },
                wgpu::Extent3d {
                    width: region.width,
                    height: region.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        regions
    }

    fn upload_instances(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instances: &[ImageInstance],
    ) {
        let capacity = grown_capacity(self.instance_capacity, instances.len());
        if capacity != self.instance_capacity {
            self.instance_buffer =
                create_buffer::<ImageInstance>(device, "Kai image instances", capacity);
            self.instance_capacity = capacity;
        }
        if !instances.is_empty() {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instances));
        }
    }

    fn draw(&self, pass: &mut wgpu::RenderPass<'_>, range: Range<u32>) {
        if let Some((_, bind_group)) = &self.texture {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
            pass.draw(0..6, range);
        }
    }
}

/// Effect texture and the bind group that samples it.
struct BackgroundTarget {
    view: wgpu::TextureView,
    composite_bind_group: wgpu::BindGroup,
    size: (u32, u32),
}

/// Pipelines and cached frame of the background effect.
struct BackgroundPass {
    effect_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    effect_bind_group: wgpu::BindGroup,
    sampler: wgpu::Sampler,
    target: Option<BackgroundTarget>,
    /// Uniforms of the frame in `target`.
    rendered: Option<BackgroundUniforms>,
    /// Uniforms of the frame being drawn; `None` without an effect.
    current: Option<BackgroundUniforms>,
}

const BACKGROUND_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

impl BackgroundPass {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Kai background shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../background.wgsl").into()),
        });
        let pipeline = |label, entry, format| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_full"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let effect_pipeline = pipeline("Kai background effect", "fs_effect", BACKGROUND_FORMAT);
        let composite_pipeline = pipeline("Kai background composite", "fs_composite", format);
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Kai background uniforms"),
            size: std::mem::size_of::<BackgroundUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let effect_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Kai background effect bind group"),
            layout: &effect_pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Kai background sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            effect_pipeline,
            composite_pipeline,
            uniforms,
            effect_bind_group,
            sampler,
            target: None,
            rendered: None,
            current: None,
        }
    }

    /// Select the frame to draw and make room for it; returns whether the
    /// effect texture must be rendered again.
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        uniforms: Option<BackgroundUniforms>,
    ) {
        self.current = uniforms;
        let Some(uniforms) = uniforms else {
            return;
        };
        let size = (uniforms.target[0] as u32, uniforms.target[1] as u32);
        if self
            .target
            .as_ref()
            .is_none_or(|target| target.size != size)
        {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Kai background effect"),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: BACKGROUND_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let composite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Kai background composite bind group"),
                layout: &self.composite_pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.uniforms.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            self.target = Some(BackgroundTarget {
                view,
                composite_bind_group,
                size,
            });
            self.rendered = None;
        }
        if self.rendered != Some(uniforms) {
            queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&uniforms));
        }
    }

    /// Render the effect texture if its frame changed.
    fn encode_effect(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let (Some(uniforms), Some(target)) = (self.current, &self.target) else {
            return;
        };
        if self.rendered == Some(uniforms) {
            return;
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Kai background effect"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.effect_pipeline);
        pass.set_bind_group(0, &self.effect_bind_group, &[]);
        pass.draw(0..3, 0..1);
        drop(pass);
        self.rendered = Some(uniforms);
    }

    /// Stretch the effect over the window (first draw of the frame).
    fn composite(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let (Some(_), Some(target)) = (self.current, &self.target) {
            pass.set_pipeline(&self.composite_pipeline);
            pass.set_bind_group(0, &target.composite_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
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
    masks: MaskAtlas,
    mask_texture: wgpu::Texture,
    background: BackgroundPass,
    images: ImagePass,
    /// Prepared draws of the last frame, for [`Self::redraw_background`].
    steps: Vec<Step>,
    clear: Color,
    scale: f32,
    /// Whether `steps` match the instance buffer and text renderers.
    prepared: bool,
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
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or_else(|| {
                "El adaptador no ofrece configuración para esta superficie".to_owned()
            })?;
        // Gamma blending: write sRGB-encoded values (`Bgra8Unorm` on Metal).
        let gamma = config.format.remove_srgb_suffix();
        let formats = surface.get_capabilities(&adapter).formats;
        if !linear_blend_forced() && formats.contains(&gamma) {
            config.format = gamma;
        }
        // Explicit; on Metal the layer keeps `colorspace = nil` (Paso 15).
        config.color_space = wgpu::SurfaceColorSpace::Srgb;
        eprintln!(
            "Superficie {:?}: mezcla en {}",
            config.format,
            if config.format.is_srgb() {
                "lineal"
            } else {
                "gamma"
            }
        );
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
    /// physical pixels in RGBA8 sRGB. Renders into `Rgba8Unorm` (gamma
    /// blending, like the window), so the bytes need no GPU conversion.
    pub async fn new_offscreen(width: u32, height: u32) -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (_, device, queue) = request_device(&instance, None).await?;
        Ok(Self::with_device(
            instance,
            device,
            queue,
            Target::Offscreen,
            if linear_blend_forced() {
                wgpu::TextureFormat::Rgba8UnormSrgb
            } else {
                wgpu::TextureFormat::Rgba8Unorm
            },
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
                        4 => Float32x4,
                        5 => Float32x4
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
        let mask_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Kai icon masks"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let mask_view = mask_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Kai quad bind group"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&mask_view),
                },
            ],
        });
        let instance_buffer = create_instance_buffer(&device, MIN_INSTANCE_CAPACITY);
        let cache = Cache::new(&device);
        let viewport = Viewport::new(&device, &cache);
        // `Web`: sRGB colors and blending on a non-sRGB target; `Accurate`
        // there would encode the text twice and wash it out.
        let color_mode = if format.is_srgb() {
            ColorMode::Accurate
        } else {
            ColorMode::Web
        };
        let atlas = TextAtlas::with_color_mode(&device, &queue, &cache, format, color_mode);
        let background = BackgroundPass::new(&device, format);
        let images = ImagePass::new(&device, format);
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
            masks: MaskAtlas::default(),
            mask_texture,
            background,
            images,
            steps: Vec::new(),
            clear: Color::rgb(0, 0, 0),
            scale: 1.0,
            prepared: false,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.width = width;
        self.height = height;
        self.prepared = false;
        if let Target::Surface { surface, config } = &mut self.target {
            config.width = width;
            config.height = height;
            surface.configure(&self.device, config);
        }
    }

    /// Upload instances and shape text for one frame.
    fn prepare(&mut self, list: &DrawList, text: &mut TextStore, scale: f32) -> Result<(), String> {
        self.prepared = false;
        self.clear = list.clear;
        self.scale = scale;
        self.prepare_background(list.background.as_ref());
        let (width, height) = (self.width, self.height);
        let batches = resolve(list);
        let keys: Vec<MaskKey> = batches
            .iter()
            .filter_map(|batch| match batch {
                Batch::Quads(quads) => Some(quads),
                Batch::Images(_) | Batch::Texts(_) => None,
            })
            .flatten()
            .filter_map(|clipped| mask_key(&clipped.quad, scale))
            .collect();
        let masks = self.masks.prepare(&keys);
        self.upload_masks();
        let mut bitmaps: Vec<&Bitmap> = Vec::new();
        for batch in &batches {
            if let Batch::Images(images) = batch {
                for clipped in images {
                    if !bitmaps.contains(&&clipped.image.bitmap) {
                        bitmaps.push(&clipped.image.bitmap);
                    }
                }
            }
        }
        let regions = self
            .images
            .prepare(&self.device, &self.queue, &self.uniforms, &bitmaps);
        let mut instances = Vec::new();
        let mut image_instances = Vec::new();
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
                        &masks,
                    ));
                    steps.push(Step::Quads(start..instances.len() as u32));
                }
                Batch::Images(images) => {
                    let start = image_instances.len() as u32;
                    image_instances.extend(build_image_instances(
                        images,
                        scale,
                        width,
                        height,
                        self.linear_target,
                        &regions,
                    ));
                    steps.push(Step::Images(start..image_instances.len() as u32));
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
        self.images
            .upload_instances(&self.device, &self.queue, &image_instances);
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
        self.steps = steps;
        self.prepared = true;
        Ok(())
    }

    fn prepare_background(&mut self, layer: Option<&BackgroundLayer>) {
        let uniforms = layer.map(|layer| {
            background_uniforms(
                layer,
                self.width,
                self.height,
                self.scale,
                self.linear_target,
            )
        });
        self.background.prepare(&self.device, &self.queue, uniforms);
    }

    fn encode(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) -> Result<(), String> {
        self.background.encode_effect(encoder);
        let clear = self.clear;
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
        self.background.composite(&mut pass);
        for step in &self.steps {
            match step {
                Step::Quads(range) => {
                    pass.set_pipeline(&self.pipeline);
                    pass.set_bind_group(0, &self.bind_group, &[]);
                    pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
                    pass.draw(0..6, range.clone());
                }
                Step::Images(range) => self.images.draw(&mut pass, range.clone()),
                Step::Text(index) => self.text_renderers[*index]
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
        self.prepare(list, text, scale)?;
        self.present()
    }

    /// Present the last prepared frame again over `background` (a new
    /// frame of an animated effect), without preparing quads or text.
    /// Returns `Ok(false)` when there is nothing prepared (or the frame was
    /// skipped): the caller should render a full frame.
    pub fn redraw_background(
        &mut self,
        background: Option<&BackgroundLayer>,
    ) -> Result<bool, String> {
        if !self.prepared {
            return Ok(false);
        }
        self.prepare_background(background);
        self.present()
    }

    fn present(&mut self) -> Result<bool, String> {
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
        self.encode(&mut encoder, &view)?;
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        self.atlas.trim();
        if suboptimal {
            if let Target::Surface { surface, config } = &self.target {
                surface.configure(&self.device, config);
            }
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
        self.prepare(list, text, scale)?;
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
        self.encode(&mut encoder, &view)?;
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

    fn upload_masks(&mut self) {
        for (region, mask) in self.masks.take_uploads() {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.mask_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: region.x,
                        y: region.y,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &mask,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(region.size),
                    rows_per_image: Some(region.size),
                },
                wgpu::Extent3d {
                    width: region.size,
                    height: region.size,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    fn ensure_instance_capacity(&mut self, needed: usize) {
        let capacity = grown_capacity(self.instance_capacity, needed);
        if capacity != self.instance_capacity {
            self.instance_buffer = create_instance_buffer(&self.device, capacity);
            self.instance_capacity = capacity;
        }
    }
}

/// `KAI_LINEAR_BLEND=1` in debug builds: sRGB formats and linear blending,
/// to compare with the old rendering (Paso 19, diagnostics).
fn linear_blend_forced() -> bool {
    cfg!(debug_assertions) && std::env::var("KAI_LINEAR_BLEND").as_deref() == Ok("1")
}

fn create_instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
    create_buffer::<QuadInstance>(device, "Kai quad instances", capacity)
}

fn create_buffer<T>(device: &wgpu::Device, label: &str, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: (capacity * std::mem::size_of::<T>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{MIN_INSTANCE_CAPACITY, build_instances, grown_capacity, mask_key, text_bounds};
    use crate::geometry::Rect;
    use crate::icons::Icon;
    use crate::render::icons::{AtlasRect, MaskKey};
    use crate::render::{ClippedQuad, Color, Mask, Quad};

    #[test]
    fn background_uniforms_describe_the_layer_at_one_eighth_resolution() {
        use crate::background::{BackgroundEffect, BackgroundKind, BackgroundMotion};
        use crate::theme::Appearance;
        assert_eq!(super::background_texture_size(2048, 1440), (256, 180));
        assert_eq!(super::background_texture_size(2049, 3), (257, 1));
        let effect =
            BackgroundEffect::new(BackgroundKind::Aurora).with_motion(BackgroundMotion::Animated);
        let layer = crate::background::BackgroundLayer::new(
            &effect,
            Color::hex(0xffffff),
            Appearance::Light,
        )
        .unwrap()
        .at(2.5);
        let u = super::background_uniforms(&layer, 2048, 1440, 2.0, false);
        assert_eq!(u.size, [1024.0, 720.0]);
        assert_eq!((u.time, u.cycle), (2.5, 16.0));
        assert_eq!(u.effect, [2.0, 0.0, 80.0, 1.0]);
        assert_eq!(u.target, [256.0, 180.0, 2048.0, 1440.0]);
        assert_eq!(u.base, [1.0; 4]);
        assert_eq!(u.flags[0], 0.0);
        assert_eq!(
            u.flags[2],
            crate::background::AURORA_WHITEN,
            "light aurora whitens"
        );
        let dark = crate::background::BackgroundLayer::new(
            &effect,
            Color::hex(0x1e1e1e),
            Appearance::Dark,
        )
        .unwrap();
        assert_eq!(
            super::background_uniforms(&dark, 2048, 1440, 2.0, false).flags[2],
            0.0
        );
        // Same layer, same uniforms: the cached effect texture is reused.
        assert_eq!(
            u,
            super::background_uniforms(&layer, 2048, 1440, 2.0, false)
        );
        assert_ne!(
            u,
            super::background_uniforms(&layer.at(2.6), 2048, 1440, 2.0, false)
        );
    }

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
            mask: None,
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
        let none = HashMap::new();
        let linear = build_instances(&quads, 2.0, 800, 600, true, &none);
        assert_eq!(linear[0].rect, [100.0, 50.0, 200.0, 100.0]);
        assert_eq!(linear[0].params, [12.0, 2.0, 0.0, 0.0]);
        assert_eq!(linear[0].clip, [0.0, 0.0, 800.0, 600.0]);
        assert_eq!(linear[1].clip, [20.0, 40.0, 80.0, 120.0]);
        assert!((linear[0].fill[1] - 0.2158).abs() < 0.001);

        let srgb = build_instances(&quads, 1.0, 800, 600, false, &none);
        assert!((srgb[0].fill[1] - 128.0 / 255.0).abs() < 0.0001);
        assert_eq!(srgb[0].uv, [0.0; 4]);
    }

    #[test]
    fn masked_quads_snap_to_pixels_and_point_at_their_atlas_region() {
        let icon = Quad::icon(
            Rect {
                x: 10.3,
                y: 4.0,
                width: 16.0,
                height: 16.0,
            },
            Color::rgb(0, 222, 239),
            Mask {
                icon: Icon::Search,
                stroke_width: 2.0,
            },
        );
        let key = mask_key(&icon, 2.0).unwrap();
        assert_eq!(key, MaskKey::new(Icon::Search, 32.0, 2.0));
        let quads = [ClippedQuad {
            quad: icon,
            clip: None,
        }];
        assert!(
            build_instances(&quads, 2.0, 800, 600, true, &HashMap::new()).is_empty(),
            "masks missing from the atlas are skipped"
        );
        let masks = HashMap::from([(
            key,
            AtlasRect {
                x: 5,
                y: 7,
                size: 32,
            },
        )]);
        let instance = build_instances(&quads, 2.0, 800, 600, true, &masks)[0];
        assert_eq!(instance.rect, [21.0, 8.0, 32.0, 32.0]);
        assert_eq!(instance.uv, [5.0, 7.0, 32.0, 0.0]);
        assert_eq!(instance.params[3], 1.0);
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

    /// Offscreen render of `list` at scale 1 (needs a GPU).
    fn render(
        list: &crate::render::DrawList,
        text: &mut crate::render::text::TextStore,
        size: u32,
    ) -> Vec<u8> {
        let mut renderer = pollster::block_on(super::WgpuRenderer::new_offscreen(size, size))
            .expect("offscreen renderer");
        renderer.render_to_rgba(list, text, 1.0).expect("render")
    }

    #[test]
    fn images_scale_to_pixels_and_map_their_crop_into_the_atlas() {
        use crate::render::images::{ImageRegion, test_pattern};
        use crate::render::{ClippedImage, ImageDraw};
        let photo = test_pattern(4, 4);
        let image = ImageDraw {
            source: Rect {
                x: 0.25,
                y: 0.0,
                width: 0.5,
                height: 1.0,
            },
            radius: 8.0,
            opacity: 0.5,
            ..ImageDraw::new(
                Rect {
                    x: 10.0,
                    y: 20.0,
                    width: 30.0,
                    height: 40.0,
                },
                photo.clone(),
            )
        };
        let images = [ClippedImage { image, clip: None }];
        assert!(
            super::build_image_instances(&images, 2.0, 800, 600, false, &HashMap::new()).is_empty(),
            "images missing from the atlas are skipped"
        );
        let regions = HashMap::from([(
            photo.id(),
            ImageRegion {
                x: 101,
                y: 1,
                width: 200,
                height: 100,
            },
        )]);
        let instance = super::build_image_instances(&images, 2.0, 800, 600, true, &regions)[0];
        assert_eq!(instance.rect, [20.0, 40.0, 60.0, 80.0]);
        assert_eq!(instance.source, [151.0, 1.0, 100.0, 100.0]);
        assert_eq!(instance.clip, [0.0, 0.0, 800.0, 600.0]);
        assert_eq!(instance.params, [16.0, 0.5, 1.0, 0.0]);
        assert_eq!(instance.region, [101.0, 1.0, 200.0, 100.0]);
    }

    /// Writes `target/snapshots/images-primitive.bmp`: the test pattern
    /// whole, cropped into a circle and into `rounded-xl`, half
    /// transparent, and under a clip.
    #[test]
    #[ignore = "needs a GPU"]
    fn images_draw_rounded_cropped_and_clipped() {
        use crate::render::ImageDraw;
        use crate::render::images::test_pattern;
        let photo = test_pattern(256, 128);
        let mut list = crate::render::DrawList::new(Color::hex(0xffffff));
        let at = |x: f32, y: f32, w: f32, h: f32| Rect {
            x,
            y,
            width: w,
            height: h,
        };
        list.image(ImageDraw::new(at(8.0, 8.0, 128.0, 64.0), photo.clone()));
        list.image(ImageDraw {
            source: at(0.25, 0.0, 0.5, 1.0),
            radius: 32.0,
            ..ImageDraw::new(at(152.0, 8.0, 64.0, 64.0), photo.clone())
        });
        list.image(ImageDraw {
            radius: 12.0,
            ..ImageDraw::new(at(8.0, 88.0, 128.0, 64.0), photo.clone())
        });
        list.image(ImageDraw {
            opacity: 0.5,
            ..ImageDraw::new(at(152.0, 88.0, 64.0, 64.0), photo.clone())
        });
        list.quad(Quad::fill(
            at(152.0, 160.0, 64.0, 16.0),
            Color::hex(0x1e73ae),
        ));
        list.push_clip(at(8.0, 160.0, 64.0, 24.0));
        list.image(ImageDraw::new(at(8.0, 160.0, 128.0, 64.0), photo.clone()));
        list.pop_clip();
        let mut text = crate::render::text::TextStore::new(cosmic_text::FontSystem::new());
        let mut renderer = pollster::block_on(super::WgpuRenderer::new_offscreen(232, 232))
            .expect("offscreen renderer");
        let rgba = renderer
            .render_to_rgba(&list, &mut text, 1.0)
            .expect("render");
        let px = |x: u32, y: u32| -> [u8; 4] {
            rgba[((y * 232 + x) * 4) as usize..][..4]
                .try_into()
                .unwrap()
        };
        let source = |x: u32, y: u32| -> [u8; 4] {
            photo.pixels()[((y * 256 + x) * 4) as usize..][..4]
                .try_into()
                .unwrap()
        };
        let near = |a: [u8; 4], b: [u8; 4]| a.iter().zip(b).all(|(a, b)| a.abs_diff(b) <= 12);
        // The whole image at half size: (8 + 20, 8 + 20) shows source (40, 40).
        assert!(near(px(28, 28), source(40, 40)), "{:?}", px(28, 28));
        // Circle: its corner is the white background, its center the crop.
        assert_eq!(px(153, 9), [255, 255, 255, 255]);
        assert!(near(px(184, 40), source(128, 64)), "{:?}", px(184, 40));
        // rounded-xl leaves the corner white, the inside is drawn.
        assert_eq!(px(9, 89), [255, 255, 255, 255]);
        assert!(near(px(40, 120), source(64, 64)));
        // Half transparent over white is lighter than the source.
        let faded = px(184, 120);
        let solid = source(128, 64);
        assert!(faded[0] >= solid[0] && faded[1] >= solid[1] && faded[2] >= solid[2]);
        // Clipped below 184.
        assert_eq!(px(40, 190), [255, 255, 255, 255]);
        assert!(near(px(40, 170), source(64, 20)));

        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/snapshots");
        std::fs::create_dir_all(&dir).unwrap();
        crate::snapshot::write_bmp(
            &dir.join("images-primitive.bmp"),
            &crate::snapshot::Image {
                width: 232,
                height: 232,
                rgba,
            },
        )
        .unwrap();
    }

    #[test]
    #[ignore = "needs a GPU"]
    fn gamma_blend_matches_appkit() {
        // 11 §4.4: black α .098 over `#ececec` is 236 × (1 − .098) ≈ 212.9
        // in gamma; blending in linear light would give ≈ 226.
        let mut list = crate::render::DrawList::new(Color::hex(0xececec));
        let all = Rect {
            x: 0.0,
            y: 0.0,
            width: 4.0,
            height: 4.0,
        };
        list.quad(Quad::fill(all, Color::rgba(0, 0, 0, 25)));
        let mut text = crate::render::text::TextStore::new(cosmic_text::FontSystem::new());
        let rgba = render(&list, &mut text, 4);
        let center = &rgba[(2 * 4 + 2) * 4..][..4];
        for channel in &center[..3] {
            assert!(channel.abs_diff(213) <= 1, "{center:?}");
        }
        assert_eq!(center[3], 255);
    }

    #[test]
    #[ignore = "needs a GPU"]
    fn text_is_not_double_encoded() {
        use crate::element::WidgetId;
        use crate::geometry::Point;
        use crate::render::TextId;
        use crate::render::text::{TextStore, TextStyle};
        let mut text = TextStore::new(cosmic_text::FontSystem::new());
        let id = TextId::content(WidgetId(1));
        let style = TextStyle {
            font_size: 48.0,
            line_height: 56.0,
            weight: cosmic_text::Weight(700),
            wrap: false,
            letter_spacing: 0.0,
        };
        text.sync_text(id, "H", style);
        let mut list = crate::render::DrawList::new(Color::hex(0xffffff));
        list.text(id, Point { x: 8.0, y: 4.0 }, Color::hex(0x000000));
        let rgba = render(&list, &mut text, 64);
        let darkest = rgba.chunks(4).map(|px| px[0]).min().unwrap_or(255);
        assert!(darkest < 10, "darkest glyph pixel {darkest}");
    }
}
