//! More marketing (email headers, web ads, a landing page, listing and pricing posts, podcast
//! cover art) and logos with curved text.

use genha_doc::{BorderLines, CellRange, ShapeKind, TextAlign};

use super::{Def, LOGOS, MARKETING, def, lin, radial, solid};
use crate::build::{B, Sty, Theme};

pub(super) const DEFS: &[Def] = &[
    def(
        "bakery-circle-logo",
        "Bakery circle badge logo",
        LOGOS,
        "logo",
        &["logo", "badge", "bakery", "circle", "curved text", "food"],
        || Theme::new(0x9a3412, 0xfbbf24, 0x431407, 0xfff7ed, "Serif", "Sans"),
        bakery_logo,
    ),
    def(
        "bike-shop-logo",
        "Bike shop round logo",
        LOGOS,
        "logo",
        &["logo", "bike", "cycling", "shop", "curved text", "round"],
        || Theme::new(0x0369a1, 0xfacc15, 0x0c2d48, 0xf0f9ff, "Display", "Sans"),
        bike_logo,
    ),
    def(
        "lettermark-logo",
        "Hexagon lettermark logo",
        LOGOS,
        "logo",
        &["logo", "lettermark", "initial", "hexagon", "tech", "modern"],
        || Theme::new(0x0f766e, 0xf97316, 0x042f2e, 0xffffff, "Display", "Sans"),
        lettermark_logo,
    ),
    def(
        "team-crest-logo",
        "Football club crest logo",
        LOGOS,
        "logo",
        &["logo", "crest", "sports", "football", "club", "shield"],
        || Theme::new(0x1e40af, 0xdc2626, 0x0b1437, 0xffffff, "Display", "Sans"),
        crest_logo,
    ),
    def(
        "launch-email-header",
        "App launch email header",
        MARKETING,
        "email-header",
        &["email header", "launch", "app", "product", "newsletter"],
        || Theme::new(0xff6b6b, 0x4ecdc4, 0x1a1a2e, 0xffffff, "Display", "Sans"),
        launch_email,
    ),
    def(
        "webinar-leaderboard-ad",
        "Webinar leaderboard ad",
        MARKETING,
        "banner-leaderboard",
        &["web banner", "ad", "leaderboard", "webinar", "728x90", "display ad"],
        || Theme::new(0x2563eb, 0xfde047, 0x0f172a, 0xffffff, "Display", "Sans"),
        webinar_leaderboard,
    ),
    def(
        "app-rectangle-ad",
        "Meal planner app ad",
        MARKETING,
        "banner-rectangle",
        &["web banner", "ad", "app", "download", "300x250", "food"],
        || Theme::new(0x16a34a, 0xfef08a, 0x052e16, 0xffffff, "Display", "Sans"),
        app_ad,
    ),
    def(
        "launch-landing-page",
        "Product launch landing page",
        MARKETING,
        "webpage",
        &["landing page", "website", "launch", "pricing", "web", "product"],
        || Theme::new(0x6d28d9, 0x22d3ee, 0x0f0a1f, 0xfbfaff, "Display", "Sans"),
        landing_page,
    ),
    def(
        "just-listed-post",
        "Just listed home post",
        MARKETING,
        "instagram-post",
        &["real estate", "just listed", "house", "property", "instagram", "realtor"],
        || Theme::new(0x14532d, 0xeab308, 0x0a1f12, 0xfafaf5, "Serif", "Sans"),
        just_listed,
    ),
    def(
        "pricing-plans-post",
        "Pricing plans comparison post",
        MARKETING,
        "instagram-portrait",
        &["pricing", "plans", "comparison", "table", "subscription", "instagram"],
        || Theme::new(0xe11d48, 0xfde68a, 0x1f1d2b, 0xfffafa, "Display", "Sans"),
        pricing_post,
    ),
    def(
        "podcast-cover-art",
        "Podcast cover art",
        MARKETING,
        "instagram-post",
        &["podcast", "cover art", "show", "audio", "square", "branding"],
        || Theme::new(0xff7a00, 0x00b4d8, 0x14110f, 0xfff3e6, "Display", "Sans"),
        podcast_cover,
    ),
];

// ---------------------------------------------------------------- logos

