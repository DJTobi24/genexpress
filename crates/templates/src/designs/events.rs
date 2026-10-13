//! More print: event posters and flyers, menus, a recipe card and a property listing.

use genha_doc::{BorderLines, CellRange, ShapeKind, StrokeStyle, TextAlign};

use super::{Def, PRINT, def, lin, radial, solid};
use crate::build::{B, Sty, Theme};

pub(super) const DEFS: &[Def] = &[
    def(
        "river-run-poster",
        "River run charity poster",
        PRINT,
        "poster-a3",
        &["poster", "charity", "run", "race", "sport", "qr code", "a3"],
        || Theme::new(0x0ea5e9, 0xf43f5e, 0x0c1a2b, 0xf0f9ff, "Display", "Sans"),
        river_run_poster,
    ),
    def(
        "pottery-workshop-flyer",
        "Pottery workshop flyer",
        PRINT,
        "flyer-a5",
        &["flyer", "workshop", "pottery", "class", "craft", "a5"],
        || Theme::new(0xb7472a, 0x5b8e7d, 0x22252c, 0xf7f3ee, "Serif", "Sans"),
        pottery_flyer,
    ),
    def(
        "grand-opening-flyer",
        "Grand opening flyer",
        PRINT,
        "flyer-letter",
        &["flyer", "grand opening", "shop", "opening", "launch", "letter"],
        || Theme::new(0x9333ea, 0xfbbf24, 0x1e1033, 0xfaf5ff, "Display", "Sans"),
        grand_opening,
    ),
    def(
        "season-sale-poster",
        "End of season sale poster",
        PRINT,
        "poster-18x24",
        &["poster", "sale", "discount", "retail", "shop window", "typography"],
        || Theme::new(0xff3d00, 0xfff200, 0x111111, 0xf6f4ef, "Display", "Sans"),
        season_sale,
    ),
    def(
        "farmers-market-poster",
        "Farmers market poster",
        PRINT,
        "poster-a3",
        &["poster", "market", "farmers market", "food", "community", "local"],
        || Theme::new(0x606c38, 0xdda15e, 0x283618, 0xfefae0, "Serif", "Sans"),
        farmers_market,
    ),
    def(
        "cocktail-menu",
        "Cocktail bar menu",
        PRINT,
        "flyer-a4",
        &["menu", "bar", "cocktails", "drinks", "price list", "table"],
        || Theme::new(0xe0a526, 0xd1495b, 0x14121c, 0xf8f1e5, "Serif", "Sans"),
        cocktail_menu,
    ),
    def(
        "brunch-menu",
        "Weekend brunch menu",
        PRINT,
        "flyer-letter",
        &["menu", "brunch", "cafe", "restaurant", "food", "breakfast"],
        || Theme::new(0xf28482, 0x84a59d, 0x2f3e46, 0xfff7ed, "Serif", "Sans"),
        brunch_menu,
    ),
    def(
        "recipe-card",
        "Lemon cake recipe card",
        PRINT,
        "postcard",
        &["recipe", "recipe card", "baking", "cooking", "food", "kitchen"],
        || Theme::new(0xf2a541, 0x6a994e, 0x3a2e1f, 0xfffbf0, "Serif", "Sans"),
        recipe_card,
    )
    .landscape(),
    def(
        "listing-flyer",
        "Just listed property flyer",
        PRINT,
        "flyer-letter",
        &["real estate", "property", "listing", "house", "for sale", "flyer", "qr code"],
        || Theme::new(0x1e3a5f, 0xc9a227, 0x0f1c2e, 0xfbfaf7, "Serif", "Sans"),
        listing_flyer,
    ),
    def(
        "film-night-poster",
        "Outdoor film night poster",
        PRINT,
        "poster-a3",
        &["poster", "cinema", "film", "movie night", "outdoor", "summer"],
        || Theme::new(0x4f46e5, 0xfde047, 0x0b0a1f, 0xeef2ff, "Display", "Sans"),
        film_night,
    ),
];

// ---------------------------------------------------------------- posters and flyers

