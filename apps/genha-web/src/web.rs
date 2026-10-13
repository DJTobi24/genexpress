//! The browser shell: starts eframe's web runner on the page's canvas with the GenHa UI.

use std::sync::Arc;

use genha_ui_egui::{App, AppOptions};
use wasm_bindgen::JsCast as _;

const CANVAS_ID: &str = "genha_canvas";
const LOADING_ID: &str = "genha_loading";

pub fn start() {
    // A second logger (a page that starts twice) is not worth failing over.
    eframe::WebLogger::init(log::LevelFilter::Info).ok();
    let Some(canvas) = element(CANVAS_ID).and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok()) else {
        log::error!("genha-web: the page has no <canvas id=\"{CANVAS_ID}\">");
        return;
    };
    // Pages can't read installed fonts: this is the built-in fallback faces (fonts the user added
    // come from the browser's storage, installed ones when the user allows it).
    let book = genha_ui_egui::fonts::system_or_fallback();
    log::info!("genha-web: {} built-in font faces", book.len());
    let fonts = Arc::new(book);
    wasm_bindgen_futures::spawn_local(async move {
        // Data Recovery copies, recent designs and added fonts, from IndexedDB.
        let store = genha_ui_egui::open_web_store().await;
        let opts = AppOptions { dark: prefers_dark(), store: Some(store), instance: Some(genha_ui_egui::web_instance_id()), ..Default::default() };
        let result = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |_cc| Ok(Box::new(Shell { app: App::new(fonts, opts), title: String::new() }))),
            )
            .await;
        match result {
            Ok(()) => {
                if let Some(el) = element(LOADING_ID) {
                    el.remove();
                }
            }
            Err(e) => {
                log::error!("genha-web: could not start: {e:?}");
                if let Some(el) = element(LOADING_ID) {
                    el.set_text_content(Some("GenHa could not start: this browser needs WebGL2."));
                }
            }
        }
    });
}

/// Whether the browser asks for dark pages (`None`: it doesn't say).
fn prefers_dark() -> Option<bool> {
    let query = web_sys::window()?.match_media("(prefers-color-scheme: dark)").ok()??;
    Some(query.matches())
}

fn element(id: &str) -> Option<web_sys::Element> {
    web_sys::window()?.document()?.get_element_by_id(id)
}

struct Shell {
    app: App,
    title: String,
}

impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.app.ui(ui);
        // The tab's title follows the design, as the desktop window's does.
        let title = match self.app.session.active() {
            Some(d) => format!("{}{} — GenHa", d.display_name(), if d.dirty { " •" } else { "" }),
            None => "GenHa".to_string(),
        };
        if title != self.title {
            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                doc.set_title(&title);
            }
            self.title = title;
        }
    }
}
