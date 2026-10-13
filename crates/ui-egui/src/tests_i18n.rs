//! Translations: every string the UI shows has a German translation, nothing in the German
//! catalogue is unused, the German UI shows no English, and the UI's fallback fonts.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde_json::json;

use crate::{App, AppOptions};
use genha_i18n::{Lang, Setting};

/// A string literal starting at `s` (after the opening quote is expected at s[0]); returns the
/// text (escapes resolved) and the rest after the closing quote.
fn literal(s: &str) -> Option<(String, &str)> {
    let s = s.trim_start().strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = s.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return Some((out, &s[i + 1..])),
            '\\' => match chars.next()?.1 {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                '\n' => {
                    // A line continuation: skip the next line's leading spaces.
                    while chars.clone().next().is_some_and(|(_, c)| c.is_whitespace()) {
                        chars.next();
                    }
                }
                other => out.push(other),
            },
            c => out.push(c),
        }
    }
    None
}

/// The English strings the UI's code translates: the literals in `tr("…")`, `tr!("…", …)`,
/// `N_("…")` (plain) and `tr_n!("…", "…", n)` (counted: singular → plural).
fn source_keys(src: &str) -> (BTreeSet<String>, BTreeMap<String, String>) {
    let mut plain = BTreeSet::new();
    let mut counted = BTreeMap::new();
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    for (i, _) in src.match_indices(['t', 'N']) {
        if src[..i].chars().next_back().is_some_and(ident) {
            continue;
        }
        let rest = &src[i..];
        let (kind, after) = if let Some(r) = rest.strip_prefix("tr_n!(") {
            ("n", r)
        } else if let Some(r) = rest.strip_prefix("tr!(").or_else(|| rest.strip_prefix("tr(")).or_else(|| rest.strip_prefix("N_(")) {
            ("1", r)
        } else {
            continue;
        };
        let Some((one, after)) = literal(after) else { continue };
        if kind == "n" {
            let Some((other, _)) = after.trim_start().strip_prefix(',').and_then(literal) else { continue };
            counted.insert(one, other);
        } else {
            plain.insert(one);
        }
    }
    (plain, counted)
}

const SOURCES: &[(&str, &str)] = &[
    ("lib.rs", include_str!("lib.rs")),
    ("editor.rs", include_str!("editor.rs")),
    ("properties.rs", include_str!("properties.rs")),
    ("data_props.rs", include_str!("data_props.rs")),
    ("home.rs", include_str!("home.rs")),
    ("designs.rs", include_str!("designs.rs")),
    ("pages.rs", include_str!("pages.rs")),
    ("popups.rs", include_str!("popups.rs")),
    ("art.rs", include_str!("art.rs")),
    ("motion.rs", include_str!("motion.rs")),
    ("fonts.rs", include_str!("fonts.rs")),
    ("canvas.rs", include_str!("canvas.rs")),
    ("clipboard.rs", include_str!("clipboard.rs")),
    ("textedit.rs", include_str!("textedit.rs")),
    ("web.rs", include_str!("web.rs")),
    ("control.rs", include_str!("control.rs")),
    ("io.rs", include_str!("io.rs")),
    ("models.rs", include_str!("models.rs")),
    ("apps/genha", include_str!("../../../apps/genha/src/main.rs")),
    // Engine strings shown in the UI: the quick actions, a duplicate's title.
    ("engine home.rs", include_str!("../../engine/src/cmd/home.rs")),
    ("engine designs.rs", include_str!("../../engine/src/cmd/designs.rs")),
];

