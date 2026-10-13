//! The home screen: a navigation rail (New design, Home, My designs, Templates, Brand, the
//! project folders, Settings) beside one of its pages.
//!
//! - Home: one search over everything, a greeting, five start tiles (open a file, new design,
//!   edit a photo, create a video, create a presentation), recent designs, quick actions and a
//!   row of templates.
//! - My designs ([`crate::designs`]), Templates, Brand and Settings ([`crate::pages`]).
//! - Pop-ups (size picker, video and presentation starters, QR code, rename and delete
//!   confirmations) and the search results ([`crate::popups`]).
//!
//! Everything acts through engine commands (`file.new`, `template.use`, `design.*`, `folder.*`,
//! `quick.image`…); this module only lays them out.

use serde_json::json;

use crate::art::{self, Art, Icon, Scene};
use crate::popups::Modal;
use crate::theme::Tokens;
use crate::{App, io};
use genha_engine::cmd::designs::{DesignEntry, FolderFilter, Sort};
use genha_i18n::{N_, tr, tr_n};

/// Width of the navigation rail.
pub const RAIL_W: f32 = 96.0;
/// Widest the page content gets (wider windows centre it).
const CONTENT_MAX: f32 = 1280.0;

/// The home screen's pages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Home,
    Designs,
    Templates,
    Brand,
    Settings,
}

impl Page {
    pub const ALL: [Page; 5] = [Page::Home, Page::Designs, Page::Templates, Page::Brand, Page::Settings];
    pub fn id(self) -> &'static str {
        match self {
            Page::Home => "home",
            Page::Designs => "designs",
            Page::Templates => "templates",
            Page::Brand => "brand",
            Page::Settings => "settings",
        }
    }
    pub fn parse(s: &str) -> Option<Page> {
        Page::ALL.into_iter().find(|p| p.id() == s)
    }
    /// The page's name in English (shown translated).
    pub fn label(self) -> &'static str {
        match self {
            Page::Home => N_("Home"),
            Page::Designs => N_("My designs"),
            Page::Templates => N_("Templates"),
            Page::Brand => N_("Brand"),
            Page::Settings => N_("Settings"),
        }
    }
    fn icon(self) -> Icon {
        match self {
            Page::Home => Icon::Lib("home"),
            Page::Designs => Icon::Folder,
            Page::Templates => Icon::Grid,
            Page::Brand => Icon::Brand,
            Page::Settings => Icon::Lib("settings"),
        }
    }
}