fn bakery_logo(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.ellipse("Badge", 50.0, 50.0, 400.0, 400.0, solid(t.primary));
    b.outline("Inner ring", ShapeKind::Ellipse, 110.0, 110.0, 280.0, 280.0, t.secondary, 3.0);
    b.text("Top arc", 50.0, 50.0, 400.0, "Hearth & Grain", t.h(30.0, t.light).upper().ls(80.0));
    b.curve_last(120.0, 132.0);
    b.text("Bottom arc", 90.0, 345.0, 320.0, "Bakery · est. 2026", t.b(22.0, t.secondary).bold().upper().ls(250.0));
    b.curve_last(-110.0, 94.0);
    // A wheat ear: leaves either side of a stem.
    b.rect("Stem", 247.0, 170.0, 6.0, 170.0, 3.0, solid(t.secondary));
    for i in 0..4 {
        let y = 160.0 + 36.0 * f64::from(i);
        b.icon(&format!("Grain left {}", i + 1), "leaf", 206.0, y, 44.0, 44.0, solid(t.secondary));
        b.rotate_last(-20.0);
        b.icon(&format!("Grain right {}", i + 1), "leaf", 250.0, y, 44.0, 44.0, solid(t.secondary));
        b.rotate_last(70.0);
    }
    for (i, x) in [118.0, 366.0].into_iter().enumerate() {
        b.shape(&format!("Star {}", i + 1), ShapeKind::Star { points: 5, inner_ratio: 0.45 }, x, 238.0, 18.0, 18.0, solid(t.light));
    }
}

fn bike_logo(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.outline("Outer ring", ShapeKind::Ellipse, 40.0, 40.0, 420.0, 420.0, t.primary, 10.0);
    b.ellipse("Disc", 130.0, 150.0, 240.0, 240.0, solid(t.secondary));
    b.icon("Bike", "bike", 160.0, 180.0, 180.0, 180.0, solid(t.dark));
    b.text("Top arc", 50.0, 50.0, 400.0, "Spoke & Chain", t.h(34.0, t.primary).upper().ls(60.0));
    b.curve_last(120.0, 136.0);
    b.text("Bottom arc", 100.0, 378.0, 300.0, "Repairs · Rentals", t.b(20.0, t.dark).bold().upper().ls(250.0));
    b.curve_last(-90.0, 76.0);
}

fn lettermark_logo(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.shape("Hexagon", ShapeKind::Polygon { sides: 6 }, 160.0, 60.0, 180.0, 180.0, lin(135.0, t.primary, t.primary.lerp(t.dark, 0.35)));
    b.text("Letter", 160.0, 100.0, 180.0, "N", t.h(100.0, t.light).lh(1.0));
    b.ctext("Wordmark", 280.0, 440.0, "Nordvik", t.h(72.0, t.dark).ls(-20.0));
    b.mark_last("vik", t.primary, false);
    b.rect("Accent", 220.0, 376.0, 60.0, 6.0, 3.0, solid(t.secondary));
    b.ctext("Tagline", 400.0, 440.0, "Cloud tools for builders", t.b(18.0, t.dark).upper().ls(250.0));
}

fn crest_logo(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.icon("Shield", "shield", 110.0, 96.0, 280.0, 300.0, solid(t.primary));
    for i in 0..3 {
        b.rect(&format!("Stripe {}", i + 1), 178.0 + 56.0 * f64::from(i), 200.0, 28.0, 96.0, 0.0, solid(if i == 1 { t.secondary } else { t.light }));
    }
    b.shape("Star", ShapeKind::Star { points: 5, inner_ratio: 0.45 }, 220.0, 124.0, 60.0, 60.0, solid(t.light));
    b.text("Founded", 150.0, 302.0, 200.0, "1921", t.h(26.0, t.light).ls(200.0));
    b.text("Top arc", 70.0, 8.0, 360.0, "Harbour City", t.h(34.0, t.primary).upper().ls(100.0));
    b.curve_last(70.0, 86.0);
    b.ctext("Name", 414.0, 340.0, "Football club", t.h(28.0, t.dark).upper().ls(150.0));
    b.rect("Rule left", 40.0, 430.0, 30.0, 4.0, 2.0, solid(t.secondary));
    b.rect("Rule right", 430.0, 430.0, 30.0, 4.0, 2.0, solid(t.secondary));
}

