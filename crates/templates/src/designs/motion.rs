//! Animated social posts and short videos: entrance animations and page durations, made for GIF
//! and MP4 export (they also look finished as still pictures).

use genha_doc::{AnimationKind as A, LoopKind, ShapeKind};

use super::{Def, SOCIAL, def, lin, radial, solid};
use crate::build::{B, Sty, Theme};

pub(super) const DEFS: &[Def] = &[
    def(
        "animated-flash-sale",
        "Flash sale animated post",
        SOCIAL,
        "instagram-post",
        &["animated", "sale", "discount", "instagram", "gif", "video", "shop"],
        || Theme::new(0xff4f79, 0xffd447, 0x1f1235, 0xfff7f0, "Display", "Sans"),
        flash_sale,
    ),
    def(
        "animated-launch-story",
        "Product launch animated story",
        SOCIAL,
        "instagram-story",
        &["animated", "launch", "product", "story", "reel", "video", "new"],
        || Theme::new(0x6c63ff, 0x2ee6c5, 0x0e0b2a, 0xf4f3ff, "Display", "Sans"),
        launch_story,
    ),
    def(
        "animated-quote-reel",
        "Typewriter quote reel",
        SOCIAL,
        "video-portrait",
        &["animated", "quote", "reel", "video", "typewriter", "motivation"],
        || Theme::new(0xc8553d, 0x588b8b, 0x2f1b14, 0xfbf3ea, "Serif", "Sans"),
        quote_reel,
    ),
    def(
        "animated-opening-teaser",
        "Opening night video teaser",
        SOCIAL,
        "video-landscape",
        &["animated", "video", "teaser", "opening", "restaurant", "multi-page", "mp4"],
        || Theme::new(0xe9b44c, 0x50a2a7, 0x1c1610, 0xfbf5e6, "Serif", "Sans"),
        opening_teaser,
    ),
    def(
        "animated-tips-square",
        "Three tips animated video",
        SOCIAL,
        "video-square",
        &["animated", "tips", "list", "video", "education", "square"],
        || Theme::new(0x2b59c3, 0xc6f432, 0x101418, 0xf9fff0, "Display", "Sans"),
        tips_square,
    ),
];

fn flash_sale(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.dark));
    b.ellipse("Glow", 180.0, 180.0, 900.0, 900.0, radial(t.primary.lerp(t.dark, 0.7), t.dark));
    b.animate_last(A::Fade, 1.0, 0.0);
    b.text("Kicker", 90.0, 110.0, 700.0, "48 hours only", t.b(34.0, t.secondary).bold().upper().ls(250.0).left());
    b.animate_last(A::Fade, 0.6, 0.1);
    b.text("Headline", 90.0, 170.0, 900.0, "Flash\nsale", t.h(200.0, t.light).upper().lh(0.95).left());
    b.mark_last("sale", t.primary, true);
    b.animate_last(A::SlideUp, 0.7, 0.3);
    b.text("Body", 90.0, 600.0, 560.0, "Everything in store\nand online", t.b(40.0, t.light).lh(1.35).left());
    b.animate_last(A::Fade, 0.6, 1.2);
    let m = b.mark();
    b.icon("Seal", "seal", 680.0, 520.0, 320.0, 320.0, solid(t.secondary));
    b.rotate_last(-12.0);
    b.text("Seal text", 700.0, 590.0, 280.0, "60%\noff", t.h(84.0, t.dark).upper().lh(0.95));
    b.rotate_last(-12.0);
    b.animate_since(m, A::Pop, 0.5, 0.9);
    let m = b.mark();
    b.pill("Button", 90.0, 800.0, 420.0, 110.0, solid(t.primary), "Shop the sale", t.b(42.0, t.dark).bold());
    b.animate_since(m, A::Rise, 0.6, 1.5);
    b.text("Website", 90.0, 960.0, 600.0, "yourshop.com", t.b(32.0, t.light).left());
    b.animate_last(A::Fade, 0.5, 1.8);
    b.icon("Sparkle", "sparkle", 930.0, 110.0, 70.0, 70.0, solid(t.secondary));
    b.animate_last(A::Spin, 0.6, 1.0);
    b.loop_last(LoopKind::Pulse);
    b.icon("Small sparkle", "sparkle", 610.0, 920.0, 44.0, 44.0, solid(t.light));
    b.animate_last(A::Pop, 0.4, 1.9);
    b.duration(5.0);
}