/// Template filters on the home screen and the Templates page: label, and what they keep.
pub const TEMPLATE_CHIPS: &[(&str, Chip)] = &[
    (N_("All"), Chip::All),
    (N_("Social"), Chip::Category("Social")),
    (N_("Video"), Chip::Tag("animated")),
    (N_("Presentations"), Chip::Tag("presentation")),
    (N_("Print"), Chip::Category("Print")),
    (N_("Cards & invitations"), Chip::Category("Cards & invitations")),
    (N_("Documents"), Chip::Category("Documents")),
    (N_("Marketing"), Chip::Category("Marketing")),
    (N_("Logos"), Chip::Category("Logos")),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chip {
    All,
    Category(&'static str),
    Tag(&'static str),
}

impl Chip {
    pub fn keeps(self, t: &genha_templates::TemplateInfo) -> bool {
        match self {
            Chip::All => true,
            Chip::Category(c) => t.category == c,
            Chip::Tag(g) => t.tags.contains(&g),
        }
    }
}

pub struct HomeState {
    pub page: Page,
    /// The search field at the top of the home page, and whether its results show.
    pub search: String,
    pub search_open: bool,
    /// Where the results were drawn last frame (clicks outside close them).
    pub(crate) search_rect: Option<egui::Rect>,
    pub modal: Option<Modal>,
    /// My designs: the folder shown, its search and order.
    pub folder: FolderFilter,
    pub designs_query: String,
    pub sort: Sort,
    /// Template filters: on the home page's row and on the Templates page.
    pub home_chip: usize,
    pub templates_chip: usize,
    pub templates_query: String,
    /// The custom size in the size picker.
    pub custom_w: f64,
    pub custom_h: f64,
    /// All quick actions instead of one row.
    pub all_actions: bool,
    /// The name being typed in Settings (kept when the field loses focus).
    pub name_draft: Option<String>,
}

impl Default for HomeState {
    fn default() -> Self {
        HomeState {
            page: Page::Home,
            search: String::new(),
            search_open: false,
            search_rect: None,
            modal: None,
            folder: FolderFilter::All,
            designs_query: String::new(),
            sort: Sort::Edited,
            home_chip: 0,
            templates_chip: 0,
            templates_query: String::new(),
            custom_w: 1080.0,
            custom_h: 1080.0,
            all_actions: false,
            name_draft: None,
        }
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    app.thumbs.begin_frame();
    egui::Panel::left("home_rail")
        .exact_size(RAIL_W)
        .resizable(false)
        .frame(egui::Frame::new().fill(t.panel).inner_margin(egui::Margin::symmetric(6, 12)).stroke(egui::Stroke::new(1.0, t.border)))
        .show(ui, |ui| rail(app, ui, &t));
    egui::Panel::bottom("home_status").exact_size(24.0).frame(egui::Frame::new().fill(t.bg).inner_margin(egui::Margin::symmetric(16, 3))).show(
        ui,
        |ui| {
            if let Some((msg, _)) = &app.status {
                ui.add(egui::Label::new(egui::RichText::new(msg).size(12.0).color(t.text_dim)).truncate());
            }
        },
    );
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.bg)).show(ui, |ui| match app.home.page {
        Page::Home => home_page(app, ui, &t),
        Page::Designs => crate::designs::show(app, ui, &t),
        Page::Templates => crate::pages::templates(app, ui, &t),
        Page::Brand => crate::pages::brand(app, ui, &t),
        Page::Settings => crate::pages::settings(app, ui, &t),
    });
    crate::popups::show(app, ui.ctx(), &t);
}

// ---------- the rail ----------

/// A rail entry: icon over a label; `on` when its page shows.
fn rail_item(app: &mut App, ui: &mut egui::Ui, t: &Tokens, icon: Icon, icon_color: Option<egui::Color32>, label: &str, on: bool) -> egui::Response {
    let w = ui.available_width();
    let text_color = if on { t.accent } else { t.text };
    // The label wraps onto a second line when it must ("Meine Designs"); longer ones are cut.
    let mut job = egui::text::LayoutJob::simple(label.to_string(), egui::FontId::proportional(11.5), text_color, w - 6.0);
    job.wrap.max_rows = 2;
    job.halign = egui::Align::Center;
    let galley = ui.painter().layout_job(job);
    let (rect, r) = ui.allocate_exact_size(egui::vec2(w, 40.0 + galley.size().y), egui::Sense::click());
    let p = ui.painter_at(rect);
    if on {
        p.rect_filled(rect.shrink2(egui::vec2(4.0, 2.0)), 12, t.accent_soft);
    } else if r.hovered() {
        p.rect_filled(rect.shrink2(egui::vec2(4.0, 2.0)), 12, t.panel_alt);
    }
    let fg = if on { t.accent } else { t.text_dim };
    let icon_rect = egui::Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + 20.0), egui::vec2(22.0, 22.0));
    Art::new(&mut app.thumbs, ui.ctx(), app.session.fonts.clone()).icon(&p, icon, icon_rect, icon_color.unwrap_or(fg));
    p.galley(egui::pos2(rect.center().x, rect.top() + 36.0), galley, text_color);
    r.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(label)
}

