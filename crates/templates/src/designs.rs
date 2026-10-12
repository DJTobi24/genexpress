//! The template catalogue: original designs built only from shapes, gradients and text.
//!
//! Every template is drawn with four role colours (primary, secondary, dark, light) and two fonts
//! (heading, body), see [`Theme`]; tints are mixed from those, so applying a brand kit re-colours
//! the whole design consistently.

use genexpress_color::Color;
use genexpress_doc::{BrandKit, Document, Fill, ShapeKind, TextEffect, preset};

use crate::build::{B, Sty, Theme};
use crate::shapes::library_shape;

/// A template in the catalogue.
#[derive(Clone, Debug, PartialEq)]
pub struct TemplateInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    /// The page-size preset it is designed for (see `genexpress_doc::preset`).
    pub preset: &'static str,
    /// Page size in pixels (the preset's, turned to landscape for landscape designs).
    pub width: f64,
    pub height: f64,
    pub tags: &'static [&'static str],
}

struct Def {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    preset: &'static str,
    landscape: bool,
    tags: &'static [&'static str],
    theme: fn() -> Theme,
    build: fn(&mut B),
}

const DEFS: &[Def] = &[
    Def {
        id: "quote-card",
        name: "Quiet quote card",
        category: "Social",
        preset: "instagram-post",
        landscape: false,
        tags: &["quote", "instagram", "minimal", "square"],
        theme: || Theme::new(0x2f5d50, 0xe8b04b, 0x1d2b26, 0xf4efe6, "Serif", "Sans"),
        build: quote_card,
    },
    Def {
        id: "sale-burst",
        name: "Weekend sale burst",
        category: "Social",
        preset: "instagram-post",
        landscape: false,
        tags: &["sale", "discount", "instagram", "shop", "bold"],
        theme: || Theme::new(0xe63946, 0xffd166, 0x1d1d2c, 0xfff8ec, "Display", "Sans"),
        build: sale_burst,
    },
    Def {
        id: "event-night",
        name: "Design night event",
        category: "Social",
        preset: "instagram-post",
        landscape: false,
        tags: &["event", "announcement", "instagram", "dark", "tickets"],
        theme: || Theme::new(0x3a0ca3, 0xf72585, 0x10002b, 0xf8f7ff, "Display", "Sans"),
        build: event_night,
    },
    Def {
        id: "story-promo",
        name: "Fresh drops story",
        category: "Social",
        preset: "instagram-story",
        landscape: false,
        tags: &["story", "promo", "instagram", "shop", "vertical"],
        theme: || Theme::new(0xff7a59, 0xffe066, 0x2b2d42, 0xfffaf3, "Display", "Sans"),
        build: story_promo,
    },
    Def {
        id: "video-thumbnail",
        name: "Ten tips thumbnail",
        category: "Social",
        preset: "youtube-thumbnail",
        landscape: false,
        tags: &["youtube", "thumbnail", "video", "bold"],
        theme: || Theme::new(0xffcc00, 0xff3b30, 0x111111, 0xffffff, "Display", "Sans"),
        build: video_thumbnail,
    },
    Def {
        id: "workshop-post",
        name: "Garden workshop post",
        category: "Social",
        preset: "facebook-post",
        landscape: false,
        tags: &["facebook", "community", "workshop", "event"],
        theme: || Theme::new(0x0f766e, 0xf59e0b, 0x0b1f1e, 0xf0fdfa, "Sans", "Sans"),
        build: workshop_post,
    },
    Def {
        id: "profile-banner",
        name: "Professional profile banner",
        category: "Social",
        preset: "linkedin-banner",
        landscape: false,
        tags: &["linkedin", "banner", "profile", "business"],
        theme: || Theme::new(0x1e3a8a, 0x38bdf8, 0x0f172a, 0xf8fafc, "Sans", "Sans"),
        build: profile_banner,
    },
    Def {
        id: "habits-pin",
        name: "Cozy habits pin",
        category: "Social",
        preset: "pinterest-pin",
        landscape: false,
        tags: &["pinterest", "list", "tips", "home"],
        theme: || Theme::new(0x3a5a40, 0xdda15e, 0x283618, 0xfefae0, "Serif", "Sans"),
        build: habits_pin,
    },
    Def {
        id: "studio-flyer",
        name: "Open studio flyer",
        category: "Print",
        preset: "flyer-a4",
        landscape: false,
        tags: &["flyer", "event", "print", "a4"],
        theme: || Theme::new(0x6d28d9, 0xfacc15, 0x1e1b4b, 0xfaf5ff, "Display", "Sans"),
        build: studio_flyer,
    },
    Def {
        id: "festival-poster",
        name: "Sound waves poster",
        category: "Print",
        preset: "poster-a3",
        landscape: false,
        tags: &["poster", "music", "festival", "geometric"],
        theme: || Theme::new(0xef476f, 0x06d6a0, 0x073b4c, 0xfffcf2, "Display", "Sans"),
        build: festival_poster,
    },
    Def {
        id: "business-card",
        name: "Clean business card",
        category: "Print",
        preset: "business-card",
        landscape: false,
        tags: &["business card", "contact", "print", "minimal"],
        theme: || Theme::new(0x14213d, 0xfca311, 0x0b132b, 0xffffff, "Sans", "Sans"),
        build: business_card,
    },
    Def {
        id: "birthday-invitation",
        name: "Confetti birthday invitation",
        category: "Print",
        preset: "invitation-5x7",
        landscape: false,
        tags: &["invitation", "birthday", "party", "card"],
        theme: || Theme::new(0xff8fab, 0xffc857, 0x3d2c3e, 0xfff5f7, "Serif", "Sans"),
        build: birthday_invitation,
    },
    Def {
        id: "bistro-menu",
        name: "Bistro menu",
        category: "Print",
        preset: "flyer-a4",
        landscape: false,
        tags: &["menu", "restaurant", "food", "print"],
        theme: || Theme::new(0x9a3412, 0x65a30d, 0x292524, 0xfefae0, "Serif", "Sans"),
        build: bistro_menu,
    },
    Def {
        id: "achievement-certificate",
        name: "Achievement certificate",
        category: "Print",
        preset: "document-a4",
        landscape: true,
        tags: &["certificate", "award", "school", "landscape"],
        theme: || Theme::new(0x1f4e79, 0xc9a227, 0x1b1b1b, 0xfffdf7, "Serif", "Sans"),
        build: certificate,
    },
    Def {
        id: "team-presentation",
        name: "Quarterly review deck",
        category: "Documents",
        preset: "presentation",
        landscape: false,
        tags: &["presentation", "slides", "business", "report"],
        theme: || Theme::new(0x2563eb, 0xf97316, 0x0f172a, 0xf8fafc, "Sans", "Sans"),
        build: presentation,
    },
    Def {
        id: "modern-resume",
        name: "Modern resume",
        category: "Documents",
        preset: "resume",
        landscape: false,
        tags: &["resume", "cv", "job", "a4"],
        theme: || Theme::new(0x334155, 0x0ea5e9, 0x0f172a, 0xffffff, "Sans", "Sans"),
        build: resume,
    },
    Def {
        id: "badge-logo",
        name: "Roastery badge logo",
        category: "Marketing",
        preset: "logo",
        landscape: false,
        tags: &["logo", "badge", "brand", "coffee"],
        theme: || Theme::new(0xb45309, 0xfde68a, 0x422006, 0xfffbeb, "Serif", "Sans"),
        build: badge_logo,
    },
];

