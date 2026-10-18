use std::sync::Arc;

use arboard::Clipboard;
use bytemuck::{Pod, Zeroable};
use cosmic_text::{Motion, Selection};
use glyphon::{
    Action as TextAction, Attrs, Buffer, Cache, Color as TextColor, Edit, Editor, Family,
    FontSystem, Metrics, Resolution, Shaping, SwashCache, TextArea, TextAtlas, TextBounds,
    TextRenderer, Viewport,
};
use wgpu::{CurrentSurfaceTexture, SurfaceTexture};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

pub mod geometry;
pub mod grid_layout;
pub mod layout;
pub mod platform_adapter;
pub mod signal;
pub mod ui_engine;

use geometry::{PhysicalPoint, Point, Rect, ScaleFactor, Size};
use layout::{DemoLayout, TextBlockHeights};
use platform_adapter::{adapt_ime_event, adapt_keyboard_event, adapt_modifiers};
use ui_engine::{FocusTarget, ImeIntent, Intent, Key, Modifiers, TextIntent, UiModel, dispatch};

const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.055,
    g: 0.075,
    b: 0.10,
    a: 1.0,
};
const MAX_VERTICES: usize = 256;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

struct TextState {
    font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    renderer: TextRenderer,
    heading: Buffer,
    description: Buffer,
    button_label: Buffer,
    status: Buffer,
    input_placeholder: Buffer,
    input: Editor<'static>,
    preedit: Option<String>,
    scale_factor: f32,
}

