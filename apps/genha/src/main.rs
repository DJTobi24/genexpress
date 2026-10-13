//! The GenHa desktop app.
//!
//! `genha [--control PORT] [--dark] [FILE…]`
// A release build on Windows is a GUI program: no console window opens with it (`genha-cli` is
// the console program).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "macos")]
mod macos;

use std::sync::Arc;

use genha_ui_egui::i18n::tr;
use genha_ui_egui::{App, AppOptions, control::ControlServer};

const USAGE: &str = "usage: genha [--control PORT] [--dark] [FILE.genha | PICTURE …]

  --control PORT   accept JSON-lines automation on 127.0.0.1:PORT (see docs/control-protocol.md)
  --dark           start in dark mode";

struct Shell {
    app: App,
    title: String,
    confirm_close: bool,
    allow_close: bool,
}

impl Shell {
    /// Opens the files the system handed over while running (macOS: Finder, the Dock).
    #[cfg(target_os = "macos")]
    fn open_from_system(&mut self) {
        let paths = macos::take_opened();
        if paths.is_empty() {
            return;
        }
        for p in paths {
            self.app.run("file.open", &serde_json::json!({"path": p.display().to_string()}));
        }
        if !self.app.session.docs.is_empty() {
            self.app.screen = genha_ui_egui::Screen::Editor;
            self.app.view.fit_pending = true;
        }
    }
}

impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        #[cfg(target_os = "macos")]
        self.open_from_system();
        self.app.ui(ui);
        // Window title follows the design.
        let title = match self.app.session.active() {
            Some(d) => format!("{}{} — GenHa", genha_ui_egui::shown_name(&d.display_name()), if d.dirty { " •" } else { "" }),
            None => "GenHa".to_string(),
        };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        // Designs kept by GenHa get their last changes in My designs; ask before closing with
        // unsaved changes.
        let closing = ctx.input(|i| i.viewport().close_requested());
        if closing {
            self.app.sync_library();
        }
        let dirty = self.app.session.docs.iter().any(|d| d.dirty);
        if closing && dirty && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_close = true;
        }
        if self.confirm_close {
            egui::Window::new(tr("Unsaved changes")).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO).show(
                &ctx,
                |ui| {
                    ui.label(tr("Some designs have changes that are not saved."));
                    ui.horizontal(|ui| {
                        if ui.button(tr("Quit without saving")).clicked() {
                            self.allow_close = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button(tr("Cancel")).clicked() {
                            self.confirm_close = false;
                        }
                    });
                },
            );
        }
    }
}

fn main() -> std::process::ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let mut opts = AppOptions::default();
    let mut port = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--control" => match args.next().and_then(|p| p.parse::<u16>().ok()) {
                Some(p) => port = Some(p),
                None => {
                    eprintln!("--control needs a port number\n\n{USAGE}");
                    return std::process::ExitCode::FAILURE;
                }
            },
            "--dark" => opts.dark = Some(true),
            "-h" | "--help" => {
                println!("{USAGE}");
                return std::process::ExitCode::SUCCESS;
            }
            // macOS adds a process serial number when an app is started from Finder on old
            // systems; it is not a file.
            a if a.starts_with("-psn_") => {}
            f => opts.open.push(f.into()),
        }
    }
    // Recent designs, Data Recovery copies, added fonts and downloaded models. The folder comes
    // from the environment (HOME, APPDATA, XDG_DATA_HOME), never from the working directory, so it
    // is the same when the app starts from Finder, Explorer, a menu or a terminal.
    opts.data_dir = genha_engine::store::user_data_dir();
    // The system's fonts, or built-in fallback faces when there are none.
    let fonts = Arc::new(genha_ui_egui::fonts::system_or_fallback());
    log::info!("{} font families found", fonts.families().len());
    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("GenHa")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 600.0])
            .with_drag_and_drop(true)
            .with_icon(Arc::new(window_icon()))
            // Wayland and X11 match the window with the installed `.desktop` file by this id.
            .with_app_id(genha_brand::APP_ID),
        ..Default::default()
    };
    let creator: eframe::AppCreator<'_> = Box::new(move |cc| {
        #[cfg(target_os = "macos")]
        macos::set_context(&cc.egui_ctx);
        let mut app = App::new(fonts, opts);
        if let Some(port) = port {
            match ControlServer::start(port, cc.egui_ctx.clone()) {
                Ok(c) => {
                    log::info!("control channel on 127.0.0.1:{port}");
                    app.control = Some(c);
                }
                Err(e) => log::error!("control channel: could not bind 127.0.0.1:{port}: {e}"),
            }
        }
        Ok(Box::new(Shell { app, title: String::new(), confirm_close: false, allow_close: false }))
    });
    match run(native, creator) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("GenHa could not start: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// The app icon for the window (and, on macOS, the Dock), drawn by `genha_brand`.
fn window_icon() -> egui::IconData {
    const SIZE: u32 = 256;
    egui::IconData { rgba: genha_brand::rgba(SIZE, genha_brand::Style::native()), width: SIZE, height: SIZE }
}

#[cfg(not(target_os = "macos"))]
fn run(native: eframe::NativeOptions, creator: eframe::AppCreator<'_>) -> Result<(), String> {
    eframe::run_native(genha_brand::APP_NAME, native, creator).map_err(|e| e.to_string())
}

/// macOS: what `eframe::run_native` does, with GenHa's application delegate installed between
/// creating the event loop and running it (files opened from Finder, see `macos.rs`).
#[cfg(target_os = "macos")]
fn run(native: eframe::NativeOptions, creator: eframe::AppCreator<'_>) -> Result<(), String> {
    let event_loop = winit::event_loop::EventLoop::<eframe::UserEvent>::with_user_event().build().map_err(|e| e.to_string())?;
    let _delegate = macos::install();
    let mut app = eframe::create_native(genha_brand::APP_NAME, native, creator, &event_loop);
    event_loop.run_app(&mut app).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn brand_extensions_match_the_file_format() {
        assert_eq!(genha_brand::EXTENSION, genha_format::EXTENSION);
        assert_eq!(genha_brand::LEGACY_EXTENSION, genha_format::LEGACY_EXTENSION);
    }

    #[test]
    fn window_icon_is_square_rgba() {
        let icon = super::window_icon();
        assert_eq!(icon.rgba.len(), (icon.width * icon.height * 4) as usize);
    }
}
