//! Social media templates: posts, carousels, stories, covers, banners and pins.

use genha_color::Color;
use genha_doc::{Fill, ShapeKind, TextEffect};

use super::{Def, SOCIAL, def, lin, radial, solid};
use crate::build::{B, Sty, Theme};

pub(super) const DEFS: &[Def] = &[
    def(
        "quote-card",
        "Quiet quote card",
        SOCIAL,
        "instagram-post",
        &["quote", "instagram", "minimal", "square", "calm"],
        || Theme::new(0x2f5d50, 0xe8b04b, 0x1d2b26, 0xf4efe6, "Serif", "Sans"),
        quote_card,
    ),
    def(
        "quote-bold",
        "Loud and clear quote",
        SOCIAL,
        "instagram-post",
        &["quote", "instagram", "bold", "motivation", "yellow", "square"],
        || Theme::new(0xffde21, 0xff3864, 0x111111, 0xffffff, "Display", "Sans"),
        quote_bold,
    ),
    def(
        "quote-night",
        "Night thoughts quote",
        SOCIAL,
        "instagram-post",
        &["quote", "instagram", "dark", "moody", "night", "square"],
        || Theme::new(0x4a4e69, 0xc9ada7, 0x0f0e17, 0xf2e9e4, "Serif", "Sans"),
        quote_night,
    ),
    def(
        "quote-pastel",
        "Soft pastel quote",
        SOCIAL,
        "instagram-portrait",
        &["quote", "instagram", "pastel", "soft", "kind", "portrait"],
        || Theme::new(0xb8c0ff, 0xffc8dd, 0x2b2d42, 0xfffafd, "Serif", "Sans"),
        quote_pastel,
    ),
    def(
        "sale-burst",
        "Weekend sale burst",
        SOCIAL,
        "instagram-post",
        &["sale", "discount", "instagram", "shop", "bold"],
        || Theme::new(0xe63946, 0xffd166, 0x1d1d2c, 0xfff8ec, "Display", "Sans"),
        sale_burst,
    ),
    def(
        "event-night",
        "Design night event",
        SOCIAL,
        "instagram-post",
        &["event", "announcement", "instagram", "dark", "tickets"],
        || Theme::new(0x3a0ca3, 0xf72585, 0x10002b, 0xf8f7ff, "Display", "Sans"),
        event_night,
    ),
    def(
        "tips-portrait",
        "Three tips portrait post",
        SOCIAL,
        "instagram-portrait",
        &["tips", "list", "how to", "instagram", "portrait", "education"],
        || Theme::new(0x5b4bdb, 0xffb547, 0x1c1a33, 0xf6f4ff, "Display", "Sans"),
        tips_portrait,
    ),
    def(
        "garden-carousel",
        "Balcony garden carousel",
        SOCIAL,
        "instagram-post",
        &["carousel", "guide", "instagram", "multi-page", "garden", "tips"],
        || Theme::new(0x0f9d7a, 0xffd23f, 0x0b2e26, 0xf1fbf7, "Display", "Sans"),
        garden_carousel,
    ),
    def(
        "numbers-infographic",
        "Year in numbers infographic",
        SOCIAL,
        "instagram-portrait",
        &["infographic", "statistics", "numbers", "report", "recap", "instagram"],
        || Theme::new(0x3a86ff, 0xff006e, 0x14213d, 0xf7f9fc, "Display", "Sans"),
        numbers_infographic,
    ),
    def(
        "story-promo",
        "Fresh drops story",
        SOCIAL,
        "instagram-story",
        &["story", "promo", "instagram", "shop", "vertical"],
        || Theme::new(0xff7a59, 0xffe066, 0x2b2d42, 0xfffaf3, "Display", "Sans"),
        story_promo,
    ),
    def(
        "story-poll",
        "This or that poll story",
        SOCIAL,
        "instagram-story",
        &["story", "poll", "question", "this or that", "instagram", "vertical"],
        || Theme::new(0xff5c8a, 0x4cc9f0, 0x1b1035, 0xfff4f8, "Display", "Sans"),
        story_poll,
    ),
    def(
        "story-countdown",
        "Launch countdown story",
        SOCIAL,
        "instagram-story",
        &["story", "countdown", "launch", "coming soon", "instagram", "vertical"],
        || Theme::new(0xff6b35, 0x2ec4b6, 0x101820, 0xfffaf0, "Display", "Mono"),
        story_countdown,
    ),
    def(
        "story-quote",
        "Slow morning quote story",
        SOCIAL,
        "instagram-story",
        &["story", "quote", "calm", "morning", "instagram", "vertical"],
        || Theme::new(0xc1666b, 0xd4b483, 0x3a2e39, 0xf4f1de, "Serif", "Sans"),
        story_quote,
    ),
    def(
        "tiktok-cover",
        "Day in my life video cover",
        SOCIAL,
        "tiktok-video",
        &["tiktok", "cover", "vlog", "video", "reel", "vertical"],
        || Theme::new(0x8338ec, 0x3df5c4, 0x0d0d14, 0xffffff, "Display", "Sans"),
        tiktok_cover,
    ),
    def(
        "video-thumbnail",
        "Ten tips thumbnail",
        SOCIAL,
        "youtube-thumbnail",
        &["youtube", "thumbnail", "video", "bold"],
        || Theme::new(0xffcc00, 0xff3b30, 0x111111, 0xffffff, "Display", "Sans"),
        video_thumbnail,
    ),
    def(
        "channel-banner",
        "Cozy channel banner",
        SOCIAL,
        "youtube-banner",
        &["youtube", "banner", "channel art", "header", "diy"],
        || Theme::new(0xff9f1c, 0x5bc0eb, 0x1a1423, 0xfdfffc, "Display", "Sans"),
        channel_banner,
    ),
    def(
        "x-header",
        "Writer profile header",
        SOCIAL,
        "x-header",
        &["x", "twitter", "header", "banner", "profile"],
        || Theme::new(0x06d6a0, 0x118ab2, 0x073b4c, 0xf7fff7, "Sans", "Sans"),
        x_header,
    ),
    def(
        "workshop-post",
        "Garden workshop post",
        SOCIAL,
        "facebook-post",
        &["facebook", "community", "workshop", "event"],
        || Theme::new(0x0f766e, 0xf59e0b, 0x0b1f1e, 0xf0fdfa, "Sans", "Sans"),
        workshop_post,
    ),
    def(
        "book-swap-event",
        "Book swap event cover",
        SOCIAL,
        "facebook-post",
        &["facebook", "event cover", "event", "community", "books", "library"],
        || Theme::new(0x3d5a80, 0xee6c4d, 0x293241, 0xe0fbfc, "Display", "Sans"),
        book_swap_event,
    ),
    def(
        "bakery-cover",
        "Bakery page cover",
        SOCIAL,
        "facebook-cover",
        &["facebook", "cover", "bakery", "small business", "cafe", "food"],
        || Theme::new(0xb5651d, 0xf4d35e, 0x3d2914, 0xfff8ec, "Serif", "Sans"),
        bakery_cover,
    ),
    def(
        "hiring-post",
        "We're hiring post",
        SOCIAL,
        "linkedin-post",
        &["linkedin", "hiring", "job", "recruiting", "business", "career"],
        || Theme::new(0x2d6a4f, 0x95d5b2, 0x081c15, 0xf8fff9, "Sans", "Sans"),
        hiring_post,
    ),
    def(
        "profile-banner",
        "Professional profile banner",
        SOCIAL,
        "linkedin-banner",
        &["linkedin", "banner", "profile", "business"],
        || Theme::new(0x1e3a8a, 0x38bdf8, 0x0f172a, 0xf8fafc, "Sans", "Sans"),
        profile_banner,
    ),
    def(
        "habits-pin",
        "Cozy habits pin",
        SOCIAL,
        "pinterest-pin",
        &["pinterest", "list", "tips", "home"],
        || Theme::new(0x3a5a40, 0xdda15e, 0x283618, 0xfefae0, "Serif", "Sans"),
        habits_pin,
    ),
    def(
        "recipe-pin",
        "Tomato pasta recipe pin",
        SOCIAL,
        "pinterest-pin",
        &["pinterest", "recipe", "food", "cooking", "ingredients"],
        || Theme::new(0xe76f51, 0xf4a261, 0x264653, 0xfdf6ec, "Serif", "Sans"),
        recipe_pin,
    ),
    def(
        "packing-pin",
        "Weekend packing list pin",
        SOCIAL,
        "pinterest-pin",
        &["pinterest", "travel", "checklist", "packing", "list"],
        || Theme::new(0x0077b6, 0xffb703, 0x023047, 0xf1f8ff, "Display", "Sans"),
        packing_pin,
    ),
];

