//! More social formats: reel covers, announcements, LinkedIn and X posts, thumbnails, pins,
//! greetings and stories.

use genha_doc::{ChartKind, LegendPosition, ShapeKind, TextEffect, ValueFormat};

use super::{Def, SOCIAL, def, lin, radial, solid};
use crate::build::{B, Sty, Theme};

pub(super) const DEFS: &[Def] = &[
    def(
        "workout-reel-cover",
        "Twenty minute workout reel cover",
        SOCIAL,
        "instagram-story",
        &["reel", "cover", "fitness", "workout", "instagram", "vertical"],
        || Theme::new(0xff6b35, 0x00d9c0, 0x111827, 0xf9fafb, "Display", "Sans"),
        workout_reel,
    ),
    def(
        "podcast-episode-post",
        "New podcast episode post",
        SOCIAL,
        "instagram-post",
        &["podcast", "episode", "audio", "announcement", "instagram", "square"],
        || Theme::new(0x7f5af0, 0x2cb67d, 0x16161a, 0xfaf8ff, "Display", "Sans"),
        podcast_episode,
    ),
    def(
        "moving-announcement",
        "We're moving announcement",
        SOCIAL,
        "instagram-portrait",
        &["announcement", "moving", "new address", "small business", "instagram", "portrait"],
        || Theme::new(0x0081a7, 0xf07167, 0x073642, 0xfdfcdc, "Display", "Sans"),
        moving_announcement,
    ),
    def(
        "growth-chart-post",
        "Year of growth chart post",
        SOCIAL,
        "linkedin-post",
        &["linkedin", "chart", "growth", "business", "results", "statistics"],
        || Theme::new(0x1d4ed8, 0xf97316, 0x0b1437, 0xf8fafc, "Sans", "Sans"),
        growth_post,
    ),
    def(
        "webinar-post",
        "Free webinar invitation post",
        SOCIAL,
        "linkedin-post",
        &["linkedin", "webinar", "event", "speakers", "online", "business"],
        || Theme::new(0x0f766e, 0xfbbf24, 0x042f2e, 0xf0fdfa, "Sans", "Sans"),
        webinar_post,
    ),
    def(
        "charity-run-post",
        "Charity fun run post",
        SOCIAL,
        "facebook-post",
        &["facebook", "charity", "fun run", "5k", "event", "community", "sport"],
        || Theme::new(0x16a34a, 0xfacc15, 0x052e16, 0xf7fee7, "Display", "Sans"),
        charity_run_post,
    ),
    def(
        "shortcut-tip-post",
        "Keyboard shortcut tip post",
        SOCIAL,
        "x-post",
        &["x", "twitter", "tip", "tech", "how to", "shortcut"],
        || Theme::new(0x334155, 0x22d3ee, 0x030712, 0xf3f4f6, "Sans", "Mono"),
        shortcut_tip,
    ),
    def(
        "versus-thumbnail",
        "Head to head review thumbnail",
        SOCIAL,
        "youtube-thumbnail",
        &["youtube", "thumbnail", "review", "versus", "comparison", "tech"],
        || Theme::new(0xef4444, 0x3b82f6, 0x0a0a0a, 0xffffff, "Display", "Sans"),
        versus_thumbnail,
    ),
    def(
        "budget-pin",
        "Simple monthly budget pin",
        SOCIAL,
        "pinterest-pin",
        &["pinterest", "budget", "money", "infographic", "chart", "finance"],
        || Theme::new(0x2a9d8f, 0xe9c46a, 0x264653, 0xfdfaf3, "Serif", "Sans"),
        budget_pin,
    ),
    def(
        "holiday-greeting-post",
        "Season's greetings wreath post",
        SOCIAL,
        "instagram-post",
        &["holiday", "christmas", "greeting", "wreath", "winter", "square"],
        || Theme::new(0xb91c1c, 0x15803d, 0x1c1917, 0xfefce8, "Serif", "Sans"),
        holiday_greeting,
    ),
    def(
        "daily-special-story",
        "Today's special restaurant story",
        SOCIAL,
        "facebook-story",
        &["story", "restaurant", "menu", "special", "food", "facebook"],
        || Theme::new(0xe76f51, 0x2a9d8f, 0x1d1d1b, 0xfff8f0, "Serif", "Sans"),
        daily_special,
    ),
    def(
        "highlight-quote-post",
        "Highlighted words quote post",
        SOCIAL,
        "instagram-post",
        &["quote", "typography", "highlight", "instagram", "minimal", "square"],
        || Theme::new(0xff5d73, 0x2b59c3, 0x101418, 0xffffff, "Display", "Sans"),
        highlight_quote,
    ),
];

