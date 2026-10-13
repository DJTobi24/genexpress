//! Editing plain text by character index: selections, word and paragraph boundaries, and the
//! changes a text caret makes (typing, deleting, moving, undo). Pure functions without fonts;
//! [`crate::caret`] maps the indices to positions in laid-out text.
//!
//! Indices count `char`s (not bytes), from 0 to the text's length.

use std::ops::Range;

use unicode_segmentation::GraphemeCursor;

/// Longest text an edit may produce, in bytes (the engine refuses longer text).
pub const MAX_EDIT_BYTES: usize = 100_000;
/// Most undo steps kept while editing.
const MAX_UNDO: usize = 500;

/// Characters in `s`.
pub fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Byte offset of character `i` in `s` (the end when `i` is past it).
pub fn byte_at(s: &str, i: usize) -> usize {
    s.char_indices().nth(i).map_or(s.len(), |(b, _)| b)
}

/// Characters `range` of `s` (clamped).
pub fn slice(s: &str, range: Range<usize>) -> &str {
    let (a, b) = (byte_at(s, range.start), byte_at(s, range.end.max(range.start)));
    s.get(a..b).unwrap_or("")
}

/// The grapheme boundary before character `i` (a letter with its accents, an emoji sequence…).
pub fn prev_grapheme(text: &str, i: usize) -> usize {
    let b = byte_at(text, i);
    let mut c = GraphemeCursor::new(b, text.len(), true);
    match c.prev_boundary(text, 0) {
        Ok(Some(p)) => char_len(text.get(..p).unwrap_or("")),
        _ => i.saturating_sub(1),
    }
}

/// The grapheme boundary after character `i`.
pub fn next_grapheme(text: &str, i: usize) -> usize {
    let n = char_len(text);
    let b = byte_at(text, i);
    let mut c = GraphemeCursor::new(b, text.len(), true);
    match c.next_boundary(text, 0) {
        Ok(Some(p)) => char_len(text.get(..p).unwrap_or("")),
        _ => (i + 1).min(n),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    Newline,
    Space,
    Word,
    Punct,
}

fn class(c: char) -> Class {
    if c == '\n' {
        Class::Newline
    } else if c.is_whitespace() {
        Class::Space
    } else if c.is_alphanumeric() || matches!(c, '_' | '\'' | '\u{2019}') {
        Class::Word
    } else {
        Class::Punct
    }
}

/// Where Ctrl/Alt+Left lands: the start of the word before `i` (spaces skipped).
pub fn prev_word(text: &str, i: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut i = i.min(chars.len());
    while i > 0 && chars.get(i - 1).is_some_and(|c| c.is_whitespace()) {
        i -= 1;
    }
    let Some(&c) = i.checked_sub(1).and_then(|j| chars.get(j)) else { return i };
    let k = class(c);
    while i > 0 && chars.get(i - 1).is_some_and(|c| class(*c) == k) {
        i -= 1;
    }
    i
}

/// Where Ctrl/Alt+Right lands: the end of the word after `i` (spaces skipped).
pub fn next_word(text: &str, i: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut i = i.min(n);
    while chars.get(i).is_some_and(|c| c.is_whitespace()) {
        i += 1;
    }
    let Some(&c) = chars.get(i) else { return n };
    let k = class(c);
    while chars.get(i).is_some_and(|c| class(*c) == k) {
        i += 1;
    }
    i
}

/// The word (or run of spaces, or of punctuation) at caret `i`: what a double-click selects.
/// At the end of a word, that word.
pub fn word_at(text: &str, i: usize) -> Range<usize> {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let i = i.min(n);
    let here = chars.get(i).copied().filter(|c| *c != '\n');
    let before = i.checked_sub(1).and_then(|j| chars.get(j)).copied().filter(|c| *c != '\n');
    // A word ending at the caret wins (a click on a word's last half lands after it), then the
    // character after the caret.
    let pick = match (here, before) {
        (_, Some(b)) if class(b) == Class::Word => Some(i - 1),
        (Some(_), _) => Some(i),
        (None, Some(_)) => Some(i - 1),
        (None, None) => None,
    };
    let Some(p) = pick else { return i..i };
    let Some(&c) = chars.get(p) else { return i..i };
    let k = class(c);
    let mut a = p;
    while a > 0 && chars.get(a - 1).is_some_and(|c| class(*c) == k) {
        a -= 1;
    }
    let mut b = p + 1;
    while chars.get(b).is_some_and(|c| class(*c) == k) {
        b += 1;
    }
    a..b
}

/// The paragraph (text between line breaks) holding caret `i`: what a triple-click selects.
pub fn paragraph_at(text: &str, i: usize) -> Range<usize> {
    let chars: Vec<char> = text.chars().collect();
    let i = i.min(chars.len());
    let mut a = i;
    while a > 0 && chars.get(a - 1).is_some_and(|c| *c != '\n') {
        a -= 1;
    }
    let mut b = i;
    while chars.get(b).is_some_and(|c| *c != '\n') {
        b += 1;
    }
    a..b
}

/// Text typed or pasted, made safe to insert: line breaks normalized to `\n`, other control
/// characters (except tabs) dropped.
pub fn clean_input(s: &str) -> String {
    s.replace("\r\n", "\n").chars().map(|c| if c == '\r' { '\n' } else { c }).filter(|c| !c.is_control() || matches!(c, '\n' | '\t')).collect()
}

/// A text with a selection: the state a text caret edits.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EditBuffer {
    pub text: String,
    /// Where the selection started (equal to `caret` when nothing is selected).
    pub anchor: usize,
    /// Where the caret is.
    pub caret: usize,
}