// ---------------------------------------------------------------- quotes

fn quote_card(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.outline("Frame", ShapeKind::Rect { corner_radius: 0.0 }, 60.0, 60.0, 960.0, 960.0, t.primary, 4.0);
    b.ctext("Quote mark", 150.0, 200.0, "“", t.h(220.0, t.secondary).lh(1.0));
    b.ctext("Quote", 400.0, 840.0, "Small steps every day\nadd up to big changes.", t.h(64.0, t.dark).italic().lh(1.25).with(|s| s.bold = false));
    b.rect("Divider", 500.0, 620.0, 80.0, 6.0, 3.0, solid(t.secondary));
    b.ctext("Author", 660.0, 500.0, "— Your Name", t.b(34.0, t.primary).upper().ls(120.0).bold());
    b.ctext("Handle", 880.0, 400.0, "@yourhandle", t.b(28.0, t.primary));
}

fn quote_bold(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.primary));
    b.rect("Marker stroke", 84.0, 640.0, 640.0, 54.0, 6.0, solid(t.secondary));
    b.rotate_last(-2.0);
    b.text("Quote", 90.0, 120.0, 900.0, "Done is\nbetter\nthan\nperfect", t.h(150.0, t.dark).upper().lh(0.95).left());
    b.text("Context", 90.0, 840.0, 640.0, "A reminder for Monday", t.b(38.0, t.dark).left());
    b.text("Handle", 90.0, 900.0, 640.0, "@yourhandle", t.b(32.0, t.dark).bold().left());
    b.icon("Arrow", "arrow-diagonal", 830.0, 810.0, 160.0, 160.0, solid(t.dark));
}

