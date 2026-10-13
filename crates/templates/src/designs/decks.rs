//! Decks with charts and tables (every slide has speaker notes) and a long-form infographic.

use genha_doc::{BorderLines, CellRange, ChartKind, LegendPosition, TextAlign};

use super::{DOCUMENTS, Def, def, lin, solid};
use crate::build::{B, Sty, Theme};

pub(super) const DEFS: &[Def] = &[
    def(
        "sales-review-deck",
        "Sales review deck with charts",
        DOCUMENTS,
        "presentation",
        &["presentation", "slides", "sales", "report", "charts", "table", "business"],
        || Theme::new(0x4f46e5, 0x10b981, 0x111827, 0xf9fafb, "Sans", "Sans"),
        sales_deck,
    ),
    def(
        "lesson-deck",
        "Water cycle lesson slides",
        DOCUMENTS,
        "presentation-4x3",
        &["presentation", "lesson", "school", "education", "science", "quiz", "slides"],
        || Theme::new(0x0369a1, 0xfacc15, 0x0c4a6e, 0xf0f9ff, "Display", "Sans"),
        lesson_deck,
    ),
    def(
        "commute-infographic",
        "Commute survey infographic",
        DOCUMENTS,
        "webpage",
        &["infographic", "survey", "charts", "statistics", "report", "transport"],
        || Theme::new(0x059669, 0x6366f1, 0x064e3b, 0xf7fdf9, "Display", "Sans"),
        commute_infographic,
    ),
];

/// A slide's kicker, title and page number on a light slide.
fn head(b: &mut B, kicker: &str, title: &str, page: usize) {
    let t = b.t.clone();
    b.rect("Side mark", 0.0, 110.0, 16.0, 120.0, 0.0, solid(t.primary));
    b.text("Kicker", 120.0, 100.0, 1200.0, kicker, t.b(28.0, t.primary).bold().upper().ls(300.0).left());
    b.text("Title", 120.0, 148.0, 1680.0, title, t.h(68.0, t.dark).left());
    b.text("Page number", 1700.0, 990.0, 100.0, &format!("{page:02}"), t.b(24.0, t.primary).bold().right());
}