fn size(d: &Def) -> (f64, f64) {
    let (w, h) = preset::find(d.preset).map_or((1080.0, 1080.0), |p| (p.width, p.height));
    if d.landscape { (w.max(h), w.min(h)) } else { (w, h) }
}

/// Every template, grouped by category.
pub fn catalog() -> Vec<TemplateInfo> {
    DEFS.iter()
        .map(|d| {
            let (width, height) = size(d);
            TemplateInfo { id: d.id, name: d.name, category: d.category, preset: d.preset, width, height, tags: d.tags }
        })
        .collect()
}

/// A new document from template `id`. With a brand kit, the template's primary, secondary, dark
/// and light colours become the kit's colours 0 to 3, its fonts the kit's heading and body fonts,
/// and the kit becomes the document's brand.
pub fn instantiate(id: &str, brand: Option<&BrandKit>) -> Option<Document> {
    let def = DEFS.iter().find(|d| d.id == id)?;
    let (w, h) = size(def);
    let theme = (def.theme)().branded(brand);
    let mut b = B::new(def.name, w, h, theme);
    (def.build)(&mut b);
    let mut doc = b.finish();
    if let Some(kit) = brand {
        doc.brand = kit.clone();
    }
    Some(doc)
}

fn solid(c: Color) -> Fill {
    Fill::solid(c)
}

fn lin(angle: f64, a: Color, b: Color) -> Fill {
    Fill::linear(angle, a, b)
}

// ---------------------------------------------------------------- social

fn quote_card(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.outline("Frame", ShapeKind::Rect { corner_radius: 0.0 }, 60.0, 60.0, 960.0, 960.0, t.primary, 4.0);
    b.ctext("Quote mark", 150.0, 200.0, "“", t.h(220.0, t.secondary).lh(1.0));
    b.ctext("Quote", 400.0, 840.0, "Small steps every day\nadd up to big changes.", t.h(64.0, t.dark).italic().lh(1.25).with(|s| s.bold = false));
    b.rect("Divider", 500.0, 620.0, 80.0, 6.0, 3.0, solid(t.secondary));
    b.ctext("Author", 660.0, 500.0, "— Your Name", t.b(34.0, t.primary).upper().ls(120.0).bold());
    b.ctext("Handle", 880.0, 400.0, "@yourhandle", t.b(28.0, t.primary.lerp(t.light, 0.3)));
}