fn quote_night(b: &mut B) {
    let t = b.t.clone();
    b.bg(lin(90.0, t.dark, t.primary));
    b.icon("Moon", "moon", 800.0, 110.0, 170.0, 170.0, solid(t.secondary));
    for (i, (x, y, s)) in
        [(150.0, 170.0, 44.0), (300.0, 110.0, 24.0), (660.0, 230.0, 30.0), (200.0, 880.0, 28.0), (880.0, 860.0, 40.0)].into_iter().enumerate()
    {
        b.icon(&format!("Star {}", i + 1), "sparkle", x, y, s, s, solid(t.light.with_alpha(if i % 2 == 0 { 220 } else { 140 })));
    }
    b.ctext("Quote", 380.0, 960.0, "The quiet hours\nare where ideas\ngrow roots.", t.h(72.0, t.light).italic().lh(1.25).with(|s| s.bold = false));
    b.rect("Divider", 500.0, 720.0, 80.0, 4.0, 2.0, solid(t.secondary));
    b.ctext("Author", 760.0, 600.0, "Night notes", t.b(32.0, t.secondary).bold().upper().ls(300.0));
}

fn quote_pastel(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.primary.lerp(t.light, 0.85)));
    b.icon("Blob top", "blob-2", 40.0, 60.0, 560.0, 560.0, solid(t.primary));
    b.icon("Blob bottom", "blob-3", 520.0, 760.0, 500.0, 500.0, solid(t.secondary));
    b.rect("Card", 140.0, 360.0, 800.0, 620.0, 48.0, solid(t.light));
    b.ctext("Quote", 470.0, 760.0, "Be gentle with\nyourself, you're\ndoing your best.", t.h(64.0, t.dark).italic().lh(1.3).with(|s| s.bold = false));
    b.icon("Heart", "heart", 500.0, 780.0, 80.0, 80.0, solid(t.secondary.lerp(t.dark, 0.25)));
    b.ctext("Author", 880.0, 600.0, "Kind words", t.b(30.0, t.dark.lerp(t.light, 0.3)).bold().upper().ls(250.0));
    b.icon("Sparkle", "sparkle", 880.0, 140.0, 90.0, 90.0, solid(t.dark));
    b.ctext("Handle", 1240.0, 500.0, "@softnotes", t.b(30.0, t.dark).bold());
}

// ---------------------------------------------------------------- posts

fn sale_burst(b: &mut B) {
    let t = b.t.clone();
    b.bg(lin(135.0, t.primary, t.primary.lerp(t.dark, 0.35)));
    b.shape("Burst", ShapeKind::Star { points: 16, inner_ratio: 0.84 }, 200.0, 130.0, 680.0, 680.0, solid(t.secondary));
    b.ctext("Kicker", 290.0, 520.0, "Summer", t.h(64.0, t.dark).upper().ls(200.0));
    b.ctext("Discount", 372.0, 560.0, "50% OFF", t.h(120.0, t.dark).lh(1.0));
    b.ctext("Subline", 525.0, 480.0, "On everything in store", t.b(36.0, t.dark));
    b.rect("Dates pill", 340.0, 880.0, 400.0, 80.0, 40.0, solid(t.dark));
    b.ctext("Dates", 899.0, 380.0, "This weekend only", t.b(34.0, t.light).bold().lh(1.15));
    b.ctext("Website", 985.0, 400.0, "yourshop.com", t.b(30.0, t.light).bold());
}

fn event_night(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.dark));
    b.ellipse("Glow", 140.0, 140.0, 900.0, 900.0, radial(t.primary, t.dark));
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

fn tips_portrait(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.ellipse("Accent circle", 800.0, 70.0, 200.0, 200.0, solid(t.secondary));
    b.icon("Accent icon", "lightbulb", 850.0, 120.0, 100.0, 100.0, solid(t.dark));
    b.text("Kicker", 90.0, 100.0, 600.0, "Quick guide", t.b(32.0, t.primary).bold().upper().ls(250.0).left());
    b.text("Headline", 90.0, 160.0, 900.0, "3 ways to\nfocus better", t.h(104.0, t.dark).lh(1.05).left());
    let steps = [
        ("Pick one task", "Write it down before you open your inbox."),
        ("Set a timer", "Work for 25 minutes, then rest for five."),
        ("Mute the noise", "Notifications off until the timer rings."),
    ];
    for (i, (title, body)) in steps.iter().enumerate() {
        let y = 450.0 + 260.0 * i as f64;
        let n = i + 1;
        b.rect(&format!("Step {n} card"), 90.0, y, 900.0, 220.0, 32.0, solid(t.primary.lerp(t.light, 0.88)));
        b.ellipse(&format!("Step {n} disc"), 130.0, y + 50.0, 120.0, 120.0, solid(if i == 1 { t.secondary } else { t.primary }));
        b.text(&format!("Step {n} number"), 130.0, y + 76.0, 120.0, &n.to_string(), t.h(60.0, if i == 1 { t.dark } else { t.light }).lh(1.0));
        b.text(&format!("Step {n} title"), 290.0, y + 52.0, 680.0, title, t.h(48.0, t.dark).left());
        b.text(&format!("Step {n} text"), 290.0, y + 120.0, 680.0, body, t.b(30.0, t.dark.lerp(t.light, 0.25)).left());
    }
    b.text("Handle", 90.0, 1240.0, 400.0, "@yourhandle", t.b(30.0, t.primary).bold().left());
    b.text("Save hint", 560.0, 1240.0, 360.0, "Save for later", t.b(30.0, t.dark).right());
    b.icon("Bookmark", "bookmark", 940.0, 1236.0, 44.0, 44.0, solid(t.primary));
}

