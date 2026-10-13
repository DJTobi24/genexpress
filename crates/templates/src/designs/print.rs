//! Print templates: flyers, posters, menus and price lists.

use genha_doc::{Fill, ShapeKind};

use super::{Def, PRINT, def, lin, solid};
use crate::build::{B, Sty, Theme};

pub(super) const DEFS: &[Def] = &[
    def(
        "studio-flyer",
        "Open studio flyer",
        PRINT,
        "flyer-a4",
        &["flyer", "event", "print", "a4", "art"],
        || Theme::new(0x6d28d9, 0xfacc15, 0x1e1b4b, 0xfaf5ff, "Display", "Sans"),
        studio_flyer,
    ),
    def(
        "concert-flyer",
        "Live at the Vault concert flyer",
        PRINT,
        "flyer-a4",
        &["flyer", "concert", "music", "gig", "night", "a4", "dark"],
        || Theme::new(0xff4d6d, 0xffd60a, 0x0b090a, 0xf5f3f4, "Display", "Sans"),
        concert_flyer,
    ),
    def(
        "job-flyer",
        "Barista wanted job flyer",
        PRINT,
        "flyer-a4",
        &["flyer", "job", "hiring", "vacancy", "cafe", "a4"],
        || Theme::new(0x4361ee, 0x4cc9f0, 0x14112b, 0xf5f7ff, "Sans", "Sans"),
        job_flyer,
    ),
    def(
        "garage-sale-flyer",
        "Garage sale flyer",
        PRINT,
        "flyer-letter",
        &["flyer", "garage sale", "yard sale", "sale", "community", "letter"],
        || Theme::new(0x2b9348, 0xffba08, 0x1b1b1b, 0xfffdf5, "Display", "Sans"),
        garage_sale,
    ),
    def(
        "lost-pet-flyer",
        "Lost cat flyer",
        PRINT,
        "flyer-letter",
        &["flyer", "lost pet", "missing", "cat", "dog", "tear-off", "letter"],
        || Theme::new(0xd00000, 0x3f88c5, 0x1d1d1d, 0xffffff, "Display", "Sans"),
        lost_pet,
    ),
    def(
        "yoga-flyer",
        "Sunrise yoga flyer",
        PRINT,
        "flyer-a5",
        &["flyer", "yoga", "fitness", "class", "wellness", "a5"],
        || Theme::new(0x8e7dbe, 0xf2b5d4, 0x2e294e, 0xfaf7ff, "Serif", "Sans"),
        yoga_flyer,
    ),
    def(
        "festival-poster",
        "Sound waves poster",
        PRINT,
        "poster-a3",
        &["poster", "music", "festival", "geometric"],
        || Theme::new(0xef476f, 0x06d6a0, 0x073b4c, 0xfffcf2, "Display", "Sans"),
        festival_poster,
    ),
    def(
        "climb-poster",
        "Keep climbing poster",
        PRINT,
        "poster-a3",
        &["poster", "motivational", "inspiration", "mountain", "office"],
        || Theme::new(0x1d3557, 0xe63946, 0x0d1b2a, 0xf1faee, "Display", "Sans"),
        climb_poster,
    ),
    def(
        "exhibition-poster",
        "Forms and colour exhibition poster",
        PRINT,
        "poster-18x24",
        &["poster", "exhibition", "art", "gallery", "museum", "geometric"],
        || Theme::new(0xe07a5f, 0x81b29a, 0x3d405b, 0xf4f1de, "Serif", "Sans"),
        exhibition_poster,
    ),
    def(
        "bistro-menu",
        "Bistro menu",
        PRINT,
        "flyer-a4",
        &["menu", "restaurant", "food", "print"],
        || Theme::new(0x9a3412, 0x65a30d, 0x292524, 0xfefae0, "Serif", "Sans"),
        bistro_menu,
    ),
    def(
        "cafe-menu",
        "Chalkboard café menu",
        PRINT,
        "flyer-letter",
        &["menu", "cafe", "coffee", "chalkboard", "price list", "food"],
        || Theme::new(0xf4a259, 0x8cb369, 0x2f3e46, 0xf4f1bb, "Display", "Sans"),
        cafe_menu,
    ),
    def(
        "salon-price-list",
        "Salon price list",
        PRINT,
        "flyer-a5",
        &["price list", "salon", "beauty", "services", "menu", "a5"],
        || Theme::new(0xa26769, 0xd5b9b2, 0x2f2a2b, 0xfdf8f5, "Serif", "Sans"),
        price_list,
    ),
];