fn workout_reel(b: &mut B) {
    // Titles stay inside the middle 1080 × 1440 that profile grids show.
    let t = b.t.clone();
    b.bg(solid(t.dark));
    b.shape(
        "Diagonal band",
        ShapeKind::Path { d: "M0 520 L1080 260 L1080 900 L0 1160 Z".into(), view_width: 1080.0, view_height: 1920.0 },
        0.0,
        0.0,
        1080.0,
        1920.0,
        solid(t.primary),
    );
    b.text("Episode", 90.0, 300.0, 600.0, "Episode 4", t.b(40.0, t.dark).bold().upper().ls(200.0).left());
    b.effect_last(TextEffect::Highlight { color: t.secondary, padding: 14.0, corner_radius: 8.0 });
    b.text("Minutes", 90.0, 480.0, 560.0, "20", t.h(360.0, t.dark).lh(1.0).left());
    b.text("Minutes label", 590.0, 600.0, 400.0, "min", t.h(140.0, t.dark).lh(1.0).left());
    b.text("Headline", 90.0, 1190.0, 900.0, "Full body\nworkout", t.h(130.0, t.light).upper().lh(1.0).left());
    b.mark_last("workout", t.secondary, true);
    for (i, label) in ["No equipment", "Beginner friendly"].iter().enumerate() {
        let x = 90.0 + 420.0 * i as f64;
        b.pill(&format!("Tag {}", i + 1), x, 1480.0, 390.0, 80.0, solid(t.light.lerp(t.dark, 0.85)), label, t.b(34.0, t.light).bold());
    }
    b.icon("Lightning", "lightning", 860.0, 1010.0, 130.0, 130.0, solid(t.secondary));
    b.text("Handle", 90.0, 1600.0, 900.0, "@movewithmaya", t.b(38.0, t.light).bold().left());
}

fn podcast_episode(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.dark));
    b.rect("Card", 60.0, 60.0, 960.0, 960.0, 40.0, solid(t.primary.lerp(t.dark, 0.7)));
    b.text("Show", 120.0, 130.0, 600.0, "The Slow Lane", t.b(34.0, t.secondary).bold().upper().ls(250.0).left());
    b.text("Episode", 120.0, 190.0, 600.0, "Episode 37 · out now", t.b(32.0, t.light).left());
    b.ellipse("Mic disc", 760.0, 110.0, 200.0, 200.0, solid(t.primary));
    b.icon("Microphone", "microphone", 810.0, 160.0, 100.0, 100.0, solid(t.light));
    b.text("Title", 120.0, 360.0, 840.0, "Why boredom is\ngood for your\nbest ideas", t.h(82.0, t.light).lh(1.1).left());
    b.mark_last("boredom", t.secondary, true);
    b.text("Guest", 120.0, 680.0, 840.0, "with guest Dr. Lena Ortiz", t.b(38.0, t.light).left());
    b.mark_last("Dr. Lena Ortiz", t.secondary, true);
    // A waveform of rounded bars.
    let heights = [30.0, 60.0, 90.0, 50.0, 120.0, 80.0, 40.0, 100.0, 140.0, 70.0, 110.0, 50.0, 90.0, 130.0, 60.0, 30.0, 80.0, 40.0];
    for (i, h) in heights.iter().enumerate() {
        let x = 120.0 + 42.0 * i as f64;
        b.rect(&format!("Wave bar {}", i + 1), x, 850.0 - h / 2.0, 22.0, *h, 11.0, solid(if i < 9 { t.secondary } else { t.light.with_alpha(110) }));
    }
    b.icon("Play", "play-button", 900.0, 805.0, 90.0, 90.0, solid(t.light));
    b.text("Listen", 120.0, 940.0, 840.0, "Listen wherever you get your podcasts", t.b(28.0, t.light).left());
}