fn sales_deck(b: &mut B) {
    let t = b.t.clone();
    let muted = t.dark.lerp(t.light, 0.3);
    b.name_page("Title");
    b.bg(solid(t.dark));
    for (i, hgt) in [180.0, 280.0, 240.0, 380.0, 470.0].into_iter().enumerate() {
        let x = 1180.0 + 120.0 * i as f64;
        b.rect(&format!("Deco bar {}", i + 1), x, 820.0 - hgt, 80.0, hgt, 12.0, solid(if i == 4 { t.secondary } else { t.primary }));
    }
    b.text("Kicker", 120.0, 380.0, 900.0, "Northwind Outdoor", t.b(32.0, t.secondary).bold().upper().ls(300.0).left());
    b.text("Title", 120.0, 440.0, 1000.0, "Q4 sales\nreview", t.h(130.0, t.light).lh(1.0).left());
    b.text("Subtitle", 120.0, 740.0, 1000.0, "Leadership meeting · January 2027", t.b(36.0, t.light).left());
    b.notes("Welcome. Twenty minutes: the headline numbers, regions, customers, then three decisions we need today.");

    b.new_page("Headlines");
    b.bg(solid(t.light));
    head(b, "The quarter in four numbers", "Our best quarter yet", 2);
    let tiles = [
        ("$4.8M", "revenue", "+18% on Q3"),
        ("12,400", "new customers", "+9% on Q3"),
        ("$86", "average order", "+$4 on Q3"),
        ("41%", "repeat buyers", "−2 points on Q3"),
    ];
    for (i, (value, label, change)) in tiles.iter().enumerate() {
        let x = 120.0 + 430.0 * i as f64;
        b.rect(&format!("Tile {}", i + 1), x, 340.0, 390.0, 420.0, 28.0, solid(t.primary.lerp(t.light, 0.92)));
        b.text(&format!("Tile {} value", i + 1), x + 40.0, 400.0, 330.0, value, t.h(88.0, t.primary).left());
        b.text(&format!("Tile {} label", i + 1), x + 40.0, 520.0, 330.0, label, t.b(32.0, t.dark).left());
        let up = !change.starts_with('−');
        b.text(&format!("Tile {} change", i + 1), x + 40.0, 660.0, 330.0, change, t.b(28.0, muted).bold().left());
        let first = change.split(' ').next().unwrap_or("");
        b.mark_last(first, if up { t.secondary.lerp(t.dark, 0.5) } else { t.dark }, true);
    }
    b.notes("Revenue and new customers are both up; repeat buying dipped slightly, more on that on slide 5.");

    b.new_page("Regions");
    b.bg(solid(t.light));
    head(b, "Revenue by region", "Online passed the North for the first time", 3);
    let mut c = t.chart(
        ChartKind::Bar,
        &["North", "South", "East", "West", "Online"],
        &[("Q3", &[1.10, 0.62, 0.48, 0.71, 1.15]), ("Q4", &[1.12, 0.70, 0.55, 0.86, 1.57])],
        24.0,
        t.dark,
    );
    c.colors = vec![t.primary.lerp(t.light, 0.5), t.primary];
    c.legend_position = LegendPosition::Top;
    c.y_title = "Revenue ($M)".into();
    b.chart("Region chart", 120.0, 300.0, 1100.0, 660.0, c);
    b.rect("Insight card", 1300.0, 300.0, 500.0, 660.0, 28.0, solid(t.dark));
    b.text("Insight kicker", 1350.0, 350.0, 400.0, "What changed", t.b(26.0, t.secondary).bold().upper().ls(250.0).left());
    b.text(
        "Insight",
        1350.0,
        410.0,
        420.0,
        "Online grew 37% after\nthe new checkout.\n\nThe West gained most\nfrom the pop-up stores.\n\nThe North held steady.",
        t.b(32.0, t.light).lh(1.4).left(),
    );
    b.mark_last("37%", t.secondary, true);
    b.notes("Bars are revenue in millions of dollars. The online jump lines up with the checkout launch on 3 October.");

    b.new_page("Customers");
    b.bg(solid(t.light));
    head(b, "Customers", "Monthly active customers, thousands", 4);
    let mut c = t.chart(
        ChartKind::Line,
        &["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"],
        &[
            ("2025", &[31.0, 30.0, 33.0, 35.0, 38.0, 41.0, 43.0, 42.0, 40.0, 41.0, 46.0, 52.0]),
            ("2026", &[36.0, 35.0, 39.0, 42.0, 46.0, 50.0, 53.0, 52.0, 51.0, 55.0, 61.0, 70.0]),
        ],
        22.0,
        t.dark,
    );
    c.colors = vec![t.dark.lerp(t.light, 0.5), t.secondary];
    c.show_values = false;
    c.legend_position = LegendPosition::Top;
    b.chart("Customer chart", 120.0, 300.0, 1680.0, 640.0, c);
    b.notes("Every month of 2026 beat the same month of 2025. December is our peak, as usual.");

    b.new_page("Region table");
    b.bg(solid(t.light));
    head(b, "Detail", "Region by region", 5);
    let rows: &[&[&str]] = &[
        &["Region", "Q3", "", "Q4", ""],
        &["", "Revenue", "Growth", "Revenue", "Growth"],
        &["North", "$1.10M", "+2%", "$1.12M", "+2%"],
        &["South", "$0.62M", "+5%", "$0.70M", "+13%"],
        &["East", "$0.48M", "−4%", "$0.55M", "+15%"],
        &["West", "$0.71M", "+8%", "$0.86M", "+21%"],
        &["Online", "$1.15M", "+11%", "$1.57M", "+37%"],
        &["Total", "$4.06M", "+5%", "$4.80M", "+18%"],
    ];
    let mut tb = t.table(rows, 26.0);
    tb.align = TextAlign::Center;
    tb.column_widths = vec![0.24, 0.19, 0.19, 0.19, 0.19];
    tb.borders = BorderLines::Horizontal;
    // Two header rows: the quarters span their two columns, "Region" spans both rows.
    tb.merges = vec![CellRange::new(0, 0, 2, 1), CellRange::new(0, 1, 1, 2), CellRange::new(0, 3, 1, 2)];
    tb.style_cells(CellRange::new(1, 1, 1, 4), |s| {
        s.fill = Some(t.primary.lerp(t.light, 0.75));
        s.bold = Some(true);
    });
    tb.style_cells(CellRange::new(2, 0, 6, 1), |s| {
        s.align = Some(TextAlign::Left);
        s.bold = Some(true);
    });
    for (r, row) in rows.iter().enumerate().skip(2) {
        for c in [2, 4] {
            let down = row.get(c).is_some_and(|v| v.starts_with('−'));
            let fill = if down { t.dark.lerp(t.light, 0.85) } else { t.secondary.lerp(t.light, 0.8) };
            tb.style_cells(CellRange::cell(r, c), |s| s.fill = Some(fill));
        }
    }
    tb.style_cells(CellRange::new(7, 0, 1, 5), |s| {
        s.fill = Some(t.dark);
        s.text_color = Some(t.light);
        s.bold = Some(true);
    });
    // 26 px text and 13 px padding: 58.5 px a row.
    b.table("Region table", 120.0, 300.0, 1680.0, 8.0 * 58.5 + 2.0, tb);
    b.text("Footnote", 120.0, 800.0, 1680.0, "Growth compares each quarter with the one before it.", t.b(24.0, muted).left());
    b.notes("East was the only region to shrink in Q3, and it recovered strongly in Q4.");

    b.new_page("Next steps");
    b.bg(lin(135.0, t.primary, t.primary.lerp(t.dark, 0.55)));
    b.text("Kicker", 120.0, 160.0, 1200.0, "Decisions for today", t.b(28.0, t.light).bold().upper().ls(300.0).left());
    b.text("Title", 120.0, 210.0, 1500.0, "Three things to agree", t.h(80.0, t.light).left());
    let steps = ["Move two pop-up stores to the East", "Fund a loyalty pilot to lift repeat buying", "Hire four people for online support"];
    for (i, s) in steps.iter().enumerate() {
        let y = 420.0 + 150.0 * i as f64;
        b.ellipse(&format!("Step {} disc", i + 1), 120.0, y, 100.0, 100.0, solid(t.secondary));
        b.text(&format!("Step {} number", i + 1), 120.0, y + 24.0, 100.0, &(i + 1).to_string(), t.h(48.0, t.dark).lh(1.0));
        b.text(&format!("Step {}", i + 1), 270.0, y + 22.0, 1400.0, s, t.b(46.0, t.light).left());
    }
    b.text("Page number", 1700.0, 990.0, 100.0, "06", t.b(24.0, t.light).bold().right());
    b.notes("Ask for a decision on each. Budget details are in the appendix sent on Monday.");
}