fn garden_carousel(b: &mut B) {
    let t = b.t.clone();
    let dots = |b: &mut B, active: usize, on: Color, off: Color| {
        for i in 0..5 {
            let c = if i == active { on } else { off };
            b.ellipse(&format!("Page dot {}", i + 1), 90.0 + 30.0 * i as f64, 1000.0, 14.0, 14.0, solid(c));
        }
    };
    b.name_page("Cover");
    b.bg(solid(t.primary.lerp(t.dark, 0.35)));
    b.ellipse("Leaf disc", 640.0, 470.0, 380.0, 380.0, solid(t.secondary));
    b.icon("Leaf", "leaf", 735.0, 565.0, 190.0, 190.0, solid(t.primary.lerp(t.dark, 0.3)));
    b.text("Kicker", 90.0, 130.0, 700.0, "A 3-step guide", t.b(34.0, t.secondary).bold().upper().ls(200.0).left());
    b.text("Headline", 90.0, 190.0, 760.0, "Start a\nbalcony\ngarden", t.h(120.0, t.light).lh(1.05).left());
    b.text("Swipe hint", 90.0, 900.0, 500.0, "Swipe for the steps", t.b(34.0, t.light).left());
    b.icon("Swipe arrow", "chevron-right", 940.0, 960.0, 60.0, 60.0, solid(t.secondary));
    dots(b, 0, t.light, t.light.with_alpha(110));

    let steps = [
        (
            "Pick the sunny side",
            "Most herbs and vegetables want six hours\nof sun. Watch your balcony for a day\nbefore you buy a single pot.",
            "sun",
            "South and west sides get the most light.",
        ),
        (
            "Start with easy wins",
            "Mint, basil, chives and cherry tomatoes\nforgive beginner mistakes and grow\nhappily in small containers.",
            "flower",
            "Give mint a pot of its own: it spreads.",
        ),
        (
            "Water little and often",
            "Check the soil every morning: if the top\ncentimetre is dry, give it a drink.\nPots dry out faster than beds.",
            "cloud",
            "Water in the morning, not at midday.",
        ),
    ];
    for (i, (title, body, icon, tip)) in steps.iter().enumerate() {
        let n = i + 1;
        b.new_page(&format!("Step {n}"));
        b.bg(solid(t.light));
        b.rect("Progress track", 90.0, 90.0, 900.0, 10.0, 5.0, solid(t.primary.lerp(t.light, 0.8)));
        b.rect("Progress", 90.0, 90.0, 900.0 * (n as f64 + 1.0) / 5.0, 10.0, 5.0, solid(t.primary));
        b.text("Step number", 90.0, 170.0, 500.0, &format!("0{n}"), t.h(220.0, t.primary).lh(1.0).left());
        b.ellipse("Icon disc", 760.0, 170.0, 230.0, 230.0, solid(t.secondary));
        b.icon("Icon", icon, 815.0, 225.0, 120.0, 120.0, solid(t.dark));
        b.text("Title", 90.0, 470.0, 900.0, title, t.h(72.0, t.dark).left());
        b.text("Body", 90.0, 580.0, 900.0, body, t.b(38.0, t.dark.lerp(t.light, 0.2)).lh(1.5).left());
        b.rect("Tip card", 90.0, 810.0, 900.0, 120.0, 24.0, solid(t.secondary.lerp(t.light, 0.6)));
        b.icon("Tip icon", "lightbulb", 125.0, 840.0, 60.0, 60.0, solid(t.primary));
        b.text("Tip", 215.0, 849.0, 760.0, tip, t.b(34.0, t.dark).bold().left());
        dots(b, n, t.primary, t.primary.lerp(t.light, 0.7));
    }

    b.new_page("Follow");
    b.bg(solid(t.dark));
    b.icon("Sparkles", "sparkles", 450.0, 150.0, 180.0, 180.0, solid(t.secondary));
    b.ctext("Headline", 380.0, 960.0, "Ready to grow?", t.h(96.0, t.light));
    b.ctext("Body", 510.0, 900.0, "Follow for a new guide every week", t.b(38.0, t.light.lerp(t.dark, 0.25)));
    b.pill("Save button", 340.0, 620.0, 400.0, 100.0, solid(t.secondary), "Save this post", t.b(38.0, t.dark).bold());
    b.ctext("Handle", 800.0, 500.0, "@balconygreens", t.b(34.0, t.primary).bold());
    dots(b, 4, t.light, t.light.with_alpha(90));
}