fn rail(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    ui.spacing_mut().item_spacing.y = 2.0;
    // New design: the round "+".
    let (rect, r) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 70.0), egui::Sense::click());
    let c = egui::pos2(rect.center().x, rect.top() + 24.0);
    let p = ui.painter_at(rect);
    let radius = if r.hovered() { 24.0 } else { 23.0 };
    p.circle_filled(c + egui::vec2(0.0, 2.0), radius + 1.0, t.art_shadow);
    p.circle_filled(c, radius, t.accent);
    Art::new(&mut app.thumbs, ui.ctx(), app.session.fonts.clone()).icon(
        &p,
        Icon::Plus,
        egui::Rect::from_center_size(c, egui::vec2(26.0, 26.0)),
        t.accent_text,
    );
    p.text(egui::pos2(rect.center().x, rect.top() + 60.0), egui::Align2::CENTER_CENTER, tr("Create"), egui::FontId::proportional(11.5), t.text);
    if r.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tr("New design")).clicked() {
        crate::popups::open_size_picker(app, None);
    }
    ui.add_space(6.0);
    for page in [Page::Home, Page::Designs, Page::Templates, Page::Brand] {
        let on = app.home.page == page && (page != Page::Designs || app.home.folder == FolderFilter::All);
        if rail_item(app, ui, t, page.icon(), None, tr(page.label()), on).clicked() {
            go(app, page);
        }
    }
    ui.add_space(8.0);
    ui.separator();
    // Project folders.
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        ui.label(egui::RichText::new(tr("Projects")).size(11.0).color(t.text_dim));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (rect, r) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::click());
            if r.hovered() {
                ui.painter().rect_filled(rect, 6, t.accent_soft);
            }
            Art::new(&mut app.thumbs, ui.ctx(), app.session.fonts.clone()).icon(ui.painter(), Icon::Plus, rect.shrink(3.0), t.accent);
            if r.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tr("New folder")).clicked() {
                app.home.modal = Some(Modal::NewFolder { name: String::new(), color: 0 });
            }
        });
    });
    let settings_h = 60.0;
    let max_h = (ui.available_height() - settings_h).max(0.0);
    let folders = app.library.folders.clone();
    egui::ScrollArea::vertical().id_salt("rail_folders").max_height(max_h).auto_shrink([false, true]).show(ui, |ui| {
        if folders.is_empty() {
            ui.add_space(4.0);
            ui.vertical_centered(|ui| ui.label(egui::RichText::new(tr("No folders yet")).size(10.5).color(t.text_dim)));
        }
        for f in &folders {
            let on = app.home.page == Page::Designs && app.home.folder == FolderFilter::Folder(f.id.clone());
            let color = f.color.as_deref().and_then(genha_color::Color::parse).map(crate::theme::to_egui);
            let r = rail_item(app, ui, t, Icon::Folder, color, &f.name, on);
            if crate::designs::drop_target(app, ui, &r, Some(f.id.clone()), t) {
                continue;
            }
            if r.clicked() {
                app.home.page = Page::Designs;
                app.home.folder = FolderFilter::Folder(f.id.clone());
            }
        }
    });
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
        let on = app.home.page == Page::Settings;
        if rail_item(app, ui, t, Page::Settings.icon(), None, tr(Page::Settings.label()), on).clicked() {
            go(app, Page::Settings);
        }
    });
}

/// Shows `page` (My designs: all designs).
pub fn go(app: &mut App, page: Page) {
    app.home.page = page;
    app.home.search_open = false;
    if page == Page::Designs {
        app.home.folder = FolderFilter::All;
    }
}

// ---------- shared pieces ----------

/// Lays out `f` in a column at most [`CONTENT_MAX`] wide, centred, with page margins.
pub(crate) fn column(ui: &mut egui::Ui, f: impl FnOnce(&mut egui::Ui)) {
    column_max(ui, CONTENT_MAX, f);
}

/// [`column`] at most `max` wide.
pub(crate) fn column_max(ui: &mut egui::Ui, max: f32, f: impl FnOnce(&mut egui::Ui)) {
    let avail = ui.available_width();
    let side = 32.0f32.min(avail * 0.04).max(16.0);
    let width = (avail - 2.0 * side).clamp(200.0, max.max(200.0));
    let pad = ((avail - width) / 2.0).max(0.0);
    ui.horizontal_top(|ui| {
        ui.add_space(pad);
        ui.vertical(|ui| {
            ui.set_width(width);
            f(ui);
        });
    });
}

/// A section title with an optional "See all" link on the right; whether that was clicked.
pub(crate) fn section(ui: &mut egui::Ui, t: &Tokens, title: &str, see_all: Option<&str>) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).size(19.0).strong().color(t.text));
        if let Some(link) = see_all {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let r = ui.add(egui::Label::new(egui::RichText::new(link).color(t.accent).strong()).sense(egui::Sense::click()));
                clicked = r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
            });
        }
    });
    ui.add_space(8.0);
    clicked
}