fn moving_announcement(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.rect("Top block", 0.0, 0.0, 1080.0, 620.0, 0.0, solid(t.primary));
    b.icon("House", "home", 700.0, 200.0, 300.0, 300.0, solid(t.light));
    b.icon("Arrow", "arrow-right", 590.0, 310.0, 100.0, 84.0, solid(t.secondary));
    b.text("Kicker", 90.0, 120.0, 600.0, "Big news", t.b(36.0, t.light).bold().upper().ls(300.0).left());
    b.text("Headline", 90.0, 180.0, 560.0, "We're\nmoving!", t.h(110.0, t.light).lh(1.05).left());
    b.text("Lead", 90.0, 700.0, 900.0, "Same team, same coffee,\nmore seats and a garden.", t.b(46.0, t.dark).lh(1.3).left());
    b.mark_last("a garden", t.secondary.lerp(t.dark, 0.3), true);
    b.rect("Address card", 90.0, 900.0, 900.0, 260.0, 32.0, solid(t.primary.lerp(t.light, 0.88)));
    b.icon("Pin", "location-pin", 130.0, 950.0, 110.0, 110.0, solid(t.secondary.lerp(t.dark, 0.2)));
    b.text("Address label", 280.0, 940.0, 680.0, "Find us from 3 May at", t.b(32.0, t.dark).left());
    b.text("Address", 280.0, 990.0, 680.0, "27 Canal Road,\nOld Town", t.h(56.0, t.dark).lh(1.15).left());
    b.text("Footer", 90.0, 1230.0, 900.0, "Last day at the old shop: 30 April", t.b(32.0, t.primary.lerp(t.dark, 0.35)).bold().left());
}

fn growth_post(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.text("Kicker", 60.0, 60.0, 460.0, "2026 in review", t.b(24.0, t.primary).bold().upper().ls(250.0).left());
    b.text("Headline", 60.0, 104.0, 460.0, "We doubled\nour reach\nthis year", t.h(60.0, t.dark).lh(1.1).left());
    b.mark_last("doubled", t.primary, true);
    b.text(
        "Body",
        60.0,
        340.0,
        440.0,
        "Thank you to every customer\nand partner who shared\nour work along the way.",
        t.b(22.0, t.dark.lerp(t.light, 0.2)).lh(1.45).left(),
    );
    b.rect("Accent", 60.0, 470.0, 80.0, 6.0, 3.0, solid(t.secondary));
    b.text("Company", 60.0, 520.0, 440.0, "Northline Studio", t.b(24.0, t.dark).bold().left());
    b.rect("Chart card", 560.0, 50.0, 590.0, 527.0, 24.0, solid(t.primary.lerp(t.light, 0.93)));
    let mut c =
        t.chart(ChartKind::Bar, &["Q1", "Q2", "Q3", "Q4"], &[("2025", &[12.0, 15.0, 17.0, 21.0]), ("2026", &[22.0, 29.0, 35.0, 43.0])], 18.0, t.dark);
    c.legend_position = LegendPosition::Top;
    c.colors = vec![t.secondary, t.primary];
    c.y_title = "Readers (thousands)".into();
    b.chart("Growth chart", 590.0, 80.0, 530.0, 467.0, c);
}

fn webinar_post(b: &mut B) {
    let t = b.t.clone();
    b.bg(lin(0.0, t.dark, t.primary.lerp(t.dark, 0.4)));
    b.text("Kicker", 60.0, 60.0, 600.0, "Free webinar", t.b(24.0, t.secondary).bold().upper().ls(300.0).left());
    b.text("Headline", 60.0, 104.0, 640.0, "Pricing your work\nwithout the panic", t.h(54.0, t.light).lh(1.15).left());
    b.text("When", 60.0, 260.0, 640.0, "Thursday 20 November · 6 pm CET · 45 minutes", t.b(22.0, t.light).left());
    b.mark_last("Thursday 20 November", t.secondary, true);
    b.pill("Register button", 60.0, 330.0, 280.0, 64.0, solid(t.secondary), "Save my seat", t.b(26.0, t.dark).bold());
    b.text("Link", 60.0, 420.0, 600.0, "studio.example/webinar", t.b(22.0, t.light).left());
    let speakers = [("IN", "Ines Novak", "Pricing coach"), ("TB", "Tomás Berg", "Studio founder")];
    for (i, (initials, name, role)) in speakers.iter().enumerate() {
        let x = 760.0 + 220.0 * i as f64;
        b.ellipse(&format!("{name} photo"), x, 120.0, 180.0, 180.0, solid(t.light));
        b.text(&format!("{name} initials"), x, 178.0, 180.0, initials, t.h(56.0, t.primary).lh(1.0));
        b.text(&format!("{name} name"), x - 20.0, 320.0, 220.0, name, t.b(24.0, t.light).bold());
        b.text(&format!("{name} role"), x - 20.0, 356.0, 220.0, role, t.b(20.0, t.secondary));
    }
    b.rect("Footer rule", 60.0, 520.0, 1080.0, 2.0, 1.0, solid(t.light.with_alpha(80)));
    b.text("Host", 60.0, 545.0, 1080.0, "Hosted by Studio Collective · recording sent to everyone who registers", t.b(20.0, t.light).left());
}