fn lesson_deck(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.name_page("Title");
    b.bg(lin(90.0, t.primary, t.dark));
    b.icon("Sun", "sun", 700.0, 70.0, 220.0, 220.0, solid(t.secondary));
    b.icon("Cloud", "cloud", 560.0, 230.0, 260.0, 170.0, solid(t.light));
    b.icon("Wave", "wave-divider", 0.0, 560.0, w, 120.0, solid(t.primary.lerp(t.light, 0.45)));
    b.rect("Sea", 0.0, 650.0, w, 118.0, 0.0, solid(t.primary.lerp(t.light, 0.45)));
    b.text("Kicker", 70.0, 250.0, 500.0, "Year 5 science", t.b(22.0, t.secondary).bold().upper().ls(300.0).left());
    b.text("Title", 70.0, 290.0, 560.0, "The water\ncycle", t.h(84.0, t.light).lh(1.0).left());
    b.text("Subtitle", 70.0, 480.0, 560.0, "Where does rain come from?", t.b(26.0, t.light).left());
    b.notes("Start with a question: where was this rain a week ago? Take three answers before moving on.");

    b.new_page("Four stages");
    b.bg(solid(t.light));
    b.text("Title", 60.0, 50.0, 900.0, "Four stages, round and round", t.h(44.0, t.dark).left());
    let stages = [
        ("sun", "1. Evaporation", "The sun warms water and\nit rises as vapour.", 80.0, 150.0),
        ("cloud", "2. Condensation", "Vapour cools into tiny\ndrops: clouds form.", 560.0, 150.0),
        ("umbrella", "3. Precipitation", "Drops join, get heavy\nand fall as rain or snow.", 560.0, 450.0),
        ("leaf", "4. Collection", "Water gathers in rivers,\nlakes, seas and soil.", 80.0, 450.0),
    ];
    for (i, (icon, title, body, x, y)) in stages.iter().enumerate() {
        let n = i + 1;
        b.rect(&format!("Stage {n} card"), *x, *y, 384.0, 250.0, 24.0, solid(t.primary.lerp(t.light, 0.88)));
        b.ellipse(&format!("Stage {n} disc"), x + 24.0, y + 24.0, 90.0, 90.0, solid(if i % 2 == 0 { t.secondary } else { t.primary }));
        b.icon(&format!("Stage {n} icon"), icon, x + 44.0, y + 44.0, 50.0, 50.0, solid(if i % 2 == 0 { t.dark } else { t.light }));
        b.text(&format!("Stage {n} title"), x + 24.0, y + 136.0, 340.0, title, t.h(26.0, t.dark).left());
        b.text(&format!("Stage {n} text"), x + 24.0, y + 174.0, 340.0, body, t.b(18.0, t.dark).lh(1.35).left());
    }
    for (i, (x, y, rot)) in [(477.0, 245.0, 0.0), (722.0, 405.0, 90.0), (477.0, 545.0, 180.0), (242.0, 405.0, 270.0)].into_iter().enumerate() {
        b.icon(&format!("Cycle arrow {}", i + 1), "arrow-right", x, y, 70.0, 60.0, solid(t.primary));
        b.rotate_last(rot);
    }
    b.text("Page number", w - 120.0, h - 50.0, 60.0, "02", t.b(16.0, t.primary).bold().right());
    b.notes("Point at each card in order. Ask: what makes stage 3 happen? (Drops join until they are too heavy.)");

    b.new_page("Key words");
    b.bg(solid(t.light));
    b.text("Title", 60.0, 50.0, 900.0, "Key words", t.h(44.0, t.dark).left());
    let rows: &[&[&str]] = &[
        &["Word bank", ""],
        &["Word", "What it means"],
        &["Vapour", "Water as an invisible gas"],
        &["Condense", "To turn from gas back to liquid"],
        &["Precipitation", "Rain, snow, sleet or hail"],
        &["Run-off", "Water flowing over the ground"],
        &["Groundwater", "Water stored in soil and rock"],
    ];
    let mut tb = t.table(rows, 22.0);
    tb.column_widths = vec![0.34, 0.66];
    tb.merges = vec![CellRange::new(0, 0, 1, 2)];
    tb.style_cells(CellRange::new(0, 0, 1, 2), |s| s.align = Some(TextAlign::Center));
    tb.style_cells(CellRange::new(1, 0, 1, 2), |s| {
        s.fill = Some(t.secondary);
        s.text_color = Some(t.dark);
        s.bold = Some(true);
    });
    tb.style_cells(CellRange::new(2, 0, 5, 1), |s| {
        s.bold = Some(true);
        s.text_color = Some(t.primary.lerp(t.dark, 0.3));
    });
    // 22 px text, 11 px padding: 49.5 px a row.
    b.table("Word bank", 60.0, 140.0, w - 120.0, 7.0 * 49.5 + 2.0, tb);
    b.text("Task", 60.0, 560.0, w - 120.0, "Use three of these words in a sentence about a puddle.", t.b(22.0, t.dark).left());
    b.mark_last("three", t.primary, true);
    b.text("Page number", w - 120.0, h - 50.0, 60.0, "03", t.b(16.0, t.primary).bold().right());
    b.notes("Read each word aloud together. Pairs then write their puddle sentence on mini whiteboards.");

    b.new_page("Quiz");
    b.bg(solid(t.dark));
    b.text("Kicker", 60.0, 50.0, 600.0, "Quick quiz", t.b(20.0, t.secondary).bold().upper().ls(300.0).left());
    b.text("Title", 60.0, 84.0, 900.0, "Check your understanding", t.h(44.0, t.light).left());
    let qs = [
        ("What turns water into vapour?", ["Wind", "The sun's heat", "Salt"]),
        ("Clouds are made of…", ["Tiny drops of water", "Smoke", "Cotton"]),
        ("Which is not precipitation?", ["Snow", "Hail", "Fog"]),
    ];
    for (i, (q, answers)) in qs.iter().enumerate() {
        let y = 180.0 + 180.0 * i as f64;
        b.text(&format!("Question {}", i + 1), 60.0, y, w - 120.0, &format!("{}. {q}", i + 1), t.b(26.0, t.light).bold().left());
        for (j, (a, letter)) in answers.iter().zip(["A", "B", "C"]).enumerate() {
            let x = 60.0 + 300.0 * j as f64;
            b.pill(
                &format!("Q{} answer {letter}", i + 1),
                x,
                y + 50.0,
                280.0,
                64.0,
                solid(t.primary),
                &format!("{letter}  {a}"),
                t.b(22.0, t.light).bold(),
            );
        }
    }
    b.text("Page number", w - 120.0, h - 50.0, 60.0, "04", t.b(16.0, t.secondary).bold().right());
    b.notes("Answers: 1 B (the sun's heat), 2 A (tiny drops of water), 3 C (fog is a cloud at ground level).");
}