impl TextState {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let renderer =
            TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);

        let heading = text_buffer(&mut font_system, 28.0, 36.0, "Kai UI Desktop", true);
        let description = text_buffer(
            &mut font_system,
            15.0,
            22.0,
            "Geometría en unidades lógicas · superficie GPU en píxeles físicos",
            false,
        );
        let button_label = text_buffer(&mut font_system, 15.0, 20.0, "Probar interacción", true);
        let status = text_buffer(
            &mut font_system,
            14.0,
            20.0,
            "Click en el campo y escribe · Tab cambia el foco",
            false,
        );
        let input_buffer = text_buffer(&mut font_system, 15.0, 22.0, "", false);
        let input_placeholder = text_buffer(
            &mut font_system,
            15.0,
            22.0,
            "Escribe aquí; prueba acentos e IME",
            false,
        );

        Self {
            font_system,
            swash_cache,
            viewport,
            atlas,
            renderer,
            heading,
            description,
            button_label,
            status,
            input_placeholder,
            input: Editor::new(input_buffer),
            preedit: None,
            scale_factor: 1.0,
        }
    }

    fn set_scale_factor(&mut self, scale_factor: ScaleFactor) {
        self.scale_factor = scale_factor.as_f32();
    }

    fn set_status(&mut self, text: &str) {
        self.status.set_text(
            text,
            &Attrs::new().family(Family::SansSerif),
            Shaping::Advanced,
            None,
        );
        self.status.shape_until_scroll(&mut self.font_system, false);
    }

    fn measure_wrapped_height(
        font_system: &mut FontSystem,
        buffer: &mut Buffer,
        width: f32,
    ) -> f32 {
        let width = width.max(1.0);
        buffer.set_size(Some(width), Some(10_000.0));
        buffer.shape_until_scroll(font_system, false);
        let height = buffer
            .layout_runs()
            .map(|run| run.line_top + run.line_height)
            .fold(0.0_f32, f32::max)
            .max(1.0);
        buffer.set_size(Some(width), Some(height));
        buffer.shape_until_scroll(font_system, false);
        height
    }

    fn compute_layout(&mut self, viewport: Size) -> DemoLayout {
        let content_width = DemoLayout::content_width_for_viewport(viewport);
        let heading =
            Self::measure_wrapped_height(&mut self.font_system, &mut self.heading, content_width);
        let description = Self::measure_wrapped_height(
            &mut self.font_system,
            &mut self.description,
            content_width,
        );
        let status =
            Self::measure_wrapped_height(&mut self.font_system, &mut self.status, content_width);
        let button_label_width = (220.0_f32.min(content_width) - 40.0).max(1.0);
        Self::measure_wrapped_height(
            &mut self.font_system,
            &mut self.button_label,
            button_label_width,
        );

        DemoLayout::compute_with_text_heights(
            viewport,
            TextBlockHeights {
                heading,
                description,
                status,
            },
        )
    }

    fn input_display_buffer(&mut self, logical_width: f32) -> Buffer {
        let mut display = self.input.clone();
        if let Some(preedit) = self.preedit.as_deref() {
            display.insert_string(preedit, None);
        }
        let mut buffer = display.with_buffer(|buffer| buffer.clone());
        buffer.set_size(Some(logical_width), Some(28.0));
        buffer.shape_until_scroll(&mut self.font_system, false);
        buffer
    }

    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        layout: DemoLayout,
        scroll_offset: f32,
    ) -> Result<(), String> {
        let input_width = layout.input_content_width();
        self.input
            .with_buffer_mut(|buffer| buffer.set_size(Some(input_width), Some(28.0)));
        self.input.shape_as_needed(&mut self.font_system, false);
        let input_buffer = self.input_display_buffer(input_width);
        let input_has_text = self
            .input
            .with_buffer(|buffer| buffer.lines.iter().any(|line| !line.text().is_empty()));
        self.input_placeholder
            .set_size(Some(input_width), Some(28.0));
        self.input_placeholder
            .shape_until_scroll(&mut self.font_system, false);

        let input_area_buffer = if input_has_text || self.preedit.is_some() {
            &input_buffer
        } else {
            &self.input_placeholder
        };

        self.viewport.update(queue, Resolution { width, height });
        self.renderer
            .prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                [
                    TextArea {
                        buffer: &self.heading,
                        left: layout.heading_origin.x * self.scale_factor,
                        top: (layout.heading_origin.y - scroll_offset) * self.scale_factor,
                        scale: self.scale_factor,
                        bounds: text_bounds(layout.content_viewport(), self.scale_factor),
                        default_color: TextColor::rgb(244, 248, 250),
                        custom_glyphs: &[],
                    },
                    TextArea {
                        buffer: &self.description,
                        left: layout.description_origin.x * self.scale_factor,
                        top: (layout.description_origin.y - scroll_offset) * self.scale_factor,
                        scale: self.scale_factor,
                        bounds: text_bounds(layout.content_viewport(), self.scale_factor),
                        default_color: TextColor::rgb(174, 192, 204),
                        custom_glyphs: &[],
                    },
                    TextArea {
                        buffer: &self.button_label,
                        left: layout.button_label_origin.x * self.scale_factor,
                        top: (layout.button_label_origin.y - scroll_offset) * self.scale_factor,
                        scale: self.scale_factor,
                        bounds: text_bounds(layout.content_viewport(), self.scale_factor),
                        default_color: TextColor::rgb(255, 255, 255),
                        custom_glyphs: &[],
                    },
                    TextArea {
                        buffer: &self.status,
                        left: layout.status_origin.x * self.scale_factor,
                        top: (layout.status_origin.y - scroll_offset) * self.scale_factor,
                        scale: self.scale_factor,
                        bounds: text_bounds(layout.content_viewport(), self.scale_factor),
                        default_color: TextColor::rgb(174, 192, 204),
                        custom_glyphs: &[],
                    },
                    TextArea {
                        buffer: input_area_buffer,
                        left: layout.input_text_origin.x * self.scale_factor,
                        top: (layout.input_text_origin.y - scroll_offset) * self.scale_factor,
                        scale: self.scale_factor,
                        bounds: text_bounds(layout.content_viewport(), self.scale_factor),
                        default_color: TextColor::rgb(236, 242, 246),
                        custom_glyphs: &[],
                    },
                ],
                &mut self.swash_cache,
            )
            .map_err(|error| format!("No se pudo preparar el texto: {error}"))
    }
}

fn text_buffer(
    font_system: &mut FontSystem,
    font_size: f32,
    line_height: f32,
    text: &str,
    semibold: bool,
) -> Buffer {
    let mut buffer = Buffer::new(font_system, Metrics::new(font_size, line_height));
    let attrs = if semibold {
        Attrs::new()
            .family(Family::SansSerif)
            .weight(glyphon::Weight::SEMIBOLD)
    } else {
        Attrs::new().family(Family::SansSerif)
    };
    buffer.set_text(text, &attrs, Shaping::Advanced, None);
    buffer.shape_until_scroll(font_system, false);
    buffer
}

fn text_bounds(rect: Rect, scale_factor: f32) -> TextBounds {
    TextBounds {
        left: (rect.x * scale_factor).floor() as i32,
        top: (rect.y * scale_factor).floor() as i32,
        right: ((rect.x + rect.width) * scale_factor).ceil() as i32,
        bottom: ((rect.y + rect.height) * scale_factor).ceil() as i32,
    }
}

