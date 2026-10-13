//! GenHa in the browser.
//!
//! Runs the same [`genha_ui_egui::App`] as the desktop app through eframe's web runner (WebGL2)
//! on the page's `<canvas id="genha_canvas">`. Build with `cargo xtask web` (see
//! docs/development.md › Web); the page is `dist/web/index.html`.
//!
//! Differences from the desktop app:
//! - Open… and Upload use the browser's file picker, and files can be dropped on the page; the
//!   bytes arrive a frame later (`genha_ui_egui` › `web.rs`);
//! - Save and Download make the file in memory and the browser downloads it;
//! - no control channel (browsers can't listen on sockets), no recent files, no Data Recovery;
//! - fonts: the browser gives pages no access to installed fonts, so text is set in built-in
//!   fallback faces (`genha_ui_egui::fonts`).

#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(target_arch = "wasm32")]
fn main() {
    web::start();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!(
        "genha-web only runs in the browser: build it with `cargo xtask web`, then serve dist/web \
         (for example `python3 -m http.server -d dist/web 8080`) and open http://127.0.0.1:8080. \
         For the desktop app run `cargo run --release -p genha`."
    );
}