fn launch_story(b: &mut B) {
    let t = b.t.clone();
    b.bg(lin(90.0, t.dark, t.primary.lerp(t.dark, 0.55)));
    b.text("Kicker", 90.0, 200.0, 900.0, "New launch", t.b(40.0, t.secondary).bold().upper().ls(300.0).left());
    b.animate_last(A::Fade, 0.6, 0.0);
    b.text("Headline", 90.0, 260.0, 900.0, "Meet\nAurora", t.h(170.0, t.light).lh(1.0).left());
    b.mark_last("Aurora", t.secondary, true);
    b.animate_last(A::Rise, 0.8, 0.2);
    b.text("Lead", 90.0, 630.0, 900.0, "Our quietest headphones yet", t.b(44.0, t.light).left());
    b.animate_last(A::Fade, 0.6, 0.7);
    b.ellipse("Halo", 240.0, 740.0, 600.0, 600.0, radial(t.primary, t.primary.lerp(t.dark, 0.6)));
    b.animate_last(A::Zoom, 0.8, 0.8);
    b.icon("Headphones", "headphones", 340.0, 820.0, 400.0, 400.0, solid(t.light));
    b.animate_last(A::Pop, 0.6, 1.1);
    b.loop_last(LoopKind::Float);
    let features = ["40-hour battery", "Quiet mode in one tap", "Folds flat for travel"];
    for (i, f) in features.iter().enumerate() {
        let y = 1400.0 + 90.0 * i as f64;
        let n = i + 1;
        let m = b.mark();
        b.icon(&format!("Feature {n} check"), "check-circle", 90.0, y, 56.0, 56.0, solid(t.secondary));
        b.text(&format!("Feature {n}"), 176.0, y + 2.0, 800.0, f, t.b(44.0, t.light).left());
        b.animate_since(m, A::SlideLeft, 0.5, 1.6 + 0.25 * i as f64);
    }
    let m = b.mark();
    b.pill("Pre-order button", 190.0, 1700.0, 700.0, 120.0, solid(t.secondary), "Pre-order today", t.b(48.0, t.dark).bold());
    b.animate_since(m, A::Bounce, 0.7, 2.5);
    b.duration(6.0);
}

fn quote_reel(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.rect("Top band", 0.0, 0.0, b.w, 24.0, 0.0, solid(t.primary));
    b.icon("Blob", "blob-2", 620.0, 140.0, 380.0, 380.0, solid(t.secondary.lerp(t.light, 0.7)));
    b.animate_last(A::Fade, 1.0, 0.0);
    b.icon("Quote marks", "quote-marks", 90.0, 520.0, 180.0, 180.0, solid(t.primary));
    b.animate_last(A::Pop, 0.5, 0.2);
    b.text(
        "Quote",
        90.0,
        780.0,
        900.0,
        "Start where you are.\nUse what you have.\nDo what you can.",
        t.h(80.0, t.dark).italic().lh(1.3).left().with(|s| s.bold = false),
    );
    b.mark_last("Do what you can.", t.primary, true);
    b.animate_last(A::Typewriter, 3.0, 0.6);
    b.rect("Divider", 90.0, 1160.0, 120.0, 6.0, 3.0, solid(t.secondary));
    b.animate_last(A::SlideRight, 0.5, 3.8);
    b.text("Author", 90.0, 1200.0, 900.0, "Words for Monday", t.b(40.0, t.secondary.lerp(t.dark, 0.35)).bold().upper().ls(200.0).left());
    b.animate_last(A::Fade, 0.6, 4.0);
    b.icon("Wave", "wave-divider", 0.0, 1620.0, 1080.0, 300.0, solid(t.primary));
    b.rect("Wave base", 0.0, 1855.0, 1080.0, 65.0, 0.0, solid(t.primary));
    b.text("Handle", 90.0, 1862.0, 900.0, "@yourhandle", t.b(36.0, t.light).bold().left());
    b.animate_last(A::Fade, 0.6, 4.4);
    b.duration(7.0);
}