// ---------------------------------------------------------------- flyers

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

fn concert_flyer(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    let cx = w / 2.0;
    b.bg(solid(t.dark));
    for (i, r) in [300.0, 240.0, 180.0].into_iter().enumerate() {
        b.outline(
            &format!("Sound ring {}", i + 1),
            ShapeKind::Ellipse,
            cx - r,
            330.0 - r,
            2.0 * r,
            2.0 * r,
            t.primary.with_alpha(90 + 50 * i as u8),
            4.0,
        );
    }
    b.ellipse("Stage light", cx - 120.0, 210.0, 240.0, 240.0, solid(t.primary));
    b.icon("Microphone", "microphone", cx - 60.0, 270.0, 120.0, 120.0, solid(t.dark));
    b.icon("Sparkle", "sparkle", 640.0, 80.0, 60.0, 60.0, solid(t.secondary));
    b.ctext("Kicker", 660.0, 600.0, "Live music", t.b(24.0, t.secondary).bold().upper().ls(400.0));
    b.ctext("Headline", 700.0, 740.0, "Live at\nThe Vault", t.h(96.0, t.light).upper().lh(1.0));
    b.ctext("Line-up", 925.0, 740.0, "Nova Lane · Static Bloom · DJ Mira", t.b(26.0, t.light.lerp(t.dark, 0.2)));
    for (i, (label, value)) in [("Date", "Fri 21 Nov"), ("Doors", "8 pm"), ("Tickets", "$12")].iter().enumerate() {
        let x = 40.0 + 240.0 * i as f64;
        b.text(&format!("{label} label"), x, 985.0, 230.0, label, t.b(18.0, t.secondary).bold().upper().ls(200.0));
        b.text(&format!("{label} value"), x, 1013.0, 230.0, value, t.b(26.0, t.light).bold());
    }
    b.rect("Footer", 0.0, h - 50.0, w, 50.0, 0.0, solid(t.primary));
    b.ctext("Footer text", h - 38.0, 700.0, "thevault.example · Main Street 42 · 18+", t.b(20.0, t.dark).bold());
}

fn job_flyer(b: &mut B) {
    let t = b.t.clone();
    let w = b.w;
    b.bg(solid(t.light));
    b.rect("Header", 0.0, 0.0, w, 420.0, 0.0, lin(135.0, t.dark, t.primary.lerp(t.dark, 0.5)));
    b.ellipse("Icon disc", 480.0, 90.0, 240.0, 240.0, solid(t.secondary));
    b.icon("Coffee", "coffee", 540.0, 150.0, 120.0, 120.0, solid(t.dark));
    b.text("Kicker", 60.0, 110.0, 400.0, "Join our team", t.b(24.0, t.secondary).bold().upper().ls(300.0).left());
    b.text("Headline", 60.0, 160.0, 420.0, "Barista\nwanted", t.h(84.0, t.light).left());
    b.text(
        "Intro",
        60.0,
        460.0,
        680.0,
        "Small café, big hearts. We're looking for someone\nwho loves coffee and people in equal measure.",
        t.b(20.0, t.dark).lh(1.5).left(),
    );
    let lists = [
        ("What you'll do", 560.0, ["Make great coffee with a smile", "Keep the café bright and tidy", "Help regulars find a new favourite"]),
        ("What we offer", 760.0, ["Fair pay plus a share of tips", "Free drinks and lunch on shift", "Flexible shifts, 20 to 35 hours"]),
    ];
    for (title, y, items) in lists {
        b.text(&format!("{title} heading"), 60.0, y, 600.0, title, t.h(28.0, t.primary).left());
        for (i, item) in items.iter().enumerate() {
            let iy = y + 50.0 + 42.0 * i as f64;
            b.icon(&format!("{title} check {}", i + 1), "check-circle", 60.0, iy, 26.0, 26.0, solid(t.secondary));
            b.text(&format!("{title} item {}", i + 1), 100.0, iy, 620.0, item, t.b(20.0, t.dark).left());
        }
    }
    b.rect("Apply box", 60.0, 960.0, w - 120.0, 110.0, 20.0, solid(t.primary));
    b.icon("Envelope", "envelope", 90.0, 985.0, 60.0, 60.0, solid(t.light));
    b.text("Apply title", 175.0, 983.0, 540.0, "Apply by 30 November", t.h(26.0, t.light).left());
    b.text("Apply text", 175.0, 1022.0, 540.0, "Send a short note to jobs@cafe.example", t.b(20.0, t.light).left());
}

