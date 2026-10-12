//! Headless frames: the whole UI runs without a window.

use std::sync::Arc;

use serde_json::json;

use crate::{App, AppOptions, Screen};

fn run_frames(app: &mut App, ctx: &egui::Context, n: usize) {
    for _ in 0..n {
        let input =
            egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 900.0))), ..Default::default() };
        let mut out = ctx.run_ui(input, |ui| app.ui(ui));
        // No renderer here to consume the texture uploads.
        out.textures_delta.clear();
    }
}

#[test]
fn home_and_editor_frames_run() {
    let ctx = egui::Context::default();
    let mut app = App::new(Arc::new(genha_text::FontBook::new()), AppOptions::default());
    run_frames(&mut app, &ctx, 3);
    assert_eq!(app.screen, Screen::Home);
    app.run("template.use", &json!({"id": genha_templates::catalog()[0].id}));
    assert_eq!(app.screen, Screen::Editor);
    for tab in crate::LeftTab::ALL {
        app.left_tab = tab;
        run_frames(&mut app, &ctx, 2);
    }
    // Selection shows the element's properties.
    let first = app.session.active().unwrap().doc.pages[0].elements.last().unwrap().id.0;
    app.run("select.set", &json!({"ids": [first]}));
    run_frames(&mut app, &ctx, 2);
    app.dark = true;
    run_frames(&mut app, &ctx, 2);
    assert!(app.status.as_ref().is_none_or(|(m, _)| !m.contains("Internal error")), "{:?}", app.status);
}
