//! Documents: presentations (multi-slide, with speaker notes) and resumes.

use genha_doc::ShapeKind;

use super::{DOCUMENTS, Def, def, lin, solid};
use crate::build::{B, Sty, Theme};

pub(super) const DEFS: &[Def] = &[
    def(
        "team-presentation",
        "Quarterly review deck",
        DOCUMENTS,
        "presentation",
        &["presentation", "slides", "business", "report"],
        || Theme::new(0x2563eb, 0xf97316, 0x0f172a, 0xf8fafc, "Sans", "Sans"),
        presentation,
    ),
    def(
        "pitch-deck",
        "Startup pitch deck",
        DOCUMENTS,
        "presentation",
        &["presentation", "pitch deck", "slides", "startup", "investors", "business"],
        || Theme::new(0x7b2cbf, 0x00bbf9, 0x1a0b2e, 0xfbf8ff, "Display", "Sans"),
        pitch_deck,
    ),
    def(
        "annual-report-deck",
        "Annual report slides",
        DOCUMENTS,
        "presentation-4x3",
        &["presentation", "report", "annual report", "slides", "charity", "numbers"],
        || Theme::new(0x13786d, 0xed217c, 0x2d3047, 0xf7f7f2, "Sans", "Sans"),
        report_deck,
    ),
    def(
        "modern-resume",
        "Modern resume",
        DOCUMENTS,
        "resume",
        &["resume", "cv", "job", "a4"],
        || Theme::new(0x334155, 0x38bdf8, 0x0f172a, 0xffffff, "Sans", "Sans"),
        resume,
    ),
    def(
        "classic-resume",
        "Classic one-column resume",
        DOCUMENTS,
        "resume",
        &["resume", "cv", "job", "classic", "serif", "a4"],
        || Theme::new(0x22223b, 0x9a8c98, 0x1a1a1d, 0xffffff, "Serif", "Sans"),
        classic_resume,
    ),
    def(
        "creative-resume",
        "Creative resume with skill bars",
        DOCUMENTS,
        "resume",
        &["resume", "cv", "job", "creative", "designer", "a4"],
        || Theme::new(0xf3722c, 0x277da1, 0x1d1d1f, 0xfffdfa, "Display", "Sans"),
        creative_resume,
    ),
];

// ---------------------------------------------------------------- presentations

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
        b.text(&format!("Card {n} number"), x + 40.0, 364.0, 96.0, &n.to_string(), t.h(48.0, if i == 1 { t.dark } else { t.light }).lh(1.0));
        b.text(&format!("Card {n} title"), x + 40.0, 488.0, 420.0, title, t.h(44.0, t.dark).left());
        b.text(&format!("Card {n} text"), x + 40.0, 568.0, 420.0, body, t.b(30.0, t.dark.lerp(t.light, 0.25)).lh(1.45).left());
    }
    b.text("Page number", 1680.0, 975.0, 100.0, "02", t.b(28.0, t.primary).bold().right());
    b.notes("Each of these moved a key metric; details in the appendix.");
}

/// A slide header: kicker, title and page number.
fn slide_head(b: &mut B, kicker: &str, title: &str, page: usize, dark_bg: bool) {
    let t = b.t.clone();
    let ink = if dark_bg { t.light } else { t.dark };
    b.text("Kicker", 140.0, 110.0, 1200.0, kicker, t.b(30.0, if dark_bg { t.secondary } else { t.primary }).bold().upper().ls(300.0).left());
    b.text("Title", 140.0, 160.0, 1640.0, title, t.h(80.0, ink).left());
    b.text("Page number", 1680.0, 990.0, 100.0, &format!("{page:02}"), t.b(26.0, if dark_bg { t.secondary } else { t.primary }).bold().right());
}

