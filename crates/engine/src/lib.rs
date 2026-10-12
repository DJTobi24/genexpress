//! The GenExpress engine façade (layer L5).
//!
//! Every user-visible action is a command with a stable id (`shape.add`, `object.group`,
//! `page.resize`…) and JSON parameters. The egui UI, the CLI, the control channel and the MCP
//! server all go through [`Session::execute`]. Tools (pointer gestures) are hosted here too and
//! reduce to commands, so every gesture is undoable and replayable.
#![forbid(unsafe_code)]

pub mod cmd;
pub mod guard;
pub mod inspect;

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;

pub use cmd::{CommandInfo, CommandSpec, command_specs, find_command};
pub use genexpress_doc as doc;
use genexpress_doc::{Document, Element, NodeId};
pub use genexpress_tools as tools;
use genexpress_tools::{Invocation, Preview, ToolContext, ToolOutput, ToolState};

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("unknown command `{0}`")]
    UnknownCommand(String),
    #[error("command `{0}` is not available right now: {1}")]
    Disabled(String, String),
    #[error("invalid parameters for `{cmd}`: {msg}")]
    BadParams { cmd: String, msg: String },
    #[error("no document open")]
    NoDocument,
    #[error("no such object {0}")]
    NoNode(NodeId),
    #[error("{0}")]
    Other(String),
    /// A bug: the command panicked. The document was rolled back to its state before the command.
    #[error("internal error in `{cmd}`: {msg} (the document was left as it was before; please report this bug)")]
    Internal { cmd: String, msg: String },
}

impl From<genexpress_doc::DocError> for EngineError {
    fn from(e: genexpress_doc::DocError) -> Self {
        match e {
            genexpress_doc::DocError::NoNode(id) => EngineError::NoNode(id),
            e => EngineError::Other(e.to_string()),
        }
    }
}

impl From<genexpress_format::FormatError> for EngineError {
    fn from(e: genexpress_format::FormatError) -> Self {
        EngineError::Other(e.to_string())
    }
}

impl From<genexpress_effects::EffectError> for EngineError {
    fn from(e: genexpress_effects::EffectError) -> Self {
        EngineError::Other(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, EngineError>;

/// One undo step: the state before an edit.
#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub label: String,
    pub doc: Arc<Document>,
    pub selection: Vec<NodeId>,
    pub page: usize,
}

#[derive(Clone, Debug)]
pub struct History {
    pub undo: Vec<HistoryEntry>,
    pub redo: Vec<HistoryEntry>,
    pub limit: usize,
}

impl Default for History {
    fn default() -> Self {
        History { undo: vec![], redo: vec![], limit: 200 }
    }
}

/// An open document with its selection, current page and history.
#[derive(Clone, Debug)]
pub struct DocState {
    pub doc: Arc<Document>,
    pub selection: Vec<NodeId>,
    /// The page being edited.
    pub page: usize,
    pub history: History,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    /// Bumped on every change (the UI re-renders when it moves).
    pub revision: u64,
}

impl DocState {
    pub fn new(doc: Document) -> Self {
        DocState { doc: Arc::new(doc), selection: vec![], page: 0, history: History::default(), path: None, dirty: false, revision: 0 }
    }

    /// Selected elements that still exist, in selection order.
    pub fn selected_elements(&self) -> Vec<&Element> {
        self.selection.iter().filter_map(|id| self.doc.find(*id).map(|(_, e)| e)).collect()
    }

    /// File name for titles: the file's name, else the document title.
    pub fn display_name(&self) -> String {
        self.path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| self.doc.title.clone())
    }
}

/// Everything an app instance has open.
pub struct Session {
    pub docs: Vec<DocState>,
    pub active: usize,
    /// Copied elements (with the assets they use).
    pub clipboard: cmd::edit::Clipboard,
    /// Copied style (Copy/Paste style).
    pub style_clipboard: Option<cmd::style::CopiedStyle>,
    pub tools: ToolState,
    /// The preview of a gesture in progress.
    pub preview: Option<Preview>,
    /// Status messages for the UI (latest last).
    pub messages: Vec<String>,
}

impl Default for Session {
    fn default() -> Self {
        Session::new()
    }
}

impl Session {
    pub fn new() -> Self {
        Session {
            docs: vec![],
            active: 0,
            clipboard: Default::default(),
            style_clipboard: None,
            tools: ToolState::default(),
            preview: None,
            messages: vec![],
        }
    }

    /// Opens `doc` as the active document; returns its index.
    pub fn open_document(&mut self, doc: Document) -> usize {
        self.docs.push(DocState::new(doc));
        self.active = self.docs.len() - 1;
        self.preview = None;
        self.active
    }

    pub fn active(&self) -> Option<&DocState> {
        self.docs.get(self.active)
    }

    pub fn active_mut(&mut self) -> Option<&mut DocState> {
        self.docs.get_mut(self.active)
    }

    pub fn doc(&self) -> Result<&DocState> {
        self.active().ok_or(EngineError::NoDocument)
    }

    pub fn doc_mut(&mut self) -> Result<&mut DocState> {
        self.active_mut().ok_or(EngineError::NoDocument)
    }

