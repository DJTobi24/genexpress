//! Marketing: logos, web banner ads and email headers.

use genha_doc::ShapeKind;

use super::{Def, LOGOS, MARKETING, def, lin, solid};
use crate::build::{B, Sty, Theme};

pub(super) const DEFS: &[Def] = &[
    def(
        "badge-logo",
        "Roastery badge logo",
        LOGOS,
        "logo",
        &["logo", "badge", "brand", "coffee"],
        || Theme::new(0xb45309, 0xfde68a, 0x422006, 0xfffbeb, "Serif", "Sans"),
        badge_logo,
    ),
    def(
        "monogram-logo",
        "Florist monogram logo",
        LOGOS,
        "logo",
        &["logo", "monogram", "initial", "brand", "florist", "elegant"],
        || Theme::new(0x283618, 0xbc6c25, 0x1a1f16, 0xfefae0, "Serif", "Sans"),
        monogram_logo,
    ),
    def(
        "wordmark-logo",
        "Studio wordmark logo",
        LOGOS,
        "logo",
        &["logo", "wordmark", "text logo", "brand", "studio", "modern"],
        || Theme::new(0x0096c7, 0xff7d00, 0x001d3d, 0xffffff, "Display", "Sans"),
        wordmark_logo,
    ),
    def(
        "emblem-logo",
        "Trail club emblem logo",
        LOGOS,
        "logo",
        &["logo", "emblem", "badge", "shield", "outdoor", "club"],
        || Theme::new(0x344e41, 0xf2cc8f, 0x1b2621, 0xf6fff8, "Display", "Sans"),
        emblem_logo,
    ),
    def(
        "leaderboard-ad",
        "Summer sale leaderboard ad",
        MARKETING,
        "banner-leaderboard",
        &["web banner", "ad", "leaderboard", "728x90", "sale", "display ad"],
        || Theme::new(0xff595e, 0xffca3a, 0x1a1423, 0xffffff, "Display", "Sans"),
        leaderboard_ad,
    ),
    def(
        "rectangle-ad",
        "Coding course rectangle ad",
        MARKETING,
        "banner-rectangle",
        &["web banner", "ad", "rectangle", "300x250", "course", "display ad"],
        || Theme::new(0x4ea8de, 0xfdc500, 0x0d1b2a, 0xffffff, "Display", "Sans"),
        rectangle_ad,
    ),
    def(
        "newsletter-header",
        "Sunday letter email header",
        MARKETING,
        "email-header",
        &["email header", "newsletter", "email", "header", "blog"],
        || Theme::new(0xe36414, 0x0f4c5c, 0x2b2118, 0xfffbf5, "Serif", "Sans"),
        newsletter_header,
    ),
];

// ---------------------------------------------------------------- logos

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

fn monogram_logo(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.outline("Outer ring", ShapeKind::Ellipse, 110.0, 50.0, 280.0, 280.0, t.primary, 6.0);
    b.ellipse("Disc", 130.0, 70.0, 240.0, 240.0, solid(t.primary));
    b.text("Initial", 130.0, 112.0, 240.0, "M", t.h(150.0, t.light).lh(1.0).with(|s| s.bold = false));
    b.icon("Leaf left", "leaf", 92.0, 372.0, 30.0, 30.0, solid(t.secondary));
    b.rotate_last(-90.0);
    b.icon("Leaf right", "leaf", 378.0, 372.0, 30.0, 30.0, solid(t.secondary));
    b.ctext("Name", 360.0, 260.0, "Maple\n& Moss", t.h(30.0, t.dark).lh(1.15));
    b.ctext("Tagline", 440.0, 440.0, "Florist · est. 2026", t.b(16.0, t.secondary).bold().upper().ls(300.0));
}

fn wordmark_logo(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.icon("Spark", "sparkle", 225.0, 100.0, 50.0, 50.0, solid(t.secondary));
    b.ctext("Wordmark", 180.0, 480.0, "brightline", t.h(76.0, t.dark).ls(-30.0));
    b.rect("Underline", 110.0, 285.0, 260.0, 10.0, 5.0, solid(t.primary));
    b.ellipse("Dot", 380.0, 280.0, 20.0, 20.0, solid(t.secondary));
    b.ctext("Tagline", 320.0, 440.0, "Creative studio", t.b(20.0, t.primary).bold().upper().ls(400.0));
}

