//! The GenHa user interface (layer L6), built on egui.
//!
//! The UI is thin: it reads engine state and acts through [`App::run`], which executes engine
//! commands. Two screens: [`home`] (start a design from a size, a template or a picture; quick
//! actions) and the editor ([`editor`], [`canvas`], [`properties`]).
#![forbid(unsafe_code)]

pub mod canvas;
pub mod control;
pub mod editor;
pub mod home;
mod io;
pub mod properties;
pub mod theme;
mod thumbs;

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
#[derive(Clone, Debug, Default)]
pub struct AppOptions {
    /// Files to open at start.
    pub open: Vec<std::path::PathBuf>,
    pub dark: Option<bool>,
}

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
    pub control: Option<control::ControlServer>,
    font_families: Vec<String>,
    theme_applied: Option<bool>,
}

impl App {
    pub fn new(fonts: Arc<FontBook>, opts: AppOptions) -> Self {
        let mut families = fonts.families();
        for g in ["Display", "Mono", "Serif", "Sans"] {
            families.insert(0, g.to_string());
        }
        let mut session = Session::new();
        session.fonts = fonts;
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
            control: None,
            font_families: families,
            theme_applied: None,
        };
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

    /// Runs an engine command; errors go to the status bar. Returns the result on success.
    pub fn run(&mut self, id: &str, params: &Value) -> Option<Value> {
        match self.session.execute(id, params) {
            Ok(v) => {
                if id.starts_with("file.") || id.starts_with("template.") || id == "document.switch" {
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
        self.handle_dropped_files(&ctx);
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
            Screen::Editor => editor::show(self, ui),
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for f in dropped {
            let path = f.path().to_path_buf();
            if !path.as_os_str().is_empty() && path.exists() {
                let p = path.display().to_string();
                let is_picture = genha_effects::OutputFormat::from_extension(path.extension().and_then(|e| e.to_str()).unwrap_or("")).is_some();
                if is_picture && self.screen == Screen::Editor {
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
        0.0
    }
}

#[cfg(test)]
mod tests;