fn sale_burst(b: &mut B) {
    let t = b.t.clone();
    b.bg(lin(135.0, t.primary, t.primary.lerp(t.dark, 0.35)));
    b.shape("Burst", ShapeKind::Star { points: 16, inner_ratio: 0.84 }, 200.0, 130.0, 680.0, 680.0, solid(t.secondary));
    b.ctext("Kicker", 290.0, 520.0, "Summer", t.h(64.0, t.dark).upper().ls(200.0));
    b.ctext("Discount", 372.0, 560.0, "50% OFF", t.h(120.0, t.dark).lh(1.0));
    b.ctext("Subline", 525.0, 480.0, "On everything in store", t.b(36.0, t.dark));
    b.rect("Dates pill", 340.0, 880.0, 400.0, 80.0, 40.0, solid(t.dark));
    b.ctext("Dates", 899.0, 380.0, "This weekend only", t.b(34.0, t.light).bold().lh(1.15));
    b.ctext("Website", 985.0, 400.0, "yourshop.com", t.b(28.0, t.light));
}

fn event_night(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.dark));
    b.shape(
        "Glow",
        ShapeKind::Ellipse,
        140.0,
        140.0,
        900.0,
        900.0,
        Fill::Radial {
            stops: vec![genexpress_doc::GradientStop { offset: 0.0, color: t.primary }, genexpress_doc::GradientStop { offset: 1.0, color: t.dark }],
        },
    );
    b.rect("Accent bar", 80.0, 120.0, 12.0, 300.0, 6.0, solid(t.secondary));
    b.text("Kicker", 120.0, 120.0, 500.0, "Live event", t.b(36.0, t.secondary).bold().upper().ls(300.0).left());
    b.text("Headline", 120.0, 180.0, 840.0, "Design\nNight 2026", t.h(120.0, t.light).lh(1.05).left());
    b.text(
        "Details",
        120.0,
        520.0,
        840.0,
        "Friday, 14 November · 7 pm\nThe Warehouse, Main Street",
        t.b(40.0, t.light.lerp(t.primary, 0.25)).lh(1.4).left(),
    );
    b.rect("Ticket button", 120.0, 840.0, 360.0, 96.0, 48.0, solid(t.secondary));
    b.text("Ticket label", 140.0, 865.0, 320.0, "Get tickets", t.b(40.0, t.light).bold().lh(1.15));
    b.shape("Sparkle", ShapeKind::Star { points: 4, inner_ratio: 0.3 }, 860.0, 780.0, 120.0, 120.0, solid(t.secondary));
    b.shape("Small sparkle", ShapeKind::Star { points: 4, inner_ratio: 0.3 }, 790.0, 700.0, 50.0, 50.0, solid(t.light));
}

fn story_promo(b: &mut B) {
    let t = b.t.clone();
    b.bg(lin(90.0, t.light, t.secondary.lerp(t.light, 0.5)));
    b.ellipse("Sun", 220.0, 180.0, 640.0, 640.0, lin(90.0, t.secondary, t.primary));
    b.ellipse("Sun ring", 300.0, 260.0, 480.0, 480.0, Fill::None);
    if let Some(e) = b.last()
        && let genexpress_doc::ElementKind::Shape(s) = &mut e.kind
    {
        s.stroke = Some(genexpress_doc::StrokeStyle { color: t.light.with_alpha(160), width: 6.0, dash: vec![18.0, 14.0] });
    }
    b.ctext("Kicker", 900.0, 900.0, "New collection", t.b(44.0, t.dark).bold().upper().ls(250.0));
    b.ctext("Headline", 970.0, 920.0, "Fresh Drops\nEvery Friday", t.h(110.0, t.dark).lh(1.05));
    b.rect("Ground", 0.0, 1320.0, 1080.0, 600.0, 0.0, solid(t.dark));
    b.ctext("Body", 1420.0, 900.0, "Swipe up to shop the full range", t.b(44.0, t.light));
    b.rect("Button", 290.0, 1560.0, 500.0, 120.0, 60.0, solid(t.primary));
    b.ctext("Button label", 1590.0, 460.0, "Shop now", t.b(48.0, t.dark).bold().lh(1.15));
    if let Some(arrow) = library_shape("arrow-up") {
        b.shape("Swipe arrow", arrow, 500.0, 1750.0, 80.0, 80.0, solid(t.light));
    }
}

fn video_thumbnail(b: &mut B) {
    let t = b.t.clone();
    b.bg(lin(0.0, t.dark, t.dark.lerp(t.secondary, 0.35)));
    b.shape(
        "Slanted panel",
        ShapeKind::Path { d: "M140 0 H520 V720 H0 Z".into(), view_width: 520.0, view_height: 720.0 },
        760.0,
        0.0,
        520.0,
        720.0,
        solid(t.primary),
    );
    b.ellipse("Number disc", 880.0, 160.0, 300.0, 300.0, solid(t.dark));
    b.text("Number", 900.0, 195.0, 260.0, "10", t.h(150.0, t.primary).lh(1.0));
    b.text("Number label", 910.0, 355.0, 240.0, "Tips", t.b(48.0, t.light).bold().upper().ls(200.0));
    b.text("Headline", 60.0, 110.0, 640.0, "Build a\nbetter\nmorning", t.h(110.0, t.light).upper().lh(1.0).left());
    b.effect_last(TextEffect::Shadow { color: Color::rgba(0, 0, 0, 150), offset_x: 6.0, offset_y: 6.0, blur: 0.0 });
    b.rect("Tag", 60.0, 520.0, 380.0, 80.0, 16.0, solid(t.secondary));
    b.text("Tag label", 80.0, 537.0, 340.0, "Watch now", t.b(40.0, t.light).bold().upper().ls(100.0).lh(1.15));
    b.text("Channel", 60.0, 635.0, 400.0, "@yourchannel", t.b(30.0, t.light.lerp(t.dark, 0.3)).left());
}

