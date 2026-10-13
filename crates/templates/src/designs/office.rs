//! Office and school documents: letterhead, invoice, cover letter, resume, newsletter, timetable
//! and worksheet.

use genha_doc::{BorderLines, CellRange, ChartKind, ShapeKind, StrokeStyle, TextAlign};

use super::{DOCUMENTS, Def, def, lin, solid};
use crate::build::{B, Sty, Theme};

pub(super) const DEFS: &[Def] = &[
    def(
        "company-letterhead",
        "Company letterhead",
        DOCUMENTS,
        "document-a4",
        &["letterhead", "letter", "stationery", "business", "a4", "brand"],
        || Theme::new(0x0f4c81, 0x5bc0be, 0x0b132b, 0xffffff, "Sans", "Sans"),
        letterhead,
    ),
    def(
        "service-invoice",
        "Service invoice",
        DOCUMENTS,
        "document-a4",
        &["invoice", "bill", "payment", "business", "freelance", "table"],
        || Theme::new(0x0f766e, 0xfbbf24, 0x134e4a, 0xffffff, "Sans", "Sans"),
        invoice,
    ),
    def(
        "cover-letter",
        "Modern cover letter",
        DOCUMENTS,
        "document-a4",
        &["cover letter", "letter", "job", "application", "career", "a4"],
        || Theme::new(0xbe123c, 0xfda4af, 0x1f2937, 0xffffff, "Serif", "Sans"),
        cover_letter,
    ),
    def(
        "minimal-resume",
        "Minimal two-column resume",
        DOCUMENTS,
        "resume",
        &["resume", "cv", "job", "minimal", "two column", "a4"],
        || Theme::new(0x111827, 0x047857, 0x030712, 0xffffff, "Sans", "Sans"),
        minimal_resume,
    ),
    def(
        "community-newsletter",
        "Community newsletter",
        DOCUMENTS,
        "document-a4",
        &["newsletter", "community", "news", "print", "chart", "a4"],
        || Theme::new(0xc2410c, 0x0369a1, 0x1c1917, 0xfffaf5, "Serif", "Sans"),
        newsletter,
    ),
    def(
        "class-timetable",
        "Weekly class timetable",
        DOCUMENTS,
        "document-a4",
        &["timetable", "schedule", "school", "education", "table", "week"],
        || Theme::new(0x2563eb, 0xf59e0b, 0x1e293b, 0xffffff, "Display", "Sans"),
        timetable,
    )
    .landscape(),
    def(
        "fractions-worksheet",
        "Fractions worksheet",
        DOCUMENTS,
        "document-a4",
        &["worksheet", "school", "education", "maths", "kids", "printable"],
        || Theme::new(0x7c3aed, 0x10b981, 0x1e1b4b, 0xffffff, "Display", "Sans"),
        worksheet,
    ),
];