/// Every English string the UI shows: (from the UI's code, plain), (counted), (from data in lower
/// crates: commands, sizes, charts, tools, animations, filters, collages, templates).
fn ui_strings() -> (BTreeSet<String>, BTreeMap<String, String>, BTreeSet<String>) {
    let mut plain = BTreeSet::new();
    let mut counted = BTreeMap::new();
    for (_, src) in SOURCES {
        let (p, c) = source_keys(src);
        plain.extend(p);
        counted.extend(c);
    }
    let mut data: BTreeSet<String> = BTreeSet::new();
    let mut add = |s: &str| {
        data.insert(s.to_string());
    };
    for spec in genha_engine::command_specs() {
        // The palette lists the commands that have a menu, under their menu's name.
        if !spec.menu.is_empty() {
            add(spec.label);
            spec.menu.iter().for_each(|m| add(m));
        }
    }
    // Why a command is not available (the common ones; others stay English).
    for why in ["no document open", "nothing selected", "select at least two objects", "nothing to undo", "nothing to redo", "the clipboard is empty"]
    {
        add(why);
    }
    for p in genha_doc::preset::PRESETS {
        add(p.label);
        add(p.category);
    }
    genha_doc::ChartKind::ALL.iter().for_each(|k| add(k.label()));
    genha_doc::BorderLines::ALL.iter().for_each(|b| add(b.label()));
    genha_tools::ToolId::ALL.iter().for_each(|t| add(t.label()));
    genha_motion::ENTRANCES.iter().for_each(|e| add(e.2));
    genha_motion::LOOPS.iter().for_each(|e| add(e.2));
    for kind in ["Shape", "Text", "Image", "Group", "Chart", "Table"] {
        add(kind);
    }
    genha_effects::filter_presets().iter().for_each(|f| add(f.label));
    for n in 1..=12 {
        genha_layout::collage_layouts(n).iter().for_each(|l| add(l.label));
    }
    for t in genha_templates::catalog() {
        add(t.category);
    }
    for s in genha_templates::shape_library() {
        add(s.label);
        add(s.category);
    }
    for t in genha_templates::text_presets() {
        add(t.label);
        add(t.sample);
    }
    genha_templates::palettes().iter().for_each(|p| add(p.name));
    (plain, counted, data)
}

#[test]
fn every_ui_string_has_a_german_translation() {
    let de = Lang::De.catalog().expect("the German catalogue");
    let (plain, counted, data) = ui_strings();
    let missing: Vec<&String> = plain.iter().chain(&data).filter(|k| de.get(k).is_none()).collect();
    let missing_counted: Vec<&String> = counted
        .iter()
        .filter(|(one, other)| de.get_plural(one, 0).is_none() || !de.entries().any(|e| &e.id == *one && e.plural.as_ref() == Some(*other)))
        .map(|(one, _)| one)
        .collect();
    // The other way round: everything in the catalogue is shown somewhere.
    let known: BTreeSet<&String> = plain.iter().chain(&data).chain(counted.keys()).collect();
    // Entries with a context: the picture filters' names (`trc("filter", …)`).
    let filters: Vec<&str> = genha_effects::filter_presets().iter().map(|f| f.label).collect();
    let stale: Vec<&String> = de
        .entries()
        .filter(|e| match &e.context {
            Some(c) => !(c == "filter" && filters.contains(&e.id.as_str())),
            None => !known.contains(&e.id),
        })
        .map(|e| &e.id)
        .collect();
    assert!(
        missing.is_empty() && missing_counted.is_empty() && stale.is_empty(),
        "crates/i18n/po/de.po needs work.\nMissing ({}): {missing:#?}\nMissing counted ({}): {missing_counted:#?}\nUnused ({}): {stale:#?}",
        missing.len(),
        missing_counted.len(),
        stale.len()
    );
    eprintln!("{} UI strings, {} counted, {} from data; {} German entries", plain.len(), counted.len(), data.len(), de.entries().count());
}

#[test]
fn key_scanner_reads_the_forms_the_code_uses() {
    let (plain, counted) = source_keys(
        r#"tr("A") genha_i18n::tr!("B {x}", x = 1) N_("C") tr_n!(
            "{n} d",
            "{n} ds", 3) str("not me") as_str("nor me") tr("Say \"hi\"\n")"#,
    );
    assert_eq!(plain.into_iter().collect::<Vec<_>>(), ["A", "B {x}", "C", "Say \"hi\"\n"]);
    assert_eq!(counted.get("{n} d").map(String::as_str), Some("{n} ds"));
}

fn app_in(lang: &str) -> App {
    let mut app = App::new(Arc::new(genha_text::FontBook::new()), AppOptions::default());
    app.run("ui.setLanguage", &json!({"lang": lang})).unwrap();
    app
}

fn frame(app: &mut App, ctx: &egui::Context) -> Vec<String> {
    let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 900.0))), ..Default::default() };
    let mut out = ctx.run_ui(input, |ui| app.ui(ui));
    out.textures_delta.clear();
    let mut texts = vec![];
    fn collect(shape: &egui::Shape, texts: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(t) => texts.push(t.galley.text().to_string()),
            egui::Shape::Vec(v) => v.iter().for_each(|s| collect(s, texts)),
            _ => {}
        }
    }
    out.shapes.iter().for_each(|c| collect(&c.shape, &mut texts));
    texts
}