fn charity_run_post(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.rect("Colour block", 700.0, 0.0, 500.0, 630.0, 0.0, solid(t.primary));
    b.ellipse("Distance disc", 790.0, 110.0, 320.0, 320.0, solid(t.secondary));
    b.text("Distance", 790.0, 185.0, 320.0, "5K", t.h(140.0, t.dark).lh(1.0));
    b.text("Distance label", 790.0, 335.0, 320.0, "fun run", t.b(30.0, t.dark).bold().upper().ls(200.0));
    b.icon("Heart", "heart", 1070.0, 480.0, 80.0, 80.0, solid(t.light));
    b.text("Kicker", 60.0, 70.0, 600.0, "Charity event", t.b(24.0, t.primary.lerp(t.dark, 0.3)).bold().upper().ls(300.0).left());
    b.text("Headline", 60.0, 115.0, 620.0, "Run for\nthe river", t.h(92.0, t.dark).lh(1.0).left());
    b.mark_last("the river", t.primary, true);
    b.text("Details", 60.0, 330.0, 620.0, "Sunday 7 September · 9 am\nStart at Willow Park bandstand", t.b(28.0, t.dark).lh(1.45).left());
    b.text("Cause", 60.0, 430.0, 620.0, "Every entry plants ten trees on the riverbank", t.b(22.0, t.dark.lerp(t.light, 0.2)).left());
    b.pill("Sign-up button", 60.0, 500.0, 300.0, 70.0, solid(t.dark), "Sign up · $15", t.b(28.0, t.light).bold());
}

fn shortcut_tip(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.dark));
    for row in 0..5 {
        for col in 0..6 {
            b.ellipse("Grid dot", 1100.0 + 70.0 * f64::from(col), 400.0 + 70.0 * f64::from(row), 8.0, 8.0, solid(t.light.with_alpha(40)));
        }
    }
    b.text("Kicker", 120.0, 120.0, 900.0, "Tip of the day", t.h(32.0, t.secondary).upper().ls(300.0).left());
    b.text("Headline", 120.0, 180.0, 1300.0, "Paste without the formatting", t.h(80.0, t.light).left());
    let keys = [("Ctrl", 220.0), ("Shift", 260.0), ("V", 160.0)];
    let mut x = 120.0;
    for (i, (key, kw)) in keys.iter().enumerate() {
        b.rect(&format!("{key} key shadow"), x, 400.0, *kw, 170.0, 28.0, solid(t.primary.lerp(t.dark, 0.4)));
        b.rect(&format!("{key} key"), x, 390.0, *kw, 160.0, 28.0, solid(t.primary));
        b.text(&format!("{key} label"), x, 433.0, *kw, key, t.b(60.0, t.light).bold().lh(1.15));
        x += kw;
        if i < keys.len() - 1 {
            b.icon(&format!("Plus {}", i + 1), "plus", x + 30.0, 445.0, 60.0, 60.0, solid(t.secondary));
            x += 120.0;
        }
    }
    b.text("Mac", 120.0, 620.0, 1300.0, "On a Mac: Cmd + Shift + Option + V", t.b(40.0, t.light).left());
    b.mark_last("Cmd + Shift + Option + V", t.secondary, false);
    b.rect("Rule", 120.0, 740.0, 1360.0, 2.0, 1.0, solid(t.light.with_alpha(60)));
    b.text("Handle", 120.0, 775.0, 700.0, "@quickkeys", t.b(32.0, t.light).bold().left());
    b.text("Series", 780.0, 775.0, 700.0, "Tip 12 of 30", t.b(32.0, t.secondary).right());
}