// ---------------------------------------------------------------- ads, headers and pages

/// A phone outline with a screen of `screen` colour, drawn in `x, y, w, h`.
fn phone(b: &mut B, x: f64, y: f64, w: f64, h: f64, body: genha_color::Color, screen: genha_doc::Fill) {
    b.rect("Phone", x, y, w, h, w * 0.16, solid(body));
    b.rect("Phone screen", x + w * 0.06, y + w * 0.12, w * 0.88, h - w * 0.24, w * 0.1, screen);
}

fn launch_email(b: &mut B) {
    let t = b.t.clone();
    b.bg(lin(0.0, t.light, t.secondary.lerp(t.light, 0.7)));
    b.ellipse("Circle", 430.0, 20.0, 160.0, 160.0, solid(t.primary));
    phone(b, 468.0, 30.0, 84.0, 160.0, t.dark, lin(90.0, t.secondary, t.primary));
    b.icon("Check", "check-circle", 490.0, 80.0, 40.0, 40.0, solid(t.light));
    b.text("Kicker", 30.0, 42.0, 380.0, "Out today", t.b(12.0, t.primary.lerp(t.dark, 0.35)).bold().upper().ls(300.0).left());
    b.text("Title", 30.0, 64.0, 400.0, "Meet Pocket Planner 2.0", t.h(30.0, t.dark).left());
    b.mark_last("2.0", t.primary.lerp(t.dark, 0.3), true);
    b.text("Subline", 30.0, 108.0, 400.0, "Your week, sorted in under a minute.", t.b(15.0, t.dark).left());
    b.pill("Button", 30.0, 140.0, 150.0, 36.0, solid(t.dark), "See what's new", t.b(13.0, t.light).bold());
}

fn webinar_leaderboard(b: &mut B) {
    let t = b.t.clone();
    b.bg(lin(0.0, t.dark, t.primary));
    b.ellipse("Icon disc", 14.0, 13.0, 64.0, 64.0, solid(t.secondary));
    b.icon("Megaphone", "megaphone", 28.0, 27.0, 36.0, 36.0, solid(t.dark));
    b.text("Kicker", 94.0, 14.0, 330.0, "Free webinar · 3 Dec", t.b(12.0, t.secondary).bold().upper().ls(200.0).left());
    b.text("Headline", 94.0, 34.0, 420.0, "Grow your newsletter to 10k", t.h(24.0, t.light).left());
    b.mark_last("10k", t.secondary, true);
    b.pill("Button", 580.0, 24.0, 130.0, 42.0, solid(t.secondary), "Register", t.b(16.0, t.dark).bold());
}

fn app_ad(b: &mut B) {
    let t = b.t.clone();
    b.bg(lin(135.0, t.primary.lerp(t.dark, 0.3), t.primary.lerp(t.dark, 0.6)));
    phone(b, 200.0, 40.0, 84.0, 170.0, t.dark, solid(t.light));
    for i in 0..4 {
        let y = 70.0 + 30.0 * f64::from(i);
        b.rect(
            &format!("List row {}", i + 1),
            214.0,
            y,
            56.0,
            20.0,
            5.0,
            solid(match i {
                1 => t.secondary,
                2 => t.primary,
                _ => t.primary.lerp(t.light, 0.8),
            }),
        );
    }
    b.text("Headline", 16.0, 26.0, 180.0, "Plan a week\nof dinners\nin 5 minutes", t.h(22.0, t.light).lh(1.1).left());
    b.mark_last("5 minutes", t.secondary, true);
    b.text("Subline", 16.0, 112.0, 180.0, "Shopping list included.\nFree for 30 days.", t.b(12.0, t.light).lh(1.4).left());
    b.pill("Button", 16.0, 172.0, 150.0, 40.0, solid(t.secondary), "Get the app", t.b(15.0, t.dark).bold());
    b.text("Brand", 16.0, 224.0, 180.0, "Supper Club app", t.b(11.0, t.light).bold().left());
}

