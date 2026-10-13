//! The GenHa desktop app.
//!
//! `genha [--control PORT] [--dark] [FILE…]`

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

impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
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
            f => opts.open.push(f.into()),
        }
    }
    // Recent designs, Data Recovery copies, added fonts and downloaded models.
    opts.data_dir = genha_engine::store::user_data_dir();
    // The system's fonts, or built-in fallback faces when there are none.
    let fonts = Arc::new(genha_ui_egui::fonts::system_or_fallback());
    log::info!("{} font families found", fonts.families().len());
    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("GenHa")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 600.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    let result = eframe::run_native(
        "GenHa",
        native,
        Box::new(move |cc| {
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
        }),
    );
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("GenHa could not start: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