fn garage_sale(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.rect("Header", 0.0, 0.0, w, 360.0, 0.0, solid(t.primary));
    b.icon("Price tag", "tag", 560.0, 70.0, 200.0, 200.0, solid(t.secondary));
    b.rotate_last(-12.0);
    b.text("Kicker", 60.0, 70.0, 480.0, "Everything must go", t.b(24.0, t.light).bold().upper().ls(300.0).left());
    b.text("Headline", 60.0, 110.0, 520.0, "Garage\nsale", t.h(110.0, t.light).upper().lh(0.95).left());
    b.rect("Date banner", 60.0, 400.0, w - 120.0, 90.0, 16.0, solid(t.dark));
    b.ctext("Date", 427.0, w - 140.0, "Saturday 4 October · 8 am – 2 pm", t.b(32.0, t.light).bold());
    let columns = [["Furniture", "Books and records", "Kids' toys", "Kitchenware"], ["Garden tools", "Bikes", "Clothes", "House plants"]];
    for (c, items) in columns.iter().enumerate() {
        let x = 60.0 + 360.0 * c as f64;
        for (i, item) in items.iter().enumerate() {
            let y = 540.0 + 56.0 * i as f64;
            b.icon(&format!("Bullet {}", c * 4 + i + 1), "check", x, y + 6.0, 26.0, 26.0, solid(t.primary));
            b.text(&format!("Item {}", c * 4 + i + 1), x + 40.0, y, 300.0, item, t.b(28.0, t.dark).left());
        }
    }
    b.text("Prices", 60.0, 790.0, w - 120.0, "Prices from $1", t.h(56.0, t.primary).left());
    b.icon("Pin", "location-pin", 60.0, 892.0, 50.0, 50.0, solid(t.secondary));
    b.text("Address", 125.0, 900.0, 560.0, "27 Linden Avenue, Oakridge", t.b(30.0, t.dark).bold().left());
    b.text("Signs", 380.0, 962.0, 280.0, "Follow the signs", t.b(24.0, t.dark).right());
    b.icon("Arrow", "arrow-right", 676.0, 955.0, 80.0, 40.0, solid(t.secondary));
    b.rect("Bottom strip", 0.0, h - 40.0, w, 40.0, 0.0, solid(t.secondary));
}