fn workshop_post(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.rect("Colour block", 0.0, 0.0, 520.0, 630.0, 0.0, lin(90.0, t.primary, t.primary.lerp(t.dark, 0.4)));
    b.ellipse("Sun dot", 60.0, 60.0, 160.0, 160.0, solid(t.secondary));
    b.outline("Ring", ShapeKind::Ellipse, 300.0, 380.0, 180.0, 180.0, t.light, 6.0);
    b.ellipse("Small dot", 230.0, 250.0, 40.0, 40.0, solid(t.light.with_alpha(180)));
    b.text("Kicker", 580.0, 150.0, 560.0, "Community", t.b(28.0, t.primary).bold().upper().ls(250.0).left());
    b.text("Headline", 580.0, 200.0, 580.0, "Join our weekend\ngarden workshop", t.h(58.0, t.dark).lh(1.15).left());
    b.text("Body", 580.0, 370.0, 580.0, "Learn to grow herbs at home.\nFree for members.", t.b(28.0, t.dark.lerp(t.light, 0.25)).lh(1.4).left());
    b.rect("Sign-up pill", 580.0, 490.0, 300.0, 70.0, 35.0, solid(t.secondary));
    b.text("Sign-up label", 600.0, 506.0, 260.0, "Sign up free", t.b(30.0, t.dark).bold().lh(1.15));
}

fn profile_banner(b: &mut B) {
    let t = b.t.clone();
    b.bg(lin(0.0, t.dark, t.primary));
    let stripe = || ShapeKind::Path { d: "M80 0 H140 L60 396 H0 Z".into(), view_width: 140.0, view_height: 396.0 };
    b.shape("Stripe 1", stripe(), 1300.0, 0.0, 140.0, 396.0, solid(t.secondary.with_alpha(210)));
    b.shape("Stripe 2", stripe(), 1372.0, 0.0, 140.0, 396.0, solid(t.secondary.with_alpha(140)));
    b.shape("Stripe 3", stripe(), 1444.0, 0.0, 140.0, 396.0, solid(t.secondary.with_alpha(70)));
    b.text("Name", 560.0, 100.0, 700.0, "Alex Morgan", t.h(72.0, t.light).left());
    b.text("Role", 560.0, 196.0, 720.0, "Product Designer · Speaker · Mentor", t.b(34.0, t.secondary).left());
    b.rect("Accent line", 560.0, 268.0, 120.0, 6.0, 3.0, solid(t.secondary));
    b.text("Website", 560.0, 292.0, 500.0, "alexmorgan.design", t.b(28.0, t.light.lerp(t.dark, 0.25)).left());
}

fn habits_pin(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.rect("Header panel", 0.0, 0.0, 1000.0, 620.0, 0.0, lin(90.0, t.primary, t.primary.lerp(t.dark, 0.4)));
    b.ellipse("Accent circle", 700.0, 330.0, 240.0, 240.0, solid(t.secondary));
    b.text("Kicker", 80.0, 130.0, 560.0, "5 easy habits", t.b(34.0, t.secondary).bold().upper().ls(300.0).left());
    b.text("Headline", 80.0, 200.0, 700.0, "For a calmer,\ncozier home", t.h(96.0, t.light).left());
    let items = [
        ("Open the windows every morning", "Fresh air resets the whole room"),
        ("Keep one surface clear", "A tidy table feels like a deep breath"),
        ("Add something green", "Plants soften corners and lift moods"),
        ("Layer warm lighting", "Swap one ceiling light for two lamps"),
        ("Make the bed first", "A small win to start the day"),
    ];
    for (i, (title, note)) in items.iter().enumerate() {
        let y = 700.0 + 140.0 * i as f64;
        let n = i + 1;
        b.ellipse(&format!("Number {n} disc"), 80.0, y, 80.0, 80.0, solid(if i % 2 == 0 { t.secondary } else { t.primary.lerp(t.light, 0.7) }));
        b.text(&format!("Number {n}"), 80.0, y + 17.0, 80.0, &n.to_string(), t.b(36.0, t.dark).bold().lh(1.15));
        b.text(&format!("Habit {n}"), 190.0, y + 2.0, 720.0, title, t.h(40.0, t.dark).left());
        b.text(&format!("Habit {n} note"), 190.0, y + 52.0, 720.0, note, t.b(28.0, t.dark.lerp(t.light, 0.35)).left());
    }
    b.ctext("Website", 1410.0, 400.0, "yourblog.com", t.b(28.0, t.primary).bold());
}