fn river_run_poster(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.rect("Sky", 0.0, 0.0, w, 900.0, 0.0, lin(90.0, t.primary.lerp(t.dark, 0.5), t.primary.lerp(t.dark, 0.25)));
    b.icon("Wave", "wave-divider", 0.0, 760.0, w, 200.0, solid(t.light));
    b.ellipse("Sun", 760.0, 120.0, 240.0, 240.0, solid(t.secondary));
    b.text("Kicker", 80.0, 120.0, 640.0, "Charity run · year 6", t.b(32.0, t.light).bold().upper().ls(300.0).left());
    b.text("Headline", 80.0, 180.0, 700.0, "River\nRun", t.h(210.0, t.light).upper().lh(0.92).left());
    b.text("Year", 80.0, 580.0, 700.0, "Sunday 14 June", t.h(64.0, t.light).left());
    // The route: a dashed line from start to finish flag.
    b.shape(
        "Route",
        ShapeKind::Path { d: "M0 100 C120 0 240 180 380 80 C520 -20 600 120 760 40".into(), view_width: 760.0, view_height: 160.0 },
        120.0,
        960.0,
        760.0,
        160.0,
        genha_doc::Fill::None,
    );
    b.dash_last(t.secondary, 8.0, &[18.0, 14.0]);
    b.ellipse("Start", 104.0, 1044.0, 32.0, 32.0, solid(t.primary));
    b.text("Start label", 80.0, 1086.0, 200.0, "Start", t.b(24.0, t.dark).bold().left());
    b.text("Finish label", 800.0, 1040.0, 200.0, "Finish", t.b(24.0, t.dark).bold().right());
    b.icon("Finish flag", "flag", 860.0, 920.0, 110.0, 110.0, solid(t.secondary));
    let facts = [("Distances", "5K · 10K"), ("Start", "9 am, Mill Bridge"), ("Entry", "$20, kids free")];
    for (i, (label, value)) in facts.iter().enumerate() {
        let x = 80.0 + 330.0 * i as f64;
        b.text(&format!("{label} label"), x, 1180.0, 300.0, label, t.b(24.0, t.primary.lerp(t.dark, 0.45)).bold().upper().ls(200.0).left());
        b.text(&format!("{label} value"), x, 1220.0, 300.0, value, t.b(30.0, t.dark).bold().left());
    }
    b.rect("Footer", 0.0, h - 260.0, w, 260.0, 0.0, solid(t.dark));
    b.text("Cause", 80.0, h - 210.0, 720.0, "Every runner funds a metre\nof new riverbank path.", t.b(34.0, t.light).lh(1.4).left());
    b.text("Register", 80.0, h - 90.0, 720.0, "riverrun.example", t.b(30.0, t.secondary).bold().left());
    b.rect("QR card", w - 280.0, h - 230.0, 200.0, 200.0, 16.0, solid(t.light));
    b.qr("Registration QR code", "https://riverrun.example/register", w - 270.0, h - 220.0, 180.0, t.dark);
}

/// A simple vase drawn as a path (a view box of 100 × 140).
const VASE: &str =
    "M38 0 H62 V10 C62 18 74 24 82 40 C94 64 90 100 76 124 C72 132 64 140 50 140 C36 140 28 132 24 124 C10 100 6 64 18 40 C26 24 38 18 38 10 Z";