/// How long ago `then` (seconds since 1970) was: "just now", "6 days ago"…; empty when unknown.
pub fn ago(now: f64, then: u64) -> String {
    if then == 0 {
        return String::new();
    }
    let d = (now.max(0.0) as u64).saturating_sub(then);
    const MIN: u64 = 60;
    const HOUR: u64 = 60 * MIN;
    const DAY: u64 = 24 * HOUR;
    match d {
        d if d < MIN => tr("just now").to_string(),
        d if d < HOUR => tr_n!("{n} minute ago", "{n} minutes ago", d / MIN),
        d if d < DAY => tr_n!("{n} hour ago", "{n} hours ago", d / HOUR),
        d if d < 7 * DAY => tr_n!("{n} day ago", "{n} days ago", d / DAY),
        d if d < 30 * DAY => tr_n!("{n} week ago", "{n} weeks ago", d / (7 * DAY)),
        d if d < 365 * DAY => tr_n!("{n} month ago", "{n} months ago", d / (30 * DAY)),
        d => tr_n!("{n} year ago", "{n} years ago", d / (365 * DAY)),
    }
}

/// The greeting for the hour of the day (0–23).
pub fn greeting(hour: u32, name: &str) -> String {
    let name = name.trim();
    match (hour, name.is_empty()) {
        (5..=11, true) => tr("Good morning").to_string(),
        (12..=17, true) => tr("Good afternoon").to_string(),
        (_, true) => tr("Good evening").to_string(),
        (5..=11, false) => genha_i18n::tr!("Good morning, {name}", name = name),
        (12..=17, false) => genha_i18n::tr!("Good afternoon, {name}", name = name),
        (_, false) => genha_i18n::tr!("Good evening, {name}", name = name),
    }
}

/// The local hour (0–23).
pub(crate) fn local_hour() -> u32 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let tz = jiff::tz::TimeZone::try_system().unwrap_or(jiff::tz::TimeZone::UTC);
        jiff::Timestamp::try_from(std::time::SystemTime::now()).map(|ts| ts.to_zoned(tz).hour().clamp(0, 23) as u32).unwrap_or(12)
    }
    #[cfg(target_arch = "wasm32")]
    {
        crate::web::local_hour()
    }
}

/// Starts a design (`file.new` with `params`), in the folder My designs shows and with the brand
/// palette chosen on the Brand page.
pub(crate) fn create(app: &mut App, params: serde_json::Value, palette: Option<&str>) -> bool {
    let folder = match (&app.home.page, &app.home.folder) {
        (Page::Designs, FolderFilter::Folder(f)) => Some(f.clone()),
        _ => None,
    };
    if app.run("file.new", &params).is_none() {
        return false;
    }
    if let Some(id) = palette {
        app.run("brand.usePalette", &json!({"id": id}));
    }
    file_into(app, folder);
    true
}

/// Puts the active design into `folder` (when it is in My designs).
pub(crate) fn file_into(app: &mut App, folder: Option<String>) {
    let Some(folder) = folder else { return };
    if let Some(id) = app.session.active().and_then(|d| d.library_id.clone()).filter(|id| !id.is_empty()) {
        app.run("design.move", &json!({"ids": [id], "folder": folder}));
    }
}

// ---------- the home page ----------

fn home_page(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    egui::ScrollArea::vertical().id_salt("home_scroll").auto_shrink([false, false]).show(ui, |ui| {
        column(ui, |ui| {
            ui.add_space(18.0);
            crate::popups::search_field(app, ui, t);
            ui.add_space(26.0);
            let hello = greeting(local_hour(), &app.session.user_name);
            ui.label(egui::RichText::new(hello).size(16.0).color(t.text_dim));
            ui.add_space(2.0);
            ui.label(egui::RichText::new(tr("What would you like to start with?")).size(31.0).strong().color(t.text));
            ui.add_space(18.0);
            start_tiles(app, ui, t);
            ui.add_space(30.0);
            recovered(app, ui, t);
            recent(app, ui, t);
            quick_actions(app, ui, t);
            ui.add_space(30.0);
            templates_row(app, ui, t);
            ui.add_space(40.0);
        });
    });
}

/// The start tiles: label, illustration.
const TILES: [(&str, Scene); 5] = [
    (N_("Open a file"), Scene::OpenFile),
    (N_("New design"), Scene::NewDesign),
    (N_("Edit a photo"), Scene::EditPhoto),
    (N_("Create a video"), Scene::Video),
    (N_("Create a presentation"), Scene::Presentation),
];

