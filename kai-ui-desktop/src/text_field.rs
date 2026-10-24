//! Editable text field state over `cosmic-text`: one line that scrolls
//! sideways, or several wrapped lines that scroll vertically (text areas).
//!
//! Coordinates returned by [`TextFieldView`] are logical points relative to the
//! field's text origin, with the scroll already applied.

use std::ops::Range;
use std::time::Duration;

use cosmic_text::{
    Action, Buffer, Cursor, Edit, Editor, FontSystem, Metrics, Motion, Selection, Shaping, Wrap,
};

use crate::fonts::ui_font_attrs;
use crate::geometry::{Point, Rect};
use crate::ui_engine::{ImeIntent, Key, Modifiers, TextIntent};

pub const CARET_WIDTH: f32 = 1.5;
/// Half-period of the text insertion-point blink (AppKit `NSTextView` ≈ 530 ms).
pub const CARET_BLINK_PERIOD: Duration = Duration::from_millis(530);

/// Whether the caret is in its visible half given elapsed time since the last
/// reset to visible. With `reduce_motion`, the caret stays visible.
pub fn caret_blink_visible(elapsed: Duration, reduce_motion: bool) -> bool {
    if reduce_motion {
        return true;
    }
    let period = CARET_BLINK_PERIOD.as_nanos();
    if period == 0 {
        return true;
    }
    (elapsed.as_nanos() / period).is_multiple_of(2)
}
const PREEDIT_UNDERLINE_HEIGHT: f32 = 1.5;
const BULLET: char = '•';

/// System clipboard access, injectable so editing can be tested without one.
///
/// Plain text is required. Optional HTML / RTF let Document (and hosts) round-trip
/// rich runs: WordCraft `edit.copy` emits HTML; `edit.paste` accepts `html` / `rtf`.
/// Defaults are no-ops so TextField and simple mocks stay text-only.
pub trait ClipboardPort {
    fn get_text(&mut self) -> Result<String, String>;
    fn set_text(&mut self, text: String) -> Result<(), String>;

    /// HTML fragment from the host clipboard, if any (`None` = absent / unsupported).
    fn get_html(&mut self) -> Result<Option<String>, String> {
        Ok(None)
    }

    /// Place HTML plus optional plain alternative. Default: store only `alt_text` via [`Self::set_text`].
    fn set_html(&mut self, _html: String, alt_text: Option<String>) -> Result<(), String> {
        match alt_text {
            Some(text) => self.set_text(text),
            None => Ok(()),
        }
    }

    /// RTF from the host, if any. arboard has no RTF API — [`SystemClipboard`] leaves this `None`;
    /// in-memory ports may carry RTF for Document paste tests / future hosts.
    fn get_rtf(&mut self) -> Result<Option<String>, String> {
        Ok(None)
    }

    fn set_rtf(&mut self, _rtf: String) -> Result<(), String> {
        Ok(())
    }

    /// Raw image pixels from the host clipboard (`None` = absent / unsupported).
    /// Bytes are tightly packed RGBA8, row-major; `width * height * 4 == bytes.len()`.
    /// arboard exposes this; used by Document paste when there is no HTML/RTF/text.
    fn get_image(&mut self) -> Result<Option<ClipboardImage>, String> {
        Ok(None)
    }

    /// Place an RGBA8 image on the host clipboard. Default: no-op success.
    fn set_image(&mut self, _image: ClipboardImage) -> Result<(), String> {
        Ok(())
    }
}

/// RGBA8 bitmap for [`ClipboardPort`] image get/set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipboardImage {
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
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

    fn get_html(&mut self) -> Result<Option<String>, String> {
        match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get().html()) {
            Ok(html) if !html.trim().is_empty() => Ok(Some(html)),
            Ok(_) => Ok(None),
            // Empty board / no HTML type is normal — fall back to plain text.
            Err(_) => Ok(None),
        }
    }

    fn set_html(&mut self, html: String, alt_text: Option<String>) -> Result<(), String> {
        arboard::Clipboard::new()
            .and_then(|mut clipboard| clipboard.set_html(html, alt_text))
            .map_err(|error| error.to_string())
    }

    fn get_image(&mut self) -> Result<Option<ClipboardImage>, String> {
        match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_image()) {
            Ok(img) => Ok(Some(ClipboardImage {
                width: img.width as u32,
                height: img.height as u32,
                bytes: img.bytes.into_owned(),
            })),
            Err(_) => Ok(None),
        }
    }

    fn set_image(&mut self, image: ClipboardImage) -> Result<(), String> {
        let data = arboard::ImageData {
            width: image.width as usize,
            height: image.height as usize,
            bytes: image.bytes.into(),
        };
        arboard::Clipboard::new()
            .and_then(|mut clipboard| clipboard.set_image(data))
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
    pub scroll_y: f32,
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
    scroll_y: f32,
    width: f32,
    height: f32,
    line_height: f32,
    /// Wrapped lines, Return inserts a line break (text areas).
    multiline: bool,
    /// Draw one bullet per character and refuse to copy (password fields).
    masked: bool,
    /// Allow navigation, selection and copy but no edits.
    read_only: bool,
    /// Avoid reshaping a cloned editor multiple times per frame.
    display_cache: Option<DisplayState>,
    /// Text before each edit, oldest first ([`TextIntent::Undo`]).
    undo: Vec<Snapshot>,
    /// Edits taken back, the last one on top ([`TextIntent::Redo`]).
    redo: Vec<Snapshot>,
    /// The run of edits the next one may join as a single undo step.
    group: Option<EditGroup>,
}