fn numbers_infographic(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.text("Kicker", 90.0, 90.0, 600.0, "2026 recap", t.b(32.0, t.primary).bold().upper().ls(300.0).left());
    b.text("Headline", 90.0, 140.0, 900.0, "Our year\nin numbers", t.h(96.0, t.dark).left());
    b.icon("Chart icon", "chart-bars", 840.0, 100.0, 150.0, 150.0, solid(t.secondary));
    let stats = [
        ("users", "3,200", "new members joined", 0.64),
        ("calendar", "48", "workshops held", 0.48),
        ("leaf", "1,150", "trees planted", 0.77),
        ("heart", "96%", "would recommend us", 0.96),
    ];
    for (i, (icon, value, label, share)) in stats.iter().enumerate() {
        let y = 420.0 + 200.0 * i as f64;
        let n = i + 1;
        let c = if i % 2 == 0 { t.primary } else { t.secondary };
        b.ellipse(&format!("Stat {n} disc"), 90.0, y, 120.0, 120.0, solid(c));
        b.icon(&format!("Stat {n} icon"), icon, 120.0, y + 30.0, 60.0, 60.0, solid(t.light));
        b.text(&format!("Stat {n} value"), 250.0, y - 6.0, 500.0, value, t.h(72.0, t.dark).left());
        b.text(&format!("Stat {n} label"), 250.0, y + 78.0, 700.0, label, t.b(30.0, t.dark.lerp(t.light, 0.3)).left());
        b.rect(&format!("Stat {n} track"), 250.0, y + 136.0, 740.0, 14.0, 7.0, solid(c.lerp(t.light, 0.8)));
        b.rect(&format!("Stat {n} bar"), 250.0, y + 136.0, 740.0 * share, 14.0, 7.0, solid(c));
    }
    b.text("Footer", 90.0, 1240.0, 900.0, "Thank you for growing with us", t.b(32.0, t.primary).bold().left());
}

// ---------------------------------------------------------------- stories

fn story_promo(b: &mut B) {
    let t = b.t.clone();
    b.bg(lin(90.0, t.light, t.secondary.lerp(t.light, 0.5)));
    b.ellipse("Sun", 220.0, 180.0, 640.0, 640.0, lin(90.0, t.secondary, t.primary));
    b.ellipse("Sun ring", 300.0, 260.0, 480.0, 480.0, Fill::None);
    b.dash_last(t.light.with_alpha(160), 6.0, &[18.0, 14.0]);
    b.ctext("Kicker", 900.0, 900.0, "New collection", t.b(44.0, t.dark).bold().upper().ls(250.0));
    b.ctext("Headline", 970.0, 920.0, "Fresh Drops\nEvery Friday", t.h(110.0, t.dark).lh(1.05));
    b.rect("Ground", 0.0, 1320.0, 1080.0, 600.0, 0.0, solid(t.dark));
    b.ctext("Body", 1420.0, 900.0, "Swipe up to shop the full range", t.b(44.0, t.light));
    b.rect("Button", 290.0, 1560.0, 500.0, 120.0, 60.0, solid(t.primary));
    b.ctext("Button label", 1590.0, 460.0, "Shop now", t.b(48.0, t.dark).bold().lh(1.15));
    b.icon("Swipe arrow", "arrow-up", 500.0, 1750.0, 80.0, 80.0, solid(t.light));
}

fn story_poll(b: &mut B) {
    let t = b.t.clone();
    b.bg(lin(90.0, t.dark, t.primary.lerp(t.dark, 0.55)));
    b.ctext("Kicker", 190.0, 900.0, "Quick poll", t.b(40.0, t.secondary).bold().upper().ls(300.0));
    b.ctext("Headline", 260.0, 980.0, "Early bird or\nnight owl?", t.h(104.0, t.light));
    let options = [("Early bird", "Up with the sun", "sun", t.primary, t.dark), ("Night owl", "Best ideas at 1 am", "moon", t.secondary, t.dark)];
    for (i, (title, sub, icon, fill, ink)) in options.iter().enumerate() {
        let y = 600.0 + 520.0 * i as f64;
        let n = if i == 0 { "A" } else { "B" };
        b.rect(&format!("Option {n} card"), 120.0, y, 840.0, 440.0, 56.0, solid(*fill));
        b.icon(&format!("Option {n} icon"), icon, 190.0, y + 100.0, 240.0, 240.0, solid(*ink));
        b.text(&format!("Option {n} title"), 480.0, y + 140.0, 450.0, title, t.h(72.0, *ink).left());
        b.text(&format!("Option {n} text"), 480.0, y + 240.0, 450.0, sub, t.b(36.0, *ink).left());
    }
    b.ellipse("Versus disc", 465.0, 1005.0, 150.0, 150.0, solid(t.light));
    b.text("Versus", 465.0, 1052.0, 150.0, "VS", t.h(52.0, t.dark).lh(1.0));
    b.ctext("Footer", 1660.0, 900.0, "Vote with the poll sticker below", t.b(38.0, t.light));
    b.icon("Down arrow", "arrow-down", 505.0, 1740.0, 70.0, 70.0, solid(t.secondary));
}