fn emblem_logo(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.icon("Shield", "shield", 90.0, 24.0, 320.0, 360.0, solid(t.primary));
    b.ellipse("Sun", 282.0, 112.0, 44.0, 44.0, solid(t.secondary));
    b.shape("Peak", ShapeKind::Triangle, 160.0, 140.0, 130.0, 110.0, solid(t.light));
    b.shape("Small peak", ShapeKind::Triangle, 240.0, 170.0, 110.0, 80.0, solid(t.secondary));
    b.ctext("Established", 262.0, 220.0, "Est. 2026", t.b(14.0, t.secondary).bold().upper().ls(300.0));
    b.icon("Ribbon", "ribbon-banner", 50.0, 290.0, 400.0, 140.0, solid(t.secondary));
    b.ctext("Name", 330.0, 250.0, "Trail club", t.h(30.0, t.dark).upper().lh(1.1));
    b.ctext("Tagline", 440.0, 440.0, "Hike · Camp · Explore", t.b(16.0, t.primary).bold().upper().ls(300.0));
}

// ---------------------------------------------------------------- ads and headers

fn leaderboard_ad(b: &mut B) {
    let t = b.t.clone();
    b.bg(lin(0.0, t.primary, t.primary.lerp(t.dark, 0.3)));
    b.icon("Seal", "seal", 12.0, 5.0, 80.0, 80.0, solid(t.secondary));
    b.text("Seal text", 12.0, 31.0, 80.0, "40%", t.h(22.0, t.dark).lh(1.0));
    b.text("Headline", 108.0, 12.0, 400.0, "Summer sale", t.h(32.0, t.light).left());
    b.text("Subline", 108.0, 52.0, 400.0, "Up to 40% off everything", t.b(16.0, t.light).left());
    b.icon("Sparkle", "sparkle", 500.0, 24.0, 40.0, 40.0, solid(t.light.with_alpha(200)));
    b.pill("Button", 560.0, 22.0, 150.0, 46.0, solid(t.secondary), "Shop now", t.b(18.0, t.dark).bold());
}

fn rectangle_ad(b: &mut B) {
    let t = b.t.clone();
    b.bg(lin(135.0, t.dark, t.primary.lerp(t.dark, 0.6)));
    b.icon("Rocket", "rocket", 190.0, 24.0, 90.0, 90.0, solid(t.secondary));
    b.rotate_last(30.0);
    b.text("Headline", 20.0, 24.0, 180.0, "Learn to\ncode in\n30 days", t.h(30.0, t.light).lh(1.05).left());
    b.text("Subline", 20.0, 133.0, 260.0, "Free first lesson,\nno experience needed", t.b(13.0, t.light.lerp(t.dark, 0.2)).lh(1.4).left());
    b.pill("Button", 20.0, 188.0, 160.0, 44.0, solid(t.primary), "Start now", t.b(16.0, t.dark).bold());
}

fn newsletter_header(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.rect("Colour block", 440.0, 0.0, 160.0, 200.0, 0.0, solid(t.secondary));
    b.icon("Envelope", "envelope", 480.0, 60.0, 80.0, 80.0, solid(t.light));
    b.icon("Sparkle", "sparkle", 548.0, 26.0, 24.0, 24.0, solid(t.primary));
    b.text("Kicker", 30.0, 40.0, 380.0, "Issue 42 · October", t.b(12.0, t.primary).bold().upper().ls(250.0).left());
    b.text("Title", 30.0, 62.0, 400.0, "The Sunday Letter", t.h(36.0, t.dark).left());
    b.text("Subline", 30.0, 118.0, 400.0, "Ideas worth keeping, once a week", t.b(15.0, t.dark.lerp(t.light, 0.25)).left());
    b.rect("Rule", 30.0, 160.0, 60.0, 4.0, 2.0, solid(t.primary));
}