fn landing_page(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    // Navigation.
    b.shape("Logo mark", ShapeKind::Polygon { sides: 6 }, 120.0, 40.0, 44.0, 44.0, solid(t.primary));
    b.text("Logo", 176.0, 46.0, 300.0, "Tandem", t.h(30.0, t.dark).left());
    for (i, item) in ["Features", "Pricing", "Stories"].iter().enumerate() {
        b.text(&format!("Nav {item}"), 760.0 + 150.0 * i as f64, 52.0, 140.0, item, t.b(22.0, t.dark));
    }
    b.pill("Nav button", 1180.0, 36.0, 140.0, 54.0, solid(t.dark), "Sign in", t.b(20.0, t.light).bold());
    // Hero.
    b.rect("Hero", 0.0, 130.0, w, 760.0, 0.0, lin(135.0, t.dark, t.primary.lerp(t.dark, 0.35)));
    b.text("Hero kicker", 120.0, 230.0, 700.0, "New · shared to-do lists", t.b(24.0, t.secondary).bold().upper().ls(250.0).left());
    b.text("Hero headline", 120.0, 280.0, 800.0, "Chores, split\nfairly. Finally.", t.h(88.0, t.light).lh(1.05).left());
    b.mark_last("fairly.", t.secondary, true);
    b.text(
        "Hero body",
        120.0,
        520.0,
        720.0,
        "Tandem shares the jobs at home so nobody\nkeeps score. Free for two people, forever.",
        t.b(28.0, t.light).lh(1.45).left(),
    );
    b.pill("Hero button", 120.0, 650.0, 300.0, 80.0, solid(t.secondary), "Start free", t.b(28.0, t.dark).bold());
    b.text("Hero note", 450.0, 672.0, 400.0, "No card needed", t.b(24.0, t.light).left());
    phone(b, 940.0, 200.0, 300.0, 600.0, t.light, solid(t.primary.lerp(t.light, 0.9)));
    for (i, (task, done)) in [("Bins out", true), ("Water plants", false), ("Book dentist", true), ("Fix the shelf", false)].iter().enumerate() {
        let y = 300.0 + 110.0 * i as f64;
        b.rect(&format!("Task card {}", i + 1), 980.0, y, 220.0, 90.0, 16.0, solid(t.light));
        b.icon(
            &format!("Task check {}", i + 1),
            "check-circle",
            996.0,
            y + 25.0,
            40.0,
            40.0,
            solid(if *done { t.primary } else { t.dark.lerp(t.light, 0.8) }),
        );
        b.text(&format!("Task {}", i + 1), 1046.0, y + 30.0, 150.0, task, t.b(20.0, t.dark).bold().left());
    }
    // Features.
    b.ctext("Features title", 960.0, 1200.0, "Why couples and flatmates love it", t.h(52.0, t.dark));
    let features = [
        ("arrows-swap", "Fair by default", "Jobs rotate so the same\nperson isn't always on bins."),
        ("bell", "Gentle nudges", "One reminder, never a\nnagging stream."),
        ("chart-pie", "See the split", "A weekly look at who did\nwhat, without blame."),
    ];
    for (i, (icon, title, body)) in features.iter().enumerate() {
        let x = 120.0 + 420.0 * i as f64;
        b.ellipse(&format!("Feature {} disc", i + 1), x, 1080.0, 100.0, 100.0, solid(t.primary.lerp(t.light, 0.85)));
        b.icon(&format!("Feature {} icon", i + 1), icon, x + 25.0, 1105.0, 50.0, 50.0, solid(t.primary));
        b.text(&format!("Feature {} title", i + 1), x, 1210.0, 380.0, title, t.h(32.0, t.dark).left());
        b.text(&format!("Feature {} text", i + 1), x, 1262.0, 380.0, body, t.b(24.0, t.dark).lh(1.45).left());
    }
    // Pricing.
    b.ctext("Pricing title", 1440.0, 1200.0, "Simple pricing", t.h(52.0, t.dark));
    let rows: &[&[&str]] = &[
        &["", "Duo", "Household"],
        &["Price", "Free", "$3 a month"],
        &["People", "2", "Up to 8"],
        &["Shared lists", "3", "Unlimited"],
        &["Rotating chores", "Yes", "Yes"],
        &["Weekly summary", "", "Yes"],
        &["Start in a minute, cancel any time", "", ""],
    ];
    let mut tb = t.table(rows, 24.0);
    tb.align = TextAlign::Center;
    tb.column_widths = vec![0.4, 0.3, 0.3];
    tb.borders = BorderLines::Horizontal;
    tb.merges = vec![CellRange::new(6, 0, 1, 3)];
    tb.style_cells(CellRange::new(1, 0, 5, 1), |s| {
        s.align = Some(TextAlign::Left);
        s.bold = Some(true);
    });
    tb.style_cells(CellRange::new(0, 2, 6, 1), |s| s.fill = Some(t.secondary.lerp(t.light, 0.6)));
    tb.style_cells(CellRange::cell(0, 2), |s| {
        s.fill = Some(t.secondary);
        s.text_color = Some(t.dark);
    });
    tb.style_cells(CellRange::new(6, 0, 1, 3), |s| {
        s.fill = Some(t.dark);
        s.text_color = Some(t.light);
    });
    // 24 px text and 12 px padding: 54 px a row.
    b.table("Pricing table", 270.0, 1540.0, 900.0, 7.0 * 54.0 + 2.0, tb);
    // Testimonial and footer.
    b.icon("Quote marks", "quote-marks", 120.0, 1990.0, 70.0, 70.0, solid(t.primary));
    b.text(
        "Testimonial",
        220.0,
        1990.0,
        1100.0,
        "“We haven't argued about the dishes in months.”",
        t.h(40.0, t.dark).italic().with(|s| s.bold = false).left(),
    );
    b.text("Testimonial author", 220.0, 2050.0, 1100.0, "Maya and Jonas, Hamburg", t.b(24.0, t.dark).bold().left());
    b.rect("Footer", 0.0, h - 200.0, w, 200.0, 0.0, solid(t.dark));
    b.text("Footer", 120.0, h - 120.0, 900.0, "Tandem · made in Hamburg · hello@tandem.example", t.b(22.0, t.light).left());
    b.text("Footer links", w - 620.0, h - 120.0, 500.0, "Privacy · Terms · Help", t.b(22.0, t.light).right());
}