fn start_tiles(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    let gap = 14.0;
    let avail = ui.available_width();
    let w = ((avail - 4.0 * gap) / 5.0).max(150.0);
    let h = (w * 0.78).clamp(150.0, 196.0);
    let mut clicked = None;
    let mut hovered_files = false;
    egui::ScrollArea::horizontal().id_salt("start_tiles").auto_shrink([false, true]).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (i, (label, scene)) in TILES.iter().enumerate() {
                let (rect, r) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
                let [a, b, ink] = t.tiles[i];
                let p = ui.painter_at(rect.expand(4.0));
                let lift = if r.hovered() { -2.0 } else { 0.0 };
                let tile = rect.translate(egui::vec2(0.0, lift));
                if r.hovered() {
                    p.rect_filled(tile.translate(egui::vec2(0.0, 4.0)), 18, t.art_shadow);
                }
                art::gradient_rect(&p, tile, 18.0, a, b);
                let text = egui::RichText::new(tr(label)).size(15.0).strong().color(t.text);
                let galley = egui::WidgetText::from(text).into_galley(ui, Some(egui::TextWrapMode::Wrap), w - 28.0, egui::TextStyle::Body);
                let label_top = tile.bottom() - 14.0 - galley.size().y;
                let art_rect = egui::Rect::from_min_max(tile.min + egui::vec2(10.0, 10.0), egui::pos2(tile.right() - 10.0, label_top - 6.0));
                let mut painter_art = Art::new(&mut app.thumbs, ui.ctx(), app.session.fonts.clone());
                art::scene(&mut painter_art, &p, t, *scene, art_rect, ink);
                p.galley(egui::pos2(tile.left() + 16.0, label_top), galley, t.text);
                // The open tile is also where files are dropped: it lights up while one is held over the window.
                if *scene == Scene::OpenFile && ui.ctx().input(|i| !i.raw.hovered_files.is_empty()) {
                    hovered_files = true;
                    p.rect_stroke(tile, 18, egui::Stroke::new(2.5, ink), egui::StrokeKind::Inside);
                }
                let hint = match scene {
                    Scene::OpenFile => tr("Drop a file here or click to browse"),
                    Scene::NewDesign => tr("Pick a size or type your own"),
                    Scene::EditPhoto => tr("Open a picture with the picture tools"),
                    Scene::Video => tr("A story, square or wide video with motion"),
                    Scene::Presentation => tr("16:9 slides, blank or from a template"),
                };
                if r.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(hint).clicked() {
                    clicked = Some(*scene);
                }
            }
        });
    });
    if hovered_files {
        ui.ctx().request_repaint();
    }
    match clicked {
        Some(Scene::OpenFile) => open_any(app),
        Some(Scene::NewDesign) => crate::popups::open_size_picker(app, None),
        Some(Scene::EditPhoto) => edit_photo(app),
        Some(Scene::Video) => app.home.modal = Some(Modal::Video),
        Some(Scene::Presentation) => app.home.modal = Some(Modal::Presentation),
        None => {}
    }
}

/// "Open a file": a design, a picture or SVG (a new design from it), or a font (added).
pub(crate) fn open_any(app: &mut App) {
    let fonts = crate::fonts::FONT_FILES;
    #[cfg(target_arch = "wasm32")]
    {
        // In the browser the file arrives a frame later (web.rs).
        let _ = &app;
        crate::web::pick(crate::web::Purpose::Open, &[io::DESIGNS, io::PICTURES, fonts]);
    }
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(path) = io::pick_open(&[(tr("Designs, pictures and fonts"), &ALL_FILES), io::DESIGNS, io::PICTURES, fonts]) {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        if fonts.1.contains(&ext.as_str()) {
            crate::fonts::add_font_file(app, &path);
        } else {
            app.run("file.open", &json!({"path": path.display().to_string()}));
        }
    }
}

/// Every file "Open a file" takes.
#[cfg(not(target_arch = "wasm32"))]
const ALL_FILES: [&str; 13] = ["genha", "genexpress", "png", "jpg", "jpeg", "webp", "gif", "bmp", "svg", "ttf", "otf", "ttc", "otc"];

/// "Edit a photo": a picture opens as a design of its own size with the picture selected, so the
/// picture tools (adjust, filters, background removal, crop) show.
pub(crate) fn edit_photo(app: &mut App) {
    #[cfg(target_arch = "wasm32")]
    {
        // The picture arrives a frame later and is selected then (web.rs).
        let _ = &app;
        crate::web::pick(crate::web::Purpose::Photo, &[io::PICTURES]);
    }
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(path) = io::pick_open(&[io::PICTURES])
        && app.run("file.open", &json!({"path": path.display().to_string()})).is_some()
    {
        select_picture(app);
    }
}