fn pottery_flyer(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.rect("Top band", 0.0, 0.0, w, 330.0, 0.0, solid(t.secondary.lerp(t.light, 0.75)));
    b.shape("Vase", ShapeKind::Path { d: VASE.into(), view_width: 100.0, view_height: 140.0 }, 360.0, 70.0, 140.0, 196.0, solid(t.primary));
    b.rect("Vase stripe", 380.0, 150.0, 100.0, 10.0, 0.0, solid(t.light));
    b.shape("Small vase", ShapeKind::Path { d: VASE.into(), view_width: 100.0, view_height: 140.0 }, 440.0, 190.0, 80.0, 112.0, solid(t.secondary));
    b.text("Kicker", 40.0, 80.0, 300.0, "Weekend workshop", t.b(14.0, t.primary.lerp(t.dark, 0.3)).bold().upper().ls(250.0).left());
    b.text("Headline", 40.0, 110.0, 320.0, "Pottery\nfor\nbeginners", t.h(46.0, t.dark).lh(1.05).left());
    b.text(
        "Intro",
        40.0,
        360.0,
        480.0,
        "Two relaxed mornings at the wheel. Make a mug\nand a bowl, glaze them, take them home.",
        t.b(14.0, t.dark).lh(1.5).left(),
    );
    let plan = [
        ("Sat 10:00", "Clay, centring and your first pull"),
        ("Sun 10:00", "Trimming, handles and glazing"),
        ("Two weeks on", "Collect your fired pieces"),
    ];
    for (i, (when, what)) in plan.iter().enumerate() {
        let y = 440.0 + 52.0 * i as f64;
        b.ellipse(&format!("Step {} dot", i + 1), 40.0, y + 5.0, 10.0, 10.0, solid(t.primary));
        b.text(&format!("Step {} time", i + 1), 62.0, y, 120.0, when, t.b(13.0, t.primary.lerp(t.dark, 0.3)).bold().left());
        b.text(&format!("Step {}", i + 1), 180.0, y, 340.0, what, t.b(13.0, t.dark).left());
    }
    b.rect("Price card", 40.0, 610.0, 480.0, 100.0, 14.0, solid(t.primary));
    b.text("Price", 60.0, 628.0, 160.0, "$85", t.h(48.0, t.light).lh(1.1).left());
    b.text("Price note", 200.0, 632.0, 300.0, "Clay, glazes, firing\nand tea included", t.b(14.0, t.light).lh(1.4).left());
    b.text("Contact", 40.0, h - 66.0, 480.0, "Book at kilnhouse.example · 555 0100 310", t.b(12.0, t.dark).bold().left());
}

fn grand_opening(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.dark));
    for (i, (x, y, s)) in [(60.0, 60.0, 40.0), (700.0, 110.0, 56.0), (90.0, 520.0, 30.0), (720.0, 470.0, 36.0)].into_iter().enumerate() {
        b.icon(&format!("Sparkle {}", i + 1), "sparkle", x, y, s, s, solid(if i % 2 == 0 { t.secondary } else { t.primary.lerp(t.light, 0.4) }));
    }
    b.ctext("Kicker", 110.0, 600.0, "You're invited to our", t.b(22.0, t.secondary).bold().upper().ls(300.0));
    b.ctext("Headline", 150.0, 700.0, "Grand\nopening", t.h(120.0, t.light).upper().lh(0.95));
    // A badge with text bent over its top.
    b.ellipse("Badge", 268.0, 420.0, 280.0, 280.0, solid(t.primary));
    b.outline("Badge ring", ShapeKind::Ellipse, 284.0, 436.0, 248.0, 248.0, t.secondary, 3.0);
    b.text("Badge arc", 288.0, 436.0, 240.0, "Opening weekend", t.b(20.0, t.light).bold().upper().ls(150.0));
    b.curve_last(150.0, 92.0);
    b.text("Badge offer", 268.0, 530.0, 280.0, "20% off", t.h(52.0, t.light).lh(1.1));
    b.text("Badge note", 268.0, 600.0, 280.0, "everything", t.b(18.0, t.light).bold().upper().ls(200.0));
    b.ctext("Date", 760.0, 700.0, "Saturday 4 October · 10 am", t.h(36.0, t.light));
    b.ctext("Extras", 820.0, 640.0, "Free coffee for the first 100 guests,\nlive music and a prize draw at 3 pm.", t.b(20.0, t.light).lh(1.5));
    b.rect("Footer", 0.0, h - 90.0, w, 90.0, 0.0, solid(t.secondary));
    b.ctext("Address", h - 62.0, 700.0, "Lumen Home Store · 52 High Street · lumenhome.example", t.b(18.0, t.dark).bold());
}

