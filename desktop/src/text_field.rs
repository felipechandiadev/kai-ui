//! Single-line editable text field state over `cosmic-text`.
//!
//! Coordinates returned by [`TextFieldView`] are logical points relative to the
//! field's text origin, with the horizontal scroll already applied.

use std::ops::Range;

use cosmic_text::{
    Action, Attrs, Buffer, Cursor, Edit, Editor, Family, FontSystem, Metrics, Motion, Selection,
    Shaping, Wrap,
};

use crate::geometry::{Point, Rect};
use crate::ui_engine::{ImeIntent, Key, Modifiers, TextIntent};

pub const CARET_WIDTH: f32 = 1.5;
const PREEDIT_UNDERLINE_HEIGHT: f32 = 1.5;

/// System clipboard access, injectable so editing can be tested without one.
pub trait ClipboardPort {
    fn get_text(&mut self) -> Result<String, String>;
    fn set_text(&mut self, text: String) -> Result<(), String>;
}

pub struct SystemClipboard;

impl ClipboardPort for SystemClipboard {
    fn get_text(&mut self) -> Result<String, String> {
        arboard::Clipboard::new()
            .and_then(|mut clipboard| clipboard.get_text())
            .map_err(|error| error.to_string())
    }

    fn set_text(&mut self, text: String) -> Result<(), String> {
        arboard::Clipboard::new()
            .and_then(|mut clipboard| clipboard.set_text(text))
            .map_err(|error| error.to_string())
    }
}

/// Outcome of applying a text intent. `changed` means the field must be
/// redrawn and the IME cursor area refreshed; `status` reports errors the
/// user should know about (clipboard access).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextFieldEffect {
    pub changed: bool,
    pub status: Option<String>,
}

impl TextFieldEffect {
    fn changed() -> Self {
        Self {
            changed: true,
            status: None,
        }
    }

