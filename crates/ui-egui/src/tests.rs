//! Headless frames: the whole UI runs without a window.

use std::sync::Arc;

use serde_json::json;

use crate::{App, AppOptions, Screen};

/// The app in English, whatever this computer's language (the tests match English messages).
pub(crate) fn english(mut app: App) -> App {
    app.session.language = genha_i18n::Setting::Fixed(genha_i18n::Lang::En);
    genha_i18n::set_language(genha_i18n::Lang::En);
    app
}

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
    let mut app = english(App::new(Arc::new(genha_text::FontBook::new()), AppOptions::default()));
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
    let mut app = english(App::new(Arc::new(genha_text::FontBook::new()), AppOptions::default()));
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
    let mut app = english(App::new(Arc::new(genha_text::FontBook::new()), AppOptions::default()));
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
    let mut app = english(App::new(Arc::new(genha_text::FontBook::new()), AppOptions::default()));
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
    let mut app = english(App::new(Arc::new(genha_text::FontBook::new()), AppOptions::default()));
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

#[test]
fn recovery_and_recent_designs_through_a_store() {
    // The browser's setup: a store instead of a folder, one run after another.
    let store = Box::new(genha_engine::store::MemStore::default());
    let opts = AppOptions { store: Some(store), instance: Some("run1".into()), ..Default::default() };
    let mut app = english(App::new(Arc::new(genha_text::FontBook::new()), opts));
    app.run("template.use", &json!({"id": genha_templates::catalog()[0].id}));
    app.run("recovery.save", &json!({}));
    // A download records it in the recent designs.
    let bytes = genha_engine::cmd::file::design_bytes(&mut app.session).unwrap();
    assert!(!bytes.is_empty());
    app.run("text.add", &json!({"text": "Unsaved"}));
    app.run("recovery.save", &json!({}));
    let store = app.session.store.take();
    // The next run (a reload) offers the copy and lists the design.
    let mut next =
        english(App::new(Arc::new(genha_text::FontBook::new()), AppOptions { store, instance: Some("run2".into()), ..Default::default() }));
    assert_eq!(next.recovered.len(), 1);
    assert_eq!(crate::home::recent_designs(&next).len(), 1);
    let ctx = egui::Context::default();
    run_frames(&mut next, &ctx, 3);
    let id = crate::home::recent_designs(&next)[0].id.clone();
    next.run("recent.open", &json!({"id": id}));
    assert_eq!(next.screen, Screen::Editor);
    // Copying objects puts their text on the system clipboard.
    next.run("text.add", &json!({"text": "Copy me"}));
    next.run("edit.copy", &json!({}));
    assert_eq!(next.clip_out.as_deref(), Some("Copy me"));
    run_frames(&mut next, &ctx, 1);
    assert!(next.clip_out.is_none());
    assert!(next.status.as_ref().is_none_or(|(m, _)| !m.contains("Internal error")), "{:?}", next.status);
}