fn story_countdown(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.dark));
    b.icon("Sparkles", "sparkles", 800.0, 130.0, 180.0, 180.0, solid(t.secondary));
    b.text("Kicker", 90.0, 260.0, 700.0, "Coming soon", t.b(40.0, t.secondary).bold().upper().ls(200.0).left());
    b.text("Headline", 90.0, 330.0, 900.0, "The new\ncollection", t.h(130.0, t.light).lh(1.05).left());
    b.text("Lead", 90.0, 650.0, 900.0, "drops in", t.b(48.0, t.light.lerp(t.dark, 0.3)).left());
    for (i, (value, label)) in [("03", "Days"), ("12", "Hours"), ("45", "Minutes")].iter().enumerate() {
        let x = 90.0 + 310.0 * i as f64;
        b.rect(&format!("{label} box"), x, 780.0, 280.0, 300.0, 32.0, solid(t.primary.lerp(t.dark, 0.3)));
        b.text(&format!("{label} value"), x, 830.0, 280.0, value, t.h(150.0, t.light).lh(1.0));
        b.text(&format!("{label} label"), x, 1110.0, 280.0, label, t.b(34.0, t.light).upper().ls(150.0));
    }
    b.text("Date", 90.0, 1250.0, 900.0, "Friday · 10 am", t.h(64.0, t.primary).left());
    b.text("Reminder hint", 90.0, 1340.0, 900.0, "Set a reminder so you don't miss it", t.b(36.0, t.light).left());
    b.pill("Reminder button", 90.0, 1460.0, 460.0, 110.0, solid(t.secondary), "Remind me", t.b(44.0, t.dark).bold());
    b.outline("Ring", ShapeKind::Ellipse, 700.0, 1480.0, 300.0, 300.0, t.primary, 6.0);
    b.icon("Bell", "bell", 790.0, 1570.0, 120.0, 120.0, solid(t.light));
}

fn story_quote(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.icon("Blob", "blob-1", 600.0, 110.0, 420.0, 420.0, solid(t.primary.lerp(t.light, 0.55)));
    b.icon("Small blob", "blob-3", 60.0, 1300.0, 260.0, 260.0, solid(t.secondary.lerp(t.light, 0.4)));
    b.icon("Quote marks", "quote-marks", 440.0, 520.0, 200.0, 200.0, solid(t.secondary));
    b.ctext("Quote", 760.0, 960.0, "Slow mornings\nmake for\nkind days.", t.h(92.0, t.dark).italic().lh(1.2).with(|s| s.bold = false));
    b.rect("Divider", 500.0, 1160.0, 80.0, 6.0, 3.0, solid(t.secondary));
    b.ctext("Author", 1200.0, 800.0, "Notes to self", t.b(36.0, t.primary).bold().upper().ls(250.0));
    b.icon("Wave", "wave-divider", 0.0, 1600.0, 1080.0, 320.0, solid(t.primary));
    b.rect("Wave base", 0.0, 1855.0, 1080.0, 65.0, 0.0, solid(t.primary));
    b.ctext("Handle", 1790.0, 600.0, "@yourhandle", t.b(34.0, t.light).bold());
}

fn tiktok_cover(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.dark));
    b.ellipse("Glow", 90.0, 300.0, 900.0, 900.0, radial(t.primary.lerp(t.dark, 0.2), t.dark));
    b.icon("Sparkle", "sparkle", 150.0, 420.0, 90.0, 90.0, solid(t.secondary));
    b.ctext("Episode", 610.0, 800.0, "Vlog · episode 12", t.b(40.0, t.dark).bold().upper().ls(150.0));
    b.effect_last(TextEffect::Highlight { color: t.secondary, padding: 14.0, corner_radius: 10.0 });
    b.icon("Brush stroke", "brush-stroke", 40.0, 690.0, 1000.0, 420.0, solid(t.primary));
    b.ctext("Title", 730.0, 980.0, "A day in\nmy life", t.h(150.0, t.light).upper().lh(1.0));
    b.icon("Play", "play-button", 440.0, 1180.0, 200.0, 200.0, solid(t.secondary));
    b.ctext("Subtitle", 1440.0, 900.0, "Studio edition · part 1", t.b(44.0, t.light));
    b.ctext("Handle", 1530.0, 600.0, "@yourname", t.b(36.0, t.light.lerp(t.dark, 0.4)));
    b.icon("Small sparkle", "sparkle", 860.0, 1250.0, 70.0, 70.0, solid(t.light));
}

// ---------------------------------------------------------------- covers and banners

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

fn channel_banner(b: &mut B) {
    // Everything that matters sits in the 1546 × 423 area every device shows (centred).
    let t = b.t.clone();
    b.bg(lin(0.0, t.dark, t.dark.lerp(t.primary, 0.3)));
    b.ellipse("Big circle", 60.0, 60.0, 560.0, 560.0, solid(t.primary));
    b.icon("Home icon", "home", 230.0, 230.0, 220.0, 220.0, solid(t.dark));
    b.outline("Ring", ShapeKind::Ellipse, 1960.0, 760.0, 520.0, 520.0, t.secondary, 24.0);
    b.icon("Blob", "blob-2", 2120.0, 140.0, 360.0, 360.0, solid(t.secondary.lerp(t.dark, 0.35)));
    b.icon("Sparkles", "sparkles", 300.0, 1000.0, 220.0, 220.0, solid(t.secondary));
    b.ctext("Channel name", 540.0, 1500.0, "Make It Cozy", t.h(150.0, t.light));
    b.ctext("Tagline", 735.0, 1500.0, "New DIY home videos every Tuesday", t.b(56.0, t.secondary));
    b.pill("Subscribe pill", 1080.0, 830.0, 400.0, 90.0, solid(t.primary), "Subscribe", t.b(44.0, t.dark).bold());
}