fn lost_pet(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.rect("Header", 0.0, 0.0, w, 170.0, 0.0, solid(t.primary));
    b.ctext("Headline", 22.0, 760.0, "Lost cat", t.h(110.0, t.light).upper().lh(1.0));
    b.rect("Photo frame", 60.0, 210.0, 360.0, 360.0, 24.0, solid(t.dark.lerp(t.light, 0.9)));
    b.icon("Photo icon", "image", 165.0, 300.0, 150.0, 150.0, solid(t.dark.lerp(t.light, 0.6)));
    b.text("Photo hint", 60.0, 480.0, 360.0, "Add a photo", t.b(20.0, t.dark.lerp(t.light, 0.25)));
    b.text("Name", 450.0, 205.0, 320.0, "Pepper", t.h(64.0, t.dark).left());
    b.text(
        "Description",
        450.0,
        300.0,
        320.0,
        "Grey tabby, green eyes\nWhite paws, blue collar\nVery shy, may hide",
        t.b(22.0, t.dark).lh(1.5).left(),
    );
    b.text("Seen label", 450.0, 430.0, 320.0, "Last seen", t.b(18.0, t.primary).bold().upper().ls(200.0).left());
    b.text("Seen", 450.0, 460.0, 320.0, "Elm Park, 2 October", t.b(24.0, t.dark).bold().left());
    b.rect("Reward banner", 60.0, 600.0, w - 120.0, 90.0, 16.0, solid(t.secondary));
    b.ctext("Reward", 618.0, w - 140.0, "Reward if found", t.h(44.0, t.light));
    b.icon("Phone", "phone", 175.0, 724.0, 56.0, 56.0, solid(t.primary));
    b.text("Phone number", 250.0, 720.0, 520.0, "Call 555 0100 200", t.h(48.0, t.dark).left());
    let strips = 8;
    let sw = w / f64::from(strips);
    for i in 0..strips {
        let x = sw * f64::from(i);
        b.outline(&format!("Tear-off {}", i + 1), ShapeKind::Rect { corner_radius: 0.0 }, x + 4.0, 824.0, sw - 8.0, h - 832.0, t.dark, 1.5);
        b.dash_last(t.dark.with_alpha(140), 1.5, &[6.0, 5.0]);
        let (cx, cy) = (x + sw / 2.0, 824.0 + (h - 832.0) / 2.0);
        let mut e = b.text_el(&format!("Tear-off number {}", i + 1), cx - 105.0, 0.0, 210.0, "Pepper · 555 0100 200", t.b(16.0, t.dark).bold());
        e.frame.y = cy - e.frame.height / 2.0;
        e.frame.rotation = -90.0;
        b.push(e);
    }
}

fn yoga_flyer(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    let aw = w - 200.0;
    let arch = format!("M0 {r} A{r} {r} 0 0 1 {aw} {r} V380 H0 Z", r = aw / 2.0);
    b.shape("Arch", ShapeKind::Path { d: arch, view_width: aw, view_height: 380.0 }, 100.0, 60.0, aw, 380.0, solid(t.primary.lerp(t.light, 0.72)));
    b.ellipse("Sun", w / 2.0 - 80.0, 130.0, 160.0, 160.0, solid(t.secondary));
    b.icon("Flower", "flower", w / 2.0 - 50.0, 300.0, 100.0, 100.0, solid(t.primary));
    b.ctext("Kicker", 470.0, 400.0, "Every Sunday", t.b(16.0, t.primary).bold().upper().ls(400.0));
    b.ctext("Title", 498.0, w - 40.0, "Sunrise Yoga", t.h(52.0, t.dark));
    b.ctext("Body", 572.0, w - 80.0, "Gentle flow for every level\nin the park by the lake", t.b(18.0, t.dark.lerp(t.light, 0.25)).lh(1.5));
    for (i, (time, what)) in [("7:00", "Breathe"), ("7:15", "Flow"), ("8:00", "Rest")].iter().enumerate() {
        let x = 50.0 + 158.0 * i as f64;
        b.rect(&format!("{what} card"), x, 640.0, 142.0, 72.0, 14.0, solid(t.secondary.lerp(t.light, 0.55)));
        b.text(&format!("{what} time"), x, 650.0, 142.0, time, t.b(20.0, t.dark).bold());
        b.text(&format!("{what} label"), x, 680.0, 142.0, what, t.b(15.0, t.dark.lerp(t.light, 0.2)));
    }
    b.rect("Footer", 0.0, h - 50.0, w, 50.0, 0.0, solid(t.primary.lerp(t.dark, 0.3)));
    b.ctext("Footer text", h - 36.0, w - 40.0, "First class free · yogaclub.example", t.b(15.0, t.light).bold());
}

// ---------------------------------------------------------------- posters

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

fn climb_poster(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(lin(90.0, t.light, t.primary.lerp(t.light, 0.55)));
    b.ellipse("Sun", 780.0, 200.0, 180.0, 180.0, solid(t.secondary));
    b.shape("Far mountain", ShapeKind::Triangle, 440.0, 420.0, 680.0, 660.0, solid(t.primary.lerp(t.light, 0.35)));
    b.shape("Mountain", ShapeKind::Triangle, 60.0, 300.0, 760.0, 780.0, solid(t.primary));
    b.shape("Snow cap", ShapeKind::Triangle, 330.0, 300.0, 220.0, 226.0, solid(t.light));
    b.icon("Flag", "flag", 427.0, 196.0, 100.0, 104.0, solid(t.secondary));
    b.rect("Ground", 0.0, 1080.0, w, h - 1080.0, 0.0, solid(t.dark));
    b.text("Headline", 90.0, 1120.0, 940.0, "Keep\nclimbing", t.h(150.0, t.light).upper().lh(0.95).left());
    b.text("Subline", 90.0, 1450.0, 940.0, "Every step counts, even the small ones.", t.b(36.0, t.light.lerp(t.dark, 0.2)).left());
    b.rect("Accent", 90.0, 1520.0, 120.0, 8.0, 4.0, solid(t.secondary));
}

