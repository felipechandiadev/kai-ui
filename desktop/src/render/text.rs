//! Font system and text buffers addressed by [`TextId`].

use std::collections::HashMap;

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Weight, Wrap};

use super::TextId;
use crate::geometry::Size;
use crate::layout::TextMeasure;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    pub font_size: f32,
    pub line_height: f32,
    pub weight: Weight,
    pub wrap: bool,
}

impl TextStyle {
    fn attrs(self) -> Attrs<'static> {
        Attrs::new().family(Family::SansSerif).weight(self.weight)
    }
}

struct Entry {
    buffer: Buffer,
    style: TextStyle,
    /// Source text for [`TextStore::sync_text`]; empty for installed buffers.
    text: String,
}

pub struct TextStore {
    pub font_system: FontSystem,
    entries: HashMap<TextId, Entry>,
}

impl TextStore {
    pub fn new(font_system: FontSystem) -> Self {
        Self {
            font_system,
            entries: HashMap::new(),
        }
    }

    /// Create or update a buffer; reshapes only when text or style changed.
    /// Returns whether anything changed.
    pub fn sync_text(&mut self, id: TextId, text: &str, style: TextStyle) -> bool {
        if let Some(entry) = self.entries.get(&id)
            && entry.text == text
            && entry.style == style
        {
            return false;
        }
        let mut buffer = Buffer::new(
            &mut self.font_system,
            Metrics::new(style.font_size, style.line_height),
        );
        // CSS `overflow-wrap: normal`: break between words only.
        buffer.set_wrap(if style.wrap { Wrap::Word } else { Wrap::None });
        buffer.set_text(text, &style.attrs(), Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut self.font_system, false);
        self.entries.insert(
            id,
            Entry {
                buffer,
                style,
                text: text.to_owned(),
            },
        );
        true
    }

    /// Install a buffer produced elsewhere (e.g. the text field display).
    pub fn set_buffer(&mut self, id: TextId, buffer: Buffer, style: TextStyle) {
        self.entries.insert(
            id,
            Entry {
                buffer,
                style,
                text: String::new(),
            },
        );
    }

    pub fn buffer(&self, id: TextId) -> Option<&Buffer> {
        self.entries.get(&id).map(|entry| &entry.buffer)
    }

    /// Split borrow used by the renderer: font system plus buffer lookup.
    pub fn parts(&mut self) -> (&mut FontSystem, TextBuffers<'_>) {
        (&mut self.font_system, TextBuffers(&self.entries))
    }
}

impl TextStore {
    /// Lays the buffer out at `max_width` (wrapping styles only) and returns
    /// its size. The buffer keeps that width, so measuring again at the final
    /// layout width leaves it ready to draw.
    fn measure_buffer(&mut self, id: TextId, max_width: Option<f32>) -> Size {
        let Some(entry) = self.entries.get_mut(&id) else {
            return Size::default();
        };
        let width = if entry.style.wrap {
            max_width.map(|width| width.max(1.0))
        } else {
            None
        };
        entry.buffer.set_size(width, None);
        entry
            .buffer
            .shape_until_scroll(&mut self.font_system, false);
        let (width, height) =
            entry
                .buffer
                .layout_runs()
                .fold((0.0_f32, 0.0_f32), |(width, height), run| {
                    (
                        width.max(run.line_w),
                        height.max(run.line_top + run.line_height),
                    )
                });
        Size {
            width,
            height: height.max(entry.style.line_height),
        }
    }
}

impl TextMeasure for TextStore {
    fn measure(
        &mut self,
        id: TextId,
        text: &str,
        style: TextStyle,
        max_width: Option<f32>,
    ) -> Size {
        self.sync_text(id, text, style);
        self.measure_buffer(id, max_width)
    }
}

pub struct TextBuffers<'a>(&'a HashMap<TextId, Entry>);

impl<'a> TextBuffers<'a> {
    pub fn get(&self, id: TextId) -> Option<&'a Buffer> {
        self.0.get(&id).map(|entry| &entry.buffer)
    }
}

#[cfg(test)]
mod tests {
    use cosmic_text::{FontSystem, Weight};

    use super::{TextStore, TextStyle};
    use crate::element::WidgetId;
    use crate::layout::TextMeasure;
    use crate::render::TextId;

    const BODY: TextStyle = TextStyle {
        font_size: 15.0,
        line_height: 22.0,
        weight: Weight::NORMAL,
        wrap: true,
    };
    const TEXT: TextId = TextId::content(WidgetId(1));

    #[test]
    fn wrapped_text_reports_larger_height_at_narrow_width() {
        let mut store = TextStore::new(FontSystem::new());
        let text = "This responsive description wraps when its parent box becomes narrower.";
        let wide = store.measure(TEXT, text, BODY, Some(520.0));
        let narrow = store.measure(TEXT, text, BODY, Some(170.0));
        assert!(narrow.height > wide.height);
        assert!(narrow.width <= 170.0);
        let unconstrained = store.measure(TEXT, text, BODY, None);
        assert_eq!(unconstrained.height, 22.0);
        assert!(unconstrained.width > 170.0);
        assert_eq!(
            store
                .measure(TextId::content(WidgetId(9)), "", BODY, Some(100.0))
                .height,
            22.0
        );
    }

    #[test]
    fn sync_text_reshapes_only_on_change_and_unwrapped_text_stays_single_line() {
        let mut store = TextStore::new(FontSystem::new());
        let style = TextStyle {
            wrap: false,
            ..BODY
        };
        assert!(store.sync_text(TEXT, "placeholder", style));
        assert!(!store.sync_text(TEXT, "placeholder", style));
        assert!(store.sync_text(
            TEXT,
            "un texto bastante largo que no debe envolver en varias líneas",
            style
        ));
        let size = store.measure(
            TEXT,
            "un texto bastante largo que no debe envolver en varias líneas",
            style,
            Some(50.0),
        );
        assert_eq!(size.height, 22.0);
        assert!(size.width > 50.0);
        let buffer = store.buffer(TEXT).unwrap();
        assert_eq!(buffer.layout_runs().count(), 1);
        assert_eq!(buffer.metrics().line_height, 22.0);
        assert!(store.sync_text(TEXT, "placeholder", BODY), "style change");
    }
}