fn letterhead(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.shape("Logo mark", ShapeKind::Polygon { sides: 6 }, 60.0, 50.0, 64.0, 64.0, solid(t.primary));
    b.text("Logo initials", 60.0, 69.0, 64.0, "CR", t.h(22.0, t.light).lh(1.15));
    b.text("Company", 140.0, 54.0, 340.0, "Clearwater Research", t.h(24.0, t.dark).left());
    b.text("Company line", 140.0, 88.0, 340.0, "Water quality testing & advice", t.b(13.0, t.primary).left());
    b.text("Contact", w - 300.0, 54.0, 240.0, "hello@clearwater.example\n+44 555 0100 300\nclearwater.example", t.b(12.0, t.dark).lh(1.5).right());
    b.rect("Header rule", 60.0, 140.0, w - 120.0, 2.0, 0.0, solid(t.secondary));
    b.text("Date", 60.0, 190.0, 400.0, "14 October 2026", t.b(14.0, t.dark).left());
    b.text("Recipient", 60.0, 230.0, 400.0, "Ms R. Patel\nFacilities Manager\nNorthgate School\nLeeds LS2 7AB", t.b(14.0, t.dark).lh(1.5).left());
    b.text("Salutation", 60.0, 350.0, 600.0, "Dear Ms Patel,", t.b(14.0, t.dark).left());
    b.text(
        "Body",
        60.0,
        385.0,
        w - 120.0,
        "Thank you for inviting us to test the drinking water at Northgate School. Our team\nvisited on 6 October and took samples from twelve taps across the main building.\n\nAll samples met the national standard. Two taps in the sports hall showed raised\niron levels; we recommend flushing them for two minutes each morning and testing\nagain in spring. The full report is enclosed.\n\nPlease call me with any questions.",
        t.b(14.0, t.dark).lh(1.6).left(),
    );
    b.text("Sign-off", 60.0, 640.0, 400.0, "Kind regards,", t.b(14.0, t.dark).left());
    b.text("Signer", 60.0, 700.0, 400.0, "Tom Ellery", t.h(18.0, t.dark).left());
    b.text("Signer role", 60.0, 726.0, 400.0, "Senior Scientist", t.b(13.0, t.primary).left());
    b.rect("Footer band", 0.0, h - 60.0, w, 60.0, 0.0, solid(t.primary));
    b.rect("Footer accent", 0.0, h - 66.0, w, 6.0, 0.0, solid(t.secondary));
    b.text(
        "Footer",
        60.0,
        h - 40.0,
        w - 120.0,
        "Clearwater Research Ltd · 9 Dock Street, Leeds LS10 1JF · Company no. 01234567",
        t.b(11.0, t.light).left(),
    );
}

fn invoice(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.rect("Header", 0.0, 0.0, w, 170.0, 0.0, solid(t.primary));
    b.text("Title", 60.0, 50.0, 300.0, "Invoice", t.h(48.0, t.light).left());
    b.text("Studio", 60.0, 112.0, 340.0, "Juno Design Studio", t.b(15.0, t.light).bold().left());
    b.text("Invoice meta", w - 320.0, 52.0, 260.0, "No. 2026-041\nIssued 14 Oct 2026\nDue 13 Nov 2026", t.b(13.0, t.light).lh(1.55).right());
    b.text("From heading", 60.0, 210.0, 300.0, "From", t.b(12.0, t.primary).bold().upper().ls(250.0).left());
    b.text("From", 60.0, 234.0, 300.0, "Juno Design Studio\n4 Weaver Yard, Bristol\njuno@studio.example", t.b(13.0, t.dark).lh(1.5).left());
    b.text("To heading", 400.0, 210.0, 300.0, "Bill to", t.b(12.0, t.primary).bold().upper().ls(250.0).left());
    b.text("To", 400.0, 234.0, 330.0, "Harbour Books Ltd\nAttn: Carla Mendes\n12 Quay Street, Bristol", t.b(13.0, t.dark).lh(1.5).left());
    let mut tb = t.table(
        &[
            &["Description", "Qty", "Rate", "Amount"],
            &["Logo refresh and brand guide", "1", "$1,800", "$1,800"],
            &["Shop window graphics", "3", "$240", "$720"],
            &["Bookmark design, two versions", "2", "$150", "$300"],
            &["Print management", "4 h", "$60", "$240"],
            &["Subtotal", "", "", "$3,060"],
            &["Tax (20%)", "", "", "$612"],
            &["Total due", "", "", "$3,672"],
        ],
        13.0,
    );
    tb.column_widths = vec![0.55, 0.12, 0.15, 0.18];
    tb.borders = BorderLines::Horizontal;
    tb.alt_row_fill = None;
    // The total rows: their label spans the first three columns.
    tb.merges = (5..8).map(|r| CellRange::new(r, 0, 1, 3)).collect();
    tb.style_cells(CellRange::new(0, 1, 8, 3), |s| s.align = Some(TextAlign::Right));
    tb.style_cells(CellRange::new(5, 0, 2, 1), |s| s.align = Some(TextAlign::Right));
    tb.style_cells(CellRange::new(7, 0, 1, 4), |s| {
        s.fill = Some(t.primary.lerp(t.light, 0.85));
        s.bold = Some(true);
        s.font_size = Some(15.0);
    });
    tb.style_cells(CellRange::new(7, 0, 1, 1), |s| s.align = Some(TextAlign::Right));
    // 13 px text and 7 px padding: 30.25 px a row, the total row 32.75 px.
    b.table("Items", 60.0, 330.0, w - 120.0, 7.0 * 30.25 + 33.0, tb);
    b.text("Payment heading", 60.0, 610.0, 400.0, "Payment details", t.b(12.0, t.primary).bold().upper().ls(250.0).left());
    b.text(
        "Payment",
        60.0,
        634.0,
        400.0,
        "Bank: Riverside Bank\nAccount: 12345678 · Sort code 00-11-22\nReference: 2026-041",
        t.b(13.0, t.dark).lh(1.55).left(),
    );
    b.rect("Due card", w - 300.0, 610.0, 240.0, 100.0, 12.0, solid(t.secondary));
    b.text("Due label", w - 300.0, 626.0, 240.0, "Amount due", t.b(13.0, t.dark).bold().upper().ls(200.0));
    b.text("Due amount", w - 300.0, 650.0, 240.0, "$3,672", t.h(36.0, t.dark).lh(1.2));
    b.text("Thanks", 60.0, h - 120.0, w - 120.0, "Thank you for your business!", t.h(18.0, t.primary).left());
    b.text(
        "Terms",
        60.0,
        h - 90.0,
        w - 120.0,
        "Payment within 30 days. Late payments may be charged interest at 2% a month.",
        t.b(11.0, t.dark).left(),
    );
}