/// Download › MP4 video runs on a worker thread: the editor keeps drawing (with progress in the
/// status bar) and reports the file when it is written; Cancel stops it.
#[test]
fn video_export_runs_beside_the_editor() {
    let ctx = egui::Context::default();
    let mut app = english(App::new(Arc::new(genha_text::FontBook::new()), AppOptions::default()));
    app.run("file.new", &json!({"width": 64, "height": 48}));
    app.run("shape.add", &json!({"width": 20, "height": 20}));
    app.run("animation.set", &json!({"kind": "pop"}));
    let dir = std::env::temp_dir().join(format!("genha-ui-video-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("v.mp4");
    let params = json!({"path": path.display().to_string()});
    app.editor.video_job = crate::editor::VideoJob::start(&app, &ctx, params.clone(), path.clone());
    assert!(app.editor.video_job.is_some());
    let start = std::time::Instant::now();
    while app.editor.video_job.is_some() && start.elapsed().as_secs() < 60 {
        run_frames(&mut app, &ctx, 1);
    }
    assert!(app.editor.video_job.is_none(), "the export finished");
    assert!(app.status.as_ref().is_some_and(|(m, _)| m.starts_with("Downloaded")), "{:?}", app.status);
    let info = genha_video::inspect(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!((info.width, info.height), (64, 48));

    // Cancelled before it starts: no file, a message.
    std::fs::remove_file(&path).unwrap();
    let job = crate::editor::VideoJob::start(&app, &ctx, params, path.clone()).unwrap();
    job.cancel();
    app.editor.video_job = Some(job);
    while app.editor.video_job.is_some() && start.elapsed().as_secs() < 120 {
        run_frames(&mut app, &ctx, 1);
    }
    assert!(app.status.as_ref().is_some_and(|(m, _)| m.contains("cancelled")), "{:?}", app.status);
    assert!(!path.exists());
}

/// Frames at `width` × 900; the texts drawn in the last one. Fails on "Internal error".
fn frames_at(app: &mut App, ctx: &egui::Context, width: f32, n: usize) -> Vec<String> {
    fn collect(shape: &egui::Shape, texts: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(t) => texts.push(t.galley.text().to_string()),
            egui::Shape::Vec(v) => v.iter().for_each(|s| collect(s, texts)),
            _ => {}
        }
    }
    let mut texts = vec![];
    for _ in 0..n {
        let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 900.0))), ..Default::default() };
        let mut out = ctx.run_ui(input, |ui| app.ui(ui));
        out.textures_delta.clear();
        texts.clear();
        out.shapes.iter().for_each(|c| collect(&c.shape, &mut texts));
    }
    assert!(app.status.as_ref().is_none_or(|(m, _)| !m.contains("Internal error")), "{:?}", app.status);
    texts
}

/// An app keeping My designs in memory, with three designs from templates and two folders, back
/// on the home screen.
fn app_with_designs() -> App {
    let store = Box::new(genha_engine::store::MemStore::default());
    let mut app = english(App::new(Arc::new(genha_text::FontBook::new()), AppOptions { store: Some(store), ..Default::default() }));
    for t in genha_templates::catalog().iter().take(3) {
        app.run("template.use", &json!({"id": t.id}));
    }
    let work = app.run("folder.create", &json!({"name": "Summer fair", "color": "#e8a317"})).unwrap()["id"].as_str().unwrap().to_string();
    app.run("folder.create", &json!({"name": "Bakery"}));
    let first = app.library.designs[0].id.clone();
    app.run("design.move", &json!({"ids": [first], "folder": work}));
    app.go_home();
    app
}

#[test]
fn home_screen_pages_and_popups_draw() {
    let ctx = egui::Context::default();
    let mut app = app_with_designs();
    assert_eq!(app.screen, Screen::Home);
    let texts = frames_at(&mut app, &ctx, 1440.0, 4);
    for want in ["What would you like to start with?", "Open a file", "Create a presentation", "Recent designs", "Quick actions", "Summer fair"] {
        assert!(texts.iter().any(|t| t == want), "{want} not on the home page: {texts:?}");
    }
    assert_eq!(crate::home::recent_designs(&app).len(), 3);
    // My designs with a folder open: its one design.
    app.home.page = crate::home::Page::Designs;
    app.home.folder = genha_engine::cmd::designs::FolderFilter::Folder(app.library.folders[0].id.clone());
    let texts = frames_at(&mut app, &ctx, 1440.0, 3);
    assert!(texts.iter().any(|t| t == "1 design"), "{texts:?}");
    // Every page, the search results, each pop-up, at a narrow width too.
    for width in [1440.0, 1000.0] {
        for page in crate::home::Page::ALL {
            crate::home::go(&mut app, page);
            frames_at(&mut app, &ctx, width, 2);
        }
        app.home.page = crate::home::Page::Home;
        app.home.search = "poster".into();
        app.home.search_open = true;
        let texts = frames_at(&mut app, &ctx, width, 3);
        assert!(texts.iter().any(|t| t == "Templates") && texts.iter().any(|t| t == "Sizes"), "{texts:?}");
        app.home.search_open = false;
        for modal in [
            crate::popups::Modal::SizePicker { query: String::new(), palette: None },
            crate::popups::Modal::Video,
            crate::popups::Modal::Presentation,
            crate::popups::Modal::Qr { text: String::new() },
            crate::popups::Modal::NewFolder { name: String::new(), color: 0 },
        ] {
            app.home.modal = Some(modal.clone());
            frames_at(&mut app, &ctx, width, 2);
            assert_eq!(app.home.modal.as_ref(), Some(&modal), "the pop-up stays open");
        }
        app.home.modal = None;
    }
    app.dark = true;
    frames_at(&mut app, &ctx, 1440.0, 2);
}