fn season_sale(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.rect("Colour field", 0.0, 0.0, w, 1500.0, 0.0, solid(t.primary));
    b.text("Kicker", 120.0, 140.0, 1400.0, "End of season", t.b(64.0, t.dark).bold().upper().ls(300.0).left());
    b.text("Headline", 120.0, 240.0, 1500.0, "Sale", t.h(560.0, t.light).upper().lh(0.95).left());
    b.text("Up to", 120.0, 820.0, 600.0, "up to", t.b(90.0, t.dark).bold().left());
    b.text("Discount", 120.0, 930.0, 1500.0, "70% off", t.h(340.0, t.secondary).lh(1.0).left());
    b.text("Body", 120.0, 1600.0, 1480.0, "Coats, knits and boots for the colder months,\nwhile sizes last.", t.b(56.0, t.dark).lh(1.4).left());
    b.mark_last("while sizes last.", t.primary.lerp(t.dark, 0.25), true);
    b.rect("Dates bar", 120.0, 1830.0, 1488.0, 4.0, 0.0, solid(t.dark));
    let facts = [("Starts", "Fri 9 Jan"), ("Ends", "Sun 18 Jan"), ("Where", "In store & online")];
    for (i, (label, value)) in facts.iter().enumerate() {
        let x = 120.0 + 500.0 * i as f64;
        b.text(&format!("{label} label"), x, 1880.0, 460.0, label, t.b(40.0, t.primary.lerp(t.dark, 0.35)).bold().upper().ls(200.0).left());
        b.text(&format!("{label} value"), x, 1940.0, 500.0, value, t.h(52.0, t.dark).left());
    }
    b.text("Shop", 120.0, h - 160.0, 1488.0, "Northfold Outfitters", t.b(48.0, t.dark).bold().left());
    b.text("Website", w - 900.0, h - 160.0, 780.0, "northfold.example", t.b(48.0, t.dark).right());
}

fn farmers_market(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.rect("Awning", 0.0, 0.0, w, 120.0, 0.0, solid(t.primary));
    for i in 0..8 {
        let x = f64::from(i) * w / 8.0;
        b.shape(&format!("Scallop {}", i + 1), ShapeKind::Ellipse, x, 70.0, w / 8.0, 100.0, solid(if i % 2 == 0 { t.primary } else { t.secondary }));
    }
    b.ctext("Kicker", 230.0, 900.0, "Every Saturday · 8 am – 1 pm", t.b(32.0, t.primary).bold().upper().ls(200.0));
    b.ctext("Headline", 290.0, 1000.0, "Farmers\nmarket", t.h(170.0, t.dark).lh(1.0));
    b.ctext("Place", 650.0, 900.0, "Old Station Yard, Linden Road", t.b(38.0, t.dark));
    let stalls = [
        ("leaf", "Fresh veg"),
        ("flower", "Cut flowers"),
        ("cake", "Bakery"),
        ("coffee", "Coffee"),
        ("wine-glass", "Local cider"),
        ("paw", "Dog friendly"),
    ];
    for (i, (icon, label)) in stalls.iter().enumerate() {
        let (col, row) = (i % 3, i / 3);
        let x = 120.0 + 310.0 * col as f64;
        let y = 780.0 + 300.0 * row as f64;
        b.ellipse(&format!("{label} disc"), x + 55.0, y, 160.0, 160.0, solid(if (col + row) % 2 == 0 { t.secondary } else { t.primary }));
        b.icon(&format!("{label} icon"), icon, x + 95.0, y + 40.0, 80.0, 80.0, solid(t.light));
        b.text(&format!("{label} label"), x, y + 180.0, 270.0, label, t.b(32.0, t.dark).bold());
    }
    b.rect("Footer", 0.0, h - 230.0, w, 230.0, 0.0, solid(t.dark));
    b.ctext("Footer line", h - 180.0, 900.0, "40 stalls from farms within 30 miles", t.h(44.0, t.light));
    b.ctext("Footer note", h - 110.0, 900.0, "Bring a bag · Card and cash welcome", t.b(30.0, t.secondary).bold());
}

