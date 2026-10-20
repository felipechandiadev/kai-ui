//! Offscreen captures for visual review and regression tests.
//!
//! [`render`] reproduces what `runtime::WindowState::render` draws (layout
//! with real text, input contents, paint and wgpu) into an RGBA image.
//! [`diff`] and [`diff_image`] compare captures with a per-channel
//! tolerance. PNG support (`write_png`, `read_png`) needs the
//! `snapshots` feature outside this crate's own tests; BMP needs nothing.

use std::path::Path;

use cosmic_text::{FontSystem, Metrics};

use crate::background::BackgroundEffect;
use crate::element::{Element, Kind, WidgetId};
use crate::geometry::Size;
use crate::layout::{Interaction, Layout, LayoutInput, ScrollOffsets, StyleCache, text_style};
use crate::render::TextId;
use crate::render::paint::{PaintInput, paint};
use crate::render::text::TextStore;
use crate::render::wgpu_backend::WgpuRenderer;
use crate::text_field::{TextFieldDecorations, TextFieldState};
use crate::theme::Theme;

/// What to capture.
pub struct Shot<'a, Msg> {
    /// Tree with ids assigned (`runtime::build_view`).
    pub tree: &'a Element<Msg>,
    pub theme: &'a Theme,
    /// Window size in logical points.
    pub logical: Size,
    /// Backing scale factor; `2.0` in every reference capture.
    pub scale: f32,
    pub interaction: &'a Interaction,
    pub scroll: &'a ScrollOffsets,
}

/// Top-down RGBA8 pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Layout with real text + paint + offscreen render, as the runtime does,
/// including the `TextFieldState` of every input.
pub fn render<Msg>(shot: &Shot<'_, Msg>) -> Result<Image, String> {
    render_with_background(shot, BackgroundEffect::NONE, 0.0)
}

/// [`render`] over `background` at `seconds` of its animation (ignored
/// when static), as the runtime draws it for an app whose
/// [`crate::runtime::App::background`] returns `background`.
pub fn render_with_background<Msg>(
    shot: &Shot<'_, Msg>,
    background: BackgroundEffect,
    seconds: f32,
) -> Result<Image, String> {
    render_with_layout(shot, background, seconds).map(|(image, _)| image)
}

/// [`render`] cropped to the bounds of element `id` in that same layout
/// (real text), clipped to the window.
pub fn render_element<Msg>(shot: &Shot<'_, Msg>, id: WidgetId) -> Result<Image, String> {
    let (image, layout) = render_with_layout(shot, BackgroundEffect::NONE, 0.0)?;
    let bounds = layout
        .bounds(id)
        .ok_or_else(|| format!("El elemento {id:?} no está en el layout"))?;
    let scale = shot.scale;
    let clamp_x = |x: f32| (x * scale).clamp(0.0, image.width as f32);
    let clamp_y = |y: f32| (y * scale).clamp(0.0, image.height as f32);
    let (left, top) = (
        clamp_x(bounds.x).floor() as u32,
        clamp_y(bounds.y).floor() as u32,
    );
    let (right, bottom) = (
        clamp_x(bounds.x + bounds.width).ceil() as u32,
        clamp_y(bounds.y + bounds.height).ceil() as u32,
    );
    if right <= left || bottom <= top {
        return Err(format!("El elemento {id:?} queda fuera de la ventana"));
    }
    let (width, height) = (right - left, bottom - top);
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in top..bottom {
        let start = ((y * image.width + left) * 4) as usize;
        rgba.extend_from_slice(&image.rgba[start..start + (width * 4) as usize]);
    }
    Ok(Image {
        width,
        height,
        rgba,
    })
}