fn just_listed(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.rect("Picture", 60.0, 60.0, 960.0, 560.0, 32.0, lin(90.0, t.secondary.lerp(t.light, 0.7), t.primary.lerp(t.light, 0.75)));
    b.ellipse("Sun", 800.0, 110.0, 120.0, 120.0, solid(t.secondary));
    b.shape("Roof", ShapeKind::Triangle, 300.0, 200.0, 480.0, 170.0, solid(t.primary));
    b.rect("House", 340.0, 370.0, 400.0, 230.0, 0.0, solid(t.light));
    b.rect("Door", 505.0, 470.0, 70.0, 130.0, 6.0, solid(t.primary));
    b.rect("Window left", 380.0, 410.0, 90.0, 80.0, 6.0, solid(t.secondary.lerp(t.light, 0.5)));
    b.rect("Window right", 610.0, 410.0, 90.0, 80.0, 6.0, solid(t.secondary.lerp(t.light, 0.5)));
    b.icon("Tree", "tree", 120.0, 330.0, 160.0, 270.0, solid(t.primary.lerp(t.light, 0.3)));
    b.rect("Label", 90.0, 90.0, 290.0, 70.0, 12.0, solid(t.dark));
    b.text("Label text", 90.0, 106.0, 290.0, "Just listed", t.b(30.0, t.light).bold().upper().ls(200.0).lh(1.2));
    b.text("Price", 60.0, 660.0, 600.0, "$389,000", t.h(84.0, t.dark).left());
    b.text("Address", 60.0, 770.0, 600.0, "4 Larkspur Way, Millbrook", t.b(36.0, t.dark).left());
    let facts = [("home", "3 beds"), ("users", "2 baths"), ("leaf", "Garden")];
    for (i, (icon, label)) in facts.iter().enumerate() {
        let x = 60.0 + 230.0 * i as f64;
        b.icon(&format!("{label} icon"), icon, x, 860.0, 50.0, 50.0, solid(t.primary));
        b.text(label, x + 64.0, 866.0, 160.0, label, t.b(32.0, t.dark).bold().left());
    }
    b.rect("Agent card", 760.0, 660.0, 260.0, 300.0, 24.0, solid(t.primary));
    b.text("Agent kicker", 760.0, 700.0, 260.0, "Viewings", t.b(24.0, t.secondary).bold().upper().ls(200.0));
    b.text("Agent", 760.0, 750.0, 260.0, "Sat & Sun\n11 am – 3 pm", t.b(30.0, t.light).bold().lh(1.3));
    b.text("Agent phone", 760.0, 880.0, 260.0, "555 0100 220", t.b(26.0, t.light));
    b.text("Agency", 60.0, 990.0, 900.0, "Meadow & Key Estate Agents · link in bio", t.b(28.0, t.dark).left());
}