fn versus_thumbnail(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.dark));
    b.shape(
        "Left panel",
        ShapeKind::Path { d: "M0 0 H700 L580 720 H0 Z".into(), view_width: 700.0, view_height: 720.0 },
        0.0,
        0.0,
        700.0,
        720.0,
        solid(t.primary),
    );
    b.shape(
        "Right panel",
        ShapeKind::Path { d: "M120 0 H700 V720 H0 Z".into(), view_width: 700.0, view_height: 720.0 },
        580.0,
        0.0,
        700.0,
        720.0,
        solid(t.secondary),
    );
    b.icon("Camera left", "camera", 130.0, 250.0, 300.0, 300.0, solid(t.light));
    b.icon("Camera right", "camera", 850.0, 250.0, 300.0, 300.0, solid(t.light));
    b.text("Left label", 60.0, 70.0, 520.0, "$400", t.h(110.0, t.light).lh(1.0).left());
    b.effect_last(TextEffect::Shadow { color: genha_doc::Color::rgba(0, 0, 0, 140), offset_x: 5.0, offset_y: 5.0, blur: 0.0 });
    b.text("Right label", 700.0, 70.0, 520.0, "$2,000", t.h(110.0, t.light).lh(1.0).right());
    b.effect_last(TextEffect::Shadow { color: genha_doc::Color::rgba(0, 0, 0, 140), offset_x: 5.0, offset_y: 5.0, blur: 0.0 });
    b.ellipse("Versus disc", 540.0, 260.0, 200.0, 200.0, solid(t.dark));
    b.text("Versus", 540.0, 318.0, 200.0, "VS", t.h(80.0, t.light).lh(1.0));
    b.rect("Question band", 220.0, 590.0, 840.0, 100.0, 20.0, solid(t.light));
    b.text("Question", 220.0, 608.0, 840.0, "Is the pricey one worth it?", t.h(48.0, t.dark).lh(1.3));
}

fn budget_pin(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.text("Kicker", 80.0, 90.0, 840.0, "The 50 / 30 / 20 rule", t.b(32.0, t.primary).bold().upper().ls(200.0).left());
    b.text("Headline", 80.0, 150.0, 840.0, "A monthly budget\nthat stays simple", t.h(76.0, t.dark).lh(1.1).left());
    b.mark_last("simple", t.primary, true);
    let mut c = t.chart(ChartKind::Donut, &["Needs", "Wants", "Savings"], &[("Share", &[50.0, 30.0, 20.0])], 28.0, t.dark);
    c.colors = vec![t.primary, t.secondary, t.dark];
    c.value_format = ValueFormat::Percent;
    c.show_labels = false;
    c.donut_hole = 0.58;
    b.chart("Budget donut", 200.0, 370.0, 600.0, 600.0, c);
    b.ctext("Donut label", 610.0, 300.0, "of your\ntake-home pay", t.b(28.0, t.dark).lh(1.25));
    let rows = [
        ("50%", "Needs", "Rent, bills, food, getting to work", t.primary),
        ("30%", "Wants", "Eating out, hobbies, trips", t.secondary),
        ("20%", "Savings", "Emergency fund first, then goals", t.dark),
    ];
    for (i, (share, title, note, c)) in rows.iter().enumerate() {
        let y = 1030.0 + 120.0 * i as f64;
        b.rect(&format!("{title} swatch"), 80.0, y + 8.0, 70.0, 70.0, 16.0, solid(*c));
        b.text(&format!("{title} share"), 180.0, y, 160.0, share, t.h(52.0, t.dark).left());
        b.text(&format!("{title} title"), 360.0, y + 2.0, 560.0, title, t.b(34.0, t.dark).bold().left());
        b.text(&format!("{title} note"), 360.0, y + 48.0, 560.0, note, t.b(26.0, t.dark.lerp(t.light, 0.2)).left());
    }
    b.ctext("Footer", 1420.0, 840.0, "Save this pin for payday", t.b(30.0, t.primary.lerp(t.dark, 0.3)).bold());
}