/// Selects the first picture of the active design (after "Edit a photo").
pub(crate) fn select_picture(app: &mut App) {
    let id = app
        .session
        .active()
        .and_then(|d| d.doc.pages.first()?.elements.iter().find(|e| matches!(e.kind, genha_doc::ElementKind::Image(_))).map(|e| e.id.0));
    if let Some(id) = id {
        app.run("select.set", &json!({"ids": [id]}));
        app.left_tab = crate::LeftTab::Media;
    }
}

fn recovered(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    if app.recovered.is_empty() {
        return;
    }
    let mut action: Option<(&str, String)> = None;
    egui::Frame::new().fill(t.accent_soft).corner_radius(14).inner_margin(14).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(egui::RichText::new(tr("Recovered designs")).size(17.0).strong());
        let why = if cfg!(target_arch = "wasm32") {
            tr("These designs had unsaved changes when GenHa was last closed or reloaded.")
        } else {
            tr("GenHa closed unexpectedly last time. These designs had unsaved changes.")
        };
        ui.label(egui::RichText::new(why).color(t.text_dim));
        for (key, title) in &app.recovered {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(crate::shown_name(title)).strong());
                if ui.button(tr("Open")).clicked() {
                    action = Some(("recovery.open", key.clone()));
                }
                if ui.button(tr("Discard")).clicked() {
                    action = Some(("recovery.discard", key.clone()));
                }
            });
        }
    });
    ui.add_space(24.0);
    if let Some((cmd, key)) = action {
        app.recovered.retain(|(k, _)| *k != key);
        app.run(cmd, &json!({"key": key}));
        if cmd == "recovery.open" {
            app.screen = crate::Screen::Editor;
            app.view.fit_pending = true;
        }
    }
}

/// The designs opened or changed last (as `recent.list`).
pub(crate) fn recent_designs(app: &App) -> Vec<DesignEntry> {
    let mut list: Vec<DesignEntry> = app.library.designs.iter().filter(|e| e.recent).cloned().collect();
    list.sort_by_key(|e| std::cmp::Reverse(e.touched()));
    list.truncate(genha_engine::cmd::recent::MAX_RECENT);
    list
}

fn recent(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    let entries = recent_designs(app);
    if entries.is_empty() {
        return;
    }
    if section(ui, t, tr("Recent designs"), Some(tr("See all"))) {
        go(app, Page::Designs);
    }
    let mut acts = vec![];
    egui::ScrollArea::horizontal().id_salt("recent_row").auto_shrink([false, true]).show(ui, |ui| {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            for e in &entries {
                if let Some(a) = crate::designs::design_card(app, ui, t, e, 196.0, true) {
                    acts.push((e.clone(), a));
                }
            }
        });
    });
    for (e, a) in acts {
        crate::designs::act(app, &e, a);
    }
    ui.add_space(30.0);
}

fn quick_actions(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    let link = if app.home.all_actions { tr("Show fewer") } else { tr("See all") };
    if section(ui, t, tr("Quick actions"), Some(link)) {
        app.home.all_actions = !app.home.all_actions;
    }
    let mut clicked = None;
    let (w, h) = (164.0, 138.0);
    let mut card = |app: &mut App, ui: &mut egui::Ui, i: usize, a: &genha_engine::cmd::home::QuickAction| {
        let (rect, r) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
        let p = ui.painter_at(rect.expand(3.0));
        p.rect_filled(rect, 14, t.panel);
        p.rect_stroke(rect, 14, egui::Stroke::new(1.0, if r.hovered() { t.accent } else { t.border }), egui::StrokeKind::Inside);
        let [a0, a1, ink] = t.tiles[i % 5];
        let art_rect = egui::Rect::from_min_size(rect.min + egui::vec2(8.0, 8.0), egui::vec2(w - 16.0, 80.0));
        art::gradient_rect(&p, art_rect, 10.0, a0, a1);
        art::action(&mut Art::new(&mut app.thumbs, ui.ctx(), app.session.fonts.clone()), &p, t, a.id, art_rect.shrink(6.0), ink);
        let text = egui::RichText::new(tr(a.label)).size(13.5).strong().color(t.text);
        let galley = egui::WidgetText::from(text).into_galley(ui, Some(egui::TextWrapMode::Wrap), w - 20.0, egui::TextStyle::Body);
        p.galley(egui::pos2(rect.left() + 10.0, art_rect.bottom() + 8.0), galley, t.text);
        if r.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tr(a.hint)).clicked() {
            clicked = Some(a.id);
        }
    };
    let actions = genha_engine::cmd::home::QUICK_ACTIONS;
    if app.home.all_actions {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(14.0, 14.0);
            for (i, a) in actions.iter().enumerate() {
                card(app, ui, i, a);
            }
        });
    } else {
        egui::ScrollArea::horizontal().id_salt("actions_row").auto_shrink([false, true]).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                for (i, a) in actions.iter().enumerate() {
                    card(app, ui, i, a);
                }
            });
        });
    }
    if let Some(id) = clicked {
        run_quick_action(app, id);
    }
}