fn pitch_deck(b: &mut B) {
    let t = b.t.clone();
    b.name_page("Title");
    b.bg(solid(t.dark));
    b.icon("Blob", "blob-1", 1120.0, 140.0, 720.0, 720.0, solid(t.primary));
    b.icon("Rocket", "rocket", 1330.0, 350.0, 300.0, 300.0, solid(t.secondary));
    b.text("Company", 140.0, 250.0, 800.0, "Lumen", t.b(36.0, t.secondary).bold().upper().ls(400.0).left());
    b.text("Title", 140.0, 320.0, 1000.0, "Lighting for\nsmall homes", t.h(120.0, t.light).lh(1.05).left());
    b.text("Subtitle", 140.0, 640.0, 900.0, "Seed round · 2026", t.b(40.0, t.light.lerp(t.dark, 0.25)).left());
    b.rect("Accent", 140.0, 740.0, 120.0, 8.0, 4.0, solid(t.secondary));
    b.text("Website", 140.0, 940.0, 600.0, "lumen.example", t.b(28.0, t.light.lerp(t.dark, 0.4)).left());
    b.notes("Hi, we're Lumen. We make renter-friendly lighting that sets itself up.");

    b.new_page("Problem");
    b.bg(solid(t.light));
    slide_head(b, "The problem", "Rented flats have bad light", 2, false);
    let pains = ["Ceiling lights are glaring and flat", "Drilling holes can break the lease", "Smart lamps are fiddly and pricey"];
    for (i, pain) in pains.iter().enumerate() {
        let y = 400.0 + 120.0 * i as f64;
        b.icon(&format!("Problem {} icon", i + 1), "warning", 140.0, y, 64.0, 64.0, solid(t.primary));
        b.text(&format!("Problem {}", i + 1), 240.0, y + 8.0, 900.0, pain, t.b(40.0, t.dark).left());
    }
    b.rect("Stat card", 1260.0, 340.0, 520.0, 560.0, 40.0, solid(t.primary));
    b.text("Stat", 1260.0, 440.0, 520.0, "7 in 10", t.h(130.0, t.light).lh(1.0));
    b.text("Stat label", 1290.0, 640.0, 460.0, "renters say their home\nfeels too dark in winter", t.b(34.0, t.light).lh(1.4));
    b.notes("We surveyed 1,200 renters in four cities. Lighting was the top complaint after noise.");

    b.new_page("Solution");
    b.bg(solid(t.light));
    slide_head(b, "Our solution", "A clip-on light that sets itself up", 3, false);
    let features = [
        ("lightbulb", "Warm and even", "Soft light that follows\nthe time of day."),
        ("wifi", "Pairs in seconds", "Scan once, then it just\nworks with your phone."),
        ("battery", "Lasts a month", "No wires and no drilling,\ncharge it like a phone."),
    ];
    for (i, (icon, title, body)) in features.iter().enumerate() {
        let x = 140.0 + 570.0 * i as f64;
        let n = i + 1;
        b.rect(&format!("Feature {n} card"), x, 360.0, 500.0, 520.0, 32.0, solid(t.primary.lerp(t.light, 0.9)));
        b.ellipse(&format!("Feature {n} disc"), x + 40.0, 400.0, 130.0, 130.0, solid(t.secondary));
        b.icon(&format!("Feature {n} icon"), icon, x + 70.0, 430.0, 70.0, 70.0, solid(t.dark));
        b.text(&format!("Feature {n} title"), x + 40.0, 580.0, 440.0, title, t.h(48.0, t.dark).left());
        b.text(&format!("Feature {n} text"), x + 40.0, 660.0, 440.0, body, t.b(30.0, t.dark.lerp(t.light, 0.25)).lh(1.45).left());
    }
    b.notes("Three promises: good light, no setup pain, no tools.");

    b.new_page("Market");
    b.bg(solid(t.dark));
    slide_head(b, "Market", "A growing market", 4, true);
    let bars = [("2022", "1.2B", 180.0), ("2023", "1.7B", 260.0), ("2024", "2.3B", 340.0), ("2025", "3.1B", 450.0), ("2026", "4.1B", 560.0)];
    for (i, (year, value, height)) in bars.iter().enumerate() {
        let x = 160.0 + 200.0 * i as f64;
        let last = i == bars.len() - 1;
        b.rect(&format!("Bar {year}"), x, 900.0 - height, 140.0, *height, 12.0, solid(if last { t.secondary } else { t.primary }));
        b.text(&format!("Bar {year} value"), x - 10.0, 900.0 - height - 50.0, 160.0, value, t.b(30.0, t.light).bold());
        b.text(&format!("Bar {year} label"), x - 10.0, 920.0, 160.0, year, t.b(26.0, t.light.lerp(t.dark, 0.3)));
    }
    b.text("Market size", 1240.0, 420.0, 560.0, "$4.1B", t.h(130.0, t.secondary).lh(1.0).left());
    b.text("Market label", 1240.0, 580.0, 560.0, "addressable market\nin Europe by 2026", t.b(36.0, t.light).lh(1.4).left());
    b.notes("Figures are illustrative: replace with your own market research.");

    b.new_page("Ask");
    b.bg(lin(135.0, t.primary, t.primary.lerp(t.dark, 0.5)));
    b.text("Kicker", 140.0, 200.0, 900.0, "The ask", t.b(30.0, t.secondary).bold().upper().ls(300.0).left());
    b.text("Title", 140.0, 250.0, 1000.0, "Join us", t.h(130.0, t.light).left());
    b.text("Body", 140.0, 440.0, 1000.0, "We're raising $1.5M to ship\nour first 20,000 lights.", t.b(48.0, t.light).lh(1.35).left());
    b.pill("Contact", 140.0, 640.0, 560.0, 96.0, solid(t.secondary), "hello@lumen.example", t.b(36.0, t.dark).bold());
    for (i, (initials, name)) in [("AK", "Ada, CEO"), ("JM", "Jun, hardware"), ("SR", "Sol, design")].iter().enumerate() {
        let x = 1180.0 + 220.0 * i as f64;
        b.ellipse(&format!("{name} photo"), x, 380.0, 180.0, 180.0, solid(t.light));
        b.text(&format!("{name} initials"), x, 440.0, 180.0, initials, t.h(56.0, t.primary).lh(1.0));
        b.text(&format!("{name} name"), x - 20.0, 590.0, 220.0, name, t.b(28.0, t.light));
    }
    b.text("Page number", 1680.0, 990.0, 100.0, "05", t.b(26.0, t.light).bold().right());
    b.notes("Thank you. Happy to take questions.");
}