fn holiday_greeting(b: &mut B) {
    let t = b.t.clone();
    b.bg(radial(t.light, t.secondary.lerp(t.light, 0.8)));
    // A wreath: leaves around a circle, with berries.
    let (cx, cy, r) = (540.0, 560.0, 260.0);
    for i in 0..18 {
        let a = f64::from(i) * 20.0;
        let (x, y) = (cx + r * a.to_radians().sin(), cy - r * a.to_radians().cos());
        b.icon(
            &format!("Leaf {}", i + 1),
            "leaf",
            x - 45.0,
            y - 45.0,
            90.0,
            90.0,
            solid(if i % 2 == 0 { t.secondary } else { t.secondary.lerp(t.dark, 0.3) }),
        );
        b.rotate_last(a + 45.0);
    }
    for i in 0..6 {
        let a = 30.0 + f64::from(i) * 60.0;
        let (x, y) = (cx + (r - 10.0) * a.to_radians().sin(), cy - (r - 10.0) * a.to_radians().cos());
        b.ellipse(&format!("Berry {}", i + 1), x - 16.0, y - 16.0, 32.0, 32.0, solid(t.primary));
    }
    b.text("Arc greeting", 190.0, 80.0, 700.0, "From all of us to you", t.b(36.0, t.dark).bold().upper().ls(150.0));
    b.curve_last(70.0, 150.0);
    b.ctext("Headline", 480.0, 420.0, "Happy\nholidays", t.h(84.0, t.primary).italic().lh(1.05));
    b.icon("Bow", "ribbon-banner", 390.0, 830.0, 300.0, 110.0, solid(t.primary));
    b.ctext("Signature", 960.0, 700.0, "The Fern & Fig team", t.b(34.0, t.dark).bold());
}

fn daily_special(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.rect("Top panel", 0.0, 0.0, 1080.0, 980.0, 0.0, solid(t.primary));
    b.ellipse("Plate rim", 190.0, 290.0, 700.0, 700.0, solid(t.light));
    b.ellipse("Plate", 260.0, 360.0, 560.0, 560.0, solid(t.secondary.lerp(t.light, 0.85)));
    b.icon("Leaf 1", "leaf", 380.0, 480.0, 150.0, 150.0, solid(t.secondary));
    b.rotate_last(-30.0);
    b.icon("Leaf 2", "leaf", 560.0, 560.0, 130.0, 130.0, solid(t.secondary.lerp(t.dark, 0.25)));
    b.rotate_last(60.0);
    for (i, (x, y, s)) in [(470.0, 650.0, 80.0), (560.0, 470.0, 64.0), (420.0, 760.0, 56.0)].into_iter().enumerate() {
        b.ellipse(&format!("Tomato {}", i + 1), x, y, s, s, solid(t.primary));
    }
    b.ctext("Kicker", 140.0, 900.0, "Today's special", t.b(44.0, t.dark).bold().upper().ls(300.0));
    b.ctext("Dish", 1060.0, 940.0, "Slow-roasted tomato\n& basil risotto", t.h(80.0, t.dark).lh(1.15));
    b.ctext("Price", 1300.0, 600.0, "$16", t.h(110.0, t.primary.lerp(t.dark, 0.25)).lh(1.0));
    b.ctext("Note", 1450.0, 900.0, "Served from noon until it's gone", t.b(40.0, t.dark).lh(1.3));
    b.pill("Order button", 240.0, 1600.0, 600.0, 120.0, solid(t.dark), "Reserve a table", t.b(46.0, t.light).bold());
    b.ctext("Restaurant", 1780.0, 900.0, "Casa Verde · 8 Market Lane", t.b(34.0, t.dark).ls(50.0));
}

fn highlight_quote(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.rect("Corner block", 840.0, 0.0, 240.0, 240.0, 0.0, solid(t.secondary));
    b.icon("Quote marks", "quote-marks", 900.0, 60.0, 120.0, 120.0, solid(t.light));
    b.text("Quote", 90.0, 260.0, 900.0, "Creativity is\nintelligence\nhaving fun.", t.h(116.0, t.dark).lh(1.05).left());
    b.mark_last("intelligence", t.primary, true);
    b.mark_last("fun.", t.secondary, true);
    b.rect("Divider", 90.0, 690.0, 120.0, 10.0, 5.0, solid(t.primary));
    b.text("Author", 90.0, 740.0, 900.0, "A thought for the weekend", t.b(38.0, t.dark).left());
    b.text("Handle", 90.0, 940.0, 600.0, "@yourhandle", t.b(34.0, t.dark).bold().left());
    b.shape("Star", ShapeKind::Star { points: 4, inner_ratio: 0.3 }, 900.0, 880.0, 100.0, 100.0, solid(t.primary));
}
