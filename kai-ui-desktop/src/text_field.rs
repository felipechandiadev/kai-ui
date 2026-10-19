//! Single-line editable text field state over `cosmic-text`.
//!
//! Coordinates returned by [`TextFieldView`] are logical points relative to the
//! field's text origin, with the horizontal scroll already applied.

use std::ops::Range;

use cosmic_text::{
    Action, Buffer, Cursor, Edit, Editor, FontSystem, Metrics, Motion, Selection, Shaping, Wrap,
};

use crate::fonts::ui_font_attrs;
use crate::geometry::{Point, Rect};
use crate::ui_engine::{ImeIntent, Key, Modifiers, TextIntent};

pub const CARET_WIDTH: f32 = 1.5;
const PREEDIT_UNDERLINE_HEIGHT: f32 = 1.5;
const BULLET: char = '•';

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
    /// Draw one bullet per character and refuse to copy (password fields).
    masked: bool,
    /// Allow navigation, selection and copy but no edits.
    read_only: bool,
    /// Avoid reshaping a cloned editor multiple times per frame.
    display_cache: Option<DisplayState>,
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
        buffer.set_text("", &ui_font_attrs(), Shaping::Advanced, None);
        Self {
            editor: Editor::new(buffer),
            preedit: None,
            scroll_x: 0.0,
            width: 1.0,
            line_height: metrics.line_height,
            masked: false,
            read_only: false,
            display_cache: None,
        }
    }

    fn invalidate_display_cache(&mut self) {
        self.display_cache = None;
    }

    pub fn editor(&self) -> &Editor<'static> {
        &self.editor
    }

    pub fn set_masked(&mut self, font_system: &mut FontSystem, masked: bool) {
        if masked != self.masked {
            self.masked = masked;
            self.invalidate_display_cache();
            self.reveal_caret(font_system);
        }
    }

    pub fn set_read_only(&mut self, font_system: &mut FontSystem, read_only: bool) {
        if read_only != self.read_only {
            self.read_only = read_only;
            if read_only {
                self.set_preedit(font_system, None);
            }
        }
    }

    pub fn select_all(&mut self, font_system: &mut FontSystem) {
        self.editor
            .action(font_system, Action::Motion(Motion::BufferStart));
        self.editor
            .set_selection(Selection::Normal(self.editor.cursor()));
        self.editor
            .action(font_system, Action::Motion(Motion::BufferEnd));
        self.after_edit(font_system);
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
            TextIntent::InsertText(_) | TextIntent::Paste if self.read_only => {
                TextFieldEffect::default()
            }
            TextIntent::InsertText(text) => TextFieldEffect {
                changed: self.insert_text(font_system, &text),
                status: None,
            },
            TextIntent::Ime(event) => self.ime(event, font_system),
            TextIntent::SelectAll => {
                self.select_all(font_system);
                TextFieldEffect::changed()
            }
            TextIntent::Copy | TextIntent::Cut if self.masked => TextFieldEffect::default(),
            TextIntent::Copy => self.copy(false, font_system, clipboard),
            TextIntent::Cut => self.copy(!self.read_only, font_system, clipboard),
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
            Key::Backspace | Key::Delete if self.read_only => {
                return TextFieldEffect::default();
            }
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
            ImeIntent::Preedit { .. } | ImeIntent::Commit(_) if self.read_only => {
                TextFieldEffect::default()
            }
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
        let current = self.text();
        if text == current {
            return false;
        }
        let cursor = self.editor.cursor();
        let caret_byte = cursor.index.min(current.len());
        self.editor.with_buffer_mut(|buffer| {
            buffer.set_text(&text, &ui_font_attrs(), Shaping::Advanced, None);
        });
        let clamped = caret_byte.min(text.len());
        self.editor.set_cursor(Cursor::new(0, clamped));
        self.editor.set_selection(Selection::None);
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
        self.invalidate_display_cache();
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
        let local = self.on_line(local);
        if self.masked {
            if let Some(cursor) = self.masked_hit(font_system, local) {
                self.editor.set_cursor(cursor);
            }
        } else {
            self.editor.action(
                font_system,
                Action::Click {
                    x: (local.x + self.scroll_x) as i32,
                    y: local.y as i32,
                },
            );
        }
        self.editor.set_selection(Selection::None);
        self.after_edit(font_system);
    }

    /// Extend the selection to a point relative to the text origin.
    pub fn drag(&mut self, font_system: &mut FontSystem, local: Point) {
        let local = self.on_line(local);
        if self.masked {
            if let Some(cursor) = self.masked_hit(font_system, local) {
                if self.editor.selection() == Selection::None {
                    self.editor
                        .set_selection(Selection::Normal(self.editor.cursor()));
                }
                self.editor.set_cursor(cursor);
            }
        } else {
            self.editor.action(
                font_system,
                Action::Drag {
                    x: (local.x + self.scroll_x) as i32,
                    y: local.y as i32,
                },
            );
        }
        self.after_edit(font_system);
    }

    /// Pointer positions above or below the single line (clicks delegated
    /// from the field's outline) still target it.
    fn on_line(&self, local: Point) -> Point {
        Point {
            x: local.x,
            y: local.y.clamp(0.0, (self.line_height - 1.0).max(0.0)),
        }
    }

    /// Hit-test the bullets actually on screen and map back to the real text.
    fn masked_hit(&mut self, font_system: &mut FontSystem, local: Point) -> Option<Cursor> {
        let scroll_x = self.scroll_x;
        let display = self.cached_display(font_system);
        let hit = display.buffer.hit(local.x + scroll_x, local.y)?;
        let text = self.text();
        Some(Cursor::new(0, unmasked_index(&text, hit.index)))
    }

    fn after_edit(&mut self, font_system: &mut FontSystem) {
        self.invalidate_display_cache();
        self.editor.shape_as_needed(font_system, false);
        self.reveal_caret(font_system);
    }

    pub fn view(&mut self, font_system: &mut FontSystem) -> TextFieldView {
        let scroll_x = self.scroll_x;
        let line_height = self.line_height;
        let display = self.cached_display(font_system);
        let metrics = measure_display(display, line_height);
        let has_content = display
            .buffer
            .lines
            .iter()
            .any(|line| !line.text().is_empty());
        let scroll = |rect: Rect| Rect {
            x: rect.x - scroll_x,
            ..rect
        };
        TextFieldView {
            buffer: display.buffer.clone(),
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
        let line_height = self.line_height;
        let display = self.cached_display(font_system);
        let metrics = measure_display(display, line_height);
        let caret_x = metrics.caret.map_or(0.0, |caret| caret.x);
        self.scroll_x = scroll_to_reveal(self.scroll_x, caret_x, self.width, metrics.content_width);
    }

    fn cached_display(&mut self, font_system: &mut FontSystem) -> &DisplayState {
        if self.display_cache.is_none() {
            self.display_cache = Some(self.build_display(font_system));
        }
        self.display_cache.as_ref().expect("display cache")
    }

    fn build_display(&self, font_system: &mut FontSystem) -> DisplayState {
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
        if self.masked {
            let text = display.with_buffer(|buffer| {
                buffer
                    .lines
                    .first()
                    .map_or_else(String::new, |line| line.text().to_owned())
            });
            let mask = |cursor: Cursor| Cursor::new(cursor.line, masked_index(&text, cursor.index));
            caret = caret.map(mask);
            selection = selection.map(|(start, end)| (mask(start), mask(end)));
            preedit_range = preedit_range.map(|(start, end)| (mask(start), mask(end)));
            let bullets: String = text.chars().map(|_| BULLET).collect();
            display.with_buffer_mut(|buffer| {
                buffer.set_text(&bullets, &ui_font_attrs(), Shaping::Advanced, None);
            });
        }
        display.shape_as_needed(font_system, false);
        DisplayState {
            buffer: display.with_buffer(|buffer| buffer.clone()),
            caret,
            selection,
            preedit: preedit_range,
        }
    }
}

