//! The GenHa user interface (layer L6), built on egui.
//!
//! The UI is thin: it reads engine state and acts through [`App::run`], which executes engine
//! commands. Two screens: [`home`] (start a design from a size, a template or a picture; quick
//! actions) and the editor ([`editor`], [`canvas`], [`properties`]), plus present mode ([`motion`]).
#![forbid(unsafe_code)]

pub mod canvas;
mod clipboard;
pub mod control;
pub mod editor;
pub mod fonts;
pub mod home;
mod io;
pub mod motion;
pub mod properties;
pub mod theme;
mod thumbs;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::{instance_id as web_instance_id, open_store as open_web_store};

use std::sync::Arc;

use serde_json::{Value, json};

use genha_engine::{Session, guard};
use genha_render::Renderer;
use genha_text::FontBook;

pub use thumbs::Thumbs;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Home,
    Editor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeftTab {
    Templates,
    Elements,
    Text,
    Media,
    Brand,
    Layers,
}

impl LeftTab {
    pub const ALL: [LeftTab; 6] = [LeftTab::Templates, LeftTab::Elements, LeftTab::Text, LeftTab::Media, LeftTab::Brand, LeftTab::Layers];
    pub fn label(self) -> &'static str {
        match self {
            LeftTab::Templates => "Templates",
            LeftTab::Elements => "Elements",
            LeftTab::Text => "Text",
            LeftTab::Media => "Media",
            LeftTab::Brand => "Brand",
            LeftTab::Layers => "Layers",
        }
    }
}

/// How the canvas looks at the page.
#[derive(Clone, Copy, Debug)]
pub struct View {
    /// Screen points per page pixel.
    pub zoom: f32,
    /// Where the page's top-left is, relative to the canvas area's top-left (screen points).
    pub pan: egui::Vec2,
    /// Fit the page into the canvas on the next frame.
    pub fit_pending: bool,
}

impl Default for View {
    fn default() -> Self {
        View { zoom: 0.5, pan: egui::Vec2::ZERO, fit_pending: true }
    }
}

/// Options from the command line.
#[derive(Debug, Default)]
pub struct AppOptions {
    /// Files to open at start.
    pub open: Vec<std::path::PathBuf>,
    pub dark: Option<bool>,
    /// Where the app keeps recent designs, Data Recovery copies and added fonts (none, and no
    /// `store`: none of them is kept).
    pub data_dir: Option<std::path::PathBuf>,
    /// Keep them here instead of in `data_dir` (the browser: IndexedDB).
    pub store: Option<Box<dyn genha_engine::store::Store>>,
    /// Names this run's Data Recovery copies (default: the process id).
    pub instance: Option<String>,
}

/// Seconds between Data Recovery copies of unsaved designs (the browser copies more often: a
/// closed or reloaded tab gives no chance to copy at the end).
#[cfg(not(target_arch = "wasm32"))]
const RECOVERY_INTERVAL: f64 = 60.0;
#[cfg(target_arch = "wasm32")]
const RECOVERY_INTERVAL: f64 = 15.0;

pub struct App {
    pub session: Session,
    pub renderer: Renderer,
    pub screen: Screen,
    pub left_tab: LeftTab,
    pub view: View,
    pub thumbs: Thumbs,
    pub dark: bool,
    /// Latest status message and when it was posted (seconds).
    pub status: Option<(String, f64)>,
    pub home: home::HomeState,
    pub editor: editor::EditorState,
    pub canvas: canvas::CanvasState,
    /// Playback on the canvas and present mode (UI-only state).
    pub motion: motion::MotionState,
    pub control: Option<control::ControlServer>,
    font_families: Vec<String>,
    theme_applied: Option<bool>,
    last_recovery: f64,
    /// The designs (recovery id, revision, unsaved) when copies were last written.
    recovery_state: Vec<(u64, u64, bool)>,
    /// Recently opened or saved designs, newest first (`recent.*`).
    pub recent: Vec<genha_engine::cmd::recent::RecentEntry>,
    /// Bumped when [`App::recent`] is reloaded (thumbnails are drawn again).
    pub recent_rev: u64,
    /// Copies of designs from a session that ended unexpectedly: (key, title).
    pub recovered: Vec<(String, String)>,
    /// Text to put on the system clipboard next frame (after copying objects).
    pub(crate) clip_out: Option<String>,
}