fn exhibition_poster(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.ellipse("Sun form", 200.0, 160.0, 800.0, 800.0, solid(t.primary));
    b.ellipse("Moon form", 500.0, 460.0, 200.0, 200.0, solid(t.light));
    b.rect("Square form", 1000.0, 160.0, 528.0, 528.0, 0.0, solid(t.dark));
    b.ellipse("Round form", 1000.0, 688.0, 528.0, 528.0, solid(t.secondary));
    b.rect("Bar form", 200.0, 1000.0, 800.0, 216.0, 0.0, solid(t.secondary.lerp(t.dark, 0.25)));
    b.shape("Triangle form", ShapeKind::Triangle, 1130.0, 300.0, 268.0, 240.0, solid(t.primary));
    b.text("Title", 200.0, 1330.0, 1328.0, "Forms &\nColour", t.h(200.0, t.dark).left());
    b.text("Subtitle", 200.0, 1830.0, 1328.0, "Abstract painting and print, 1950 – 1980", t.b(48.0, t.dark).left());
    b.rect("Rule", 200.0, 1950.0, 1328.0, 4.0, 0.0, solid(t.dark));
    for (i, (label, value)) in [("Dates", "12 March – 30 June"), ("Hours", "Tue–Sun · 10–18 h"), ("Venue", "City Gallery, Hall 2")].iter().enumerate()
    {
        let x = 200.0 + 450.0 * i as f64;
        b.text(&format!("{label} label"), x, 1990.0, 430.0, label, t.b(30.0, t.primary.lerp(t.dark, 0.5)).bold().upper().ls(200.0).left());
        b.text(&format!("{label} value"), x, 2040.0, 430.0, value, t.b(36.0, t.dark).left());
    }
    b.text("Website", 200.0, 2170.0, 1328.0, "citygallery.example · free entry on Sundays", t.b(32.0, t.primary.lerp(t.dark, 0.5)).bold().left());
}

// ---------------------------------------------------------------- menus and price lists