/// A template card: thumbnail, name, category.
pub(crate) fn template_card(app: &mut App, ui: &mut egui::Ui, t: &Tokens, tp: &genha_templates::TemplateInfo, width: f32) -> egui::Response {
    let ctx = ui.ctx().clone();
    let fonts = app.session.fonts.clone();
    let id = tp.id;
    let tex = app.thumbs.get(&ctx, &format!("template:{id}"), 0, || {
        let doc = genha_templates::instantiate(id, None)?;
        genha_render::render_thumbnail(&doc, 0, &fonts, 360).ok()
    });
    let box_h = width * 0.82;
    let (rect, r) = ui.allocate_exact_size(egui::vec2(width, box_h + 44.0), egui::Sense::click());
    let p = ui.painter_at(rect.expand(2.0));
    let boxr = egui::Rect::from_min_size(rect.min, egui::vec2(width, box_h));
    p.rect_filled(boxr, 12, t.panel_alt);
    if r.hovered() {
        p.rect_stroke(boxr, 12, egui::Stroke::new(1.5, t.accent), egui::StrokeKind::Inside);
    }
    let fit = crate::thumbs::fit_size(tp.width as f32, tp.height as f32, boxr.size() - egui::vec2(20.0, 20.0));
    let img = egui::Rect::from_center_size(boxr.center(), fit);
    match &tex {
        Some(tex) => {
            p.rect_filled(img.translate(egui::vec2(0.0, 2.0)), 3, t.art_shadow);
            p.image(tex.id(), img, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        }
        None => {
            p.rect_filled(img, 3, t.border);
        }
    }
    let name = egui::WidgetText::from(egui::RichText::new(tp.name).size(13.0).strong().color(t.text)).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        width,
        egui::TextStyle::Body,
    );
    p.galley(egui::pos2(rect.left() + 2.0, boxr.bottom() + 6.0), name, t.text);
    p.text(
        egui::pos2(rect.left() + 2.0, boxr.bottom() + 26.0),
        egui::Align2::LEFT_TOP,
        tr(tp.category),
        egui::FontId::proportional(11.5),
        t.text_dim,
    );
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Template filter chips; the one chosen.
pub(crate) fn chips(ui: &mut egui::Ui, t: &Tokens, chosen: &mut usize) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
        for (i, (label, _)) in TEMPLATE_CHIPS.iter().enumerate() {
            let on = *chosen == i;
            let text = egui::RichText::new(tr(label)).size(13.0).color(if on { t.accent_text } else { t.text });
            let b = egui::Button::new(text)
                .fill(if on { t.accent } else { t.panel })
                .stroke(egui::Stroke::new(1.0, if on { t.accent } else { t.border }))
                .corner_radius(14)
                .min_size(egui::vec2(0.0, 28.0));
            if ui.add(b).clicked() {
                *chosen = i;
            }
        }
    });
}

fn templates_row(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    if section(ui, t, tr("Templates"), Some(tr("See all"))) {
        app.home.templates_chip = app.home.home_chip;
        go(app, Page::Templates);
    }
    chips(ui, t, &mut app.home.home_chip);
    ui.add_space(10.0);
    let chip = TEMPLATE_CHIPS.get(app.home.home_chip).map_or(Chip::All, |c| c.1);
    let list: Vec<_> = genha_templates::catalog().into_iter().filter(|tp| chip.keeps(tp)).take(24).collect();
    let mut chosen = None;
    egui::ScrollArea::horizontal().id_salt("templates_row").auto_shrink([false, true]).show(ui, |ui| {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            for tp in &list {
                if template_card(app, ui, t, tp, 176.0).clicked() {
                    chosen = Some(tp.id);
                }
            }
        });
    });
    if let Some(id) = chosen {
        use_template(app, id);
    }
}