// ---------------------------------------------------------------- print

fn studio_flyer(b: &mut B) {
    let t = b.t.clone();
    let w = b.w;
    b.bg(solid(t.light));
    b.rect("Header", 0.0, 0.0, w, 560.0, 0.0, lin(135.0, t.primary, t.primary.lerp(t.secondary, 0.45)));
    b.ellipse("Sun", 540.0, 300.0, 200.0, 200.0, solid(t.secondary));
    b.shape("Star", ShapeKind::Star { points: 5, inner_ratio: 0.45 }, 640.0, 70.0, 90.0, 90.0, solid(t.light.with_alpha(200)));
    b.text("Kicker", 60.0, 120.0, 500.0, "Spring showcase", t.b(26.0, t.light).bold().upper().ls(300.0).left());
    b.text("Headline", 60.0, 170.0, 640.0, "Open Studio\nWeekend", t.h(84.0, t.light).lh(1.05).left());
    b.text("Dates", 60.0, 610.0, 670.0, "Sat 12 – Sun 13 April", t.b(32.0, t.dark).bold().upper().ls(50.0).left());
    b.text(
        "Description",
        60.0,
        668.0,
        670.0,
        "Meet local makers, try live demos and take home\nsomething handmade. Free entry, all ages welcome.",
        t.b(22.0, t.dark.lerp(t.light, 0.2)).lh(1.5).left(),
    );
    let cards = [("When", "10 am – 6 pm"), ("Where", "Riverside Hall"), ("Price", "Free entry")];
    for (i, (label, value)) in cards.iter().enumerate() {
        let x = 60.0 + i as f64 * 236.85;
        b.rect(&format!("{label} card"), x, 790.0, 200.0, 150.0, 18.0, solid(t.primary.lerp(t.light, 0.88)));
        b.text(&format!("{label} label"), x + 15.0, 820.0, 170.0, label, t.b(18.0, t.primary).bold().upper().ls(200.0));
        b.text(&format!("{label} value"), x + 10.0, 860.0, 180.0, value, t.b(22.0, t.dark).bold());
    }
    b.rect("Footer", 0.0, 1022.5, w, 100.0, 0.0, solid(t.dark));
    b.ctext("Website", 1055.0, 500.0, "www.yourstudio.com", t.b(24.0, t.light));
}

fn festival_poster(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.dark));
    b.outline("Outer ring", ShapeKind::Ellipse, 111.0, 130.0, 900.0, 900.0, t.light, 4.0);
    b.ellipse("Big circle", 161.0, 180.0, 800.0, 800.0, solid(t.primary));
    b.ellipse("Inner circle", 361.0, 380.0, 400.0, 400.0, solid(t.secondary));
    b.shape("Triangle", ShapeKind::Triangle, 820.0, 760.0, 260.0, 225.0, solid(t.light));
    b.ctext("Headline", 1030.0, 900.0, "Sound\nWaves", t.h(200.0, t.light).upper().lh(0.95));
    b.ctext("Lineup", 1430.0, 900.0, "Live music · Food trucks · Late night", t.b(34.0, t.light.lerp(t.secondary, 0.3)));
    b.ctext("Dates", 1490.0, 600.0, "August 22–24", t.b(40.0, t.secondary).bold().upper().ls(200.0));
}

fn business_card(b: &mut B) {
    let t = b.t.clone();
    let h = b.h;
    b.bg(solid(t.light));
    b.rect("Side band", 0.0, 0.0, 96.0, h, 0.0, solid(t.primary));
    let disc = b.el("Logo disc", genexpress_geom::Frame::new(24.0, 24.0, 48.0, 48.0), crate::build::shape_el(ShapeKind::Ellipse, solid(t.secondary)));
    let letter = b.text_el("Logo letter", 24.0, 31.0, 48.0, "A", t.h(28.0, t.primary).lh(1.0));
    b.group("Logo", vec![disc, letter]);
    b.text("Name", 116.0, 40.0, 190.0, "Alex Morgan", t.h(20.0, t.dark).left());
    b.text("Role", 116.0, 68.0, 190.0, "Creative Director", t.b(12.0, t.dark.lerp(t.light, 0.35)).left());
    b.rect("Accent line", 116.0, 96.0, 36.0, 3.0, 1.5, solid(t.secondary));
    b.text("Contact", 116.0, 112.0, 190.0, "+1 555 0100 200\nalex@studio.example\nstudio.example", t.b(11.0, t.dark).lh(1.5).left());
}