fn cover_letter(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.rect("Accent bar", 0.0, 0.0, 14.0, h, 0.0, solid(t.primary));
    b.text("Name", 70.0, 60.0, 500.0, "Hannah Kowalski", t.h(40.0, t.dark).left());
    b.text("Role", 70.0, 112.0, 500.0, "Customer Experience Lead", t.b(16.0, t.primary).bold().upper().ls(200.0).left());
    b.text("Contact", w - 290.0, 64.0, 230.0, "hannah@mail.example\n+48 555 0100 77\nKraków, Poland", t.b(12.0, t.dark).lh(1.55).right());
    b.rect("Rule", 70.0, 160.0, w - 130.0, 1.5, 0.0, solid(t.secondary));
    b.text("Date", 70.0, 200.0, 400.0, "14 October 2026", t.b(14.0, t.dark).left());
    b.text("Recipient", 70.0, 236.0, 400.0, "Hiring team\nBrightpath Travel\nWarsaw", t.b(14.0, t.dark).lh(1.5).left());
    b.text("Salutation", 70.0, 330.0, 500.0, "Dear hiring team,", t.b(15.0, t.dark).bold().left());
    b.text(
        "Body",
        70.0,
        368.0,
        w - 130.0,
        "I'm writing to apply for the Head of Customer Care role. For six years I have\nled support teams that customers actually like talking to, and I would love to\nbring that to Brightpath.\n\nAt Nordlys Air I grew a team of 5 into 22 across three time zones, cut the average\nreply time from nine hours to forty minutes and lifted satisfaction scores from\n71% to 93%. I did it by hiring for kindness, writing clear playbooks and\nmeasuring what matters.\n\nI would be glad to tell you more about how I could help your team. Thank you for\nyour time.",
        t.b(14.0, t.dark).lh(1.65).left(),
    );
    b.mark_last("from nine hours to forty minutes", t.primary, true);
    b.text("Sign-off", 70.0, 720.0, 400.0, "Warm regards,", t.b(14.0, t.dark).left());
    b.text("Signature", 70.0, 760.0, 400.0, "Hannah Kowalski", t.h(26.0, t.primary).italic().with(|s| s.bold = false).left());
}