/// Opens template `id` as a new design (in the folder My designs shows).
pub(crate) fn use_template(app: &mut App, id: &str) {
    let folder = match (&app.home.page, &app.home.folder) {
        (Page::Designs, FolderFilter::Folder(f)) => Some(f.clone()),
        _ => None,
    };
    if app.run("template.use", &json!({"id": id})).is_some() {
        file_into(app, folder);
    }
}

/// Runs quick action `action`: a picture action asks for a picture and where to save the result
/// (the browser downloads it); `qr-code` and `collage` start a design.
pub fn run_quick_action(app: &mut App, action: &str) {
    match action {
        "qr-code" => {
            app.home.modal = Some(Modal::Qr { text: String::new() });
            return;
        }
        "collage" => {
            collage(app);
            return;
        }
        _ => {}
    }
    let (ext, params) = match action {
        "remove-background" => ("png", json!({"action": "remove-background"})),
        "resize" => ("png", json!({"action": "resize", "scale": 0.5})),
        "convert-png" => ("png", json!({"action": "convert"})),
        "convert-jpeg" => ("jpg", json!({"action": "convert"})),
        "convert-webp" => ("webp", json!({"action": "convert"})),
        "auto-crop" => ("png", json!({"action": "auto-crop"})),
        "blur-background" => ("png", json!({"action": "blur-background"})),
        "change-background" => ("png", json!({"action": "change-background", "color": "#ffffff"})),
        "pixelate" => ("png", json!({"action": "pixelate"})),
        _ => ("png", json!({"action": "filter", "preset": "mono"})),
    };
    // In the browser the picture arrives later and the result is downloaded (web.rs).
    #[cfg(target_arch = "wasm32")]
    crate::web::pick(crate::web::Purpose::Quick { action: action.to_string(), ext, params: params.clone() }, &[io::PICTURES]);
    #[cfg(not(target_arch = "wasm32"))]
    let Some(input) = io::pick_open(&[io::PICTURES]) else { return };
    #[cfg(target_arch = "wasm32")]
    let Some(input) = None::<std::path::PathBuf> else { return };
    let stem = input.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "picture".into());
    let Some(output) = io::pick_save(&format!("{stem}-{action}.{ext}"), (ext, &[ext])) else { return };
    let mut p = params;
    if let Some(o) = p.as_object_mut() {
        o.insert("input".into(), json!(input.display().to_string()));
        o.insert("output".into(), json!(output.display().to_string()));
    }
    if app.run("quick.image", &p).is_some() {
        app.set_status(genha_i18n::tr!("Saved {path}", path = output.display()));
    }
}

/// "Photo collage": pictures go into a new square design and are arranged in a grid (more can be
/// added in the editor: select them and pick a layout under Elements › Collage).
fn collage(app: &mut App) {
    #[cfg(target_arch = "wasm32")]
    {
        if create(app, json!({"preset": "instagram-post", "title": tr("Photo collage")}), None) {
            crate::web::pick(crate::web::Purpose::Collage, &[io::PICTURES]);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let Some(paths) = io::pick_many(&[io::PICTURES]) else { return };
        if paths.is_empty() || !create(app, json!({"preset": "instagram-post", "title": tr("Photo collage")}), None) {
            return;
        }
        for path in paths.iter().take(9) {
            app.run("image.place", &json!({"path": path.display().to_string()}));
        }
        arrange_collage(app);
    }
}

/// Arranges every picture on the page in the first collage layout for their number.
pub(crate) fn arrange_collage(app: &mut App) {
    let ids: Vec<u64> = app
        .session
        .active()
        .and_then(|d| d.doc.pages.get(d.page))
        .map(|p| p.elements.iter().filter(|e| matches!(e.kind, genha_doc::ElementKind::Image(_))).map(|e| e.id.0).collect())
        .unwrap_or_default();
    let Some(layout) = genha_layout::collage_layouts(ids.len()).first().map(|l| l.id) else { return };
    if app.run("select.set", &json!({"ids": ids})).is_some() {
        app.run("layout.collage", &json!({"layout": layout}));
    }
}