fn report_deck(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    let head = |b: &mut B, title: &str, page: usize| {
        b.rect("Top bar", 0.0, 0.0, w, 10.0, 0.0, solid(t.primary));
        b.text("Title", 80.0, 60.0, 860.0, title, t.h(48.0, t.dark).left());
        b.text("Page number", 860.0, 710.0, 100.0, &format!("{page:02}"), t.b(18.0, t.primary).bold().right());
    };
    b.name_page("Cover");
    b.bg(solid(t.dark));
    b.rect("Accent band", 0.0, 0.0, 24.0, h, 0.0, solid(t.primary));
    b.icon("Chart", "chart-bars", 760.0, 90.0, 180.0, 180.0, solid(t.secondary));
    b.text("Kicker", 80.0, 230.0, 700.0, "Riverside Community Trust", t.b(24.0, t.primary.lerp(t.light, 0.45)).bold().upper().ls(200.0).left());
    b.text("Title", 80.0, 280.0, 860.0, "Annual report\n2026", t.h(80.0, t.light).left());
    b.text("Subtitle", 80.0, 490.0, 860.0, "What we did together this year", t.b(28.0, t.light.lerp(t.dark, 0.25)).left());
    b.notes("Thank everyone for coming; this takes about ten minutes.");

    b.new_page("At a glance");
    b.bg(solid(t.light));
    head(b, "The year at a glance", 2);
    let tiles = [("1,240", "volunteer hours"), ("86", "events hosted"), ("$212k", "raised for local projects"), ("14", "new partner groups")];
    for (i, (value, label)) in tiles.iter().enumerate() {
        let (x, y) = (80.0 + 452.0 * (i % 2) as f64, 170.0 + 260.0 * (i / 2) as f64);
        b.rect(&format!("Tile {}", i + 1), x, y, 412.0, 220.0, 20.0, solid(t.primary.lerp(t.light, 0.88)));
        b.text(&format!("Tile {} value", i + 1), x + 30.0, y + 36.0, 360.0, value, t.h(64.0, if i == 3 { t.secondary } else { t.primary }).left());
        b.text(&format!("Tile {} label", i + 1), x + 30.0, y + 130.0, 360.0, label, t.b(22.0, t.dark).left());
    }
    b.notes("Every number here is up on last year.");

    b.new_page("Spending");
    b.bg(solid(t.light));
    head(b, "Where the money went", 3);
    let rows = [("Youth programmes", 0.38), ("Green spaces", 0.27), ("Food bank", 0.21), ("Running costs", 0.14)];
    for (i, (label, share)) in rows.iter().enumerate() {
        let y = 200.0 + 110.0 * i as f64;
        b.text(&format!("{label} label"), 80.0, y, 300.0, label, t.b(24.0, t.dark).left());
        b.rect(&format!("{label} track"), 80.0, y + 40.0, 760.0, 28.0, 14.0, solid(t.dark.lerp(t.light, 0.9)));
        b.rect(&format!("{label} bar"), 80.0, y + 40.0, 760.0 * share / 0.38, 28.0, 14.0, solid(if i == 0 { t.secondary } else { t.primary }));
        b.text(&format!("{label} share"), 850.0, y + 36.0, 100.0, &format!("{:.0}%", share * 100.0), t.b(24.0, t.dark).bold().right());
    }
    b.text("Footnote", 80.0, 660.0, 860.0, "Shares of total spending; full accounts on our website.", t.b(16.0, t.dark.lerp(t.light, 0.25)).left());
    b.notes("Youth work grew most; running costs stayed under 15 percent.");

    b.new_page("Next year");
    b.bg(solid(t.light));
    head(b, "Looking ahead", 4);
    let goals = ["Open a second community kitchen", "Plant 500 street trees with schools", "Double our youth mentoring places"];
    for (i, goal) in goals.iter().enumerate() {
        let y = 190.0 + 90.0 * i as f64;
        b.icon(&format!("Goal {} check", i + 1), "check-circle", 80.0, y, 48.0, 48.0, solid(t.primary));
        b.text(&format!("Goal {}", i + 1), 150.0, y + 6.0, 780.0, goal, t.b(28.0, t.dark).left());
    }
    b.rect("Thanks banner", 80.0, 520.0, w - 160.0, 120.0, 20.0, solid(t.secondary));
    b.icon("Heart", "heart", 116.0, 552.0, 56.0, 56.0, solid(t.light));
    b.text("Thanks", 200.0, 558.0, 700.0, "Thank you to every volunteer", t.h(36.0, t.light).left());
    b.notes("Close with thanks and the volunteer sign-up link.");
}