fn opening_teaser(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.name_page("Teaser");
    b.bg(solid(t.dark));
    b.ellipse("Glow", 560.0, 140.0, 800.0, 800.0, radial(t.primary.lerp(t.dark, 0.55), t.dark));
    b.animate_last(A::Fade, 1.2, 0.0);
    b.ctext("Kicker", 380.0, 1200.0, "Harbour Street", t.b(40.0, t.primary).bold().upper().ls(400.0));
    b.animate_last(A::Fade, 0.6, 0.3);
    b.ctext("Headline", 450.0, 1500.0, "Something new\nis cooking", t.h(110.0, t.light).italic().lh(1.1));
    b.animate_last(A::Rise, 0.8, 0.6);
    b.duration(3.0);

    b.new_page("Details");
    b.bg(solid(t.light));
    b.rect("Side band", 0.0, 0.0, 640.0, h, 0.0, solid(t.secondary));
    b.animate_last(A::SlideRight, 0.6, 0.0);
    b.icon("Wine", "wine-glass", 170.0, 290.0, 300.0, 300.0, solid(t.light));
    b.animate_last(A::Pop, 0.5, 0.5);
    b.text("Kicker", 760.0, 300.0, 1000.0, "Kitchen & wine bar", t.b(44.0, t.primary.lerp(t.dark, 0.45)).bold().upper().ls(250.0).left());
    b.animate_last(A::Fade, 0.5, 0.5);
    b.text("Headline", 760.0, 380.0, 1060.0, "Small plates,\nlong tables", t.h(120.0, t.dark).lh(1.05).left());
    b.mark_last("long tables", t.primary.lerp(t.dark, 0.3), false);
    b.animate_last(A::SlideLeft, 0.7, 0.8);
    b.text("Body", 760.0, 680.0, 1000.0, "Seasonal food from the coast, natural wines", t.b(40.0, t.dark).left());
    b.animate_last(A::Fade, 0.6, 1.4);
    b.duration(3.5);

    b.new_page("Opening");
    b.bg(lin(135.0, t.dark, t.primary.lerp(t.dark, 0.6)));
    b.ctext("Kicker", 300.0, 1200.0, "Opening night", t.b(44.0, t.primary).bold().upper().ls(400.0));
    b.animate_last(A::Fade, 0.5, 0.0);
    b.ctext("Date", 370.0, 1500.0, "Friday 12 March", t.h(140.0, t.light));
    b.animate_last(A::Zoom, 0.7, 0.3);
    let m = b.mark();
    b.pill("Book button", (w - 560.0) / 2.0, 640.0, 560.0, 110.0, solid(t.primary), "Book a table", t.b(48.0, t.dark).bold());
    b.animate_since(m, A::Bounce, 0.7, 1.0);
    b.ctext("Website", 820.0, 1200.0, "tidewater.example · 14 Harbour Street", t.b(36.0, t.light).ls(50.0));
    b.animate_last(A::Fade, 0.6, 1.6);
    b.duration(4.0);
}

fn tips_square(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.rect("Header", 0.0, 0.0, b.w, 400.0, 0.0, solid(t.primary));
    b.animate_last(A::SlideDown, 0.5, 0.0);
    b.text("Kicker", 90.0, 100.0, 900.0, "Save your battery", t.b(36.0, t.secondary).bold().upper().ls(250.0).left());
    b.animate_last(A::Fade, 0.5, 0.3);
    b.text("Headline", 90.0, 160.0, 900.0, "3 phone habits\nthat really work", t.h(84.0, t.light).lh(1.1).left());
    b.animate_last(A::Rise, 0.6, 0.4);
    let tips = [
        ("Dim the screen", "The display uses the most power."),
        ("Close the map", "Location in the background drains fast."),
        ("Charge to 80%", "Batteries age more slowly that way."),
    ];
    for (i, (title, body)) in tips.iter().enumerate() {
        let y = 460.0 + 180.0 * i as f64;
        let n = i + 1;
        let m = b.mark();
        b.ellipse(&format!("Tip {n} disc"), 90.0, y, 120.0, 120.0, solid(t.secondary));
        b.text(&format!("Tip {n} number"), 90.0, y + 27.0, 120.0, &n.to_string(), t.h(60.0, t.dark).lh(1.0));
        b.text(&format!("Tip {n} title"), 250.0, y + 6.0, 740.0, title, t.h(52.0, t.dark).left());
        b.text(&format!("Tip {n} text"), 250.0, y + 74.0, 740.0, body, t.b(34.0, t.dark.lerp(t.light, 0.25)).left());
        b.animate_since(m, A::SlideLeft, 0.5, 1.0 + 0.6 * i as f64);
    }
    b.shape("Corner star", ShapeKind::Star { points: 4, inner_ratio: 0.3 }, 900.0, 60.0, 110.0, 110.0, solid(t.secondary));
    b.animate_last(A::Spin, 0.6, 0.6);
    b.text("Handle", 90.0, 1000.0, 900.0, "@dailytechtips · follow for more", t.b(32.0, t.primary).bold().left());
    b.animate_last(A::Fade, 0.5, 3.0);
    b.duration(6.0);
}