#[test]
fn designs_start_in_the_folder_shown_and_the_greeting_has_the_name() {
    let ctx = egui::Context::default();
    let mut app = app_with_designs();
    // A design made from My designs with a folder open goes into that folder.
    let folder = app.library.folders[1].id.clone();
    app.home.page = crate::home::Page::Designs;
    app.home.folder = genha_engine::cmd::designs::FolderFilter::Folder(folder.clone());
    assert!(crate::home::create(&mut app, json!({"preset": "logo"}), None));
    assert_eq!(app.screen, Screen::Editor);
    app.refresh_library();
    assert_eq!(app.library.count(&folder), 1);
    app.go_home();
    assert_eq!(app.library.designs.len(), 4);
    // Settings: the greeting's name.
    app.run("settings.set", &json!({"name": "Ada"}));
    app.home.page = crate::home::Page::Home;
    let texts = frames_at(&mut app, &ctx, 1440.0, 2);
    assert!(texts.iter().any(|t| t.starts_with("Good ") && t.ends_with(", Ada")), "{texts:?}");
    assert_eq!(crate::home::ago(1_000_000.0, 1_000_000 - 6 * 86_400), "6 days ago");
    assert_eq!(crate::home::ago(100.0, 0), "");
}

#[test]
fn text_panel_opens_for_a_text_box_and_closes_from_the_rail() {
    let ctx = egui::Context::default();
    let mut app = english(App::new(Arc::new(genha_text::FontBook::new()), AppOptions::default()));
    app.run("file.new", &json!({"width": 1080, "height": 1080}));
    app.screen = Screen::Editor;
    let a = app.run("text.add", &json!({"text": "One"})).unwrap()["id"].as_u64().unwrap();
    let b = app.run("text.add", &json!({"text": "Two"})).unwrap()["id"].as_u64().unwrap();
    let shape = app.run("shape.add", &json!({})).unwrap()["id"].as_u64().unwrap();
    app.run("select.set", &json!({"ids": [a]}));
    assert_eq!(crate::properties::text_panel_target(&app), Some(genha_doc::NodeId(a)));
    // Every tab draws.
    for tab in [crate::properties::TextTab::Edit, crate::properties::TextTab::Effects, crate::properties::TextTab::Animation] {
        app.editor.text_tab = tab;
        run_frames(&mut app, &ctx, 2);
    }
    // A shape or two objects: the properties panel instead.
    app.run("select.set", &json!({"ids": [shape]}));
    assert_eq!(crate::properties::text_panel_target(&app), None);
    app.run("select.set", &json!({"ids": [a, b]}));
    assert_eq!(crate::properties::text_panel_target(&app), None);
    // Closed for one box, it opens again for another, and for the first after deselecting.
    app.run("select.set", &json!({"ids": [a]}));
    app.editor.text_panel_closed = Some(genha_doc::NodeId(a));
    assert_eq!(crate::properties::text_panel_target(&app), None);
    run_frames(&mut app, &ctx, 1);
    app.run("select.set", &json!({"ids": [b]}));
    assert_eq!(crate::properties::text_panel_target(&app), Some(genha_doc::NodeId(b)));
    app.run("select.set", &json!({"ids": []}));
    run_frames(&mut app, &ctx, 1);
    app.run("select.set", &json!({"ids": [a]}));
    assert_eq!(crate::properties::text_panel_target(&app), Some(genha_doc::NodeId(a)));
    // A text style from the panel's row applies to the box.
    let preset = genha_templates::text_presets()[0].id;
    app.run("text.applyPreset", &json!({"id": preset}));
    run_frames(&mut app, &ctx, 2);
}
