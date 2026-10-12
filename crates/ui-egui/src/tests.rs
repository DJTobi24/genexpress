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

fn key_frame(app: &mut App, ctx: &egui::Context, time: f64, key: Option<egui::Key>) {
    let events = key
        .map(|key| vec![egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }])
        .unwrap_or_default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 900.0))),
        time: Some(time),
        events,
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| app.ui(ui));
    out.textures_delta.clear();
}

#[test]
fn playback_and_present_mode() {
    let ctx = egui::Context::default();
    let mut app = App::new(Arc::new(genha_text::FontBook::new()), AppOptions::default());
    app.run("template.use", &json!({"id": genha_templates::catalog()[0].id}));
    app.run("page.add", &json!({}));
    app.run("text.add", &json!({"text": "Second"}));
    app.run("page.goto", &json!({"index": 0}));
    app.run("animation.applyToPage", &json!({"kind": "pop"})).unwrap();
    // The page panel and an animated object's panel draw.
    key_frame(&mut app, &ctx, 0.0, None);
    let first = app.session.active().unwrap().doc.pages[0].elements.last().unwrap().id.0;
    app.run("select.set", &json!({"ids": [first]}));
    key_frame(&mut app, &ctx, 0.1, None);
    // Play: frames render while it plays, and it stops by itself at the end.
    crate::motion::play(&mut app, &ctx);
    for i in 0..5 {
        key_frame(&mut app, &ctx, 0.2 + f64::from(i) * 0.3, None);
        assert!(app.motion.playing.is_some());
    }
    key_frame(&mut app, &ctx, 100.0, None);
    assert!(app.motion.playing.is_none(), "playback ends with the page");
    // Present: F5 enters, Right moves on (and stops at the last page), N shows notes, Esc leaves.
    key_frame(&mut app, &ctx, 101.0, Some(egui::Key::F5));
    assert_eq!(app.motion.present.as_ref().map(|p| p.page), Some(0));
    key_frame(&mut app, &ctx, 101.2, None);
    key_frame(&mut app, &ctx, 101.4, Some(egui::Key::ArrowRight));
    key_frame(&mut app, &ctx, 101.6, Some(egui::Key::ArrowRight));
    assert_eq!(app.motion.present.as_ref().map(|p| p.page), Some(1));
    key_frame(&mut app, &ctx, 101.8, Some(egui::Key::N));
    assert!(app.motion.present.as_ref().is_some_and(|p| p.show_notes));
    key_frame(&mut app, &ctx, 102.0, Some(egui::Key::ArrowLeft));
    assert_eq!(app.motion.present.as_ref().map(|p| p.page), Some(0));
    key_frame(&mut app, &ctx, 102.2, Some(egui::Key::Escape));
    assert!(app.motion.present.is_none());
    assert_eq!(app.screen, Screen::Editor);
    assert!(app.status.as_ref().is_none_or(|(m, _)| !m.contains("Internal error")), "{:?}", app.status);
}

#[test]
fn chart_table_and_curve_panels_run() {
    let ctx = egui::Context::default();
    let mut app = App::new(Arc::new(genha_text::FontBook::new()), AppOptions::default());
    app.run("file.new", &json!({}));
    app.left_tab = crate::LeftTab::Elements;
    for (cmd, params) in [
        ("chart.add", json!({"kind": "donut"})),
        ("chart.add", json!({"kind": "progress"})),
        ("table.add", json!({})),
        ("text.add", json!({"text": "Bend me"})),
    ] {
        app.run(cmd, &params);
        if cmd == "text.add" {
            app.run("text.setCurve", &json!({"degrees": 120}));
        }
        run_frames(&mut app, &ctx, 2);
    }
    let n = app.session.active().unwrap().doc.pages[0].elements.len();
    assert_eq!(n, 4);
    assert!(app.status.as_ref().is_none_or(|(m, _)| !m.contains("Internal error")), "{:?}", app.status);
}