/// The texts shown over a few frames (popups and windows settle in the second).
fn shown(app: &mut App, ctx: &egui::Context) -> Vec<String> {
    frame(app, ctx);
    frame(app, ctx)
}

/// English strings on screen that have a different German translation.
fn english_leftovers(texts: &[String]) -> Vec<String> {
    let de = Lang::De.catalog().expect("the German catalogue");
    let mut out: Vec<String> = texts.iter().filter(|t| de.get(t).is_some_and(|g| g != t.as_str())).cloned().collect();
    out.sort();
    out.dedup();
    out
}

#[test]
fn the_german_ui_shows_no_english() {
    let ctx = egui::Context::default();
    let mut app = app_in("de");
    let mut seen = vec![];
    let mut check = |what: &str, texts: Vec<String>| {
        let left = english_leftovers(&texts);
        assert!(left.is_empty(), "{what}: English left in the German UI: {left:?}");
        seen.extend(texts);
    };
    check("home", shown(&mut app, &ctx));
    // The home screen's other pages, its search results and pop-ups.
    for page in crate::home::Page::ALL {
        crate::home::go(&mut app, page);
        check(page.id(), shown(&mut app, &ctx));
    }
    crate::home::go(&mut app, crate::home::Page::Home);
    app.home.search = "Plakat".into();
    app.home.search_open = true;
    check("search", shown(&mut app, &ctx));
    app.home.search_open = false;
    for modal in [
        crate::popups::Modal::SizePicker { query: String::new(), palette: None },
        crate::popups::Modal::Video,
        crate::popups::Modal::Presentation,
        crate::popups::Modal::Qr { text: String::new() },
        crate::popups::Modal::NewFolder { name: String::new(), color: 0 },
    ] {
        app.home.modal = Some(modal);
        check("pop-up", shown(&mut app, &ctx));
    }
    app.home.modal = None;
    app.run("file.new", &json!({"preset": "instagram-post", "title": "Mein Design"}));
    for tab in crate::LeftTab::ALL {
        app.left_tab = tab;
        check(tab.label(), shown(&mut app, &ctx));
    }
    // The properties of each kind of object (and of the page, with nothing selected).
    let mut rgba = vec![255u8; 8 * 8 * 4];
    rgba[..4].copy_from_slice(&[200, 30, 30, 255]);
    let png = genha_render::encode_png(&genha_render::Raster { width: 8, height: 8, rgba }).unwrap();
    use base64::Engine as _;
    let data = base64::engine::general_purpose::STANDARD.encode(png);
    for (cmd, params) in [
        ("text.add", json!({"text": "Hallo"})),
        ("shape.add", json!({"shape": "rect"})),
        ("image.place", json!({"data": data, "name": "bild.png"})),
        ("chart.add", json!({"kind": "bar"})),
        ("table.add", json!({})),
    ] {
        app.run(cmd, &params).unwrap();
        check(cmd, shown(&mut app, &ctx));
    }
    app.run("select.all", &json!({}));
    check("several objects", shown(&mut app, &ctx));
    app.run("select.none", &json!({}));
    check("page", shown(&mut app, &ctx));
    // The command palette: labels and menus in German.
    app.editor.palette_open = true;
    let palette = shown(&mut app, &ctx);
    assert!(palette.iter().any(|t| t.contains("Rückgängig")), "{palette:?}");
    assert!(palette.iter().any(|t| t.contains("Bearbeiten  ›  ")), "{palette:?}");
    check("palette", palette);
    app.editor.palette_open = false;
    app.editor.versions_open = true;
    check("versions", shown(&mut app, &ctx));
    for german in ["Vorlagen", "Ebenen", "Herunterladen", "Seite 1 von 1"] {
        assert!(seen.iter().any(|t| t.contains(german)), "{german} not shown");
    }
    assert!(app.status.as_ref().is_none_or(|(m, _)| !m.contains("Interner Fehler")), "{:?}", app.status);
}