/// Font families for pickers: generic names first, then the book's families.
fn family_list(book: &FontBook) -> Vec<String> {
    let mut families = book.families();
    for g in ["Display", "Mono", "Serif", "Sans"] {
        families.insert(0, g.to_string());
    }
    families
}

impl App {
    pub fn new(fonts: Arc<FontBook>, opts: AppOptions) -> Self {
        let mut session = Session::new();
        session.fonts = fonts;
        #[cfg(not(target_arch = "wasm32"))]
        let dir_store = opts.data_dir.as_ref().map(|d| Box::new(genha_engine::store::DirStore::new(d)) as Box<dyn genha_engine::store::Store>);
        #[cfg(target_arch = "wasm32")]
        let dir_store = None;
        session.store = opts.store.or(dir_store);
        if let Some(i) = opts.instance {
            session.instance = i;
        }
        // Fonts the user added before.
        let added = genha_engine::cmd::fonts::load_stored_fonts(&mut session);
        if added > 0 {
            log::info!("{added} added font faces");
        }
        let families = family_list(&session.fonts);
        let mut app = App {
            session,
            renderer: Renderer::new(),
            screen: Screen::Home,
            left_tab: LeftTab::Templates,
            view: View::default(),
            thumbs: Thumbs::default(),
            dark: opts.dark.unwrap_or(false),
            status: None,
            home: home::HomeState::default(),
            editor: editor::EditorState::default(),
            canvas: canvas::CanvasState::default(),
            motion: motion::MotionState::default(),
            control: None,
            font_families: families,
            theme_applied: None,
            last_recovery: now(),
            recovery_state: vec![],
            recent: vec![],
            recent_rev: 0,
            recovered: vec![],
            clip_out: None,
        };
        app.refresh_recent();
        if let Ok(list) = app.session.execute("recovery.list", &json!({})) {
            app.recovered = list
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| Some((v.get("key")?.as_str()?.to_string(), v.get("title")?.as_str()?.to_string())))
                .collect();
        }
        for path in &opts.open {
            app.run("file.open", &json!({"path": path.display().to_string()}));
        }
        if !app.session.docs.is_empty() {
            app.screen = Screen::Editor;
        }
        app
    }

    /// Font families for pickers: generic names first, then installed families.
    pub fn font_families(&self) -> &[String] {
        &self.font_families
    }

    /// Reloads the recent designs (after opening or saving).
    pub fn refresh_recent(&mut self) {
        self.recent = genha_engine::cmd::recent::entries(&self.session);
        self.recent_rev += 1;
    }

    /// Reloads the font families (after fonts were added).
    pub fn refresh_fonts(&mut self) {
        self.font_families = family_list(&self.session.fonts);
        // Text drawn before the font arrived is drawn again.
        self.thumbs.forget_prefix("");
        if self.canvas.editing_text.is_none() {
            self.canvas = canvas::CanvasState::default();
        }
    }

    /// Runs an engine command; errors go to the status bar. Returns the result on success.
    pub fn run(&mut self, id: &str, params: &Value) -> Option<Value> {
        // Paste Picture without a picture: read it from the system clipboard first.
        if id == "image.paste" && params.get("data").is_none() {
            clipboard::paste_picture(self);
            return None;
        }
        match self.session.execute(id, params) {
            Ok(v) => {
                if id.starts_with("file.") || id.starts_with("recent.") {
                    self.refresh_recent();
                }
                if id.starts_with("font.") {
                    self.refresh_fonts();
                }
                if matches!(id, "edit.copy" | "edit.cut") {
                    clipboard::copied(self);
                }
                if id.starts_with("file.") || id.starts_with("template.") || id.starts_with("recent.") || id == "document.switch" {
                    self.view.fit_pending = true;
                    if !self.session.docs.is_empty() {
                        self.screen = Screen::Editor;
                    }
                }
                Some(v)
            }
            Err(e) => {
                self.set_status(e.to_string());
                None
            }
        }
    }

    /// Writes Data Recovery copies of unsaved designs every [`RECOVERY_INTERVAL`] seconds.
    fn autosave(&mut self, ctx: &egui::Context) {
        if self.session.store.is_none() {
            return;
        }
        let t = now();
        if t - self.last_recovery < RECOVERY_INTERVAL {
            ctx.request_repaint_after(std::time::Duration::from_secs(RECOVERY_INTERVAL as u64));
            return;
        }
        self.last_recovery = t;
        // Only when something changed since the last copies.
        let state: Vec<(u64, u64, bool)> = self.session.docs.iter().map(|d| (d.recovery_id, d.revision, d.dirty)).collect();
        if state == self.recovery_state {
            return;
        }
        if (self.session.docs.iter().any(|d| d.dirty) || self.recovery_state.iter().any(|s| s.2))
            && let Err(e) = self.session.execute("recovery.save", &json!({}))
        {
            self.set_status(format!("Data Recovery: {e}"));
            return;
        }
        self.recovery_state = state;
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        log::info!("{msg}");
        self.status = Some((msg, now()));
    }

    /// Lays out the whole window. A panic in a frame is caught and reported, never fatal.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        if let Err(msg) = guard::catch_panic(|| self.ui_frame(ui)) {
            self.session.preview = None;
            self.session.tools.cancel();
            self.set_status(format!("Internal error: {msg} (please report this bug)"));
        }
    }

    fn ui_frame(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if self.theme_applied != Some(self.dark) {
            theme::apply(&ctx, self.dark);
            self.theme_applied = Some(self.dark);
        }
        control::process(self, &ctx);
        self.autosave(&ctx);
        clipboard::process(self, &ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.handle_dropped_files(&ctx);
        // Files picked or dropped in the browser arrive asynchronously.
        #[cfg(target_arch = "wasm32")]
        web::poll(self, &ctx);
        // Engine messages (repaired files…) surface in the status bar.
        if let Some(m) = self.session.messages.pop() {
            self.session.messages.clear();
            self.set_status(m);
        }
        if self.session.docs.is_empty() {
            self.screen = Screen::Home;
        }
        match self.screen {
            Screen::Home => home::show(self, ui),
            Screen::Editor if self.motion.present.is_some() => motion::present(self, ui),
            Screen::Editor => editor::show(self, ui),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for f in dropped {
            let path = f.path().to_path_buf();
            if !path.as_os_str().is_empty() && path.exists() {
                let p = path.display().to_string();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let is_picture = genha_effects::OutputFormat::from_extension(ext).is_some() || ext.eq_ignore_ascii_case("svg");
                let is_font = fonts::FONT_FILES.1.iter().any(|f| ext.eq_ignore_ascii_case(f));
                if is_font {
                    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "font.ttf".into());
                    let added = match std::fs::metadata(&path) {
                        Ok(m) if m.len() <= genha_text::MAX_FONT_FILE as u64 => std::fs::read(&path)
                            .map_err(|e| e.to_string())
                            .and_then(|b| genha_engine::cmd::fonts::add_font_bytes(&mut self.session, &name, b, true).map_err(|e| e.to_string())),
                        _ => Err(format!("{name} is too large or can't be read")),
                    };
                    match added {
                        Ok((_, families)) => {
                            self.refresh_fonts();
                            self.set_status(format!("Added {}", if families.is_empty() { name } else { families.join(", ") }));
                        }
                        Err(e) => self.set_status(e),
                    }
                } else if is_picture && self.screen == Screen::Editor {
                    self.run("image.place", &json!({"path": p}));
                } else {
                    self.run("file.open", &json!({"path": p}));
                }
            } else if let Ok(bytes) = f.bytes() {
                let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "picture".into());
                if self.screen == Screen::Editor {
                    use base64::Engine as _;
                    let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    self.run("image.place", &json!({"data": data, "name": name}));
                } else if let Err(e) = genha_engine::cmd::file::open_bytes(&mut self.session, &name, &bytes) {
                    self.set_status(e.to_string());
                } else {
                    self.screen = Screen::Editor;
                    self.view.fit_pending = true;
                }
            }
        }
    }
}

pub(crate) fn now() -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
    }
    #[cfg(target_arch = "wasm32")]
    {
        web::now()
    }
}

#[cfg(test)]
mod tests;