fn render_with_layout<Msg>(
    shot: &Shot<'_, Msg>,
    background: BackgroundEffect,
    seconds: f32,
) -> Result<(Image, Layout), String> {
    let tree = shot.tree;
    let mut text = TextStore::new(FontSystem::new());
    let mut styles = StyleCache::default();
    let layout = Layout::compute(
        tree,
        &LayoutInput {
            viewport: shot.logical,
            theme: shot.theme,
            interaction: shot.interaction,
            scroll: shot.scroll,
        },
        &mut styles,
        &mut text,
    );
    let mut decorations = Vec::new();
    tree.walk(&mut |element| {
        if let Kind::Input {
            value,
            masked,
            read_only,
            rows,
            ..
        } = &element.kind
            && let Some(node) = layout.node(element.id)
        {
            let fonts = &mut text.font_system;
            let mut field = TextFieldState::new(
                fonts,
                Metrics::new(node.text.font_size, node.text.line_height),
            );
            field.set_multiline(fonts, rows.is_some());
            field.set_masked(fonts, *masked);
            field.set_read_only(fonts, *read_only);
            if let Some(line) = node.text_box {
                field.set_width(fonts, line.width);
                field.set_height(fonts, line.height);
            }
            field.set_text(fonts, value);
            let view = field.view(fonts);
            decorations.push((element.id, view.decorations));
            text.set_buffer(
                TextId::value(element.id),
                view.buffer,
                text_style(&node.text),
            );
        }
    });
    let fields: Vec<(WidgetId, &TextFieldDecorations)> =
        decorations.iter().map(|(id, d)| (*id, d)).collect();
    let list = paint(&PaintInput {
        layout: &layout,
        theme: shot.theme,
        focused: shot.interaction.focused,
        window: shot.interaction.window,
        text_fields: &fields,
        background,
    });
    let mut list = list;
    list.background = list.background.map(|layer| layer.at(seconds));
    let (width, height) = (
        (shot.logical.width * shot.scale) as u32,
        (shot.logical.height * shot.scale) as u32,
    );
    let mut renderer = pollster::block_on(WgpuRenderer::new_offscreen(width, height))?;
    let rgba = renderer.render_to_rgba(&list, &mut text, shot.scale)?;
    Ok((
        Image {
            width,
            height,
            rgba,
        },
        layout,
    ))
}

/// 24-bit bottom-up BMP (alpha dropped).
pub fn bmp(image: &Image) -> Vec<u8> {
    let (width, height, rgba) = (image.width, image.height, &image.rgba);
    let row = (width * 3).div_ceil(4) * 4;
    let size = 54 + row * height;
    let mut out = Vec::with_capacity(size as usize);
    out.extend_from_slice(b"BM");
    for value in [size, 0, 54, 40, width, height] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    for value in [0u32, row * height, 2835, 2835, 0, 0] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    for y in (0..height).rev() {
        let start = out.len();
        for x in 0..width {
            let i = ((y * width + x) * 4) as usize;
            out.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i]]);
        }
        out.resize(start + row as usize, 0);
    }
    out
}

pub fn write_bmp(path: &Path, image: &Image) -> std::io::Result<()> {
    std::fs::write(path, bmp(image))
}