#[test]
fn language_switches_live_and_is_kept() {
    let ctx = egui::Context::default();
    let store = Box::new(genha_engine::store::MemStore::default());
    let mut app = App::new(Arc::new(genha_text::FontBook::new()), AppOptions { store: Some(store), ..Default::default() });
    app.run("ui.setLanguage", &json!({"lang": "en"})).unwrap();
    app.run("file.new", &json!({}));
    assert!(shown(&mut app, &ctx).iter().any(|t| t == "Templates"));
    // Through the control channel, as an agent would: the next frame is German.
    let r = crate::control::handle(&mut app, &ctx, "engine.execute", &json!({"command": "ui.setLanguage", "params": {"lang": "de"}})).unwrap();
    assert_eq!(r["resolved"], "de");
    let texts = shown(&mut app, &ctx);
    assert!(texts.iter().any(|t| t == "Vorlagen") && !texts.iter().any(|t| t == "Templates"), "{texts:?}");
    assert_eq!(genha_i18n::language(), Lang::De);
    // Errors from commands reach the status bar in German.
    app.run("select.none", &json!({}));
    app.run("edit.delete", &json!({}));
    assert!(app.status.as_ref().is_some_and(|(m, _)| m.contains("nicht verfügbar")), "{:?}", app.status);
    // Kept for the next run.
    let store = app.session.store.take();
    let next = App::new(Arc::new(genha_text::FontBook::new()), AppOptions { store, ..Default::default() });
    assert_eq!(next.session.language, Setting::Fixed(Lang::De));
    genha_i18n::set_language(Lang::En);
}

#[test]
fn the_palette_finds_commands_in_german_and_english() {
    genha_i18n::set_language(Lang::De);
    let undo = crate::editor::palette_entries("rückg");
    assert!(
        undo.iter().any(|e| e.spec.id == "edit.undo" && e.label == "Rückgängig" && e.menu == "Bearbeiten"),
        "{:?}",
        undo.iter().map(|e| &e.label).collect::<Vec<_>>()
    );
    // English labels and ids still find them.
    assert!(crate::editor::palette_entries("undo").iter().any(|e| e.spec.id == "edit.undo"));
    assert!(crate::editor::palette_entries("edit.redo").iter().any(|e| e.label == "Wiederholen"));
    genha_i18n::set_language(Lang::En);
    assert!(crate::editor::palette_entries("rückg").is_empty());
    assert!(crate::editor::palette_entries("undo").iter().any(|e| e.label == "Undo" && e.menu == "Edit"));
}

#[test]
fn unknown_languages_fall_back_to_english() {
    let mut app = App::new(Arc::new(genha_text::FontBook::new()), AppOptions::default());
    assert!(app.run("ui.setLanguage", &json!({"lang": "tlh"})).is_none());
    assert!(app.status.as_ref().is_some_and(|(m, _)| m.contains("unknown language")));
    genha_i18n::set_language(Lang::En);
    assert_eq!(genha_i18n::tr("Download"), "Download");
    assert_eq!(genha_i18n::tr_in(Lang::De, "No such string anywhere"), "No such string anywhere");
    assert_eq!(genha_i18n::tr_n!("{n} page", "{n} pages", 2), "2 pages");
}

/// The UI's fallback fonts come from the system's fonts, so egui can draw Arabic, Hebrew, CJK…
/// in property fields and layer names. Skipped on a computer without such fonts.
#[test]
fn ui_falls_back_to_system_fonts_for_other_scripts() {
    let book = genha_text::FontBook::system();
    let start = std::time::Instant::now();
    let faces = crate::fonts::ui_fallback_faces(&book);
    let took = start.elapsed();
    let scripts: Vec<&str> = faces.iter().flat_map(|f| f.scripts.iter().copied()).collect();
    eprintln!(
        "UI fallback faces in {took:?}: {}",
        faces.iter().map(|f| format!("{} {:?} ({} KB)", f.family, f.scripts, f.data.len() / 1024)).collect::<Vec<_>>().join("; ")
    );
    assert!(faces.len() <= 10);
    let ctx = egui::Context::default();
    ctx.set_fonts(crate::fonts::ui_font_definitions(&faces));
    let input = egui::RawInput::default();
    ctx.run_ui(input, |_| {}).textures_delta.clear();
    for (script, sample) in [("Arabic", "مرحبا"), ("Hebrew", "שלום"), ("Han", "你好"), ("Devanagari", "नमस्ते"), ("Thai", "สวัสดี")]
    {
        let has = ctx.fonts_mut(|f| f.has_glyphs(&egui::FontId::proportional(14.0), sample));
        if scripts.contains(&script) {
            assert!(has, "{script} chosen but not drawn");
        } else {
            eprintln!("no system font for {script}: skipped");
        }
    }
    // Without fonts: nothing chosen, egui's own fonts only.
    assert!(crate::fonts::ui_fallback_faces(&genha_text::FontBook::new()).is_empty());
}