fn minimal_resume(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.text("Name", 56.0, 56.0, 500.0, "Daniel Mensah", t.h(40.0, t.primary).left());
    b.text("Role", 56.0, 106.0, 500.0, "Data Analyst", t.b(18.0, t.secondary).bold().left());
    b.text("Contact", w - 276.0, 60.0, 220.0, "daniel@mensah.example\n+31 555 0100 44\nRotterdam", t.b(12.0, t.dark).lh(1.55).right());
    b.rect("Top rule", 56.0, 150.0, w - 112.0, 1.0, 0.0, solid(t.primary));
    let left = 56.0;
    let right = 290.0;
    let rw = w - right - 56.0;
    let heading = |b: &mut B, title: &str, x: f64, y: f64| {
        b.text(&format!("{title} heading"), x, y, 220.0, title, t.b(12.0, t.secondary).bold().upper().ls(300.0).left());
    };
    let body = t.b(13.0, t.dark).lh(1.55).left();
    heading(b, "Skills", left, 190.0);
    let skills = ["SQL", "Python", "Dashboards", "A/B testing", "Forecasting", "Storytelling"];
    for (i, s) in skills.iter().enumerate() {
        let y = 222.0 + 40.0 * i as f64;
        b.rect(&format!("{s} pill"), left, y, 160.0, 28.0, 14.0, solid(t.secondary.lerp(t.light, 0.85)));
        b.text(&format!("{s} label"), left, y + 6.0, 160.0, s, t.b(13.0, t.dark).bold().lh(1.2));
    }
    heading(b, "Education", left, 490.0);
    b.text("Education", left, 518.0, 200.0, "MSc Statistics\nErasmus University\n2016 – 2018", body.clone());
    heading(b, "Languages", left, 620.0);
    b.text("Languages", left, 648.0, 200.0, "English · Dutch · Twi", body.clone());
    heading(b, "Certificates", left, 700.0);
    b.text("Certificates", left, 728.0, 200.0, "Cloud data engineering\nAdvanced spreadsheets", body.clone());
    heading(b, "Profile", right, 190.0);
    b.text(
        "Profile",
        right,
        216.0,
        rw,
        "I turn messy data into decisions people trust. Calm\nexplainer, careful checker, fast learner.",
        body.clone(),
    );
    heading(b, "Experience", right, 300.0);
    let jobs = [
        (
            "Senior Data Analyst at Veldt Energy",
            "2022 – now",
            "Built the pricing dashboard used by 40 sales staff\nForecast models cut stock waste by 18%",
            "Veldt Energy",
        ),
        (
            "Data Analyst at Kade Retail",
            "2018 – 2022",
            "Ran 60+ A/B tests on the online shop\nAutomated the weekly report, saving a day a week",
            "Kade Retail",
        ),
        ("Analyst intern at City of Rotterdam", "2017", "Mapped cycling counts for the new bridge plan", "City of Rotterdam"),
    ];
    let mut y = 330.0;
    for (role, dates, details, company) in jobs {
        b.text(role, right, y, rw, role, t.b(15.0, t.dark).left());
        b.mark_last(company, t.dark, true);
        b.text(&format!("{role} dates"), right, y + 24.0, 300.0, dates, t.b(12.0, t.secondary).bold().left());
        b.text(&format!("{role} details"), right, y + 48.0, rw, details, body.clone());
        y += 48.0 + 20.0 * details.lines().count() as f64 + 36.0;
    }
    heading(b, "Selected project", right, 720.0);
    b.text(
        "Project",
        right,
        748.0,
        rw,
        "Open bike-share data explorer: 30,000 visitors in its\nfirst month, featured in the city newsletter.",
        body.clone(),
    );
    heading(b, "Awards", right, 840.0);
    b.text("Awards", right, 868.0, rw, "Analytics team of the year, Veldt Energy (2024)\nBest student thesis, Erasmus University (2018)", body);
    b.rect("Bottom rule", 56.0, h - 70.0, w - 112.0, 1.0, 0.0, solid(t.dark));
    b.text("Footer", 56.0, h - 56.0, w - 112.0, "References on request", t.b(11.0, t.dark).left());
}