fn bistro_menu(b: &mut B) {
    let t = b.t.clone();
    let w = b.w;
    b.bg(solid(t.light));
    b.shape("Leaf left", ShapeKind::Ellipse, 70.0, 110.0, 70.0, 26.0, solid(t.secondary));
    b.rotate_last(-30.0);
    b.shape("Leaf right", ShapeKind::Ellipse, w - 140.0, 110.0, 70.0, 26.0, solid(t.secondary));
    b.rotate_last(30.0);
    b.ctext("Restaurant name", 80.0, 600.0, "Little Olive", t.h(72.0, t.primary).italic());
    b.ctext("Tagline", 175.0, 500.0, "Kitchen & wine bar", t.b(20.0, t.secondary.lerp(t.dark, 0.35)).bold().upper().ls(400.0));
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

fn cafe_menu(b: &mut B) {
    let t = b.t.clone();
    let w = b.w;
    b.bg(solid(t.dark));
    b.icon("Coffee", "coffee", w / 2.0 - 38.0, 50.0, 76.0, 76.0, solid(t.primary));
    b.ctext("Name", 140.0, 700.0, "Corner Café", t.h(72.0, t.light));
    b.ctext("Tagline", 235.0, 600.0, "Coffee · Tea · Bakes", t.b(22.0, t.primary).bold().upper().ls(300.0));
    b.shape("Divider", ShapeKind::Line, 60.0, 283.0, w - 120.0, 4.0, Fill::None);
    b.dash_last(t.light.with_alpha(130), 2.0, &[8.0, 8.0]);
    type Item = (&'static str, &'static str, &'static str);
    let columns: [(&str, [Item; 5]); 2] = [
        (
            "Coffee",
            [
                ("Espresso", "2.50", "Short and strong"),
                ("Flat white", "3.40", "Velvety milk, double shot"),
                ("Cappuccino", "3.20", "Deep foam, cocoa dust"),
                ("Cold brew", "3.80", "Steeped for 18 hours"),
                ("Chai latte", "3.60", "Warm spices, oat milk"),
            ],
        ),
        (
            "Bakes",
            [
                ("Butter croissant", "2.80", "Baked every morning"),
                ("Cinnamon bun", "3.20", "Cardamom sugar glaze"),
                ("Banana bread", "3.00", "Toasted, with butter"),
                ("Lemon slice", "2.90", "Sharp and sweet"),
                ("Granola pot", "4.50", "Yoghurt, berries, honey"),
            ],
        ),
    ];
    for (c, (title, items)) in columns.iter().enumerate() {
        let x = 60.0 + 368.0 * c as f64;
        b.text(&format!("{title} heading"), x, 320.0, 328.0, title, t.h(34.0, t.secondary).left());
        for (i, (name, price, note)) in items.iter().enumerate() {
            let y = 390.0 + 84.0 * i as f64;
            b.text(name, x, y, 228.0, name, t.b(24.0, t.light).bold().left());
            b.text(&format!("{name} price"), x + 228.0, y, 100.0, price, t.b(24.0, t.primary).bold().right());
            b.text(&format!("{name} note"), x, y + 34.0, 228.0, note, t.b(16.0, t.light.lerp(t.dark, 0.3)).left());
        }
    }
    b.rect("Note box", 60.0, 840.0, w - 120.0, 130.0, 20.0, solid(t.primary));
    b.ctext("Milk note", 868.0, 660.0, "Oat, soy or almond milk at no extra cost", t.b(22.0, t.dark).bold());
    b.ctext("Hours", 915.0, 660.0, "Open every day, 7 am – 5 pm", t.b(20.0, t.dark));
    b.ctext("Website", 1000.0, 400.0, "cornercafe.example", t.b(18.0, t.light.lerp(t.dark, 0.3)));
}

fn price_list(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.icon("Flower", "flower", w / 2.0 - 20.0, 18.0, 40.0, 40.0, solid(t.primary));
    b.ctext("Name", 66.0, w - 60.0, "Bloom Studio", t.h(44.0, t.dark));
    b.ctext("Tagline", 124.0, w - 100.0, "Hair · Nails · Brows", t.b(14.0, t.primary.lerp(t.dark, 0.2)).bold().upper().ls(400.0));
    // title, top, (service, price) rows
    type Section = (&'static str, f64, &'static [(&'static str, &'static str)]);
    let sections: [Section; 3] = [
        ("Hair", 180.0, &[("Cut and finish", "45"), ("Colour", "from 70"), ("Blow-dry", "30"), ("Treatment", "25")]),
        ("Nails", 380.0, &[("Manicure", "28"), ("Gel polish", "35"), ("Pedicure", "38")]),
        ("Brows and lashes", 548.0, &[("Shape and tint", "22"), ("Lash lift", "40")]),
    ];
    for (title, y, items) in sections {
        b.text(&format!("{title} heading"), 50.0, y, 400.0, title, t.h(22.0, t.primary).upper().ls(150.0).left());
        b.rect(&format!("{title} rule"), 50.0, y + 32.0, w - 100.0, 1.5, 0.0, solid(t.secondary));
        for (i, (service, price)) in items.iter().enumerate() {
            let iy = y + 46.0 + 32.0 * i as f64;
            b.text(service, 50.0, iy, 300.0, service, t.b(16.0, t.dark).left());
            b.text(&format!("{service} price"), w - 170.0, iy, 120.0, price, t.b(16.0, t.dark).bold().right());
        }
    }
    b.rect("Footer", 0.0, h - 90.0, w, 90.0, 0.0, solid(t.primary.lerp(t.dark, 0.2)));
    b.ctext("Footer text", h - 55.0, w - 40.0, "Book online: bloom.example · 555 0100 300", t.b(15.0, t.light).bold());
}