fn film_night(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(lin(90.0, t.dark, t.primary.lerp(t.dark, 0.35)));
    b.icon("Moon", "moon", 820.0, 100.0, 180.0, 180.0, solid(t.secondary));
    for (i, (x, y, s)) in [(120.0, 140.0, 30.0), (360.0, 90.0, 20.0), (620.0, 220.0, 26.0), (980.0, 380.0, 22.0)].into_iter().enumerate() {
        b.icon(&format!("Star {}", i + 1), "sparkle", x, y, s, s, solid(t.light.with_alpha(200)));
    }
    // A screen on two posts, and a film strip across the bottom.
    b.rect("Screen post left", 220.0, 520.0, 16.0, 520.0, 0.0, solid(t.light.with_alpha(140)));
    b.rect("Screen post right", w - 236.0, 520.0, 16.0, 520.0, 0.0, solid(t.light.with_alpha(140)));
    b.rect("Screen", 160.0, 400.0, w - 320.0, 500.0, 12.0, solid(t.light));
    b.text("Kicker", 200.0, 460.0, w - 400.0, "Outdoor cinema", t.b(30.0, t.primary).bold().upper().ls(300.0));
    b.text("Headline", 200.0, 520.0, w - 400.0, "Films under\nthe stars", t.h(110.0, t.dark).lh(1.0));
    b.text("Bring", 200.0, 780.0, w - 400.0, "Bring a blanket, we bring the popcorn", t.b(30.0, t.dark).lh(1.3));
    let films = [("Fri 4 July", "The Lighthouse Keeper"), ("Fri 11 July", "Paper Moon Rising"), ("Fri 18 July", "Summer of the Comet")];
    for (i, (date, film)) in films.iter().enumerate() {
        let y = 1080.0 + 80.0 * i as f64;
        b.text(&format!("Film {} date", i + 1), 160.0, y, 260.0, date, t.b(32.0, t.secondary).bold().left());
        b.text(&format!("Film {}", i + 1), 440.0, y, 560.0, film, t.h(36.0, t.light).left());
    }
    b.rect("Film strip", 0.0, h - 220.0, w, 140.0, 0.0, solid(t.secondary));
    for i in 0..14 {
        let x = 20.0 + f64::from(i) * 80.0;
        b.rect(&format!("Sprocket top {}", i + 1), x, h - 210.0, 40.0, 24.0, 4.0, solid(t.dark));
        b.rect(&format!("Sprocket bottom {}", i + 1), x, h - 114.0, 40.0, 24.0, 4.0, solid(t.dark));
    }
    b.ctext("Strip text", h - 172.0, 900.0, "Park Lawn · gates 8 pm · free", t.h(40.0, t.dark).upper().ls(100.0).lh(1.15));
    b.ctext("Website", h - 62.0, 900.0, "filmsunderthestars.example", t.b(28.0, t.light));
}

// ---------------------------------------------------------------- menus