impl EditBuffer {
    /// `text` with the selection `anchor..caret` (clamped to the text).
    pub fn new(text: impl Into<String>, anchor: usize, caret: usize) -> Self {
        let text = text.into();
        let n = char_len(&text);
        EditBuffer { text, anchor: anchor.min(n), caret: caret.min(n) }
    }

    pub fn len(&self) -> usize {
        char_len(&self.text)
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// The selection, start first.
    pub fn range(&self) -> Range<usize> {
        self.anchor.min(self.caret)..self.anchor.max(self.caret)
    }

    pub fn has_selection(&self) -> bool {
        self.anchor != self.caret
    }

    /// The selected text.
    pub fn selected(&self) -> &str {
        slice(&self.text, self.range())
    }

    /// Selects `anchor..caret` (clamped).
    pub fn select(&mut self, anchor: usize, caret: usize) {
        let n = self.len();
        self.anchor = anchor.min(n);
        self.caret = caret.min(n);
    }

    pub fn select_all(&mut self) {
        self.select(0, self.len());
    }

    /// Moves the caret to `i`; with `extend` the selection grows or shrinks to it.
    pub fn move_to(&mut self, i: usize, extend: bool) {
        let i = i.min(self.len());
        self.caret = i;
        if !extend {
            self.anchor = i;
        }
    }

    /// Replaces the selection with `s` (cleaned, see [`clean_input`]); the caret ends after it.
    /// False when nothing changed or the text would grow past [`MAX_EDIT_BYTES`].
    pub fn insert(&mut self, s: &str) -> bool {
        let s = clean_input(s);
        let r = self.range();
        if s.is_empty() && r.is_empty() {
            return false;
        }
        let (a, b) = (byte_at(&self.text, r.start), byte_at(&self.text, r.end));
        if self.text.len() - (b - a) + s.len() > MAX_EDIT_BYTES {
            return false;
        }
        self.text.replace_range(a..b, &s);
        let at = r.start + char_len(&s);
        self.anchor = at;
        self.caret = at;
        true
    }

    /// Backspace: deletes the selection, else the grapheme (or with `word`, the word) before
    /// the caret. False when there was nothing to delete.
    pub fn backspace(&mut self, word: bool) -> bool {
        if !self.has_selection() {
            let from = if word { prev_word(&self.text, self.caret) } else { prev_grapheme(&self.text, self.caret) };
            self.anchor = from;
        }
        self.insert("")
    }

    /// Delete: deletes the selection, else the grapheme (or word) after the caret.
    pub fn delete_forward(&mut self, word: bool) -> bool {
        if !self.has_selection() {
            let to = if word { next_word(&self.text, self.caret) } else { next_grapheme(&self.text, self.caret) };
            self.anchor = to;
        }
        self.insert("")
    }

    /// Left arrow: one grapheme (or word) back; without `extend` a selection collapses to its
    /// start instead.
    pub fn left(&mut self, word: bool, extend: bool) {
        if self.has_selection() && !extend {
            let s = self.range().start;
            self.move_to(s, false);
            return;
        }
        let to = if word { prev_word(&self.text, self.caret) } else { prev_grapheme(&self.text, self.caret) };
        self.move_to(to, extend);
    }

    /// Right arrow: one grapheme (or word) on; without `extend` a selection collapses to its end.
    pub fn right(&mut self, word: bool, extend: bool) {
        if self.has_selection() && !extend {
            let e = self.range().end;
            self.move_to(e, false);
            return;
        }
        let to = if word { next_word(&self.text, self.caret) } else { next_grapheme(&self.text, self.caret) };
        self.move_to(to, extend);
    }
}

/// What kind of change an undo step holds: a run of typing or of deleting is one step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKind {
    Typing,
    Deleting,
    Other,
}

/// Undo and redo for an editing session: snapshots of the buffer before each step.
#[derive(Clone, Debug, Default)]
pub struct UndoStack {
    undo: Vec<EditBuffer>,
    redo: Vec<EditBuffer>,
    run: Option<EditKind>,
}

impl UndoStack {
    /// Call before applying a change of `kind` to `before`. Typing (or deleting) in a row
    /// makes one step until [`UndoStack::break_run`].
    pub fn record(&mut self, before: &EditBuffer, kind: EditKind) {
        self.redo.clear();
        if kind != EditKind::Other && self.run == Some(kind) {
            return;
        }
        self.undo.push(before.clone());
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        self.run = Some(kind);
    }

    /// The next change starts a new step (the caret moved, a word ended).
    pub fn break_run(&mut self) {
        self.run = None;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// The state before the last step, given the current one (kept for redo).
    pub fn undo(&mut self, current: &EditBuffer) -> Option<EditBuffer> {
        let prev = self.undo.pop()?;
        self.redo.push(current.clone());
        self.run = None;
        Some(prev)
    }

    pub fn redo(&mut self, current: &EditBuffer) -> Option<EditBuffer> {
        let next = self.redo.pop()?;
        self.undo.push(current.clone());
        self.run = None;
        Some(next)
    }
}