fn newsletter(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.text("Masthead", 50.0, 40.0, 500.0, "The Elm Street Post", t.h(42.0, t.dark).left());
    b.text("Issue", w - 250.0, 52.0, 200.0, "Issue 18 · Autumn 2026", t.b(12.0, t.dark).right());
    b.rect("Masthead rule", 50.0, 104.0, w - 100.0, 4.0, 0.0, solid(t.primary));
    b.rect("Masthead thin rule", 50.0, 112.0, w - 100.0, 1.0, 0.0, solid(t.primary));
    b.text("Lead kicker", 50.0, 140.0, 450.0, "Our garden", t.b(12.0, t.primary).bold().upper().ls(300.0).left());
    b.text("Lead headline", 50.0, 162.0, 460.0, "The community garden\nharvest is in", t.h(30.0, t.dark).lh(1.15).left());
    b.rect("Picture", 50.0, 248.0, 460.0, 200.0, 8.0, lin(135.0, t.secondary.lerp(t.light, 0.6), t.primary.lerp(t.light, 0.6)));
    b.icon("Picture icon", "flower", 240.0, 308.0, 80.0, 80.0, solid(t.light));
    b.text(
        "Lead story",
        50.0,
        466.0,
        460.0,
        "Forty volunteers picked 300 kg of apples, beans and\nsquash this year. Half went to the food bank on Mill\nRoad; the rest became soup at the harvest supper,\nwhere we raised enough to build two new raised beds\nfor the school next door.",
        t.b(13.0, t.dark).lh(1.6).left(),
    );
    b.text("Chart heading", 50.0, 610.0, 460.0, "Where the harvest went (kg)", t.h(16.0, t.dark).left());
    let mut c = t.chart(
        ChartKind::HorizontalBar,
        &["Food bank", "Harvest supper", "Volunteers", "Compost"],
        &[("Kilograms", &[150.0, 90.0, 40.0, 20.0])],
        12.0,
        t.dark,
    );
    c.colors = vec![t.primary, t.secondary, t.primary.lerp(t.dark, 0.3), t.secondary.lerp(t.dark, 0.3)];
    c.gridlines = false;
    b.chart("Harvest chart", 50.0, 640.0, 460.0, 280.0, c);
    // Sidebar.
    b.rect("Sidebar", 540.0, 140.0, w - 590.0, 780.0, 10.0, solid(t.secondary.lerp(t.light, 0.9)));
    b.text("Diary heading", 556.0, 160.0, 180.0, "Dates for\nyour diary", t.h(20.0, t.secondary.lerp(t.dark, 0.3)).lh(1.15).left());
    let dates = [("2 Nov", "Bulb planting, 10 am"), ("16 Nov", "Repair café, hall"), ("30 Nov", "Lantern walk"), ("7 Dec", "Carols & mince pies")];
    for (i, (when, what)) in dates.iter().enumerate() {
        let y = 230.0 + 66.0 * i as f64;
        b.text(&format!("Date {} day", i + 1), 556.0, y, 180.0, when, t.b(14.0, t.primary.lerp(t.dark, 0.2)).bold().left());
        b.text(&format!("Date {}", i + 1), 556.0, y + 22.0, 180.0, what, t.b(12.0, t.dark).left());
    }
    b.text("Notice heading", 556.0, 520.0, 180.0, "Lost & found", t.h(16.0, t.secondary.lerp(t.dark, 0.3)).left());
    b.text("Notice", 556.0, 548.0, 180.0, "A blue child's bike\nwas left at the hall.\nCall Pat on 555 0100.", t.b(12.0, t.dark).lh(1.5).left());
    b.text("Help heading", 556.0, 640.0, 180.0, "Can you help?", t.h(16.0, t.secondary.lerp(t.dark, 0.3)).left());
    b.text("Help", 556.0, 668.0, 180.0, "We need two drivers\nfor the food bank run\non Saturdays.", t.b(12.0, t.dark).lh(1.5).left());
    b.text("Meeting heading", 556.0, 760.0, 180.0, "Next meeting", t.h(16.0, t.secondary.lerp(t.dark, 0.3)).left());
    b.text("Meeting", 556.0, 788.0, 180.0, "Tuesday 18 November,\n7 pm at the church hall.\nEveryone welcome.", t.b(12.0, t.dark).lh(1.5).left());
    b.rect("Footer rule", 50.0, h - 80.0, w - 100.0, 1.0, 0.0, solid(t.primary));
    b.text("Footer", 50.0, h - 64.0, w - 100.0, "Elm Street Residents' Association · news@elmstreet.example", t.b(11.0, t.dark).left());
}