fn cocktail_menu(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.dark));
    b.outline("Frame", ShapeKind::Rect { corner_radius: 0.0 }, 30.0, 30.0, w - 60.0, h - 60.0, t.primary, 2.0);
    b.icon("Glass", "wine-glass", (w - 70.0) / 2.0, 70.0, 70.0, 70.0, solid(t.primary));
    b.ctext("Bar name", 150.0, 600.0, "The Copper Fox", t.h(56.0, t.light));
    b.ctext("Tagline", 222.0, 600.0, "Cocktail bar · since 2019", t.b(16.0, t.primary).upper().ls(300.0));
    let table = |rows: &[&[&str]], favourite: usize| {
        let mut tb = t.table(rows, 16.0);
        tb.font_family = t.body.clone();
        tb.text_color = t.light;
        tb.row_fill = t.dark;
        tb.alt_row_fill = Some(t.dark.lerp(t.light, 0.06));
        tb.header_fill = t.primary;
        tb.header_text = t.dark;
        tb.border = Some(StrokeStyle { color: t.primary.with_alpha(90), width: 1.0, dash: vec![] });
        tb.borders = BorderLines::Horizontal;
        tb.column_widths = vec![0.62, 0.19, 0.19];
        // The title row spans every column; the column names sit under it.
        tb.merges = vec![CellRange::new(0, 0, 1, 3)];
        tb.style_cells(CellRange::new(0, 0, 1, 3), |s| {
            s.align = Some(TextAlign::Center);
            s.font_size = Some(20.0);
        });
        tb.style_cells(CellRange::new(1, 0, 1, 3), |s| {
            s.fill = Some(t.dark.lerp(t.primary, 0.25));
            s.text_color = Some(t.primary);
            s.bold = Some(true);
        });
        tb.style_cells(CellRange::new(1, 1, rows.len() - 1, 2), |s| s.align = Some(TextAlign::Right));
        tb.style_cells(CellRange::new(favourite, 0, 1, 3), |s| {
            s.fill = Some(t.secondary);
            s.text_color = Some(t.light);
            s.bold = Some(true);
        });
        tb
    };
    let cocktails = table(
        &[
            &["Signature cocktails", "", ""],
            &["Drink", "Glass", "Jug"],
            &["Copper Fox · bourbon, honey, ginger", "$12", "$40"],
            &["Garden Party · gin, cucumber, elderflower", "$11", "$38"],
            &["Night Heron · rum, coffee, orange", "$12", "$40"],
            &["Sour Cherry · vodka, cherry, lemon", "$10", "$34"],
            &["House favourite · ask the bartender", "$13", "$44"],
        ],
        6,
    );
    // Rows: 16 px text, 8 px padding: 36 px each; the title row is 20 px: 41 px.
    b.table("Cocktail table", 70.0, 290.0, w - 140.0, 7.0 * 36.0 + 6.0, cocktails);
    let zero = table(
        &[
            &["Zero proof", "", ""],
            &["Drink", "Glass", "Jug"],
            &["Sunrise Spritz · orange, rosemary, soda", "$7", "$24"],
            &["Green Mile · apple, mint, lime", "$7", "$24"],
            &["Ginger Fizz · fresh ginger, honey", "$6", "$20"],
        ],
        4,
    );
    b.table("Zero proof table", 70.0, 620.0, w - 140.0, 5.0 * 36.0 + 6.0, zero);
    b.ctext("Happy hour", 860.0, 600.0, "Happy hour", t.h(40.0, t.primary).italic());
    b.ctext("Happy hour times", 916.0, 600.0, "Monday to Thursday, 5 – 7 pm: jugs at glass prices", t.b(16.0, t.light));
    b.ctext("Footer", h - 110.0, 600.0, "Please drink responsibly · 21+ after 9 pm", t.b(13.0, t.light).ls(50.0));
}

fn brunch_menu(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.rect("Header", 0.0, 0.0, w, 230.0, 0.0, solid(t.secondary.lerp(t.light, 0.7)));
    b.icon("Sun", "sun", 650.0, 50.0, 120.0, 120.0, solid(t.primary));
    b.text("Kicker", 60.0, 60.0, 560.0, "Saturdays & Sundays · 9 – 2", t.b(16.0, t.dark).bold().upper().ls(250.0).left());
    b.text("Headline", 60.0, 96.0, 560.0, "Weekend brunch", t.h(54.0, t.dark).left());
    let column = |b: &mut B, title: &str, x: f64, items: &[(&str, &str, &str)]| {
        b.text(&format!("{title} heading"), x, 280.0, 320.0, title, t.h(30.0, t.primary.lerp(t.dark, 0.35)).italic().left());
        b.rect(&format!("{title} rule"), x, 326.0, 320.0, 2.0, 1.0, solid(t.secondary));
        for (i, (dish, note, price)) in items.iter().enumerate() {
            let y = 350.0 + 104.0 * i as f64;
            b.text(dish, x, y, 250.0, dish, t.h(20.0, t.dark).left());
            b.text(&format!("{dish} price"), x + 250.0, y, 70.0, price, t.h(20.0, t.dark).right());
            b.text(&format!("{dish} note"), x, y + 30.0, 320.0, note, t.b(14.0, t.dark).lh(1.45).left());
        }
    };
    column(
        b,
        "Sweet",
        60.0,
        &[
            ("Buttermilk pancakes", "Maple butter, berries,\ntoasted almonds", "$12"),
            ("French toast", "Brioche, vanilla custard,\npoached pear", "$13"),
            ("Granola bowl", "Yoghurt, honey, seasonal\nfruit", "$9"),
            ("Cinnamon bun", "Baked this morning,\ncardamom glaze", "$5"),
        ],
    );
    column(
        b,
        "Savoury",
        436.0,
        &[
            ("Green eggs", "Spinach, feta, chilli oil\non sourdough", "$14"),
            ("Breakfast tacos", "Scrambled eggs, black\nbeans, pico de gallo", "$13"),
            ("Mushroom toast", "Garlic mushrooms,\nricotta, thyme", "$12"),
            ("Big brunch plate", "Eggs, beans, hash brown,\nroast tomato, toast", "$16"),
        ],
    );
    b.rect("Drinks band", 60.0, 800.0, w - 120.0, 120.0, 18.0, solid(t.primary.lerp(t.light, 0.75)));
    b.icon("Coffee", "coffee", 90.0, 830.0, 60.0, 60.0, solid(t.dark));
    b.text("Drinks", 170.0, 822.0, 560.0, "Bottomless filter coffee $4", t.h(22.0, t.dark).left());
    b.text("Drinks note", 170.0, 858.0, 560.0, "Fresh orange juice $4 · Oat, soy and almond milk free", t.b(14.0, t.dark).left());
    b.text("Footer", 60.0, h - 80.0, w - 120.0, "Ask us about allergens · Kettle & Crumb, 3 Bridge Street", t.b(13.0, t.dark).left());
}