    /// Runs command `id`. Panics inside the command are caught and the document rolled back.
    pub fn execute(&mut self, id: &str, params: &Value) -> Result<Value> {
        let spec = find_command(id).ok_or_else(|| EngineError::UnknownCommand(id.to_string()))?;
        (spec.enabled)(self).map_err(|why| EngineError::Disabled(id.to_string(), why))?;
        let saved = self.active().cloned();
        let saved_active = self.active;
        let params = if params.is_null() { Value::Object(Default::default()) } else { params.clone() };
        match guard::catch_panic(|| (spec.run)(self, &params)) {
            Ok(r) => r,
            Err(msg) => {
                self.active = saved_active;
                if let (Some(state), Some(slot)) = (saved, self.docs.get_mut(saved_active)) {
                    *slot = state;
                }
                self.preview = None;
                Err(EngineError::Internal { cmd: id.to_string(), msg })
            }
        }
    }

    /// Runs several commands as one undo step; stops at the first error (rolling back the batch).
    pub fn execute_batch(&mut self, label: &str, list: &[Invocation]) -> Result<Vec<Value>> {
        let before = self.active().cloned();
        let depth = self.active().map(|d| d.history.undo.len());
        let mut out = vec![];
        for inv in list {
            match self.execute(&inv.command, &inv.params) {
                Ok(v) => out.push(v),
                Err(e) => {
                    if let (Some(state), Some(slot)) = (before, self.active_mut()) {
                        *slot = state;
                    }
                    return Err(e);
                }
            }
        }
        if let (Some(depth), Some(d)) = (depth, self.active_mut())
            && d.history.undo.len() > depth + 1
        {
            // Keep only the first new step (the state before the batch), labelled as the batch.
            d.history.undo.truncate(depth + 1);
            if let Some(e) = d.history.undo.last_mut() {
                e.label = label.to_string();
            }
        }
        Ok(out)
    }

    /// Applies an edit as one undo step. `f` gets a writable document; on `Err` nothing changes.
    pub fn edit<T>(&mut self, label: &str, f: impl FnOnce(&mut Document, &mut EditCx) -> Result<T>) -> Result<T> {
        let d = self.doc_mut()?;
        let before = Arc::clone(&d.doc);
        let mut cx = EditCx { selection: d.selection.clone(), page: d.page };
        let mut work = (*before).clone();
        let r = f(&mut work, &mut cx)?;
        let entry = HistoryEntry { label: label.to_string(), doc: before, selection: d.selection.clone(), page: d.page };
        d.history.undo.push(entry);
        if d.history.undo.len() > d.history.limit {
            let excess = d.history.undo.len() - d.history.limit;
            d.history.undo.drain(..excess);
        }
        d.history.redo.clear();
        d.doc = Arc::new(work);
        d.page = cx.page.min(d.doc.pages.len().saturating_sub(1));
        d.selection = cx.selection;
        d.dirty = true;
        d.revision = d.revision.wrapping_add(1);
        Ok(r)
    }

    /// Changes the selection (not an undo step).
    pub fn set_selection(&mut self, ids: Vec<NodeId>) -> Result<()> {
        let d = self.doc_mut()?;
        let page = d.doc.pages.get(d.page);
        d.selection = ids.into_iter().filter(|id| page.is_some_and(|p| p.find(*id).is_some())).collect();
        d.selection.dedup();
        d.revision = d.revision.wrapping_add(1);
        Ok(())
    }

    pub fn post(&mut self, msg: impl Into<String>) {
        self.messages.push(msg.into());
        if self.messages.len() > 100 {
            self.messages.remove(0);
        }
    }

    /// Feeds a pointer event to the active tool; runs the commands it emits (stopping at the first
    /// error, which is returned) and keeps its preview for the UI.
    pub fn tool_event(&mut self, ev: genexpress_tools::PointerEvent, zoom: f64, snap: (&[f64], &[f64])) -> Result<ToolOutput> {
        let Some(d) = self.active() else { return Ok(ToolOutput::default()) };
        let doc = Arc::clone(&d.doc);
        let selection = d.selection.clone();
        let cx = ToolContext { doc: &doc, page: d.page, selection: &selection, zoom, snap_xs: snap.0, snap_ys: snap.1 };
        let out = match guard::catch_panic(|| self.tools.event(&cx, ev)) {
            Ok(o) => o,
            Err(msg) => {
                self.tools.cancel();
                self.preview = None;
                return Err(EngineError::Internal { cmd: "tool".into(), msg });
            }
        };
        self.preview = out.preview.clone();
        for inv in &out.commands {
            self.execute(&inv.command, &inv.params)?;
        }
        Ok(out)
    }

    /// The document as the UI should draw it: with the gesture preview's frames applied.
    pub fn display_document(&self) -> Option<Arc<Document>> {
        let d = self.active()?;
        match &self.preview {
            Some(p) if !p.frames.is_empty() => {
                let mut doc = (*d.doc).clone();
                for (id, f) in &p.frames {
                    if let Some(el) = doc.find_mut(*id) {
                        cmd::object::set_frame_moving_children(el, *f);
                    }
                }
                Some(Arc::new(doc))
            }
            _ => Some(Arc::clone(&d.doc)),
        }
    }
}

/// What an edit may change besides the document.
pub struct EditCx {
    pub selection: Vec<NodeId>,
    pub page: usize,
}

#[cfg(test)]
mod tests;