struct App {
    state: Option<GpuState>,
}

struct GpuState {
    _instance: wgpu::Instance,
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    geometry_pipeline: wgpu::RenderPipeline,
    geometry_buffer: wgpu::Buffer,
    text: TextState,
    layout: DemoLayout,
    scroll_offset: f32,
    scale_factor: ScaleFactor,
    cursor_logical: Option<Point>,
    modifiers: Modifiers,
    ui: UiModel,
    input_dragging: bool,
}

impl GpuState {
    async fn new(window: Arc<Window>) -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(Arc::clone(&window))
            .map_err(|error| format!("No se pudo crear la superficie GPU: {error}"))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await
            .map_err(|error| format!("No se encontró un adaptador GPU compatible: {error}"))?;
        let info = adapter.get_info();
        eprintln!(
            "Adaptador GPU: {} ({:?}, {:?})",
            info.name, info.backend, info.device_type
        );

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Kai Desktop prototype device"),
                ..Default::default()
            })
            .await
            .map_err(|error| format!("No se pudo crear el dispositivo GPU: {error}"))?;
        let size = window.inner_size();
        let config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or_else(|| {
                "El adaptador no ofrece configuración para esta superficie".to_owned()
            })?;
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Kai 2D solid geometry shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("geometry.wgsl").into()),
        });
        let geometry_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Kai 2D solid geometry pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
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
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let geometry_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Kai 2D frame geometry"),
            size: (MAX_VERTICES * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let scale_factor = ScaleFactor::new(window.scale_factor())
            .ok_or_else(|| "La ventana informó un factor de escala inválido".to_owned())?;
        let mut text = TextState::new(&device, &queue, config.format);
        text.set_scale_factor(scale_factor);
        let logical_size = scale_factor.to_logical_size(size.width, size.height);
        let layout = text.compute_layout(logical_size);
        eprintln!("Escala inicial de ventana: {:.2}x", scale_factor.get());

        Ok(Self {
            _instance: instance,
            window,
            surface,
            device,
            queue,
            config,
            geometry_pipeline,
            geometry_buffer,
            text,
            layout,
            scroll_offset: 0.0,
            scale_factor,
            cursor_logical: None,
            modifiers: Modifiers::default(),
            ui: UiModel::default(),
            input_dragging: false,
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        self.window.request_redraw();
    }

    fn set_scale_factor(&mut self, value: f64) {
        let Some(scale_factor) = ScaleFactor::new(value) else {
            return;
        };
        self.scale_factor = scale_factor;
        self.text.set_scale_factor(scale_factor);
        self.window.request_redraw();
        eprintln!("Escala de ventana actualizada: {:.2}x", scale_factor.get());
    }

    fn demo_layout(&self) -> DemoLayout {
        self.layout
    }

    fn button_hovered(&self) -> bool {
        let layout = self.demo_layout();
        self.cursor_logical.is_some_and(|point| {
            layout
                .viewport_to_content(point, self.scroll_offset)
                .is_some_and(|content_point| layout.button().contains(content_point))
        })
    }

    fn input_hovered(&self) -> bool {
        let layout = self.demo_layout();
        self.cursor_logical.is_some_and(|point| {
            layout
                .viewport_to_content(point, self.scroll_offset)
                .is_some_and(|content_point| layout.input().contains(content_point))
        })
    }

    fn input_focused(&self) -> bool {
        *self.ui.focused.get() == Some(FocusTarget::TextField)
    }

    fn button_focused(&self) -> bool {
        *self.ui.focused.get() == Some(FocusTarget::Button)
    }

    fn update_ime_cursor_area(&self) {
        if !self.input_focused() {
            return;
        }
        let layout = self.demo_layout();
        let (cursor_x, cursor_y) = self.text.input.cursor_position().unwrap_or((0, 0));
        let scale = self.scale_factor.get();
        self.window.set_ime_cursor_area(
            PhysicalPosition::new(
                ((layout.input_caret_origin.x + cursor_x as f32) as f64 * scale).round() as i32,
                ((layout.input_caret_origin.y + cursor_y as f32 - self.scroll_offset) as f64
                    * scale)
                    .round() as i32,
            ),
            PhysicalSize::new(1_u32, (22.0_f64 * scale).round() as u32),
        );
    }

    fn apply_intent(&mut self, intent: Intent) {
        let previous_focus = *self.ui.focused.get();
        let result = dispatch(&mut self.ui, intent);
        let current_focus = *self.ui.focused.get();
        if previous_focus != current_focus {
            let input_focused = current_focus == Some(FocusTarget::TextField);
            self.window.set_ime_allowed(input_focused);
            if input_focused {
                self.text
                    .set_status("Campo enfocado · escribe, pega o usa un método IME");
                self.update_ime_cursor_area();
            } else if previous_focus == Some(FocusTarget::TextField) {
                self.input_dragging = false;
                self.text.preedit = None;
            }
        }
        for action in result.text_actions {
            self.apply_text_intent(action);
        }
        if let Some((_source, count)) = result.activated {
            self.text
                .set_status(&format!("Botón activado {} vez/veces", count));
        }
        if result.changed || current_focus != previous_focus {
            self.window.request_redraw();
        }
    }

    fn apply_text_intent(&mut self, intent: TextIntent) {
        match intent {
            TextIntent::KeyDown { key, modifiers, .. } => self.handle_input_key(key, modifiers),
            TextIntent::InsertText(text) => self.handle_input_text(&text),
            TextIntent::Ime(event) => self.handle_ime_intent(event),
            TextIntent::SelectAll => self.select_all_input(),
            TextIntent::Copy => self.clipboard_copy(false),
            TextIntent::Cut => self.clipboard_copy(true),
            TextIntent::Paste => self.clipboard_paste(),
        }
    }

    fn move_input_cursor(&mut self, motion: Motion, modifiers: Modifiers) {
        let shift = modifiers.shift;
        let anchor = match self.text.input.selection() {
            Selection::Normal(anchor) if shift => anchor,
            _ => self.text.input.cursor(),
        };
        self.text
            .input
            .action(&mut self.text.font_system, TextAction::Motion(motion));
        if shift {
            self.text.input.set_selection(Selection::Normal(anchor));
        } else {
            self.text.input.set_selection(Selection::None);
        }
        self.text
            .input
            .shape_as_needed(&mut self.text.font_system, false);
        self.update_ime_cursor_area();
        self.window.request_redraw();
    }

    fn clipboard_copy(&mut self, cut: bool) {
        if let Some(text) = self.text.input.copy_selection() {
            match Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text)) {
                Ok(()) if cut => {
                    self.text.input.delete_selection();
                    self.text
                        .input
                        .shape_as_needed(&mut self.text.font_system, false);
                }
                Ok(()) => {}
                Err(error) => self.text.set_status(&format!("Portapapeles: {error}")),
            }
            self.update_ime_cursor_area();
            self.window.request_redraw();
        }
    }

    fn clipboard_paste(&mut self) {
        match Clipboard::new().and_then(|mut clipboard| clipboard.get_text()) {
            Ok(text) => {
                self.text.input.insert_string(&text, None);
                self.text
                    .input
                    .shape_as_needed(&mut self.text.font_system, false);
                self.text.preedit = None;
                self.update_ime_cursor_area();
                self.window.request_redraw();
            }
            Err(error) => self.text.set_status(&format!("Portapapeles: {error}")),
        }
    }

    fn handle_input_key(&mut self, key: Key, modifiers: Modifiers) {
        let layout = self.demo_layout();
        match key {
            Key::ArrowLeft => self.move_input_cursor(Motion::Left, modifiers),
            Key::ArrowRight => self.move_input_cursor(Motion::Right, modifiers),
            Key::ArrowUp => self.move_input_cursor(Motion::Up, modifiers),
            Key::ArrowDown => self.move_input_cursor(Motion::Down, modifiers),
            Key::Home => self.move_input_cursor(Motion::BufferStart, modifiers),
            Key::End => self.move_input_cursor(Motion::BufferEnd, modifiers),
            Key::Backspace => {
                self.text
                    .input
                    .action(&mut self.text.font_system, TextAction::Backspace);
                self.text
                    .input
                    .shape_as_needed(&mut self.text.font_system, false);
                self.update_ime_cursor_area();
                self.window.request_redraw();
            }
            Key::Delete => {
                self.text
                    .input
                    .action(&mut self.text.font_system, TextAction::Delete);
                self.text
                    .input
                    .shape_as_needed(&mut self.text.font_system, false);
                self.update_ime_cursor_area();
                self.window.request_redraw();
            }
            Key::Enter => {
                self.text
                    .input
                    .action(&mut self.text.font_system, TextAction::Enter);
                self.text
                    .input
                    .shape_as_needed(&mut self.text.font_system, false);
                self.update_ime_cursor_area();
                self.window.request_redraw();
            }
            _ => {}
        }
        let _ = layout;
    }

    fn select_all_input(&mut self) {
        self.text.input.action(
            &mut self.text.font_system,
            TextAction::Motion(Motion::BufferStart),
        );
        self.text
            .input
            .set_selection(Selection::Normal(self.text.input.cursor()));
        self.text.input.action(
            &mut self.text.font_system,
            TextAction::Motion(Motion::BufferEnd),
        );
        self.text
            .input
            .shape_as_needed(&mut self.text.font_system, false);
        self.window.request_redraw();
    }

    fn handle_ime_intent(&mut self, event: ImeIntent) {
        match event {
            ImeIntent::Enabled => {
                self.text.preedit = None;
                self.text.set_status("IME habilitado");
            }
            ImeIntent::Preedit { text, .. } => {
                let active = !text.is_empty();
                self.text.preedit = active.then_some(text);
                if active {
                    self.text.set_status("Composición IME en curso");
                }
            }
            ImeIntent::Commit(text) => {
                self.text.preedit = None;
                self.text.input.insert_string(&text, None);
                self.text
                    .input
                    .shape_as_needed(&mut self.text.font_system, false);
                self.text.set_status("Texto confirmado por IME");
                self.update_ime_cursor_area();
            }
            ImeIntent::Cancel | ImeIntent::Disabled => self.text.preedit = None,
        }
        self.window.request_redraw();
    }

    fn handle_input_text(&mut self, text: &str) {
        let committed: String = text
            .chars()
            .filter(|character| !character.is_control())
            .collect();
        if committed.is_empty() {
            return;
        }
        self.text.input.insert_string(&committed, None);
        self.text
            .input
            .shape_as_needed(&mut self.text.font_system, false);
        self.text.preedit = None;
        self.text.set_status("Texto de teclado recibido");
        self.update_ime_cursor_area();
        self.window.request_redraw();
    }

    fn input_selection_rects(&self, layout: DemoLayout) -> Vec<Rect> {
        let Some((start, end)) = self.text.input.selection_bounds() else {
            return Vec::new();
        };
        self.text.input.with_buffer(|buffer| {
            buffer
                .layout_runs()
                .flat_map(|run| {
                    run.highlight(start, end).map(move |(x, width)| Rect {
                        x: layout.input_caret_origin.x + x,
                        y: layout.input_caret_origin.y + run.line_top,
                        width,
                        height: run.line_height,
                    })
                })
                .collect()
        })
    }

    fn build_geometry(&self, layout: DemoLayout) -> Vec<Vertex> {
        let size = layout.window_size();
        let mut vertices = Vec::with_capacity(48);
        push_rect(
            &mut vertices,
            size.width,
            size.height,
            layout.panel(),
            [0.09, 0.13, 0.18, 1.0],
        );
        push_rect(
            &mut vertices,
            size.width,
            size.height,
            layout.panel().inset(geometry::Insets {
                top: 2.0,
                right: 2.0,
                bottom: 2.0,
                left: 2.0,
            }),
            [0.075, 0.105, 0.15, 1.0],
        );

        let hovered = self.button_hovered();
        let fill = if *self.ui.button_pressed.get() {
            [0.03, 0.28, 0.43, 1.0]
        } else if hovered {
            [0.12, 0.48, 0.68, 1.0]
        } else {
            [0.09, 0.39, 0.59, 1.0]
        };
        let button_ring = if self.button_focused() { 4.0 } else { 2.0 };
        push_content_rect(
            &mut vertices,
            size.width,
            size.height,
            layout.content_viewport(),
            self.scroll_offset,
            Rect {
                x: layout.button().x - button_ring,
                y: layout.button().y - button_ring,
                width: layout.button().width + button_ring * 2.0,
                height: layout.button().height + button_ring * 2.0,
            },
            if self.button_focused() {
                [1.0, 0.05, 0.78, 1.0]
            } else {
                [0.28, 0.60, 0.77, 1.0]
            },
        );
        push_content_rect(
            &mut vertices,
            size.width,
            size.height,
            layout.content_viewport(),
            self.scroll_offset,
            layout.button(),
            fill,
        );

        let input_ring = if self.input_focused() { 3.0 } else { 1.0 };
        push_content_rect(
            &mut vertices,
            size.width,
            size.height,
            layout.content_viewport(),
            self.scroll_offset,
            Rect {
                x: layout.input().x - input_ring,
                y: layout.input().y - input_ring,
                width: layout.input().width + input_ring * 2.0,
                height: layout.input().height + input_ring * 2.0,
            },
            if self.input_focused() {
                [1.0, 0.05, 0.78, 1.0]
            } else if self.input_hovered() {
                [0.30, 0.48, 0.60, 1.0]
            } else {
                [0.22, 0.32, 0.42, 1.0]
            },
        );
        push_content_rect(
            &mut vertices,
            size.width,
            size.height,
            layout.content_viewport(),
            self.scroll_offset,
            layout.input(),
            [0.035, 0.055, 0.08, 1.0],
        );
        for rect in self.input_selection_rects(layout) {
            push_content_rect(
                &mut vertices,
                size.width,
                size.height,
                layout.content_viewport(),
                self.scroll_offset,
                rect,
                [0.12, 0.37, 0.54, 0.9],
            );
        }
        if self.input_focused() && self.text.preedit.is_none() {
            if let Some((x, y)) = self.text.input.cursor_position() {
                push_content_rect(
                    &mut vertices,
                    size.width,
                    size.height,
                    layout.content_viewport(),
                    self.scroll_offset,
                    Rect {
                        x: layout.input_caret_origin.x + x as f32,
                        y: layout.input_caret_origin.y + y as f32,
                        width: 1.5,
                        height: 22.0,
                    },
                    [0.92, 0.96, 1.0, 1.0],
                );
            }
        }
        if layout.scroll_range() > 0.0 {
            let track_width = 3.0;
            let track = Rect {
                x: layout.content_viewport().x + layout.content_viewport().width - track_width,
                y: layout.content_viewport().y,
                width: track_width,
                height: layout.content_viewport().height,
            };
            let thumb_height = (track.height * track.height / layout.content_height())
                .max(20.0)
                .min(track.height);
            let thumb_travel = (track.height - thumb_height).max(0.0);
            let thumb_y = track.y
                + thumb_travel * (self.scroll_offset / layout.scroll_range()).clamp(0.0, 1.0);
            push_rect(
                &mut vertices,
                size.width,
                size.height,
                track,
                [0.20, 0.27, 0.34, 0.7],
            );
            push_rect(
                &mut vertices,
                size.width,
                size.height,
                Rect {
                    x: track.x,
                    y: thumb_y,
                    width: track.width,
                    height: thumb_height,
                },
                [0.55, 0.65, 0.74, 0.95],
            );
        }
        vertices
    }

    fn render(&mut self) -> Result<(), String> {
        let physical = self.window.inner_size();
        let viewport = self
            .scale_factor
            .to_logical_size(physical.width, physical.height);
        self.layout = self.text.compute_layout(viewport);
        self.scroll_offset = self.scroll_offset.clamp(0.0, self.layout.scroll_range());
        let layout = self.layout;
        let vertices = self.build_geometry(layout);
        self.queue
            .write_buffer(&self.geometry_buffer, 0, bytemuck::cast_slice(&vertices));
        self.text.prepare(
            &self.device,
            &self.queue,
            self.config.width,
            self.config.height,
            layout,
            self.scroll_offset,
        )?;

        let (frame, suboptimal): (SurfaceTexture, bool) = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) => (frame, false),
            CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            CurrentSurfaceTexture::Lost | CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                self.window.request_redraw();
                return Ok(());
            }
            CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => return Ok(()),
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
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Kai Desktop geometry and text"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if !vertices.is_empty() {
                pass.set_pipeline(&self.geometry_pipeline);
                pass.set_vertex_buffer(0, self.geometry_buffer.slice(..));
                pass.draw(0..vertices.len() as u32, 0..1);
            }
            self.text
                .renderer
                .render(&self.text.atlas, &self.text.viewport, &mut pass)
                .map_err(|error| format!("No se pudo dibujar el texto: {error}"))?;
        }
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        if suboptimal {
            self.surface.configure(&self.device, &self.config);
            self.window.request_redraw();
        }
        self.text.atlas.trim();
        Ok(())
    }
}

