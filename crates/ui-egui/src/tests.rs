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

fn events_frame(app: &mut App, ctx: &egui::Context, time: f64, events: Vec<egui::Event>) -> egui::FullOutput {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 900.0))),
        time: Some(time),
        events,
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| app.ui(ui));
    out.textures_delta.clear();
    out
}

fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
}

fn edited_text(app: &App, id: u64) -> String {
    match &app.session.active().unwrap().doc.element(genha_doc::NodeId(id)).unwrap().kind {
        genha_doc::ElementKind::Text(t) => t.text.clone(),
        _ => panic!("not text"),
    }
}

#[test]
fn text_is_edited_on_the_canvas() {
    let ctx = egui::Context::default();
    let mut app = App::new(Arc::new(genha_text::FontBook::new()), AppOptions::default());
    app.run("file.new", &json!({}));
    let id = app.run("text.add", &json!({"text": "Hello"})).unwrap()["id"].as_u64().unwrap();
    let steps = app.session.active().unwrap().history.undo.len();
    let none = egui::Modifiers::NONE;
    let mut t = 0.0;
    let mut frame = |app: &mut App, events: Vec<egui::Event>| {
        t += 0.1;
        events_frame(app, &ctx, t, events)
    };
    frame(&mut app, vec![]);
    // Enter edits the selected text box, all of it selected; typing replaces it.
    frame(&mut app, vec![key(egui::Key::Enter, none)]);
    assert!(app.session.text_editing.is_some());
    frame(&mut app, vec![]);
    frame(&mut app, vec![egui::Event::Text("Hi".into()), egui::Event::Text(" all".into())]);
    assert_eq!(edited_text(&app, id), "Hi all");
    // Word-wise delete, arrows, a new line, Shift+Home selecting the line.
    frame(&mut app, vec![key(egui::Key::Backspace, egui::Modifiers::CTRL)]);
    assert_eq!(edited_text(&app, id), "Hi ");
    frame(&mut app, vec![key(egui::Key::ArrowLeft, none), key(egui::Key::Enter, none), egui::Event::Text("!".into())]);
    assert_eq!(edited_text(&app, id), "Hi\n! ");
    frame(&mut app, vec![key(egui::Key::Home, egui::Modifiers::SHIFT), egui::Event::Text("?".into())]);
    assert_eq!(edited_text(&app, id), "Hi\n? ");
    // Undo inside the editor goes back step by step.
    frame(&mut app, vec![key(egui::Key::Z, egui::Modifiers::COMMAND)]);
    assert_eq!(edited_text(&app, id), "Hi\n! ");
    // Select all, copy, paste.
    let out = frame(&mut app, vec![key(egui::Key::A, egui::Modifiers::COMMAND), egui::Event::Copy]);
    assert!(out.platform_output.commands.iter().any(|c| matches!(c, egui::OutputCommand::CopyText(s) if s == "Hi\n! ")));
    assert!(out.platform_output.ime.is_some(), "the platform is told where the caret is");
    frame(&mut app, vec![egui::Event::Paste("Pasted\r\ntext".into())]);
    assert_eq!(edited_text(&app, id), "Pasted\ntext");
    // IME: the composition shows in the text, the commit replaces it.
    frame(&mut app, vec![egui::Event::Ime(egui::ImeEvent::Preedit { text: "ni".into(), active_range_chars: None })]);
    assert_eq!(edited_text(&app, id), "Pasted\ntextni");
    frame(&mut app, vec![egui::Event::Ime(egui::ImeEvent::Commit("你".into()))]);
    assert_eq!(edited_text(&app, id), "Pasted\ntext你");
    // Esc leaves; the whole session is one undo step.
    frame(&mut app, vec![key(egui::Key::Escape, none)]);
    assert!(app.session.text_editing.is_none());
    frame(&mut app, vec![]);
    assert_eq!(app.session.active().unwrap().history.undo.len(), steps + 1);
    // Keys are shortcuts again: Backspace deletes the selected box.
    frame(&mut app, vec![key(egui::Key::Backspace, none)]);
    assert!(app.session.active().unwrap().doc.find(genha_doc::NodeId(id)).is_none());
    app.run("edit.undo", &json!({}));
    app.run("edit.undo", &json!({}));
    assert_eq!(edited_text(&app, id), "Hello");
    assert!(app.status.as_ref().is_none_or(|(m, _)| !m.contains("Internal error")), "{:?}", app.status);
}

#[test]
fn clicks_place_the_caret_and_clicking_outside_ends_editing() {
    let ctx = egui::Context::default();
    let mut app = App::new(Arc::new(genha_text::FontBook::new()), AppOptions::default());
    app.run("file.new", &json!({}));
    let id = app.run("text.add", &json!({"text": "abcdef", "x": 100, "y": 100, "width": 600, "style": {"align": "left"}})).unwrap()["id"]
        .as_u64()
        .unwrap();
    let size = match &app.session.active().unwrap().doc.element(genha_doc::NodeId(id)).unwrap().kind {
        genha_doc::ElementKind::Text(t) => t.style.font_size,
        _ => panic!("not text"),
    };
    let mut t = 0.0;
    let mut frame = |app: &mut App, events: Vec<egui::Event>| {
        t += 0.1;
        events_frame(app, &ctx, t, events);
    };
    frame(&mut app, vec![]);
    app.run("text.beginEdit", &json!({"id": id, "selection": [0, 0]}));
    frame(&mut app, vec![]);
    // Page points on screen, through the canvas view.
    let canvas = ctx.read_response(egui::Id::new("genha_canvas")).expect("the canvas");
    let (zoom, origin) = (app.view.zoom, canvas.rect.min + app.view.pan);
    let at = |x: f64, y: f64| egui::pos2(origin.x + x as f32 * zoom, origin.y + y as f32 * zoom);
    let press = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
    // Between "c" and "d" (without fonts every character is 0.55 em wide).
    let p = at(100.0 + 3.0 * 0.55 * size, 100.0 + size * 0.5);
    frame(&mut app, vec![egui::Event::PointerMoved(p), press(p, true)]);
    frame(&mut app, vec![press(p, false)]);
    let e = app.session.text_editing.clone().expect("still editing");
    assert_eq!((e.anchor, e.caret), (3, 3));
    frame(&mut app, vec![egui::Event::Text("X".into())]);
    assert_eq!(edited_text(&app, id), "abcXdef");
    // A press far from the text ends editing.
    let away = at(1000.0, 1000.0);
    frame(&mut app, vec![egui::Event::PointerMoved(away), press(away, true)]);
    frame(&mut app, vec![press(away, false)]);
    assert!(app.session.text_editing.is_none());
}