fn recipe_card(b: &mut B) {
    let t = b.t.clone();
    let h = b.h;
    b.bg(solid(t.light));
    b.rect("Side panel", 0.0, 0.0, 200.0, h, 0.0, solid(t.primary.lerp(t.light, 0.7)));
    b.ellipse("Lemon", 40.0, 30.0, 120.0, 100.0, radial(t.primary.lerp(t.light, 0.3), t.primary));
    b.icon("Lemon leaf", "leaf", 120.0, 20.0, 50.0, 50.0, solid(t.secondary));
    b.text("Ingredients heading", 20.0, 150.0, 170.0, "You'll need", t.h(15.0, t.dark).upper().ls(150.0).left());
    b.text(
        "Ingredients",
        20.0,
        178.0,
        170.0,
        "3 eggs\n200 g sugar\n2 lemons, zest & juice\n150 ml olive oil\n200 g self-raising flour\nA pinch of salt",
        t.b(11.0, t.dark).lh(1.55).left(),
    );
    b.text("Kicker", 222.0, 22.0, 320.0, "From grandma's kitchen", t.b(10.0, t.secondary.lerp(t.dark, 0.35)).bold().upper().ls(250.0).left());
    b.text("Title", 222.0, 40.0, 320.0, "Lemon olive\noil cake", t.h(30.0, t.dark).lh(1.05).left());
    for (i, (icon, label)) in [("clock", "15 + 40 min"), ("users", "Serves 8"), ("cake", "Easy")].iter().enumerate() {
        let x = 222.0 + 108.0 * i as f64;
        b.icon(&format!("{label} icon"), icon, x, 118.0, 18.0, 18.0, solid(t.secondary));
        b.text(label, x + 22.0, 120.0, 84.0, label, t.b(10.0, t.dark).bold().left());
    }
    b.rect("Rule", 222.0, 148.0, 316.0, 1.5, 0.0, solid(t.primary));
    let steps = [
        "Heat the oven to 180 °C and line a 20 cm tin.",
        "Whisk eggs and sugar until pale, 3 minutes.",
        "Stir in zest, juice and oil, then fold\nin the flour and salt.",
        "Bake for 40 minutes, until a skewer comes out\nclean. Cool before slicing.",
    ];
    let mut y = 160.0;
    for (i, step) in steps.iter().enumerate() {
        let n = i + 1;
        b.ellipse(&format!("Step {n} disc"), 222.0, y, 18.0, 18.0, solid(t.primary));
        b.text(&format!("Step {n} number"), 222.0, y + 2.5, 18.0, &n.to_string(), t.b(10.0, t.dark).bold().lh(1.2));
        b.text(&format!("Step {n}"), 248.0, y + 1.0, 296.0, step, t.b(11.0, t.dark).lh(1.45).left());
        y += if step.contains('\n') { 46.0 } else { 30.0 };
    }
    b.text("Tip", 222.0, h - 34.0, 320.0, "Tip: a spoon of yoghurt keeps it extra moist.", t.b(10.0, t.dark).italic().left());
}