/// Steps [`TextFieldState`] keeps to undo.
const UNDO_LIMIT: usize = 100;

/// The text, caret and selection of a field at one moment.
#[derive(Clone, Debug, PartialEq)]
struct Snapshot {
    text: String,
    cursor: Cursor,
    selection: Selection,
}

/// A run of edits undone as one step, as in `NSTextView`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditGroup {
    /// Typing; a word typed after a space starts a new step.
    Typing { after_space: bool },
    /// Backspace or Delete, one after another.
    Deleting,
}

/// What an intent does to the text, for the undo steps.
enum EditKind {
    Typing {
        starts_word: bool,
        ends_space: bool,
    },
    Deleting,
    /// Pasting, cutting, a line break: a step of its own.
    Other,
}

impl EditKind {
    fn of(intent: &TextIntent) -> Self {
        let typing = |text: &str| Self::Typing {
            starts_word: text.chars().next().is_some_and(char::is_alphanumeric),
            ends_space: text.chars().last().is_some_and(char::is_whitespace),
        };
        match intent {
            TextIntent::InsertText(text) | TextIntent::Ime(ImeIntent::Commit(text)) => typing(text),
            TextIntent::KeyDown {
                key: Key::Backspace | Key::Delete,
                ..
            } => Self::Deleting,
            _ => Self::Other,
        }
    }
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
    content_height: f32,
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
            scroll_y: 0.0,
            width: 1.0,
            height: metrics.line_height,
            line_height: metrics.line_height,
            multiline: false,
            masked: false,
            read_only: false,
            display_cache: None,
            undo: Vec::new(),
            redo: Vec::new(),
            group: None,
        }
    }

    fn invalidate_display_cache(&mut self) {
        self.display_cache = None;
    }

    pub fn is_multiline(&self) -> bool {
        self.multiline
    }

    /// Switch between one scrolling line and wrapped lines.
    pub fn set_multiline(&mut self, font_system: &mut FontSystem, multiline: bool) {
        if multiline != self.multiline {
            self.multiline = multiline;
            self.scroll_x = 0.0;
            self.scroll_y = 0.0;
            self.fit_buffer();
            self.after_edit(font_system);
        }
    }

    /// One line never wraps and shows a line's height; wrapped lines are all
    /// laid out at the field width and scrolled here, not by the buffer.
    fn fit_buffer(&mut self) {
        let (multiline, width, line_height) = (self.multiline, self.width, self.line_height);
        self.editor.with_buffer_mut(|buffer| {
            if multiline {
                buffer.set_wrap(Wrap::Word);
                buffer.set_size(Some(width), None);
            } else {
                buffer.set_wrap(Wrap::None);
                buffer.set_size(None, Some(line_height));
            }
        });
        self.invalidate_display_cache();
    }

    fn normalize(&self, text: &str) -> String {
        if self.multiline {
            multi_line(text)
        } else {
            single_line(text)
        }
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

    /// Apply an editing intent already gated by text-field focus. Edits
    /// that change the text can be undone: a run of typing (word by word)
    /// or of deleting is one step. A masked or read-only field keeps no
    /// history.
    pub fn apply(
        &mut self,
        intent: TextIntent,
        font_system: &mut FontSystem,
        clipboard: &mut dyn ClipboardPort,
    ) -> TextFieldEffect {
        match intent {
            TextIntent::Undo => return self.step_back(true, font_system),
            TextIntent::Redo => return self.step_back(false, font_system),
            _ => {}
        }
        let kind = EditKind::of(&intent);
        let before = self.snapshot();
        let effect = self.edit(intent, font_system, clipboard);
        self.record(before, kind);
        effect
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.text(),
            cursor: self.editor.cursor(),
            selection: self.editor.selection(),
        }
    }

    /// Keep `before` as an undo step unless the edit joins the open group.
    fn record(&mut self, before: Snapshot, kind: EditKind) {
        if before.text == self.text() {
            self.group = None;
            return;
        }
        if self.masked || self.read_only {
            return;
        }
        let (joins, group) = match (self.group, kind) {
            (
                Some(EditGroup::Typing { after_space }),
                EditKind::Typing {
                    starts_word,
                    ends_space,
                },
            ) => (
                !(after_space && starts_word),
                Some(EditGroup::Typing {
                    after_space: ends_space,
                }),
            ),
            (_, EditKind::Typing { ends_space, .. }) => (
                false,
                Some(EditGroup::Typing {
                    after_space: ends_space,
                }),
            ),
            (group, EditKind::Deleting) => (
                group == Some(EditGroup::Deleting),
                Some(EditGroup::Deleting),
            ),
            (_, EditKind::Other) => (false, None),
        };
        if !joins {
            self.undo.push(before);
            if self.undo.len() > UNDO_LIMIT {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
        self.group = group;
    }

    /// Undo (`back`) or redo one step. Returns `changed` when there was one.
    fn step_back(&mut self, back: bool, font_system: &mut FontSystem) -> TextFieldEffect {
        if self.masked || self.read_only {
            return TextFieldEffect::default();
        }
        let current = self.snapshot();
        let (from, to) = if back {
            (&mut self.undo, &mut self.redo)
        } else {
            (&mut self.redo, &mut self.undo)
        };
        let Some(snapshot) = from.pop() else {
            return TextFieldEffect::default();
        };
        to.push(current);
        self.preedit = None;
        self.editor.with_buffer_mut(|buffer| {
            buffer.set_text(&snapshot.text, &ui_font_attrs(), Shaping::Advanced, None);
        });
        self.editor.set_cursor(snapshot.cursor);
        self.editor.set_selection(snapshot.selection);
        self.group = None;
        self.after_edit(font_system);
        TextFieldEffect::changed()
    }

    fn edit(
        &mut self,
        intent: TextIntent,
        font_system: &mut FontSystem,
        clipboard: &mut dyn ClipboardPort,
    ) -> TextFieldEffect {
        match intent {
            TextIntent::Undo | TextIntent::Redo => TextFieldEffect::default(),
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

    /// Where a key moves the caret, with the macOS bindings: ⌥ by word, ⌘
    /// to the line ends (← →) or the text ends (↑ ↓). A masked field has no
    /// words, so ⌥ goes to the ends as well. With several lines, ↑ ↓ move
    /// by visual line and ⌥↑ ⌥↓ to the paragraph ends.
    fn motion(&self, key: Key, modifiers: Modifiers) -> Option<Motion> {
        let word = modifiers.option && !modifiers.command;
        let line = modifiers.command && !modifiers.option;
        Some(match key {
            Key::ArrowUp if word && self.multiline => Motion::ParagraphStart,
            Key::ArrowDown if word && self.multiline => Motion::ParagraphEnd,
            Key::ArrowUp if !line && self.multiline => Motion::Up,
            Key::ArrowDown if !line && self.multiline => Motion::Down,
            Key::ArrowLeft if line => Motion::Home,
            Key::ArrowRight if line => Motion::End,
            Key::ArrowLeft if word && self.masked => Motion::BufferStart,
            Key::ArrowRight if word && self.masked => Motion::BufferEnd,
            Key::ArrowLeft if word => Motion::LeftWord,
            Key::ArrowRight if word => Motion::RightWord,
            Key::ArrowLeft => Motion::Left,
            Key::ArrowRight => Motion::Right,
            Key::ArrowUp if line => Motion::BufferStart,
            Key::ArrowDown if line => Motion::BufferEnd,
            Key::ArrowUp if word => Motion::BufferStart,
            Key::ArrowDown if word => Motion::BufferEnd,
            Key::Home | Key::ArrowUp => Motion::BufferStart,
            Key::End | Key::ArrowDown => Motion::BufferEnd,
            _ => return None,
        })
    }

    /// Backspace or Delete: the selection if there is one, otherwise one
    /// character, or up to the word (⌥) or line (⌘) boundary.
    fn delete(&mut self, key: Key, modifiers: Modifiers, font_system: &mut FontSystem) {
        let backward = key == Key::Backspace;
        let reach = if modifiers.command {
            Some(if backward { Motion::Home } else { Motion::End })
        } else if modifiers.option {
            Some(match (backward, self.masked) {
                (true, true) => Motion::BufferStart,
                (false, true) => Motion::BufferEnd,
                (true, false) => Motion::LeftWord,
                (false, false) => Motion::RightWord,
            })
        } else {
            None
        };
        match reach {
            Some(motion) if self.editor.selection() == Selection::None => {
                self.editor
                    .set_selection(Selection::Normal(self.editor.cursor()));
                self.editor.action(font_system, Action::Motion(motion));
                self.editor.delete_selection();
            }
            _ => {
                let action = if backward {
                    Action::Backspace
                } else {
                    Action::Delete
                };
                self.editor.action(font_system, action);
            }
        }
    }

    fn key_down(
        &mut self,
        key: Key,
        modifiers: Modifiers,
        font_system: &mut FontSystem,
    ) -> TextFieldEffect {
        if matches!(key, Key::Backspace | Key::Delete) {
            if self.read_only {
                return TextFieldEffect::default();
            }
            self.delete(key, modifiers, font_system);
            self.after_edit(font_system);
            return TextFieldEffect::changed();
        }
        if key == Key::Enter {
            // ⌘Return submits a text area; one line never takes Return.
            let breaks = self.multiline && !self.read_only && !modifiers.command;
            return TextFieldEffect {
                changed: breaks && self.insert_text(font_system, "\n"),
                status: None,
            };
        }
        let Some(motion) = self.motion(key, modifiers) else {
            return TextFieldEffect::default();
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
    /// the end. A value the person did not type also drops the undo
    /// history. Returns whether the text changed.
    pub fn set_text(&mut self, font_system: &mut FontSystem, text: &str) -> bool {
        let text = self.normalize(text);
        let current = self.text();
        if text == current {
            return false;
        }
        self.undo.clear();
        self.redo.clear();
        self.group = None;
        let caret = self.offset(self.editor.cursor()).min(current.len());
        self.editor.with_buffer_mut(|buffer| {
            buffer.set_text(&text, &ui_font_attrs(), Shaping::Advanced, None);
        });
        self.editor.set_cursor(cursor_at(&text, caret));
        self.editor.set_selection(Selection::None);
        self.after_edit(font_system);
        true
    }

    /// Byte offset of `cursor` in [`TextFieldState::text`].
    fn offset(&self, cursor: Cursor) -> usize {
        self.editor.with_buffer(|buffer| {
            let before: usize = buffer
                .lines
                .iter()
                .take(cursor.line)
                .map(|line| line.text().len() + 1)
                .sum();
            before + cursor.index
        })
    }

    /// Font size and line height from the input's computed style.
    pub fn set_metrics(&mut self, font_system: &mut FontSystem, metrics: Metrics) {
        if metrics.line_height == self.line_height
            && self.editor.with_buffer(|buffer| buffer.metrics()) == metrics
        {
            return;
        }
        self.line_height = metrics.line_height;
        self.editor
            .with_buffer_mut(|buffer| buffer.set_metrics(metrics));
        self.fit_buffer();
        self.after_edit(font_system);
    }

    pub fn preedit(&self) -> Option<&Preedit> {
        self.preedit.as_ref()
    }

    pub fn scroll_x(&self) -> f32 {
        self.scroll_x
    }

    pub fn scroll_y(&self) -> f32 {
        self.scroll_y
    }

    /// Visible text width in logical points; wrapped lines rewrap.
    pub fn set_width(&mut self, font_system: &mut FontSystem, width: f32) {
        let width = width.max(1.0);
        if width != self.width {
            self.width = width;
            if self.multiline {
                self.fit_buffer();
                self.after_edit(font_system);
            } else {
                self.reveal_caret(font_system);
            }
        }
    }

    /// Visible text height in logical points (wrapped lines scroll in it).
    pub fn set_height(&mut self, font_system: &mut FontSystem, height: f32) {
        let height = height.max(self.line_height);
        if height != self.height {
            self.height = height;
            self.reveal_caret(font_system);
        }
    }

    pub fn set_preedit(&mut self, font_system: &mut FontSystem, preedit: Option<Preedit>) {
        self.preedit = preedit.filter(|preedit| !preedit.text.is_empty());
        self.invalidate_display_cache();
        self.reveal_caret(font_system);
    }

    /// Insert committed text, replacing the selection. Line breaks become
    /// spaces in a single-line field. Returns whether text changed.
    pub fn insert_text(&mut self, font_system: &mut FontSystem, text: &str) -> bool {
        let text = self.normalize(text);
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
        self.group = None;
        let at = self.in_buffer(local);
        if self.masked {
            if let Some(cursor) = self.masked_hit(font_system, at) {
                self.editor.set_cursor(cursor);
            }
        } else {
            self.editor.action(
                font_system,
                Action::Click {
                    x: at.x as i32,
                    y: at.y as i32,
                },
            );
        }
        self.editor.set_selection(Selection::None);
        self.after_edit(font_system);
    }

    /// The `count`th click of a series at a point: one places the caret, two
    /// select the word under it and three the whole line. A masked field
    /// selects everything from the second click on (its words are hidden).
    pub fn click_count(&mut self, font_system: &mut FontSystem, local: Point, count: u32) {
        if count <= 1 {
            self.click(font_system, local);
            return;
        }
        if self.masked {
            self.select_all(font_system);
            return;
        }
        let at = self.in_buffer(local);
        let (x, y) = (at.x as i32, at.y as i32);
        let action = if count == 2 {
            Action::DoubleClick { x, y }
        } else {
            Action::TripleClick { x, y }
        };
        self.editor.action(font_system, action);
        self.after_edit(font_system);
    }

    /// Extend the selection to a point relative to the text origin.
    pub fn drag(&mut self, font_system: &mut FontSystem, local: Point) {
        let at = self.in_buffer(local);
        if self.masked {
            if let Some(cursor) = self.masked_hit(font_system, at) {
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
                    x: at.x as i32,
                    y: at.y as i32,
                },
            );
        }
        self.after_edit(font_system);
    }

    /// A point relative to the text origin in unscrolled buffer coordinates.
    /// Pointer positions above or below the single line (clicks delegated
    /// from the field's outline) still target it.
    fn in_buffer(&self, local: Point) -> Point {
        let y = if self.multiline {
            (local.y + self.scroll_y).max(0.0)
        } else {
            local.y.clamp(0.0, (self.line_height - 1.0).max(0.0))
        };
        Point {
            x: local.x + self.scroll_x,
            y,
        }
    }

    /// Hit-test the bullets actually on screen and map back to the real text.
    fn masked_hit(&mut self, font_system: &mut FontSystem, at: Point) -> Option<Cursor> {
        let display = self.cached_display(font_system);
        let hit = display.buffer.hit(at.x, at.y)?;
        let text = self.text();
        Some(Cursor::new(0, unmasked_index(&text, hit.index)))
    }

    fn after_edit(&mut self, font_system: &mut FontSystem) {
        self.invalidate_display_cache();
        self.editor.shape_as_needed(font_system, false);
        self.reveal_caret(font_system);
    }

    pub fn view(&mut self, font_system: &mut FontSystem) -> TextFieldView {
        let (scroll_x, scroll_y) = (self.scroll_x, self.scroll_y);
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
            y: rect.y - scroll_y,
            ..rect
        };
        TextFieldView {
            buffer: display.buffer.clone(),
            decorations: TextFieldDecorations {
                has_content,
                scroll_x,
                scroll_y,
                caret: metrics.caret.map(scroll),
                selection: metrics.selection.into_iter().map(scroll).collect(),
                preedit_underline: metrics.preedit_underline.into_iter().map(scroll).collect(),
            },
        }
    }

    fn reveal_caret(&mut self, font_system: &mut FontSystem) {
        let line_height = self.line_height;
        let (multiline, width, height) = (self.multiline, self.width, self.height);
        let display = self.cached_display(font_system);
        let metrics = measure_display(display, line_height);
        if multiline {
            let caret_y = metrics.caret.map_or(0.0, |caret| caret.y);
            self.scroll_x = 0.0;
            self.scroll_y = scroll_span_to_reveal(
                self.scroll_y,
                caret_y,
                line_height,
                height,
                metrics.content_height,
            );
        } else {
            let caret_x = metrics.caret.map_or(0.0, |caret| caret.x);
            self.scroll_x = scroll_to_reveal(self.scroll_x, caret_x, width, metrics.content_width);
        }
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
        content_height: 0.0,
    };
    for run in display.buffer.layout_runs() {
        metrics.content_width = metrics.content_width.max(run.line_w);
        metrics.content_height = metrics.content_height.max(run.line_top + run.line_height);
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

/// Text for several lines: `\r\n` and `\r` become `\n`, other control
/// characters are dropped.
fn multi_line(text: &str) -> String {
    text.replace("\r\n", "\n")
        .chars()
        .map(|character| if character == '\r' { '\n' } else { character })
        .filter(|character| *character == '\n' || !character.is_control())
        .collect()
}

/// The editor cursor at byte `offset` of `text` (lines split at `\n`).
fn cursor_at(text: &str, offset: usize) -> Cursor {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    let before = &text[..offset];
    let start = before.rfind('\n').map_or(0, |index| index + 1);
    Cursor::new(before.matches('\n').count(), offset - start)
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

/// Vertical scroll that keeps the span `[start, start + extent)` (the
/// caret's line) inside a viewport of `viewport` points, moving as little
/// as possible.
fn scroll_span_to_reveal(scroll: f32, start: f32, extent: f32, viewport: f32, content: f32) -> f32 {
    let end = start + extent;
    let mut next = scroll;
    if start < next {
        next = start;
    }
    if end > next + viewport {
        next = end - viewport;
    }
    next.clamp(0.0, (content.max(end) - viewport).max(0.0))
}

#[cfg(test)]
mod tests {
    use cosmic_text::{Edit, FontSystem, Metrics};

    use std::time::Duration;

    use super::{
        CARET_WIDTH, ClipboardPort, Preedit, TextFieldEffect, TextFieldState, caret_blink_visible,
        masked_index, scroll_to_reveal, unmasked_index,
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

    /// Type `text` one character at a time, as the keyboard does.
    fn type_chars(
        field: &mut TextFieldState,
        fonts: &mut FontSystem,
        clipboard: &mut MockClipboard,
        text: &str,
    ) {
        for c in text.chars() {
            field.apply(TextIntent::InsertText(c.to_string()), fonts, clipboard);
        }
    }

    #[test]
    fn undo_takes_back_typing_word_by_word_and_redo_brings_it_again() {
        let mut fonts = FontSystem::new();
        let mut clipboard = MockClipboard::default();
        let mut field = field(&mut fonts, 300.0);
        type_chars(&mut field, &mut fonts, &mut clipboard, "hola mundo");
        let undo =
            |field: &mut TextFieldState, fonts: &mut FontSystem, clipboard: &mut MockClipboard| {
                field.apply(TextIntent::Undo, fonts, clipboard).changed
            };

        assert!(undo(&mut field, &mut fonts, &mut clipboard));
        assert_eq!(field.text(), "hola ");
        assert!(undo(&mut field, &mut fonts, &mut clipboard));
        assert_eq!(field.text(), "");
        assert!(
            !undo(&mut field, &mut fonts, &mut clipboard),
            "nothing left"
        );

        field.apply(TextIntent::Redo, &mut fonts, &mut clipboard);
        assert_eq!(field.text(), "hola ");
        field.apply(TextIntent::Redo, &mut fonts, &mut clipboard);
        assert_eq!(field.text(), "hola mundo");
        assert_eq!(field.editor().cursor().index, "hola mundo".len());
        assert!(
            !field
                .apply(TextIntent::Redo, &mut fonts, &mut clipboard)
                .changed
        );

        field.apply(TextIntent::Undo, &mut fonts, &mut clipboard);
        type_chars(&mut field, &mut fonts, &mut clipboard, "kai");
        assert!(
            !field
                .apply(TextIntent::Redo, &mut fonts, &mut clipboard)
                .changed,
            "a new edit drops what was undone"
        );
        assert_eq!(field.text(), "hola kai");
    }

    #[test]
    fn deleting_pasting_and_moving_make_their_own_steps() {
        let mut fonts = FontSystem::new();
        let mut clipboard = MockClipboard::default();
        let mut field = field(&mut fonts, 300.0);
        type_chars(&mut field, &mut fonts, &mut clipboard, "abcd");
        for _ in 0..2 {
            field.apply(key(Key::Backspace, false), &mut fonts, &mut clipboard);
        }
        clipboard.contents = Some("XY".into());
        field.apply(TextIntent::Paste, &mut fonts, &mut clipboard);
        field.apply(key(Key::ArrowLeft, false), &mut fonts, &mut clipboard);
        type_chars(&mut field, &mut fonts, &mut clipboard, "z");
        assert_eq!(field.text(), "abXzY");

        let mut steps = Vec::new();
        while field
            .apply(TextIntent::Undo, &mut fonts, &mut clipboard)
            .changed
        {
            steps.push(field.text());
        }
        assert_eq!(steps, ["abXY", "ab", "abcd", ""]);
    }

    #[test]
    fn a_value_from_the_app_or_a_masked_field_keeps_no_history() {
        let mut fonts = FontSystem::new();
        let mut clipboard = MockClipboard::default();
        let mut field = field(&mut fonts, 300.0);
        type_chars(&mut field, &mut fonts, &mut clipboard, "hola");
        assert!(!field.set_text(&mut fonts, "hola"), "the app echoes it");
        assert!(field.set_text(&mut fonts, "otro"));
        assert!(
            !field
                .apply(TextIntent::Undo, &mut fonts, &mut clipboard)
                .changed
        );
        assert_eq!(field.text(), "otro");

        let mut password = super::TextFieldState::new(&mut fonts, Metrics::new(15.0, 22.0));
        password.set_masked(&mut fonts, true);
        type_chars(&mut password, &mut fonts, &mut clipboard, "secreto");
        assert!(
            !password
                .apply(TextIntent::Undo, &mut fonts, &mut clipboard)
                .changed
        );
        assert_eq!(password.text(), "secreto");
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

    fn with(key: Key, modifiers: Modifiers) -> TextIntent {
        TextIntent::KeyDown {
            key,
            modifiers,
            repeat: false,
        }
    }

    const OPTION: Modifiers = Modifiers {
        option: true,
        ..Modifiers::NONE
    };
    const COMMAND: Modifiers = Modifiers {
        command: true,
        ..Modifiers::NONE
    };
    const SHIFT_OPTION: Modifiers = Modifiers {
        option: true,
        shift: true,
        ..Modifiers::NONE
    };

    #[test]
    fn option_moves_by_word_and_command_to_the_line_ends() {
        let mut fonts = FontSystem::new();
        let mut clipboard = MockClipboard::default();
        let mut field = field(&mut fonts, 400.0);
        field.insert_text(&mut fonts, "uno dos tres");
        let mut press = |field: &mut TextFieldState, key, modifiers| {
            field.apply(with(key, modifiers), &mut fonts, &mut clipboard);
            field.editor().cursor().index
        };
        assert_eq!(press(&mut field, Key::ArrowLeft, OPTION), "uno dos ".len());
        assert_eq!(press(&mut field, Key::ArrowLeft, OPTION), "uno ".len());
        assert_eq!(press(&mut field, Key::ArrowRight, OPTION), "uno dos".len());
        assert_eq!(press(&mut field, Key::ArrowLeft, COMMAND), 0);
        assert_eq!(press(&mut field, Key::ArrowRight, COMMAND), 12);
        assert_eq!(press(&mut field, Key::ArrowUp, COMMAND), 0);
        assert_eq!(press(&mut field, Key::ArrowDown, COMMAND), 12);

        field.apply(
            with(Key::ArrowLeft, SHIFT_OPTION),
            &mut fonts,
            &mut clipboard,
        );
        assert_eq!(field.editor().copy_selection().as_deref(), Some("tres"));
    }

    #[test]
    fn option_and_command_delete_up_to_the_word_or_line_start() {
        let mut fonts = FontSystem::new();
        let mut clipboard = MockClipboard::default();
        let mut field = field(&mut fonts, 400.0);
        field.insert_text(&mut fonts, "uno dos tres");
        field.apply(with(Key::Backspace, OPTION), &mut fonts, &mut clipboard);
        assert_eq!(field.text(), "uno dos ");
        field.apply(with(Key::Backspace, OPTION), &mut fonts, &mut clipboard);
        assert_eq!(field.text(), "uno ");
        field.apply(with(Key::ArrowLeft, COMMAND), &mut fonts, &mut clipboard);
        field.apply(with(Key::Delete, OPTION), &mut fonts, &mut clipboard);
        assert_eq!(field.text(), " ");

        field.set_text(&mut fonts, "hola mundo");
        field.apply(with(Key::ArrowRight, COMMAND), &mut fonts, &mut clipboard);
        field.apply(with(Key::Backspace, COMMAND), &mut fonts, &mut clipboard);
        assert_eq!(field.text(), "");

        field.set_text(&mut fonts, "secreto");
        field.set_masked(&mut fonts, true);
        field.apply(with(Key::ArrowLeft, COMMAND), &mut fonts, &mut clipboard);
        field.apply(with(Key::ArrowRight, OPTION), &mut fonts, &mut clipboard);
        assert_eq!(field.editor().cursor().index, 7, "no words in a password");
        field.apply(with(Key::Backspace, OPTION), &mut fonts, &mut clipboard);
        assert_eq!(field.text(), "");
    }

    #[test]
    fn double_click_selects_a_word_and_triple_click_the_line() {
        let mut fonts = FontSystem::new();
        let mut field = field(&mut fonts, 400.0);
        field.insert_text(&mut fonts, "uno dos tres");
        field.click(&mut fonts, Point { x: 0.0, y: 5.0 });
        let x = field
            .view(&mut fonts)
            .buffer
            .layout_runs()
            .next()
            .map(|run| {
                let x = |index| {
                    run.cursor_position(&cosmic_text::Cursor::new(0, index))
                        .unwrap()
                };
                (x("uno d".len()) + x("uno do".len())) / 2.0
            })
            .unwrap();
        let at = Point { x, y: 5.0 };
        field.click_count(&mut fonts, at, 2);
        assert_eq!(field.editor().copy_selection().as_deref(), Some("dos"));
        field.click_count(&mut fonts, at, 3);
        assert_eq!(
            field.editor().copy_selection().as_deref(),
            Some("uno dos tres")
        );
        field.click_count(&mut fonts, at, 1);
        assert_eq!(field.editor().copy_selection(), None);
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

    fn area(font_system: &mut FontSystem, width: f32, rows: f32) -> TextFieldState {
        let mut area = field(font_system, width);
        area.set_multiline(font_system, true);
        area.set_height(font_system, rows * 22.0);
        area
    }

    #[test]
    fn return_breaks_lines_in_a_text_area_but_not_with_command() {
        let mut fonts = FontSystem::new();
        let mut clipboard = MockClipboard::default();
        let mut area = area(&mut fonts, 300.0, 3.0);
        area.insert_text(&mut fonts, "uno");
        assert!(
            area.apply(key(Key::Enter, false), &mut fonts, &mut clipboard)
                .changed
        );
        area.insert_text(&mut fonts, "dos\r\ntres\u{7}");
        assert_eq!(area.text(), "uno\ndos\ntres");
        assert_eq!(area.editor().cursor().line, 2);

        let submit = area.apply(with(Key::Enter, COMMAND), &mut fonts, &mut clipboard);
        assert!(!submit.changed);
        assert_eq!(area.text(), "uno\ndos\ntres");

        area.set_read_only(&mut fonts, true);
        area.apply(key(Key::Enter, false), &mut fonts, &mut clipboard);
        assert_eq!(area.text(), "uno\ndos\ntres");
    }

    #[test]
    fn arrows_move_by_line_and_the_caret_survives_controlled_updates() {
        let mut fonts = FontSystem::new();
        let mut clipboard = MockClipboard::default();
        let mut area = area(&mut fonts, 300.0, 3.0);
        area.insert_text(&mut fonts, "uno\ndos\ntres");
        let at = |area: &TextFieldState| {
            let cursor = area.editor().cursor();
            (cursor.line, cursor.index)
        };
        area.apply(key(Key::ArrowUp, false), &mut fonts, &mut clipboard);
        assert_eq!(area.editor().cursor().line, 1);
        area.apply(with(Key::ArrowUp, COMMAND), &mut fonts, &mut clipboard);
        assert_eq!(at(&area), (0, 0));
        area.apply(key(Key::ArrowDown, false), &mut fonts, &mut clipboard);
        area.apply(with(Key::ArrowRight, COMMAND), &mut fonts, &mut clipboard);
        assert_eq!(at(&area), (1, 3));

        assert!(area.set_text(&mut fonts, "UNO\nDOS\nTRES"));
        assert_eq!(at(&area), (1, 3), "the caret keeps its offset across lines");
    }

    #[test]
    fn long_text_wraps_and_scrolls_vertically_to_the_caret() {
        let mut fonts = FontSystem::new();
        let mut area = area(&mut fonts, 120.0, 2.0);
        let long = "Texto suficientemente largo para ocupar varias líneas del área";
        area.insert_text(&mut fonts, long);
        let view = area.view(&mut fonts);
        assert!(view.buffer.layout_runs().count() > 2, "wraps at the width");
        let decorations = view.decorations;
        assert_eq!(decorations.scroll_x, 0.0);
        assert!(decorations.scroll_y > 0.0, "scrolls to the last line");
        let caret = decorations.caret.expect("caret");
        assert!(caret.y >= 0.0 && caret.y + caret.height <= 44.0 + 0.01);

        area.click(&mut fonts, Point { x: 0.0, y: 0.0 });
        let first_visible = area.editor().cursor();
        assert!(first_visible.index > 0, "clicks map through the scroll");
        area.apply(
            with(Key::ArrowUp, COMMAND),
            &mut fonts,
            &mut MockClipboard::default(),
        );
        assert_eq!(area.scroll_y(), 0.0);
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

    #[test]
    fn caret_blink_phase_toggles_and_respects_reduce_motion() {
        assert!(caret_blink_visible(Duration::ZERO, false));
        assert!(caret_blink_visible(Duration::from_millis(529), false));
        assert!(!caret_blink_visible(Duration::from_millis(530), false));
        assert!(!caret_blink_visible(Duration::from_millis(1059), false));
        assert!(caret_blink_visible(Duration::from_millis(1060), false));
        assert!(
            caret_blink_visible(Duration::from_millis(530), true),
            "reduce_motion keeps the caret visible"
        );
    }
}