fn birthday_invitation(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.outline("Border", ShapeKind::Rect { corner_radius: 16.0 }, 24.0, 24.0, 432.0, 624.0, t.primary, 3.0);
    // name, shape, x, y, w, h, primary colour?, rotation
    type Confetti = (&'static str, ShapeKind, f64, f64, f64, f64, bool, f64);
    let confetti: [Confetti; 8] = [
        ("Confetti dot", ShapeKind::Ellipse, 60.0, 60.0, 18.0, 18.0, true, 0.0),
        ("Confetti dot", ShapeKind::Ellipse, 400.0, 80.0, 14.0, 14.0, false, 0.0),
        ("Confetti triangle", ShapeKind::Triangle, 90.0, 150.0, 20.0, 18.0, false, 20.0),
        ("Confetti star", ShapeKind::Star { points: 5, inner_ratio: 0.45 }, 380.0, 170.0, 24.0, 24.0, true, 0.0),
        ("Confetti dot", ShapeKind::Ellipse, 70.0, 580.0, 16.0, 16.0, false, 0.0),
        ("Confetti triangle", ShapeKind::Triangle, 392.0, 590.0, 20.0, 18.0, true, -15.0),
        ("Confetti star", ShapeKind::Star { points: 5, inner_ratio: 0.45 }, 60.0, 400.0, 18.0, 18.0, false, 0.0),
        ("Confetti dot", ShapeKind::Ellipse, 410.0, 420.0, 12.0, 12.0, true, 0.0),
    ];
    for (name, shape, x, y, w, h, primary, rot) in confetti {
        b.shape(name, shape, x, y, w, h, solid(if primary { t.primary } else { t.secondary }));
        b.rotate_last(rot);
    }
    b.ellipse("Balloon", 196.0, 64.0, 88.0, 106.0, lin(90.0, t.primary.lerp(t.light, 0.25), t.primary));
    b.shape("Balloon knot", ShapeKind::Triangle, 232.0, 168.0, 16.0, 12.0, solid(t.primary));
    b.rect("Balloon string", 239.0, 180.0, 2.0, 56.0, 1.0, solid(t.dark.lerp(t.light, 0.5)));
    b.ctext("Kicker", 256.0, 360.0, "You're invited to", t.b(16.0, t.dark).bold().upper().ls(300.0));
    b.ctext("Headline", 288.0, 400.0, "Mia's 30th\nBirthday", t.h(60.0, t.primary.lerp(t.dark, 0.35)).italic());
    b.rect("Divider", 210.0, 440.0, 60.0, 3.0, 1.5, solid(t.secondary));
    b.ctext("Details", 460.0, 400.0, "Saturday, 21 June · 7 pm\nThe Garden Room, 12 Elm Road", t.b(18.0, t.dark).lh(1.5));
    b.ctext("RSVP", 540.0, 360.0, "RSVP to Sam by 1 June", t.b(16.0, t.primary.lerp(t.dark, 0.35)).bold());
}

fn bistro_menu(b: &mut B) {
    let t = b.t.clone();
    let w = b.w;
    b.bg(solid(t.light));
    b.shape("Leaf left", ShapeKind::Ellipse, 70.0, 110.0, 70.0, 26.0, solid(t.secondary));
    b.rotate_last(-30.0);
    b.shape("Leaf right", ShapeKind::Ellipse, w - 140.0, 110.0, 70.0, 26.0, solid(t.secondary));
    b.rotate_last(30.0);
    b.ctext("Restaurant name", 80.0, 600.0, "Little Olive", t.h(72.0, t.primary).italic());
    b.ctext("Tagline", 175.0, 500.0, "Kitchen & wine bar", t.b(20.0, t.secondary).bold().upper().ls(400.0));
    b.rect("Divider", (w - 100.0) / 2.0, 220.0, 100.0, 3.0, 1.5, solid(t.secondary));
    // dish, price, description
    type Dish = (&'static str, &'static str, &'static str);
    let sections: [(&str, f64, [Dish; 2]); 3] = [
        (
            "Starters",
            270.0,
            [("Warm focaccia", "6", "Rosemary, sea salt, olive oil"), ("Burrata salad", "11", "Heirloom tomatoes, basil, toasted seeds")],
        ),
        (
            "Mains",
            520.0,
            [("Wild mushroom risotto", "18", "Parmesan, thyme, lemon zest"), ("Grilled sea bream", "22", "Fennel, capers, new potatoes")],
        ),
        ("Desserts", 790.0, [("Olive oil cake", "8", "Citrus cream, pistachio"), ("Dark chocolate mousse", "9", "Sea salt, hazelnut crumble")]),
    ];
    for (section, y, items) in sections {
        b.ctext(&format!("{section} heading"), y, 400.0, section, t.b(24.0, t.primary).bold().upper().ls(300.0));
        for (i, (dish, price, desc)) in items.iter().enumerate() {
            let iy = y + 50.0 + 80.0 * i as f64;
            b.text(dish, 100.0, iy, 460.0, dish, t.h(24.0, t.dark).left());
            b.text(&format!("{dish} price"), w - 220.0, iy, 120.0, price, t.b(22.0, t.primary).bold().right());
            b.text(&format!("{dish} description"), 100.0, iy + 32.0, 500.0, desc, t.b(16.0, t.dark.lerp(t.light, 0.35)).left());
        }
    }
    b.rect("Footer", 0.0, 1040.0, w, b.h - 1040.0, 0.0, solid(t.primary));
    b.ctext("Opening hours", 1068.0, 600.0, "Open daily 12 – 11 pm · 4 Harbour Lane", t.b(18.0, t.light));
}

fn certificate(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.outline("Outer border", ShapeKind::Rect { corner_radius: 0.0 }, 30.0, 30.0, w - 60.0, h - 60.0, t.primary, 10.0);
    b.outline("Inner border", ShapeKind::Rect { corner_radius: 0.0 }, 52.0, 52.0, w - 104.0, h - 104.0, t.secondary, 2.0);
    for (name, cx, cy) in [
        ("Corner top left", 52.0, 52.0),
        ("Corner top right", w - 52.0, 52.0),
        ("Corner bottom left", 52.0, h - 52.0),
        ("Corner bottom right", w - 52.0, h - 52.0),
    ] {
        b.rect(name, cx - 16.0, cy - 16.0, 32.0, 32.0, 0.0, solid(t.secondary));
        b.rotate_last(45.0);
    }
    b.ctext("Title", 120.0, 800.0, "Certificate", t.h(72.0, t.primary).upper().ls(150.0));
    b.ctext("Subtitle", 212.0, 600.0, "Of achievement", t.b(24.0, t.secondary).bold().upper().ls(400.0));
    b.ctext("Presented to", 290.0, 700.0, "This certificate is proudly presented to", t.b(22.0, t.dark.lerp(t.light, 0.3)));
    b.ctext("Recipient", 335.0, 800.0, "Jordan Lee", t.h(80.0, t.dark).italic().with(|s| s.bold = false));
    b.rect("Name line", (w - 500.0) / 2.0, 450.0, 500.0, 2.0, 1.0, solid(t.secondary));
    b.ctext("Reason", 478.0, 760.0, "for outstanding dedication and creativity\nin the 2026 Young Designers Programme", t.b(22.0, t.dark).lh(1.5));
    b.shape("Seal", ShapeKind::Star { points: 20, inner_ratio: 0.86 }, (w - 120.0) / 2.0, 570.0, 120.0, 120.0, solid(t.secondary));
    b.ellipse("Seal centre", (w - 80.0) / 2.0, 590.0, 80.0, 80.0, solid(t.primary));
    b.rect("Date line", 160.0, 650.0, 260.0, 2.0, 1.0, solid(t.dark));
    b.text("Date label", 160.0, 662.0, 260.0, "Date", t.b(18.0, t.dark.lerp(t.light, 0.3)));
    b.rect("Signature line", w - 420.0, 650.0, 260.0, 2.0, 1.0, solid(t.dark));
    b.text("Signature label", w - 420.0, 662.0, 260.0, "Signature", t.b(18.0, t.dark.lerp(t.light, 0.3)));
}

// ---------------------------------------------------------------- documents

fn presentation(b: &mut B) {
    let t = b.t.clone();
    b.name_page("Title");
    b.bg(solid(t.dark));
    b.ellipse("Feature circle", 1160.0, 120.0, 680.0, 680.0, lin(135.0, t.primary, t.primary.lerp(t.secondary, 0.6)));
    b.outline("Ring", ShapeKind::Ellipse, 1100.0, 640.0, 260.0, 260.0, t.secondary, 10.0);
    b.text("Kicker", 140.0, 360.0, 900.0, "Quarterly review", t.b(32.0, t.secondary).bold().upper().ls(300.0).left());
    b.text("Title", 140.0, 420.0, 1000.0, "Growing Together\nin 2026", t.h(110.0, t.light).lh(1.05).left());
    b.text("Subtitle", 140.0, 690.0, 1000.0, "Team update · Presented by Alex Morgan", t.b(36.0, t.light.lerp(t.dark, 0.3)).left());
    b.rect("Footer bar", 140.0, 900.0, 120.0, 8.0, 4.0, solid(t.primary));
    b.notes("Welcome everyone. Quick agenda: what worked, what we learned, and next quarter's goals.");

    b.new_page("Highlights");
    b.bg(solid(t.light));
    b.rect("Top bar", 0.0, 0.0, b.w, 16.0, 0.0, solid(t.primary));
    b.text("Title", 140.0, 120.0, 1200.0, "Three things that worked", t.h(72.0, t.dark).left());
    let cards = [
        ("Faster onboarding", "New users reach their\nfirst design in under\ntwo minutes."),
        ("Brand kits", "Teams keep colours\nand fonts consistent\nacross every project."),
        ("Smart resize", "One design becomes\nposts, stories and\nbanners in a click."),
    ];
    for (i, (title, body)) in cards.iter().enumerate() {
        let x = 140.0 + 570.0 * i as f64;
        let n = i + 1;
        b.rect(&format!("Card {n}"), x, 300.0, 500.0, 480.0, 24.0, solid(t.primary.lerp(t.light, 0.9)));
        b.ellipse(&format!("Card {n} disc"), x + 40.0, 344.0, 96.0, 96.0, solid(if i == 1 { t.secondary } else { t.primary }));
        b.text(&format!("Card {n} number"), x + 40.0, 364.0, 96.0, &n.to_string(), t.h(48.0, t.light).lh(1.0));
        b.text(&format!("Card {n} title"), x + 40.0, 488.0, 420.0, title, t.h(44.0, t.dark).left());
        b.text(&format!("Card {n} text"), x + 40.0, 568.0, 420.0, body, t.b(30.0, t.dark.lerp(t.light, 0.25)).lh(1.45).left());
    }
    b.text("Page number", 1680.0, 975.0, 100.0, "02", t.b(28.0, t.primary).bold().right());
    b.notes("Each of these moved a key metric; details in the appendix.");
}

fn resume(b: &mut B) {
    let t = b.t.clone();
    let h = b.h;
    b.bg(solid(t.light));
    b.rect("Sidebar", 0.0, 0.0, 260.0, h, 0.0, solid(t.primary));
    b.ellipse("Photo placeholder", 60.0, 60.0, 140.0, 140.0, solid(t.secondary));
    b.text("Initials", 60.0, 100.0, 140.0, "AM", t.h(48.0, t.light).lh(1.0));
    let side = |b: &mut B, title: &str, y: f64, body: &str| {
        b.text(&format!("{title} heading"), 36.0, y, 190.0, title, t.b(16.0, t.secondary).bold().upper().ls(200.0).left());
        b.text(title, 36.0, y + 30.0, 190.0, body, t.b(15.0, t.light).lh(1.6).left());
    };
    side(b, "Contact", 250.0, "alex@mail.example\n+1 555 0100 200\nSpringfield");
    side(b, "Skills", 390.0, "Brand identity\nTypography\nArt direction\nPrototyping\nMotion basics");
    side(b, "Languages", 590.0, "English (native)\nSpanish (fluent)");

    let x = 300.0;
    b.text("Name", x, 64.0, 450.0, "Alex Morgan", t.h(52.0, t.dark).left());
    b.text("Job title", x, 136.0, 450.0, "Senior Graphic Designer", t.b(24.0, t.secondary).left());
    b.text(
        "Summary",
        x,
        186.0,
        450.0,
        "Designer with ten years of experience turning\nbig ideas into clear, friendly visuals for\nbrands, campaigns and products.",
        t.b(16.0, t.dark.lerp(t.light, 0.2)).lh(1.55).left(),
    );
    let section = |b: &mut B, title: &str, y: f64| {
        b.text(&format!("{title} heading"), x, y, 300.0, title, t.h(18.0, t.primary).upper().ls(200.0).left());
        b.rect(&format!("{title} rule"), x, y + 28.0, 450.0, 2.0, 1.0, solid(t.secondary));
    };
    let entry = |b: &mut B, name: &str, y: f64, role: &str, dates: &str, bullets: &str| {
        b.text(name, x, y, 450.0, role, t.h(18.0, t.dark).left());
        b.text(&format!("{name} dates"), x, y + 27.0, 300.0, dates, t.b(14.0, t.secondary).left());
        if !bullets.is_empty() {
            b.text(&format!("{name} details"), x, y + 51.0, 450.0, bullets, t.b(15.0, t.dark.lerp(t.light, 0.15)).lh(1.5).left());
        }
    };
    section(b, "Experience", 300.0);
    entry(
        b,
        "Job 1",
        345.0,
        "Lead Designer · Brightside Studio",
        "2021 – present",
        "• Led rebrands for 12 clients across retail and tech\n• Built a shared design system used by 30 people",
    );
    entry(
        b,
        "Job 2",
        465.0,
        "Designer · Northwind Agency",
        "2016 – 2021",
        "• Designed campaigns seen by over two million people\n• Mentored four junior designers",
    );
    section(b, "Education", 600.0);
    entry(b, "Degree", 645.0, "BA Visual Communication · City University", "2012 – 2016", "");
    section(b, "Projects", 730.0);
    b.text(
        "Projects",
        x,
        775.0,
        450.0,
        "Open-source icon set with 400 original icons\nBook covers for an independent publisher",
        t.b(15.0, t.dark.lerp(t.light, 0.15)).lh(1.5).left(),
    );
}

// ---------------------------------------------------------------- marketing

fn badge_logo(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.ellipse("Badge", 50.0, 50.0, 400.0, 400.0, solid(t.primary));
    b.outline("Inner ring", ShapeKind::Ellipse, 70.0, 70.0, 360.0, 360.0, t.secondary, 4.0);
    b.ctext("Established", 128.0, 240.0, "Est. 2026", t.b(22.0, t.secondary).bold().upper().ls(300.0));
    b.ctext("Name", 165.0, 300.0, "Copper\n& Co.", t.h(64.0, t.light).lh(1.0));
    b.rect("Ribbon", 60.0, 310.0, 380.0, 60.0, 6.0, solid(t.secondary));
    b.ctext("Ribbon text", 326.0, 340.0, "Coffee roasters", t.b(22.0, t.dark).bold().upper().ls(150.0).lh(1.15));
    for (i, x) in [205.0, 240.0, 275.0].into_iter().enumerate() {
        b.shape(&format!("Star {}", i + 1), ShapeKind::Star { points: 5, inner_ratio: 0.45 }, x, 386.0, 20.0, 20.0, solid(t.secondary));
    }
}