fn listing_flyer(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    // A picture area: replace the drawing with a photo of the house.
    b.rect("Photo area", 0.0, 0.0, w, 470.0, 0.0, lin(90.0, t.primary.lerp(t.light, 0.55), t.primary.lerp(t.light, 0.25)));
    b.shape("Roof", ShapeKind::Triangle, 250.0, 120.0, 316.0, 120.0, solid(t.primary));
    b.rect("House", 280.0, 240.0, 256.0, 180.0, 0.0, solid(t.light));
    b.rect("Door", 384.0, 320.0, 48.0, 100.0, 4.0, solid(t.secondary));
    b.rect("Window left", 306.0, 270.0, 56.0, 50.0, 4.0, solid(t.primary.lerp(t.light, 0.5)));
    b.rect("Window right", 454.0, 270.0, 56.0, 50.0, 4.0, solid(t.primary.lerp(t.light, 0.5)));
    b.rect("Lawn", 0.0, 420.0, w, 50.0, 0.0, solid(t.primary.lerp(t.dark, 0.2)));
    b.rect("Label", 48.0, 40.0, 210.0, 48.0, 8.0, solid(t.secondary));
    b.text("Label text", 48.0, 52.0, 210.0, "Just listed", t.b(18.0, t.dark).bold().upper().ls(200.0).lh(1.2));
    b.text("Address", 48.0, 500.0, 520.0, "18 Orchard Lane", t.h(44.0, t.dark).left());
    b.text("Area", 48.0, 556.0, 520.0, "Westbrook · quiet street near the park", t.b(17.0, t.dark).left());
    b.text("Price", w - 300.0, 500.0, 252.0, "$489,000", t.h(40.0, t.primary).right());
    let mut tb = t.table(
        &[
            &["Bedrooms", "Bathrooms", "Living area", "Garden"],
            &["3", "2", "1,450 sq ft", "South facing"],
            &["Built 1962 · new roof 2023 · energy rating B", "", "", ""],
        ],
        15.0,
    );
    tb.merges = vec![CellRange::new(2, 0, 1, 4)];
    tb.align = TextAlign::Center;
    tb.style_cells(CellRange::new(1, 0, 1, 4), |s| {
        s.font_size = Some(20.0);
        s.bold = Some(true);
    });
    tb.style_cells(CellRange::new(2, 0, 1, 4), |s| {
        s.fill = Some(t.secondary.lerp(t.light, 0.75));
        s.font_size = Some(14.0);
    });
    b.table("Key facts", 48.0, 600.0, w - 96.0, 112.0, tb);
    b.text(
        "Description",
        48.0,
        740.0,
        460.0,
        "A bright family home with a big kitchen,\na sunny garden office and room to grow.\nTen minutes' walk to the station and\ntwo good schools.",
        t.b(15.0, t.dark).lh(1.55).left(),
    );
    b.rect("Agent card", 540.0, 740.0, 228.0, 200.0, 14.0, solid(t.primary));
    b.text("Agent", 556.0, 756.0, 196.0, "Call Dana Whitfield", t.b(15.0, t.light).bold());
    b.text("Phone", 556.0, 782.0, 196.0, "555 0100 820", t.b(15.0, t.secondary).bold());
    b.rect("QR card", 594.0, 812.0, 120.0, 120.0, 8.0, solid(t.light));
    b.qr("Tour QR code", "https://homes.example/18-orchard-lane", 600.0, 818.0, 108.0, t.dark);
    b.text("Tour", 48.0, 880.0, 460.0, "Scan for the 3D tour and floor plan", t.b(15.0, t.primary).bold().left());
    b.rect("Footer", 0.0, h - 70.0, w, 70.0, 0.0, solid(t.dark));
    b.text("Agency", 48.0, h - 46.0, w - 96.0, "Hearthstone Homes · hearthstone.example", t.b(16.0, t.light).left());
}