    fn with_status(changed: bool, status: impl Into<String>) -> Self {
        Self {
            changed,
            status: Some(status.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preedit {
    pub text: String,
    /// Byte range of the IME cursor inside `text`; `None` hides the caret.
    pub cursor: Option<Range<usize>>,
}

/// Geometry of the field's dynamic decorations; pure data used by paint.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextFieldDecorations {
    pub has_content: bool,
    pub scroll_x: f32,
    pub caret: Option<Rect>,
    pub selection: Vec<Rect>,
    pub preedit_underline: Vec<Rect>,
}

pub struct TextFieldView {
    /// Committed text plus any active preedit, ready to be drawn.
    pub buffer: Buffer,
    pub decorations: TextFieldDecorations,
}

pub struct TextFieldState {
    editor: Editor<'static>,
    preedit: Option<Preedit>,
    scroll_x: f32,
    width: f32,
    line_height: f32,
}

struct DisplayState {
    buffer: Buffer,
    caret: Option<Cursor>,
    selection: Option<(Cursor, Cursor)>,
    preedit: Option<(Cursor, Cursor)>,
}

struct DisplayMetrics {
    caret: Option<Rect>,
    selection: Vec<Rect>,
    preedit_underline: Vec<Rect>,
    content_width: f32,
}

impl TextFieldState {
    pub fn new(font_system: &mut FontSystem, metrics: Metrics) -> Self {
        let mut buffer = Buffer::new(font_system, metrics);
        buffer.set_wrap(Wrap::None);
        buffer.set_size(None, Some(metrics.line_height));
        buffer.set_text(
            "",
            &Attrs::new().family(Family::SansSerif),
            Shaping::Advanced,
            None,
        );
        Self {
            editor: Editor::new(buffer),
            preedit: None,
            scroll_x: 0.0,
            width: 1.0,
            line_height: metrics.line_height,
        }
    }

    pub fn editor(&self) -> &Editor<'static> {
        &self.editor
    }

    /// Apply an editing intent already gated by text-field focus.
    pub fn apply(
        &mut self,
        intent: TextIntent,
        font_system: &mut FontSystem,
        clipboard: &mut dyn ClipboardPort,
    ) -> TextFieldEffect {
        match intent {
            TextIntent::KeyDown { key, modifiers, .. } => {
                self.key_down(key, modifiers, font_system)
            }
            TextIntent::InsertText(text) => TextFieldEffect {
                changed: self.insert_text(font_system, &text),
                status: None,
            },
            TextIntent::Ime(event) => self.ime(event, font_system),
            TextIntent::SelectAll => {
                self.editor
                    .action(font_system, Action::Motion(Motion::BufferStart));
                self.editor
                    .set_selection(Selection::Normal(self.editor.cursor()));
                self.editor
                    .action(font_system, Action::Motion(Motion::BufferEnd));
                self.after_edit(font_system);
                TextFieldEffect::changed()
            }
            TextIntent::Copy => self.copy(false, font_system, clipboard),
            TextIntent::Cut => self.copy(true, font_system, clipboard),
            TextIntent::Paste => match clipboard.get_text() {
                Ok(text) => TextFieldEffect {
                    changed: self.insert_text(font_system, &text),
                    status: None,
                },
                Err(error) => TextFieldEffect::with_status(false, format!("Portapapeles: {error}")),
            },
        }
    }

    fn key_down(
        &mut self,
        key: Key,
        modifiers: Modifiers,
        font_system: &mut FontSystem,
    ) -> TextFieldEffect {
        let motion = match key {
            Key::ArrowLeft => Motion::Left,
            Key::ArrowRight => Motion::Right,
            Key::Home | Key::ArrowUp => Motion::BufferStart,
            Key::End | Key::ArrowDown => Motion::BufferEnd,
            Key::Backspace | Key::Delete => {
                let action = if key == Key::Backspace {
                    Action::Backspace
                } else {
                    Action::Delete
                };
                self.editor.action(font_system, action);
                self.after_edit(font_system);
                return TextFieldEffect::changed();
            }
            _ => return TextFieldEffect::default(),
        };
        let anchor = match self.editor.selection() {
            Selection::Normal(anchor) if modifiers.shift => anchor,
            _ => self.editor.cursor(),
        };
        self.editor.action(font_system, Action::Motion(motion));
        self.editor.set_selection(if modifiers.shift {
            Selection::Normal(anchor)
        } else {
            Selection::None
        });
        self.after_edit(font_system);
        TextFieldEffect::changed()
    }

    fn ime(&mut self, event: ImeIntent, font_system: &mut FontSystem) -> TextFieldEffect {
        match event {
            ImeIntent::Enabled => {
                self.set_preedit(font_system, None);
                TextFieldEffect::changed()
            }
            ImeIntent::Preedit { text, cursor } => {
                self.set_preedit(font_system, Some(Preedit { text, cursor }));
                TextFieldEffect::changed()
            }
            ImeIntent::Commit(text) => {
                self.insert_text(font_system, &text);
                self.set_preedit(font_system, None);
                TextFieldEffect::changed()
            }
            ImeIntent::Cancel | ImeIntent::Disabled => {
                self.set_preedit(font_system, None);
                TextFieldEffect::changed()
            }
        }
    }

    fn copy(
        &mut self,
        cut: bool,
        font_system: &mut FontSystem,
        clipboard: &mut dyn ClipboardPort,
    ) -> TextFieldEffect {
        let Some(text) = self.editor.copy_selection() else {
            return TextFieldEffect::default();
        };
        match clipboard.set_text(text) {
            Ok(()) if cut => {
                self.editor.delete_selection();
                self.after_edit(font_system);
                TextFieldEffect::changed()
            }
            Ok(()) => TextFieldEffect::default(),
            Err(error) => TextFieldEffect::with_status(false, format!("Portapapeles: {error}")),
        }
    }

    pub fn text(&self) -> String {
        self.editor.with_buffer(|buffer| {
            buffer
                .lines
                .iter()
                .map(|line| line.text())
                .collect::<Vec<_>>()
                .join("\n")
        })
    }

    /// Replace the committed text with `text` (the controlled value from the
    /// app) when it differs, dropping the selection and placing the caret at
    /// the end. Returns whether the text changed.
    pub fn set_text(&mut self, font_system: &mut FontSystem, text: &str) -> bool {
        let text = single_line(text);
        if text == self.text() {
            return false;
        }
        self.editor.with_buffer_mut(|buffer| {
            buffer.set_text(
                &text,
                &Attrs::new().family(Family::SansSerif),
                Shaping::Advanced,
                None,
            );
        });
        self.editor.set_selection(Selection::None);
        self.editor.set_cursor(Cursor::new(0, 0));
        self.editor
            .action(font_system, Action::Motion(Motion::BufferEnd));
        self.after_edit(font_system);
        true
    }

    /// Font size and line height from the input's computed style.
    pub fn set_metrics(&mut self, font_system: &mut FontSystem, metrics: Metrics) {
        if metrics.line_height == self.line_height
            && self.editor.with_buffer(|buffer| buffer.metrics()) == metrics
        {
            return;
        }
        self.line_height = metrics.line_height;
        self.editor.with_buffer_mut(|buffer| {
            buffer.set_metrics(metrics);
            buffer.set_size(None, Some(metrics.line_height));
        });
        self.after_edit(font_system);
    }

    pub fn preedit(&self) -> Option<&Preedit> {
        self.preedit.as_ref()
    }

    pub fn scroll_x(&self) -> f32 {
        self.scroll_x
    }

    /// Visible text width in logical points.
    pub fn set_width(&mut self, font_system: &mut FontSystem, width: f32) {
        let width = width.max(1.0);
        if width != self.width {
            self.width = width;
            self.reveal_caret(font_system);
        }
    }

    pub fn set_preedit(&mut self, font_system: &mut FontSystem, preedit: Option<Preedit>) {
        self.preedit = preedit.filter(|preedit| !preedit.text.is_empty());
        self.reveal_caret(font_system);
    }

    /// Insert committed text, replacing the selection. Line breaks become
    /// spaces because the field is single-line. Returns whether text changed.
    pub fn insert_text(&mut self, font_system: &mut FontSystem, text: &str) -> bool {
        let text = single_line(text);
        if text.is_empty() {
            return false;
        }
        self.preedit = None;
        self.editor.insert_string(&text, None);
        self.after_edit(font_system);
        true
    }

    /// Place the caret at a point relative to the text origin.
    pub fn click(&mut self, font_system: &mut FontSystem, local: Point) {
        self.editor.action(
            font_system,
            Action::Click {
                x: (local.x + self.scroll_x) as i32,
                y: local.y as i32,
            },
        );
        self.editor.set_selection(Selection::None);
        self.after_edit(font_system);
    }

    /// Extend the selection to a point relative to the text origin.
    pub fn drag(&mut self, font_system: &mut FontSystem, local: Point) {
        self.editor.action(
            font_system,
            Action::Drag {
                x: (local.x + self.scroll_x) as i32,
                y: local.y as i32,
            },
        );
        self.after_edit(font_system);
    }

    fn after_edit(&mut self, font_system: &mut FontSystem) {
        self.editor.shape_as_needed(font_system, false);
        self.reveal_caret(font_system);
    }

    pub fn view(&self, font_system: &mut FontSystem) -> TextFieldView {
        let display = self.display(font_system);
        let metrics = self.measure(&display);
        let has_content = display
            .buffer
            .lines
            .iter()
            .any(|line| !line.text().is_empty());
        let scroll = |rect: Rect| Rect {
            x: rect.x - self.scroll_x,
            ..rect
        };
        TextFieldView {
            buffer: display.buffer,
            decorations: TextFieldDecorations {
                has_content,
                scroll_x: self.scroll_x,
                caret: metrics.caret.map(scroll),
                selection: metrics.selection.into_iter().map(scroll).collect(),
                preedit_underline: metrics.preedit_underline.into_iter().map(scroll).collect(),
            },
        }
    }

    fn reveal_caret(&mut self, font_system: &mut FontSystem) {
        let display = self.display(font_system);
        let metrics = self.measure(&display);
        let caret_x = metrics.caret.map_or(0.0, |caret| caret.x);
        self.scroll_x = scroll_to_reveal(self.scroll_x, caret_x, self.width, metrics.content_width);
    }

    fn display(&self, font_system: &mut FontSystem) -> DisplayState {
        let mut display = self.editor.clone();
        let mut caret = Some(display.cursor());
        let mut selection = display.selection_bounds();
        let mut preedit_range = None;
        if let Some(preedit) = &self.preedit {
            display.delete_selection();
            let start = display.cursor();
            display.insert_string(&preedit.text, None);
            let end = display.cursor();
            preedit_range = Some((start, end));
            selection = None;
            caret = preedit
                .cursor
                .as_ref()
                .map(|cursor| Cursor::new(start.line, (start.index + cursor.start).min(end.index)));
        }
        display.shape_as_needed(font_system, false);
        DisplayState {
            buffer: display.with_buffer(|buffer| buffer.clone()),
            caret,
            selection,
            preedit: preedit_range,
        }
    }

    fn measure(&self, display: &DisplayState) -> DisplayMetrics {
        let mut metrics = DisplayMetrics {
            caret: None,
            selection: Vec::new(),
            preedit_underline: Vec::new(),
            content_width: 0.0,
        };
        for run in display.buffer.layout_runs() {
            metrics.content_width = metrics.content_width.max(run.line_w);
            if let Some(cursor) = display.caret
                && let Some(x) = run.cursor_position(&cursor)
            {
                metrics.caret = Some(Rect {
                    x,
                    y: run.line_top,
                    width: CARET_WIDTH,
                    height: self.line_height,
                });
            }
            if let Some((start, end)) = display.selection {
                metrics
                    .selection
                    .extend(run.highlight(start, end).map(|(x, width)| Rect {
                        x,
                        y: run.line_top,
                        width,
                        height: run.line_height,
                    }));
            }
            if let Some((start, end)) = display.preedit {
                metrics
                    .preedit_underline
                    .extend(run.highlight(start, end).map(|(x, width)| Rect {
                        x,
                        y: run.line_top + run.line_height - PREEDIT_UNDERLINE_HEIGHT,
                        width,
                        height: PREEDIT_UNDERLINE_HEIGHT,
                    }));
            }
        }
        metrics
    }
}

fn single_line(text: &str) -> String {
    text.chars()
        .map(|character| match character {
            '\r' | '\n' => ' ',
            other => other,
        })
        .filter(|character| !character.is_control())
        .collect()
}

/// Horizontal scroll that keeps a caret at `caret_x` (unscrolled) inside a
/// viewport of `viewport_width`, moving as little as possible.
pub fn scroll_to_reveal(
    scroll_x: f32,
    caret_x: f32,
    viewport_width: f32,
    content_width: f32,
) -> f32 {
    let viewport = viewport_width.max(0.0);
    let caret_end = caret_x + CARET_WIDTH;
    let mut next = scroll_x;
    if caret_x < next {
        next = caret_x;
    }
    if caret_end > next + viewport {
        next = caret_end - viewport;
    }
    let max = (content_width.max(caret_end) - viewport).max(0.0);
    next.clamp(0.0, max)
}

#[cfg(test)]
mod tests {
    use cosmic_text::{Edit, FontSystem, Metrics};

    use super::{
        CARET_WIDTH, ClipboardPort, Preedit, TextFieldEffect, TextFieldState, scroll_to_reveal,
    };
    use crate::geometry::Point;
    use crate::ui_engine::{ImeIntent, Key, Modifiers, TextIntent};

    fn field(font_system: &mut FontSystem, width: f32) -> TextFieldState {
        let mut field = TextFieldState::new(font_system, Metrics::new(15.0, 22.0));
        field.set_width(font_system, width);
        field
    }

    #[derive(Default)]
    struct MockClipboard {
        contents: Option<String>,
        fail: bool,
    }

    impl ClipboardPort for MockClipboard {
        fn get_text(&mut self) -> Result<String, String> {
            if self.fail {
                return Err("sin acceso".to_owned());
            }
            self.contents.clone().ok_or_else(|| "vacío".to_owned())
        }

        fn set_text(&mut self, text: String) -> Result<(), String> {
            if self.fail {
                return Err("sin acceso".to_owned());
            }
            self.contents = Some(text);
            Ok(())
        }
    }

    fn key(key: Key, shift: bool) -> TextIntent {
        TextIntent::KeyDown {
            key,
            modifiers: Modifiers {
                shift,
                ..Modifiers::default()
            },
            repeat: false,
        }
    }

    #[test]
    fn controlled_text_and_metrics_replace_content_only_when_different() {
        let mut fonts = FontSystem::new();
        let mut field = field(&mut fonts, 300.0);
        field.insert_text(&mut fonts, "hola");
        assert!(!field.set_text(&mut fonts, "hola"));
        assert!(field.set_text(&mut fonts, "chau\nmundo"));
        assert_eq!(field.text(), "chau mundo");
        assert_eq!(field.editor().cursor().index, "chau mundo".len());
        assert!(field.set_text(&mut fonts, ""));
        assert!(!field.view(&mut fonts).decorations.has_content);

        field.set_metrics(&mut fonts, Metrics::new(14.0, 20.0));
        let caret = field.view(&mut fonts).decorations.caret.unwrap();
        assert_eq!(caret.height, 20.0);
    }

    #[test]
    fn shift_navigation_extends_selection_and_plain_motion_clears_it() {
        let mut fonts = FontSystem::new();
        let mut clipboard = MockClipboard::default();
        let mut field = field(&mut fonts, 300.0);
        field.insert_text(&mut fonts, "hola mundo");

        field.apply(key(Key::Home, false), &mut fonts, &mut clipboard);
        for _ in 0..4 {
            assert!(
                field
                    .apply(key(Key::ArrowRight, true), &mut fonts, &mut clipboard)
                    .changed
            );
        }
        assert_eq!(field.editor().copy_selection().as_deref(), Some("hola"));

        field.apply(key(Key::ArrowRight, false), &mut fonts, &mut clipboard);
        assert_eq!(field.editor().copy_selection(), None);
        assert_eq!(field.editor().cursor().index, 5);
    }

    #[test]
    fn select_all_copy_cut_and_paste_use_the_clipboard_port() {
        let mut fonts = FontSystem::new();
        let mut clipboard = MockClipboard::default();
        let mut field = field(&mut fonts, 300.0);
        field.insert_text(&mut fonts, "kai");

        field.apply(TextIntent::SelectAll, &mut fonts, &mut clipboard);
        field.apply(TextIntent::Copy, &mut fonts, &mut clipboard);
        assert_eq!(clipboard.contents.as_deref(), Some("kai"));
        assert_eq!(field.text(), "kai");

        let cut = field.apply(TextIntent::Cut, &mut fonts, &mut clipboard);
        assert!(cut.changed);
        assert_eq!(field.text(), "");

        clipboard.contents = Some("uno\ndos".to_owned());
        assert!(
            field
                .apply(TextIntent::Paste, &mut fonts, &mut clipboard)
                .changed
        );
        assert_eq!(field.text(), "uno dos");

        assert_eq!(
            field.apply(TextIntent::Copy, &mut fonts, &mut clipboard),
            TextFieldEffect::default(),
            "copy without selection is a no-op"
        );

        clipboard.fail = true;
        let failed = field.apply(TextIntent::Paste, &mut fonts, &mut clipboard);
        assert!(!failed.changed);
        assert_eq!(failed.status.as_deref(), Some("Portapapeles: sin acceso"));
        assert_eq!(field.text(), "uno dos");
    }

    #[test]
    fn preedit_then_commit_inserts_text_exactly_once() {
        let mut fonts = FontSystem::new();
        let mut clipboard = MockClipboard::default();
        let mut field = field(&mut fonts, 300.0);

        field.apply(
            TextIntent::Ime(ImeIntent::Preedit {
                text: "にほん".to_owned(),
                cursor: Some(9..9),
            }),
            &mut fonts,
            &mut clipboard,
        );
        assert_eq!(field.text(), "");
        assert!(field.preedit().is_some());

        let commit = field.apply(
            TextIntent::Ime(ImeIntent::Commit("日本".to_owned())),
            &mut fonts,
            &mut clipboard,
        );
        assert!(commit.changed);
        assert_eq!(field.text(), "日本");
        assert!(field.preedit().is_none());

        field.apply(
            TextIntent::Ime(ImeIntent::Preedit {
                text: String::new(),
                cursor: None,
            }),
            &mut fonts,
            &mut clipboard,
        );
        assert_eq!(field.text(), "日本");
        assert!(
            field
                .view(&mut fonts)
                .decorations
                .preedit_underline
                .is_empty()
        );
    }

    #[test]
    fn backspace_deletes_selection_then_previous_character() {
        let mut fonts = FontSystem::new();
        let mut clipboard = MockClipboard::default();
        let mut field = field(&mut fonts, 300.0);
        field.insert_text(&mut fonts, "abcdef");
        field.apply(key(Key::ArrowLeft, true), &mut fonts, &mut clipboard);
        field.apply(key(Key::ArrowLeft, true), &mut fonts, &mut clipboard);

        field.apply(key(Key::Backspace, false), &mut fonts, &mut clipboard);
        assert_eq!(field.text(), "abcd");
        field.apply(key(Key::Backspace, false), &mut fonts, &mut clipboard);
        assert_eq!(field.text(), "abc");
        field.apply(key(Key::Home, false), &mut fonts, &mut clipboard);
        field.apply(key(Key::Delete, false), &mut fonts, &mut clipboard);
        assert_eq!(field.text(), "bc");
    }

    #[test]
    fn enter_does_not_alter_single_line_text() {
        let mut fonts = FontSystem::new();
        let mut clipboard = MockClipboard::default();
        let mut field = field(&mut fonts, 300.0);
        field.insert_text(&mut fonts, "x");
        let effect = field.apply(key(Key::Enter, false), &mut fonts, &mut clipboard);
        assert_eq!(effect, TextFieldEffect::default());
        assert_eq!(field.text(), "x");
    }

    #[test]
    fn scroll_to_reveal_moves_minimally_and_clamps() {
        assert_eq!(scroll_to_reveal(0.0, 50.0, 100.0, 300.0), 0.0);
        assert_eq!(
            scroll_to_reveal(0.0, 150.0, 100.0, 300.0),
            50.0 + CARET_WIDTH
        );
        assert_eq!(scroll_to_reveal(120.0, 80.0, 100.0, 300.0), 80.0);
        assert_eq!(
            scroll_to_reveal(500.0, 300.0, 100.0, 300.0),
            200.0 + CARET_WIDTH
        );
        assert_eq!(scroll_to_reveal(40.0, 0.0, 100.0, 50.0), 0.0);
    }

    #[test]
    fn long_text_stays_on_one_line_and_caret_remains_visible() {
        let mut font_system = FontSystem::new();
        let mut field = field(&mut font_system, 120.0);
        let long = "Texto suficientemente largo para desbordar el campo de una línea";
        assert!(field.insert_text(&mut font_system, long));

        let view = field.view(&mut font_system);
        assert_eq!(view.buffer.layout_runs().count(), 1);
        let view = view.decorations;
        assert!(view.scroll_x > 0.0);
        let caret = view.caret.expect("caret visible after typing");
        assert!(caret.x >= 0.0);
        assert!(caret.x + caret.width <= 120.0 + 0.01);
    }

    #[test]
    fn line_breaks_are_inserted_as_spaces() {
        let mut font_system = FontSystem::new();
        let mut field = field(&mut font_system, 300.0);
        field.insert_text(&mut font_system, "uno\ndos\r\ntres");
        assert_eq!(field.text(), "uno dos  tres");
        assert!(!field.insert_text(&mut font_system, "\u{7}"));
    }

    #[test]
    fn preedit_is_underlined_and_caret_follows_ime_cursor() {
        let mut font_system = FontSystem::new();
        let mut field = field(&mut font_system, 300.0);
        field.insert_text(&mut font_system, "ab");
        field.set_preedit(
            &mut font_system,
            Some(Preedit {
                text: "にほ".to_owned(),
                cursor: Some(3..3),
            }),
        );

        let view = field.view(&mut font_system).decorations;
        assert_eq!(view.preedit_underline.len(), 1);
        let underline = view.preedit_underline[0];
        let caret = view.caret.expect("IME cursor shown");
        assert!(caret.x > underline.x);
        assert!(caret.x < underline.x + underline.width);
        assert!(view.selection.is_empty());
        assert_eq!(field.text(), "ab");

        field.set_preedit(&mut font_system, None);
        let view = field.view(&mut font_system).decorations;
        assert!(view.preedit_underline.is_empty());
    }

    #[test]
    fn preedit_without_cursor_hides_caret() {
        let mut font_system = FontSystem::new();
        let mut field = field(&mut font_system, 300.0);
        field.set_preedit(
            &mut font_system,
            Some(Preedit {
                text: "か".to_owned(),
                cursor: None,
            }),
        );
        let view = field.view(&mut font_system).decorations;
        assert!(view.caret.is_none());
        assert!(view.has_content);
    }

    #[test]
    fn click_maps_through_horizontal_scroll() {
        let mut font_system = FontSystem::new();
        let mut field = field(&mut font_system, 80.0);
        field.insert_text(&mut font_system, "0123456789012345678901234567890123456789");
        let scroll = field.scroll_x();
        assert!(scroll > 0.0);

        field.click(&mut font_system, Point { x: 0.0, y: 5.0 });
        let index = field.editor().cursor().index;
        assert!(
            index > 0,
            "click at the visible left edge maps past scrolled-out text"
        );
    }
}