fn x_header(b: &mut B) {
    let t = b.t.clone();
    b.bg(lin(0.0, t.dark, t.primary.lerp(t.dark, 0.75)));
    for row in 0..4 {
        for col in 0..6 {
            let (x, y) = (60.0 + 36.0 * f64::from(col), 50.0 + 36.0 * f64::from(row));
            b.ellipse("Dot", x, y, 10.0, 10.0, solid(t.light.with_alpha(70)));
        }
    }
    b.text("Headline", 480.0, 100.0, 940.0, "Design systems,\ntiny tools & tea", t.h(72.0, t.light).right());
    b.text("Subline", 480.0, 290.0, 940.0, "Notes from a product designer · new post every Monday", t.b(30.0, t.primary).right());
    b.icon("Wave", "wave-divider", 0.0, 360.0, 1500.0, 140.0, solid(t.secondary));
    b.rect("Wave base", 0.0, 471.0, 1500.0, 29.0, 0.0, solid(t.secondary));
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

fn book_swap_event(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.rect("Date block", 0.0, 0.0, 380.0, 630.0, 0.0, solid(t.primary));
    b.text("Month", 0.0, 160.0, 380.0, "Sep", t.b(48.0, t.light).bold().upper().ls(300.0));
    b.text("Day", 0.0, 225.0, 380.0, "14", t.h(180.0, t.light).lh(1.0));
    b.text("Weekday", 0.0, 440.0, 380.0, "Saturday", t.b(36.0, t.light));
    b.icon("Book", "book", 960.0, 60.0, 170.0, 170.0, solid(t.secondary));
    b.text("Kicker", 440.0, 110.0, 520.0, "Free community event", t.b(26.0, t.secondary.lerp(t.dark, 0.3)).bold().upper().ls(200.0).left());
    b.text("Headline", 440.0, 160.0, 700.0, "Community\nBook Swap", t.h(80.0, t.dark).left());
    b.text(
        "Details",
        440.0,
        385.0,
        700.0,
        "Bring a book, take a book.\n10 am – 2 pm · Riverside Library",
        t.b(30.0, t.dark.lerp(t.light, 0.2)).lh(1.45).left(),
    );
    b.pill("Going button", 440.0, 500.0, 260.0, 72.0, solid(t.secondary), "I'm going", t.b(30.0, t.dark).bold());
}

fn bakery_cover(b: &mut B) {
    // The middle 1000 px carry the text: phones crop the sides of a page cover.
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.ellipse("Left circle", 40.0, 120.0, 320.0, 320.0, solid(t.primary.lerp(t.light, 0.82)));
    b.icon("Cake", "cake", 110.0, 190.0, 180.0, 180.0, solid(t.primary));
    b.ellipse("Right circle", 1280.0, 120.0, 320.0, 320.0, solid(t.secondary.lerp(t.light, 0.5)));
    b.icon("Coffee", "coffee", 1350.0, 190.0, 180.0, 180.0, solid(t.primary.lerp(t.dark, 0.3)));
    b.ctext("Kicker", 120.0, 700.0, "Since 1998", t.b(28.0, t.primary).bold().upper().ls(300.0));
    b.ctext("Name", 175.0, 1000.0, "Golden Crust Bakery", t.h(70.0, t.dark));
    b.ctext("Tagline", 290.0, 1000.0, "Fresh bread, slow coffee, good mornings", t.b(34.0, t.dark.lerp(t.light, 0.25)));
    b.rect("Divider", 780.0, 370.0, 80.0, 6.0, 3.0, solid(t.secondary));
    b.rect("Info band", 0.0, 520.0, 1640.0, 104.0, 0.0, solid(t.primary));
    b.ctext("Opening hours", 552.0, 1000.0, "Open daily 7 am – 6 pm · 12 Mill Street", t.b(30.0, t.light));
}

fn hiring_post(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.rect("Side panel", 720.0, 0.0, 480.0, 627.0, 0.0, solid(t.primary));
    b.ellipse("Icon disc", 800.0, 110.0, 320.0, 320.0, solid(t.secondary));
    b.icon("People", "users", 870.0, 180.0, 180.0, 180.0, solid(t.dark));
    b.text("Kicker", 70.0, 110.0, 600.0, "We're hiring", t.b(28.0, t.primary).bold().upper().ls(250.0).left());
    b.text("Headline", 70.0, 160.0, 620.0, "Product\nDesigner", t.h(84.0, t.dark).left());
    b.text(
        "Details",
        70.0,
        385.0,
        620.0,
        "Remote-friendly · Full time\nLisbon or anywhere in Europe",
        t.b(28.0, t.dark.lerp(t.light, 0.25)).lh(1.45).left(),
    );
    b.pill("Apply button", 70.0, 500.0, 280.0, 70.0, solid(t.secondary), "Apply today", t.b(28.0, t.dark).bold());
    b.text("Link", 740.0, 520.0, 440.0, "company.example/jobs", t.b(26.0, t.light));
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

// ---------------------------------------------------------------- pins

fn habits_pin(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.rect("Header panel", 0.0, 0.0, 1000.0, 620.0, 0.0, lin(90.0, t.primary, t.primary.lerp(t.dark, 0.4)));
    b.ellipse("Accent circle", 760.0, 400.0, 160.0, 160.0, solid(t.secondary));
    b.text("Kicker", 80.0, 130.0, 560.0, "5 easy habits", t.b(34.0, t.secondary).bold().upper().ls(300.0).left());
    b.text("Headline", 80.0, 200.0, 840.0, "For a calmer,\ncozier home", t.h(84.0, t.light).left());
    let items = [
        ("Open the windows early", "Fresh air resets the whole room"),
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
        b.text(&format!("Habit {n}"), 190.0, y + 2.0, 760.0, title, t.h(38.0, t.dark).left());
        b.text(&format!("Habit {n} note"), 190.0, y + 52.0, 760.0, note, t.b(28.0, t.dark.lerp(t.light, 0.25)).left());
    }
    b.ctext("Website", 1410.0, 400.0, "yourblog.com", t.b(28.0, t.primary).bold());
}

fn recipe_pin(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.ellipse("Plate rim", 250.0, 80.0, 500.0, 500.0, solid(t.secondary.lerp(t.light, 0.55)));
    b.ellipse("Plate", 300.0, 130.0, 400.0, 400.0, solid(t.light));
    for (i, (x, y, s)) in [(380.0, 230.0, 90.0), (520.0, 260.0, 80.0), (430.0, 360.0, 84.0), (560.0, 380.0, 66.0)].into_iter().enumerate() {
        b.ellipse(&format!("Tomato {}", i + 1), x, y, s, s, solid(t.primary));
    }
    b.icon("Basil leaf", "leaf", 330.0, 330.0, 90.0, 90.0, solid(t.dark.lerp(t.light, 0.25)));
    b.icon("Basil leaf", "leaf", 600.0, 190.0, 80.0, 80.0, solid(t.dark.lerp(t.light, 0.25)));
    b.rotate_last(80.0);
    b.ctext("Kicker", 640.0, 800.0, "15-minute dinner", t.b(30.0, t.primary.lerp(t.dark, 0.3)).bold().upper().ls(300.0));
    b.ctext("Title", 690.0, 900.0, "Sunny tomato\npasta", t.h(84.0, t.dark));
    b.rect("Ingredients box", 100.0, 930.0, 800.0, 420.0, 32.0, solid(t.primary.lerp(t.light, 0.88)));
    b.text("Ingredients heading", 150.0, 965.0, 700.0, "You'll need", t.h(40.0, t.dark).left());
    let items = ["200 g spaghetti", "A punnet of cherry tomatoes", "2 cloves of garlic", "A handful of fresh basil", "Olive oil, salt and pepper"];
    for (i, item) in items.iter().enumerate() {
        let y = 1040.0 + 56.0 * i as f64;
        b.ellipse(&format!("Bullet {}", i + 1), 150.0, y + 12.0, 18.0, 18.0, solid(t.secondary));
        b.text(&format!("Ingredient {}", i + 1), 190.0, y, 680.0, item, t.b(32.0, t.dark).left());
    }
    b.ctext("Footer", 1400.0, 800.0, "Full recipe on the blog", t.b(30.0, t.primary.lerp(t.dark, 0.3)).bold());
}

fn packing_pin(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.rect("Header", 0.0, 0.0, 1000.0, 560.0, 0.0, lin(135.0, t.primary.lerp(t.dark, 0.25), t.primary.lerp(t.dark, 0.5)));
    b.icon("Plane", "plane", 720.0, 60.0, 200.0, 200.0, solid(t.secondary));
    b.rotate_last(45.0);
    b.icon("Cloud", "cloud", 80.0, 430.0, 160.0, 110.0, solid(t.light.with_alpha(210)));
    b.icon("Small cloud", "cloud", 780.0, 400.0, 130.0, 90.0, solid(t.light.with_alpha(140)));
    b.text("Kicker", 80.0, 110.0, 600.0, "Carry-on only", t.b(32.0, t.secondary).bold().upper().ls(300.0).left());
    b.text("Headline", 80.0, 170.0, 840.0, "Weekend\npacking list", t.h(96.0, t.light).left());
    let items = [
        "Passport and tickets",
        "Phone charger",
        "Two easy outfits",
        "Comfy walking shoes",
        "Toiletries in a clear bag",
        "Reusable water bottle",
        "A good book",
        "Snacks for the road",
    ];
    for (i, item) in items.iter().enumerate() {
        let y = 610.0 + 90.0 * i as f64;
        b.icon(
            &format!("Check {}", i + 1),
            "check-circle",
            100.0,
            y,
            54.0,
            54.0,
            solid(if i < 3 { t.primary } else { t.primary.lerp(t.light, 0.6) }),
        );
        b.text(&format!("Item {}", i + 1), 180.0, y + 6.0, 740.0, item, t.b(36.0, t.dark).left());
    }
    b.rect("Footer", 0.0, 1380.0, 1000.0, 120.0, 0.0, solid(t.secondary));
    b.ctext("Footer text", 1418.0, 800.0, "Save this pin for your next trip", t.b(32.0, t.dark).bold());
}