fn timetable(b: &mut B) {
    let t = b.t.clone();
    let w = b.w;
    b.bg(solid(t.light));
    b.rect("Header", 0.0, 0.0, w, 110.0, 0.0, solid(t.primary));
    b.text("Title", 50.0, 30.0, 600.0, "Class 4B timetable", t.h(40.0, t.light).left());
    b.text("Term", w - 400.0, 42.0, 350.0, "Autumn term · Room 12", t.b(16.0, t.light).bold().right());
    let rows: &[&[&str]] = &[
        &["", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday"],
        &["9:00", "Maths", "English", "Maths", "Science", "Maths"],
        &["10:00", "English", "Maths", "Art", "English", "Spelling"],
        &["11:00", "Break", "", "", "", ""],
        &["11:20", "Science", "History", "English", "Maths", "PE"],
        &["12:20", "Lunch", "", "", "", ""],
        &["13:20", "Music", "PE", "Science", "Computing", "Class trip"],
        &["14:20", "Reading", "Geography", "Reading", "Art", ""],
    ];
    let mut tb = t.table(rows, 17.0);
    tb.align = TextAlign::Center;
    tb.alt_row_fill = None;
    tb.column_widths = vec![0.1, 0.18, 0.18, 0.18, 0.18, 0.18];
    tb.row_heights = vec![48.0, 72.0, 72.0, 40.0, 72.0, 40.0, 72.0, 72.0];
    tb.header_fill = t.dark;
    tb.border = Some(StrokeStyle { color: t.dark.lerp(t.light, 0.75), width: 1.5, dash: vec![] });
    // Break and lunch run across the week; the class trip takes Friday afternoon.
    tb.merges = vec![CellRange::new(3, 1, 1, 5), CellRange::new(5, 1, 1, 5), CellRange::new(6, 5, 2, 1)];
    tb.style_cells(CellRange::new(1, 0, 7, 1), |s| {
        s.bold = Some(true);
        s.text_color = Some(t.primary);
    });
    let tint = |c: genha_color::Color| c.lerp(t.light, 0.8);
    for (subject, fill) in [("Maths", tint(t.primary)), ("English", tint(t.secondary)), ("Science", tint(t.dark)), ("PE", tint(t.secondary))] {
        for (r, row) in rows.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                if *cell == subject {
                    tb.style_cells(CellRange::cell(r, c), |s| s.fill = Some(fill));
                }
            }
        }
    }
    for r in [3, 5] {
        tb.style_cells(CellRange::new(r, 1, 1, 5), |s| {
            s.fill = Some(t.dark.lerp(t.light, 0.9));
            s.text_color = Some(t.dark);
            s.bold = Some(true);
        });
    }
    tb.style_cells(CellRange::cell(6, 5), |s| {
        s.fill = Some(t.secondary);
        s.bold = Some(true);
    });
    b.table("Timetable", 50.0, 150.0, w - 100.0, 488.0, tb);
    b.icon("Backpack note icon", "bell", 50.0, 680.0, 36.0, 36.0, solid(t.secondary));
    b.text("Notes", 100.0, 684.0, w - 150.0, "PE kit on Tuesdays and Fridays · Library books back every Thursday", t.b(16.0, t.dark).left());
    b.text("Teacher", 100.0, 716.0, w - 150.0, "Teacher: Mr Adeyemi · Teaching assistant: Ms Lund", t.b(14.0, t.dark).left());
}