fn pricing_post(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.text("Kicker", 90.0, 90.0, 900.0, "Fitness studio memberships", t.b(32.0, t.primary).bold().upper().ls(200.0).left());
    b.text("Headline", 90.0, 150.0, 900.0, "Pick the plan\nthat fits your week", t.h(80.0, t.dark).lh(1.1).left());
    b.mark_last("your week", t.primary, true);
    let rows: &[&[&str]] = &[
        &["", "Drop-in", "Regular", "Unlimited"],
        &["", "", "", "Most popular"],
        &["Price", "$15", "$59/mo", "$89/mo"],
        &["Classes", "1", "8 a month", "Every day"],
        &["Open gym", "—", "Yes", "Yes"],
        &["Guest passes", "—", "1", "4"],
        &["Towel & locker", "—", "—", "Yes"],
    ];
    let mut tb = t.table(rows, 30.0);
    tb.align = TextAlign::Center;
    tb.column_widths = vec![0.31, 0.23, 0.23, 0.23];
    tb.header_fill = t.dark;
    tb.borders = BorderLines::Horizontal;
    tb.alt_row_fill = None;
    // A ribbon row over the popular plan, then its column in the accent colour.
    tb.merges = vec![CellRange::new(1, 0, 1, 3)];
    tb.style_cells(CellRange::cell(1, 3), |s| {
        s.fill = Some(t.primary);
        s.text_color = Some(t.light);
        s.bold = Some(true);
        s.font_size = Some(20.0);
    });
    tb.style_cells(CellRange::new(2, 3, 5, 1), |s| {
        s.fill = Some(t.secondary);
        s.bold = Some(true);
    });
    tb.style_cells(CellRange::new(2, 0, 5, 1), |s| {
        s.align = Some(TextAlign::Left);
        s.bold = Some(true);
    });
    // 30 px text, 15 px padding: 67.5 px a row; the ribbon row 60 px.
    b.table("Plans", 90.0, 400.0, 900.0, 6.0 * 67.5 + 70.0, tb);
    b.pill("Join button", 90.0, 1000.0, 420.0, 100.0, solid(t.primary), "Book a free trial", t.b(36.0, t.light).bold());
    b.text("Studio", 90.0, 1180.0, 900.0, "Pulse Studio · Canal Street · pulse.example", t.b(30.0, t.dark).left());
}

fn podcast_cover(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.dark));
    b.ellipse("Glow", 140.0, 140.0, 800.0, 800.0, radial(t.primary.lerp(t.dark, 0.72), t.dark));
    for (i, r) in [380.0, 320.0, 260.0].into_iter().enumerate() {
        b.outline(
            &format!("Ring {}", i + 1),
            ShapeKind::Ellipse,
            540.0 - r,
            540.0 - r,
            2.0 * r,
            2.0 * r,
            t.secondary.with_alpha(80 + 50 * i as u8),
            4.0,
        );
    }
    b.ellipse("Mic disc", 390.0, 250.0, 300.0, 300.0, solid(t.primary));
    b.icon("Microphone", "microphone", 465.0, 325.0, 150.0, 150.0, solid(t.dark));
    b.ctext("Title", 600.0, 960.0, "Side Hustle\nHour", t.h(130.0, t.light).upper().lh(0.95));
    b.mark_last("Hour", t.primary, true);
    b.ctext("Tagline", 880.0, 960.0, "Small business stories, every Tuesday", t.b(36.0, t.secondary).bold());
    b.text("Arc", 240.0, 96.0, 600.0, "A weekly show", t.b(30.0, t.light).bold().upper().ls(300.0));
    b.curve_last(60.0, 90.0);
}