fn measure_display(display: &DisplayState, line_height: f32) -> DisplayMetrics {
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
                height: line_height,
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

fn single_line(text: &str) -> String {
    let mut out = String::new();
    let mut pending_space = false;
    for character in text.chars() {
        match character {
            '\r' | '\n' => pending_space = !out.is_empty(),
            c if c.is_control() => {}
            other => {
                if pending_space {
                    out.push(' ');
                    pending_space = false;
                }
                out.push(other);
            }
        }
    }
    out
}

/// Byte index in the bullet string for byte `index` of `text`.
fn masked_index(text: &str, index: usize) -> usize {
    text[..index.min(text.len())].chars().count() * BULLET.len_utf8()
}

/// Byte index in `text` for byte `index` of its bullet string.
fn unmasked_index(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index / BULLET.len_utf8())
        .map_or(text.len(), |(byte, _)| byte)
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
        CARET_WIDTH, ClipboardPort, Preedit, TextFieldEffect, TextFieldState, masked_index,
        scroll_to_reveal, unmasked_index,
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
        assert_eq!(field.editor().cursor().index, "hola".len());
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
        assert_eq!(field.text(), "uno dos tres");
        assert!(!field.insert_text(&mut font_system, "\u{7}"));
        assert!(!field.insert_text(&mut font_system, "\r"));
    }

    #[test]
    fn set_text_keeps_caret_when_value_is_transformed_externally() {
        let mut font_system = FontSystem::new();
        let mut clipboard = MockClipboard::default();
        let mut field = field(&mut font_system, 300.0);
        field.insert_text(&mut font_system, "ab");
        field.apply(key(Key::ArrowLeft, false), &mut font_system, &mut clipboard);

        assert!(field.set_text(&mut font_system, "AB"));
        assert_eq!(field.text(), "AB");
        assert_eq!(field.editor().cursor().index, 1);
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
    fn masked_fields_draw_bullets_remap_carets_and_refuse_to_copy() {
        let mut fonts = FontSystem::new();
        let mut clipboard = MockClipboard::default();
        let mut field = field(&mut fonts, 300.0);
        field.insert_text(&mut fonts, "añb");
        field.set_masked(&mut fonts, true);
        let view = field.view(&mut fonts);
        assert_eq!(view.buffer.lines[0].text(), "•••");
        assert_eq!(field.text(), "añb", "the editor keeps the real value");
        let end = view.decorations.caret.unwrap().x;

        field.apply(key(Key::ArrowLeft, true), &mut fonts, &mut clipboard);
        let view = field.view(&mut fonts).decorations;
        assert_eq!(view.selection.len(), 1);
        let bullet = view.selection[0].width;
        assert!((view.caret.unwrap().x - (end - bullet)).abs() < 0.01);

        field.apply(TextIntent::Copy, &mut fonts, &mut clipboard);
        assert_eq!(
            field.apply(TextIntent::Cut, &mut fonts, &mut clipboard),
            TextFieldEffect::default()
        );
        assert_eq!(clipboard.contents, None);
        assert_eq!(field.text(), "añb");

        field.click(
            &mut fonts,
            Point {
                x: bullet * 1.4,
                y: 5.0,
            },
        );
        assert_eq!(
            field.editor().cursor().index,
            1,
            "clicks hit-test bullets, not the hidden glyphs"
        );
        field.drag(
            &mut fonts,
            Point {
                x: bullet * 2.6,
                y: 5.0,
            },
        );
        assert_eq!(field.editor().copy_selection().as_deref(), Some("ñb"));

        field.set_masked(&mut fonts, false);
        assert_eq!(field.view(&mut fonts).buffer.lines[0].text(), "añb");
    }

    #[test]
    fn read_only_fields_navigate_and_copy_but_never_edit() {
        let mut fonts = FontSystem::new();
        let mut clipboard = MockClipboard::default();
        let mut field = field(&mut fonts, 300.0);
        field.insert_text(&mut fonts, "fijo");
        field.set_read_only(&mut fonts, true);

        for intent in [
            key(Key::Backspace, false),
            key(Key::Delete, false),
            TextIntent::InsertText("x".to_owned()),
            TextIntent::Ime(ImeIntent::Preedit {
                text: "か".to_owned(),
                cursor: None,
            }),
            TextIntent::Ime(ImeIntent::Commit("か".to_owned())),
        ] {
            field.apply(intent, &mut fonts, &mut clipboard);
        }
        clipboard.contents = Some("pegado".to_owned());
        field.apply(TextIntent::Paste, &mut fonts, &mut clipboard);
        assert_eq!(field.text(), "fijo");
        assert!(field.preedit().is_none());

        field.select_all(&mut fonts);
        field.apply(TextIntent::Cut, &mut fonts, &mut clipboard);
        assert_eq!(clipboard.contents.as_deref(), Some("fijo"), "cut copies");
        assert_eq!(field.text(), "fijo");
        assert!(
            field
                .apply(key(Key::ArrowLeft, false), &mut fonts, &mut clipboard)
                .changed
        );
    }

    #[test]
    fn mask_index_mapping_round_trips_multibyte_text() {
        assert_eq!(masked_index("añb", 3), 6);
        assert_eq!(masked_index("añb", 99), 9);
        assert_eq!(unmasked_index("añb", 6), 3);
        assert_eq!(unmasked_index("añb", 9), 4);
        assert_eq!(unmasked_index("", 3), 0);
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

        field.click(&mut font_system, Point { x: 500.0, y: -12.0 });
        assert_eq!(field.editor().cursor().index, 40, "above the line");
        field.click(
            &mut font_system,
            Point {
                x: -1000.0,
                y: 60.0,
            },
        );
        assert_eq!(field.editor().cursor().index, 0, "below the line");
    }
}