#[cfg(any(test, feature = "snapshots"))]
pub fn write_png(path: &Path, image: &Image) -> std::io::Result<()> {
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut encoder = png::Encoder::new(file, image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(std::io::Error::other)?;
    writer
        .write_image_data(&image.rgba)
        .map_err(std::io::Error::other)?;
    writer.finish().map_err(std::io::Error::other)
}

/// Reads any 8-bit RGB(A) PNG as RGBA8.
#[cfg(any(test, feature = "snapshots"))]
pub fn read_png(path: &Path) -> std::io::Result<Image> {
    let file = std::io::BufReader::new(std::fs::File::open(path)?);
    let mut decoder = png::Decoder::new(file);
    decoder.set_transformations(
        png::Transformations::EXPAND | png::Transformations::STRIP_16 | png::Transformations::ALPHA,
    );
    let mut reader = decoder.read_info().map_err(std::io::Error::other)?;
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| std::io::Error::other("PNG demasiado grande"))?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(std::io::Error::other)?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return Err(std::io::Error::other(format!(
            "PNG no soportado: {:?} {:?}",
            info.color_type, info.bit_depth
        )));
    }
    buf.truncate(info.buffer_size());
    Ok(Image {
        width: info.width,
        height: info.height,
        rgba: buf,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Diff {
    /// Largest `|a - b|` over every channel.
    pub max_channel: u8,
    /// Pixels with some channel differing by more than the tolerance.
    pub differing: usize,
    /// `differing / total`.
    pub ratio: f32,
}

impl Diff {
    pub fn is_identical(&self) -> bool {
        self.differing == 0
    }
}

/// Compares two images of the same size. Images of different sizes count
/// every pixel as differing.
pub fn diff(a: &Image, b: &Image, tolerance: u8) -> Diff {
    let total = (a.width as usize * a.height as usize).max(1);
    if (a.width, a.height) != (b.width, b.height) {
        return Diff {
            max_channel: u8::MAX,
            differing: total,
            ratio: 1.0,
        };
    }
    let mut max_channel = 0;
    let mut differing = 0;
    for (pa, pb) in a.rgba.chunks_exact(4).zip(b.rgba.chunks_exact(4)) {
        let worst = pa
            .iter()
            .zip(pb)
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap_or(0);
        max_channel = max_channel.max(worst);
        if worst > tolerance {
            differing += 1;
        }
    }
    Diff {
        max_channel,
        differing,
        ratio: differing as f32 / total as f32,
    }
}

/// Opaque red where the images differ beyond `tolerance`, a faded copy of
/// `a` elsewhere, to review by eye. Uses `a`'s size.
pub fn diff_image(a: &Image, b: &Image, tolerance: u8) -> Image {
    let same_size = (a.width, a.height) == (b.width, b.height);
    let mut rgba = Vec::with_capacity(a.rgba.len());
    for (index, pa) in a.rgba.chunks_exact(4).enumerate() {
        let pb = b.rgba.get(index * 4..index * 4 + 4);
        let differs = !same_size
            || pb.is_none_or(|pb| pa.iter().zip(pb).any(|(x, y)| x.abs_diff(*y) > tolerance));
        if differs {
            rgba.extend_from_slice(&[255, 0, 0, 255]);
        } else {
            rgba.extend(pa[..3].iter().map(|c| 191 + c / 4));
            rgba.push(255);
        }
    }
    Image {
        width: a.width,
        height: a.height,
        rgba,
    }
}

#[cfg(test)]
mod tests {
    use super::{Image, bmp, diff, diff_image, read_png, write_png};

    fn image(rgba: Vec<u8>) -> Image {
        Image {
            width: 2,
            height: 2,
            rgba,
        }
    }

    #[test]
    fn diff_counts_pixels_over_tolerance() {
        let a = image(vec![10; 16]);
        let mut b = a.clone();
        b.rgba[5] = 13;
        let result = diff(&a, &b, 2);
        assert_eq!(result.differing, 1);
        assert_eq!(result.max_channel, 3);
        assert_eq!(result.ratio, 0.25);
        assert!(diff(&a, &b, 3).is_identical());
        let marked = diff_image(&a, &b, 2);
        assert_eq!(&marked.rgba[4..8], &[255, 0, 0, 255]);
        assert_ne!(&marked.rgba[0..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn different_sizes_differ_everywhere() {
        let a = image(vec![0; 16]);
        let b = Image {
            width: 1,
            height: 1,
            rgba: vec![0; 4],
        };
        assert_eq!(diff(&a, &b, 255).differing, 4);
    }

    #[test]
    fn png_round_trip() {
        let original = image((0..16).map(|v| v * 15).collect());
        let path = std::env::temp_dir().join(format!("kai-png-{}.png", std::process::id()));
        write_png(&path, &original).unwrap();
        let read = read_png(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(read, original);
    }

    #[test]
    fn bmp_is_bottom_up_bgr_with_padded_rows() {
        let out = bmp(&image(vec![
            1, 2, 3, 255, 4, 5, 6, 255, //
            7, 8, 9, 255, 10, 11, 12, 255,
        ]));
        assert_eq!(&out[..2], b"BM");
        // Rows of 6 bytes padded to 8; the bottom row comes first.
        assert_eq!(out.len(), 54 + 8 * 2);
        assert_eq!(&out[54..60], &[9, 8, 7, 12, 11, 10]);
        assert_eq!(&out[62..68], &[3, 2, 1, 6, 5, 4]);
    }
}