fn push_rect(vertices: &mut Vec<Vertex>, width: f32, height: f32, rect: Rect, color: [f32; 4]) {
    let Some(ndc) = rect.to_ndc(Size { width, height }) else {
        return;
    };
    let tl = Vertex {
        position: [ndc.left, ndc.top],
        color,
    };
    let tr = Vertex {
        position: [ndc.right, ndc.top],
        color,
    };
    let bl = Vertex {
        position: [ndc.left, ndc.bottom],
        color,
    };
    let br = Vertex {
        position: [ndc.right, ndc.bottom],
        color,
    };
    vertices.extend_from_slice(&[tl, bl, tr, tr, bl, br]);
}

fn push_content_rect(
    vertices: &mut Vec<Vertex>,
    viewport_width: f32,
    viewport_height: f32,
    clip: Rect,
    scroll_offset: f32,
    rect: Rect,
    color: [f32; 4],
) {
    if let Some(visible) = rect
        .translated(Point {
            x: 0.0,
            y: -scroll_offset,
        })
        .intersection(clip)
    {
        push_rect(vertices, viewport_width, viewport_height, visible, color);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Kai UI Desktop — prototipo")
            .with_inner_size(LogicalSize::new(1024.0, 720.0))
            .with_min_inner_size(LogicalSize::new(
                DemoLayout::MINIMUM_VIEWPORT.width as f64,
                DemoLayout::MINIMUM_VIEWPORT.height as f64,
            ));
        let window = match event_loop.create_window(attrs) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("No se pudo crear la ventana: {error}");
                event_loop.exit();
                return;
            }
        };
        match pollster::block_on(GpuState::new(window)) {
            Ok(state) => {
                state.window.request_redraw();
                self.state = Some(state);
            }
            Err(error) => {
                eprintln!("No se pudo iniciar el renderer de Kai: {error}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                state.set_scale_factor(scale_factor)
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let over_viewport = state
                    .cursor_logical
                    .is_some_and(|point| state.layout.content_viewport().contains(point));
                if over_viewport && state.layout.scroll_range() > 0.0 {
                    let delta_y = match delta {
                        MouseScrollDelta::LineDelta(_, lines) => lines * 40.0,
                        MouseScrollDelta::PixelDelta(position) => {
                            position.y as f32 / state.scale_factor.as_f32()
                        }
                    };
                    let next = state.layout.scroll_by(state.scroll_offset, delta_y);
                    if next != state.scroll_offset {
                        state.scroll_offset = next;
                        state.update_ime_cursor_area();
                        state.window.request_redraw();
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                state.cursor_logical = Some(state.scale_factor.to_logical_point(PhysicalPoint {
                    x: position.x,
                    y: position.y,
                }));
                if state.input_dragging {
                    let layout = state.demo_layout();
                    if let Some(point) = state
                        .cursor_logical
                        .and_then(|point| layout.viewport_to_content(point, state.scroll_offset))
                    {
                        let local = layout.input_local_position(point);
                        state.text.input.action(
                            &mut state.text.font_system,
                            TextAction::Drag {
                                x: local.x as i32,
                                y: local.y as i32,
                            },
                        );
                        state
                            .text
                            .input
                            .shape_as_needed(&mut state.text.font_system, false);
                        state.update_ime_cursor_area();
                    }
                }
                state.window.set_cursor(if state.input_hovered() {
                    winit::window::CursorIcon::Text
                } else {
                    winit::window::CursorIcon::Default
                });
                state.window.request_redraw();
            }
            WindowEvent::CursorLeft { .. } => {
                state.cursor_logical = None;
                state.apply_intent(Intent::CancelPointer);
                state.input_dragging = false;
                state.window.request_redraw();
            }
            WindowEvent::Focused(false) => {
                state.apply_intent(Intent::CancelPointer);
                state.input_dragging = false;
                state.window.set_ime_allowed(false);
                state.window.request_redraw();
            }
            WindowEvent::Focused(true) => {
                state.window.set_ime_allowed(state.input_focused());
                state.update_ime_cursor_area();
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                state.modifiers = adapt_modifiers(modifiers.state())
            }
            WindowEvent::MouseInput {
                state: element,
                button: MouseButton::Left,
                ..
            } => {
                let layout = state.demo_layout();
                match element {
                    ElementState::Pressed => {
                        let content_point = state.cursor_logical.and_then(|point| {
                            layout.viewport_to_content(point, state.scroll_offset)
                        });
                        let target = content_point.and_then(|p| {
                            if layout.button().contains(p) {
                                Some(FocusTarget::Button)
                            } else if layout.input().contains(p) {
                                Some(FocusTarget::TextField)
                            } else {
                                None
                            }
                        });
                        state.apply_intent(Intent::PointerDown(target));
                        state.input_dragging = target == Some(FocusTarget::TextField);
                        if state.input_dragging {
                            if let Some(point) = content_point {
                                let local = layout.input_local_position(point);
                                state.text.input.action(
                                    &mut state.text.font_system,
                                    TextAction::Click {
                                        x: local.x as i32,
                                        y: local.y as i32,
                                    },
                                );
                                state.text.input.set_selection(Selection::None);
                                state
                                    .text
                                    .input
                                    .shape_as_needed(&mut state.text.font_system, false);
                                state.update_ime_cursor_area();
                            }
                        }
                    }
                    ElementState::Released => {
                        let target = state
                            .cursor_logical
                            .and_then(|point| {
                                layout.viewport_to_content(point, state.scroll_offset)
                            })
                            .and_then(|p| {
                                if layout.button().contains(p) {
                                    Some(FocusTarget::Button)
                                } else if layout.input().contains(p) {
                                    Some(FocusTarget::TextField)
                                } else {
                                    None
                                }
                            });
                        state.apply_intent(Intent::PointerUp(target));
                        state.input_dragging = false;
                    }
                }
                state.window.request_redraw();
            }
            WindowEvent::Ime(ime) => state.apply_intent(adapt_ime_event(ime)),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                for intent in adapt_keyboard_event(&event, state.modifiers) {
                    state.apply_intent(intent);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = state.render() {
                    eprintln!("Error al presentar el frame GPU: {error}");
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !cfg!(target_os = "macos") {
        return Err("Kai UI Desktop comienza como prototipo para macOS".into());
    }
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut App { state: None })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{TextState, Vertex};
    use crate::geometry::Rect;
    use crate::push_rect;
    use cosmic_text::{
        Action, Attrs, Buffer, Edit, Editor, FontSystem, Metrics, Motion, Selection, Shaping,
    };

    #[test]
    fn logical_rect_maps_to_normalized_device_coordinates() {
        let mut vertices = Vec::<Vertex>::new();
        push_rect(
            &mut vertices,
            200.0,
            100.0,
            Rect {
                x: 50.0,
                y: 25.0,
                width: 100.0,
                height: 50.0,
            },
            [1.0, 0.5, 0.25, 1.0],
        );
        assert_eq!(vertices.len(), 6);
        assert_eq!(vertices[0].position, [-0.5, 0.5]);
        assert_eq!(vertices[5].position, [0.5, -0.5]);
    }

    #[test]
    fn editor_inserts_unicode_and_selects_text() {
        let mut font_system = FontSystem::new();
        let mut buffer = Buffer::new(&mut font_system, Metrics::new(15.0, 22.0));
        buffer.set_text("", &Attrs::new(), Shaping::Advanced, None);
        let mut editor = Editor::new(buffer);
        editor.insert_string("Kai 😀", None);
        editor.shape_as_needed(&mut font_system, false);
        editor.action(&mut font_system, Action::Motion(Motion::BufferStart));
        let anchor = editor.cursor();
        editor.set_selection(Selection::Normal(anchor));
        editor.action(&mut font_system, Action::Motion(Motion::BufferEnd));
        assert_eq!(editor.copy_selection().as_deref(), Some("Kai 😀"));
        editor.insert_string("Kai UI", None);
        let value = editor.with_buffer(|buffer| buffer.lines[0].text().to_owned());
        assert_eq!(value, "Kai UI");
    }

    #[test]
    fn cosmic_text_wraps_inside_box_and_reports_larger_height_at_narrow_width() {
        let mut font_system = FontSystem::new();
        let mut buffer = Buffer::new(&mut font_system, Metrics::new(15.0, 22.0));
        buffer.set_text(
            "This responsive description wraps when its parent box becomes narrower.",
            &Attrs::new(),
            Shaping::Advanced,
            None,
        );

        let wide_height = TextState::measure_wrapped_height(&mut font_system, &mut buffer, 520.0);
        let narrow_height = TextState::measure_wrapped_height(&mut font_system, &mut buffer, 170.0);

        assert!(narrow_height > wide_height);
    }
}