fn worksheet(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.text("Name label", 50.0, 40.0, 60.0, "Name", t.b(13.0, t.dark).left());
    b.rect("Name line", 100.0, 56.0, 330.0, 1.5, 0.0, solid(t.dark));
    b.text("Date label", 470.0, 40.0, 60.0, "Date", t.b(13.0, t.dark).left());
    b.rect("Date line", 512.0, 56.0, 230.0, 1.5, 0.0, solid(t.dark));
    b.rect("Title band", 50.0, 90.0, w - 100.0, 110.0, 18.0, solid(t.primary));
    b.text("Title", 80.0, 108.0, 500.0, "Fun with fractions", t.h(40.0, t.light).left());
    b.text("Subtitle", 80.0, 160.0, 500.0, "Year 3 · Worksheet 4", t.b(14.0, t.light).bold().left());
    b.icon("Pizza", "pizza", w - 170.0, 100.0, 90.0, 90.0, solid(t.secondary));
    let section = |b: &mut B, n: usize, y: f64, text: &str| {
        b.ellipse(&format!("Task {n} disc"), 50.0, y, 34.0, 34.0, solid(t.secondary));
        b.text(&format!("Task {n} number"), 50.0, y + 7.0, 34.0, &n.to_string(), t.h(17.0, t.dark).lh(1.15));
        b.text(&format!("Task {n}"), 100.0, y + 5.0, w - 150.0, text, t.h(17.0, t.dark).left());
    };
    section(b, 1, 230.0, "Colour in the fraction shown under each circle.");
    for (i, (parts, label)) in [(2, "1/2"), (4, "3/4"), (3, "1/3"), (4, "1/4")].into_iter().enumerate() {
        let x = 70.0 + 170.0 * i as f64;
        b.outline(&format!("Circle {}", i + 1), ShapeKind::Ellipse, x, 280.0, 120.0, 120.0, t.dark, 2.0);
        for p in 0..parts {
            let a = f64::from(p) * 360.0 / f64::from(parts);
            // A radius from the centre, turned to angle `a`.
            b.shape(
                &format!("Circle {} cut {}", i + 1, p + 1),
                ShapeKind::Line,
                x + 60.0 - 30.0,
                340.0 - 30.0 - 1.0,
                60.0,
                2.0,
                genha_doc::Fill::None,
            );
            b.dash_last(t.dark, 2.0, &[]);
            if let Some(e) = b.last() {
                e.frame.rotation = a - 90.0;
                let (s, c) = (a - 90.0).to_radians().sin_cos();
                e.frame.x = x + 60.0 + 30.0 * c - 30.0;
                e.frame.y = 340.0 + 30.0 * s - 1.0;
            }
        }
        b.text(&format!("Circle {} label", i + 1), x, 412.0, 120.0, label, t.h(22.0, t.primary).lh(1.15));
    }
    section(b, 2, 470.0, "Fill in the missing numbers.");
    let sums = ["1/4 + 1/4 =", "1/3 + __ = 2/3", "__ + 1/2 = 1", "3/4 − 1/4 ="];
    for (i, s) in sums.iter().enumerate() {
        let (col, row) = (i % 2, i / 2);
        let x = 100.0 + 330.0 * col as f64;
        let y = 520.0 + 70.0 * row as f64;
        b.text(&format!("Sum {}", i + 1), x, y, 190.0, s, t.b(20.0, t.dark).left());
        b.outline(&format!("Answer box {}", i + 1), ShapeKind::Rect { corner_radius: 8.0 }, x + 200.0, y - 8.0, 90.0, 44.0, t.secondary, 2.0);
        b.dash_last(t.secondary, 2.0, &[6.0, 4.0]);
    }
    section(b, 3, 680.0, "Word problem");
    b.text(
        "Problem",
        100.0,
        720.0,
        w - 150.0,
        "Ava has 12 strawberries. She gives a quarter to her brother and\na third to her friend. How many strawberries does she have left?",
        t.b(16.0, t.dark).lh(1.5).left(),
    );
    for i in 0..4 {
        b.rect(&format!("Working line {}", i + 1), 100.0, 820.0 + 50.0 * f64::from(i), w - 150.0, 1.0, 0.0, solid(t.dark.lerp(t.light, 0.6)));
    }
    b.rect("Star box", 50.0, h - 130.0, w - 100.0, 70.0, 14.0, solid(t.secondary.lerp(t.light, 0.8)));
    b.icon("Star", "star", 70.0, h - 115.0, 40.0, 40.0, solid(t.secondary.lerp(t.dark, 0.2)));
    b.text("Star note", 124.0, h - 108.0, w - 200.0, "Finished early? Draw a pizza and cut it into eighths!", t.b(16.0, t.dark).bold().left());
}