// ---------------------------------------------------------------- resumes

fn resume(b: &mut B) {
    let t = b.t.clone();
    let h = b.h;
    b.bg(solid(t.light));
    b.rect("Sidebar", 0.0, 0.0, 260.0, h, 0.0, solid(t.primary));
    b.ellipse("Photo placeholder", 60.0, 60.0, 140.0, 140.0, solid(t.secondary));
    b.text("Initials", 60.0, 100.0, 140.0, "AM", t.h(48.0, t.dark).lh(1.0));
    let side = |b: &mut B, title: &str, y: f64, body: &str| {
        b.text(&format!("{title} heading"), 36.0, y, 190.0, title, t.b(16.0, t.secondary).bold().upper().ls(200.0).left());
        b.text(title, 36.0, y + 30.0, 190.0, body, t.b(15.0, t.light).lh(1.6).left());
    };
    side(b, "Contact", 250.0, "alex@mail.example\n+1 555 0100 200\nSpringfield");
    side(b, "Skills", 390.0, "Brand identity\nTypography\nArt direction\nPrototyping\nMotion basics");
    side(b, "Languages", 590.0, "English (native)\nSpanish (fluent)");

    let x = 300.0;
    b.text("Name", x, 64.0, 450.0, "Alex Morgan", t.h(52.0, t.dark).left());
    b.text("Job title", x, 136.0, 450.0, "Senior Graphic Designer", t.b(24.0, t.secondary.lerp(t.dark, 0.5)).left());
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
        b.text(&format!("{name} dates"), x, y + 27.0, 300.0, dates, t.b(14.0, t.secondary.lerp(t.dark, 0.5)).left());
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

fn classic_resume(b: &mut B) {
    let t = b.t.clone();
    let w = b.w;
    let (x, cw) = (60.0, w - 120.0);
    b.bg(solid(t.light));
    b.ctext("Name", 64.0, cw, "Elena Hartmann", t.h(44.0, t.dark));
    b.ctext("Job title", 126.0, cw, "Marketing Manager", t.b(18.0, t.secondary.lerp(t.dark, 0.3)).upper().ls(300.0));
    b.ctext("Contact", 162.0, cw, "elena@mail.example · +49 555 0100 · Berlin", t.b(14.0, t.dark));
    b.rect("Top rule", x, 200.0, cw, 1.5, 0.0, solid(t.primary));
    let section = |b: &mut B, title: &str, y: f64| {
        b.text(&format!("{title} heading"), x, y, 400.0, title, t.h(18.0, t.primary).upper().ls(200.0).left());
        b.rect(&format!("{title} rule"), x, y + 30.0, cw, 1.0, 0.0, solid(t.secondary));
    };
    let entry = |b: &mut B, name: &str, y: f64, role: &str, dates: &str, details: &str| {
        b.text(name, x, y, 470.0, role, t.h(16.0, t.dark).left());
        b.text(&format!("{name} dates"), x + cw - 200.0, y, 200.0, dates, t.b(14.0, t.secondary.lerp(t.dark, 0.3)).right());
        b.text(&format!("{name} details"), x, y + 28.0, cw, details, t.b(14.0, t.dark).lh(1.5).left());
    };
    section(b, "Profile", 225.0);
    b.text(
        "Profile",
        x,
        270.0,
        cw,
        "Marketing lead with eight years in consumer brands. I plan campaigns\nend to end, from research to launch, and build teams that like\nworking together.",
        t.b(15.0, t.dark).lh(1.55).left(),
    );
    section(b, "Experience", 380.0);
    entry(
        b,
        "Job 1",
        425.0,
        "Marketing Manager · Fernway Foods",
        "2022 – now",
        "• Grew newsletter sign-ups from 8,000 to 31,000\n• Led a rebrand across 40 products and two markets",
    );
    entry(
        b,
        "Job 2",
        525.0,
        "Brand Specialist · Oakline Retail",
        "2018 – 2022",
        "• Ran seasonal campaigns with a yearly budget of $1.2M\n• Wrote a shared brand guide for six agencies",
    );
    section(b, "Education", 640.0);
    entry(b, "Masters", 685.0, "MA Strategic Marketing · Northgate University", "2016 – 2018", "Thesis on loyalty programmes in local retail");
    entry(b, "Bachelors", 755.0, "BA Media Studies · Northgate University", "2013 – 2016", "Editor of the student magazine");
    section(b, "Skills", 840.0);
    b.text(
        "Skills",
        x,
        885.0,
        cw,
        "Campaign planning · Brand strategy · Copywriting\nAnalytics · Team leadership · Public speaking",
        t.b(15.0, t.dark).lh(1.6).left(),
    );
    section(b, "Languages", 960.0);
    b.text("Languages", x, 1005.0, cw, "German (native) · English (fluent) · French (good)", t.b(15.0, t.dark).left());
}

fn creative_resume(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.rect("Header", 0.0, 0.0, w, 230.0, 0.0, solid(t.dark));
    b.ellipse("Photo placeholder", 60.0, 45.0, 140.0, 140.0, solid(t.primary));
    b.text("Initials", 60.0, 89.0, 140.0, "KO", t.h(52.0, t.dark).lh(1.0));
    b.text("Name", 230.0, 56.0, 520.0, "Kai Osei", t.h(52.0, t.light).left());
    b.text("Job title", 230.0, 126.0, 520.0, "UX and Visual Designer", t.b(22.0, t.primary).bold().left());
    b.text("Contact", 230.0, 168.0, 520.0, "kai@osei.example · osei.example · Accra", t.b(14.0, t.light.lerp(t.dark, 0.25)).left());

    let heading = |b: &mut B, title: &str, x: f64, y: f64, c| {
        b.text(&format!("{title} heading"), x, y, 220.0, title, t.h(18.0, c).upper().ls(150.0).left());
    };
    heading(b, "Skills", 60.0, 270.0, t.secondary);
    let skills = [("User research", 0.9), ("Wireframing", 0.85), ("Visual design", 0.95), ("Prototyping", 0.8), ("Design systems", 0.75)];
    for (i, (skill, level)) in skills.iter().enumerate() {
        let y = 312.0 + 56.0 * i as f64;
        b.text(skill, 60.0, y, 220.0, skill, t.b(14.0, t.dark).left());
        b.rect(&format!("{skill} track"), 60.0, y + 24.0, 200.0, 8.0, 4.0, solid(t.secondary.lerp(t.light, 0.8)));
        b.rect(&format!("{skill} level"), 60.0, y + 24.0, 200.0 * level, 8.0, 4.0, solid(t.secondary));
    }
    heading(b, "Tools", 60.0, 610.0, t.secondary);
    b.text("Tools", 60.0, 648.0, 220.0, "Illustration\nMotion basics\nHTML and CSS\nUsability testing", t.b(14.0, t.dark).lh(1.7).left());
    heading(b, "Languages", 60.0, 790.0, t.secondary);
    b.text("Languages", 60.0, 828.0, 220.0, "English (fluent)\nTwi (native)\nFrench (basic)", t.b(14.0, t.dark).lh(1.7).left());

    let x = 320.0;
    let accent = t.primary.lerp(t.dark, 0.35);
    heading(b, "About", x, 270.0, accent);
    b.text(
        "About",
        x,
        305.0,
        413.0,
        "I design calm, useful products for\nhealth and education teams, and I\nlove turning research into ideas.",
        t.b(15.0, t.dark).lh(1.55).left(),
    );
    heading(b, "Experience", x, 410.0, accent);
    b.rect("Timeline", x + 6.0, 452.0, 2.0, 330.0, 1.0, solid(t.primary.lerp(t.light, 0.6)));
    let jobs = [
        ("Senior Product Designer", "Brightpath Health · 2023 – now", "Redesigned patient booking, cutting\nmissed appointments by a fifth"),
        ("Product Designer", "Learnly · 2020 – 2023", "Built the design system for three\napps used by 200,000 students"),
        ("Junior Designer", "Studio Kente · 2018 – 2020", "Brand, web and print work for local\nnon-profits and start-ups"),
    ];
    for (i, (role, meta, details)) in jobs.iter().enumerate() {
        let y = 450.0 + 120.0 * i as f64;
        b.ellipse(&format!("{role} dot"), x, y + 4.0, 14.0, 14.0, solid(t.primary));
        b.text(role, x + 30.0, y, 383.0, role, t.h(16.0, t.dark).left());
        b.text(&format!("{role} meta"), x + 30.0, y + 24.0, 383.0, meta, t.b(13.0, t.secondary).bold().left());
        b.text(&format!("{role} details"), x + 30.0, y + 46.0, 383.0, details, t.b(14.0, t.dark).lh(1.5).left());
    }
    heading(b, "Education", x, 830.0, accent);
    b.text("Education", x, 868.0, 413.0, "BSc Computer Science\nCoastal University · 2014 – 2018", t.b(14.0, t.dark).lh(1.5).left());
    b.rect("Footer", 0.0, h - 40.0, w, 40.0, 0.0, solid(t.primary));
}