fn commute_infographic(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.rect("Header", 0.0, 0.0, w, 520.0, 0.0, lin(135.0, t.dark, t.primary.lerp(t.dark, 0.3)));
    b.icon("Bike", "bike", 1060.0, 120.0, 280.0, 280.0, solid(t.light.with_alpha(60)));
    b.text("Kicker", 120.0, 120.0, 900.0, "Riverton travel survey 2026", t.b(32.0, t.light).bold().upper().ls(250.0).left());
    b.text("Headline", 120.0, 180.0, 1000.0, "How Riverton\ngets to work", t.h(110.0, t.light).lh(1.0).left());
    b.text("Lead", 120.0, 420.0, 1000.0, "2,140 people told us how they travel, and why.", t.b(36.0, t.light).left());
    let stats = [("38%", "now walk or cycle"), ("−12%", "car trips since 2022"), ("24 min", "average commute")];
    for (i, (value, label)) in stats.iter().enumerate() {
        let x = 120.0 + 420.0 * i as f64;
        b.text(&format!("Stat {} value", i + 1), x, 600.0, 380.0, value, t.h(100.0, if i == 1 { t.secondary } else { t.primary }).left());
        b.text(&format!("Stat {} label", i + 1), x, 720.0, 380.0, label, t.b(30.0, t.dark).left());
    }
    let section = |b: &mut B, n: usize, y: f64, title: &str, note: &str| {
        b.rect(&format!("Section {n} rule"), 120.0, y, w - 240.0, 2.0, 0.0, solid(t.primary.lerp(t.light, 0.6)));
        b.text(&format!("Section {n} title"), 120.0, y + 40.0, w - 240.0, title, t.h(52.0, t.dark).left());
        b.text(&format!("Section {n} note"), 120.0, y + 112.0, w - 240.0, note, t.b(28.0, t.dark).left());
    };
    section(b, 1, 830.0, "Younger people cycle most", "Main way to work, by age group (% of people)");
    let mut c = t.chart(
        ChartKind::HorizontalBar,
        &["18–29", "30–44", "45–64", "65+"],
        &[
            ("Walk", &[18.0, 14.0, 12.0, 20.0]),
            ("Bike", &[31.0, 24.0, 15.0, 8.0]),
            ("Bus", &[22.0, 18.0, 20.0, 34.0]),
            ("Car", &[29.0, 44.0, 53.0, 38.0]),
        ],
        24.0,
        t.dark,
    );
    c.stacked = true;
    c.gridlines = false;
    c.legend_position = LegendPosition::Top;
    c.colors = vec![t.primary.lerp(t.light, 0.45), t.primary, t.secondary, t.dark.lerp(t.light, 0.55)];
    b.chart("Age chart", 120.0, 1000.0, w - 240.0, 460.0, c);
    section(b, 2, 1520.0, "Weekend cycling keeps climbing", "Bike trips per day across the town's counters");
    let mut c = t.chart(
        ChartKind::Area,
        &["2022", "2023", "2024", "2025", "2026"],
        &[("Weekdays", &[2100.0, 2600.0, 3300.0, 3900.0, 4400.0]), ("Weekends", &[1500.0, 1900.0, 2700.0, 3600.0, 4600.0])],
        24.0,
        t.dark,
    );
    c.colors = vec![t.primary, t.secondary];
    c.show_values = false;
    c.legend_position = LegendPosition::Top;
    b.chart("Trips chart", 120.0, 1690.0, w - 240.0, 440.0, c);
    b.rect("Footer", 0.0, h - 200.0, w, 200.0, 0.0, solid(t.dark));
    b.text(
        "Method",
        120.0,
        h - 150.0,
        w - 240.0,
        "Online and street survey, March–April 2026 · counter data from Riverton Council",
        t.b(26.0, t.light).left(),
    );
    b.text("Source", 120.0, h - 96.0, w - 240.0, "riverton.example/travel", t.b(26.0, t.primary.lerp(t.light, 0.5)).bold().left());
}
