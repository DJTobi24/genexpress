//! More cards and invitations: save the dates, party and graduation cards, business and loyalty
//! cards, a new year card and an award certificate.

use genha_doc::ShapeKind;

use super::{CARDS, Def, def, lin, radial, solid};
use crate::build::{B, Sty, Theme};

pub(super) const DEFS: &[Def] = &[
    def(
        "save-the-date",
        "Botanical save the date",
        CARDS,
        "postcard",
        &["save the date", "wedding", "card", "botanical", "elegant", "postcard"],
        || Theme::new(0x7d8f69, 0xd9a28b, 0x2f3a2a, 0xfbf8f2, "Serif", "Sans"),
        save_the_date,
    )
    .landscape(),
    def(
        "kids-birthday-invitation",
        "Kids dinosaur birthday invitation",
        CARDS,
        "invitation-5x7",
        &["birthday", "kids", "party", "invitation", "dinosaur", "fun"],
        || Theme::new(0x22c55e, 0xf97316, 0x14301f, 0xfffbeb, "Display", "Sans"),
        kids_birthday,
    ),
    def(
        "graduation-announcement",
        "Graduation announcement",
        CARDS,
        "invitation-5x7",
        &["graduation", "class of", "announcement", "party", "school", "university"],
        || Theme::new(0x1e3a8a, 0xd4a72c, 0x0b1533, 0xfdfbf5, "Serif", "Sans"),
        graduation,
    ),
    def(
        "business-card-gradient",
        "Gradient business card",
        CARDS,
        "business-card",
        &["business card", "gradient", "modern", "contact", "tech"],
        || Theme::new(0x7c3aed, 0x06b6d4, 0x1e1b4b, 0xffffff, "Sans", "Sans"),
        card_gradient,
    ),
    def(
        "business-card-photographer",
        "Photographer card with QR code",
        CARDS,
        "business-card-us",
        &["business card", "photographer", "qr code", "double-sided", "portfolio", "dark"],
        || Theme::new(0xf59e0b, 0x94a3b8, 0x0f172a, 0xf8fafc, "Sans", "Sans"),
        card_photographer,
    ),
    def(
        "coffee-loyalty-card",
        "Coffee loyalty stamp card",
        CARDS,
        "business-card",
        &["loyalty card", "stamp card", "coffee", "cafe", "double-sided", "small business"],
        || Theme::new(0x6f4e37, 0xe9c46a, 0x231a14, 0xf7efe5, "Serif", "Sans"),
        loyalty_card,
    ),
    def(
        "new-year-card",
        "Fireworks new year card",
        CARDS,
        "postcard",
        &["new year", "greeting card", "holiday", "fireworks", "celebration", "postcard"],
        || Theme::new(0xf472b6, 0xfacc15, 0x111827, 0xfdf2f8, "Display", "Sans"),
        new_year_card,
    )
    .landscape(),
    def(
        "employee-award-certificate",
        "Employee of the month certificate",
        CARDS,
        "document-a4",
        &["certificate", "award", "employee of the month", "recognition", "office"],
        || Theme::new(0x7c2d12, 0xd97706, 0x1c1917, 0xfffbeb, "Serif", "Sans"),
        award_certificate,
    )
    .landscape(),
];

fn save_the_date(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.outline("Border", ShapeKind::Rect { corner_radius: 0.0 }, 16.0, 16.0, w - 32.0, h - 32.0, t.primary, 1.5);
    for (name, x, y, s, rot, primary) in [
        ("Leaf top left", 26.0, 24.0, 70.0, 20.0, true),
        ("Leaf top left small", 84.0, 26.0, 44.0, 80.0, false),
        ("Leaf bottom right", w - 96.0, h - 94.0, 70.0, 200.0, true),
        ("Leaf bottom right small", w - 128.0, h - 70.0, 44.0, 260.0, false),
    ] {
        b.icon(name, "leaf", x, y, s, s, solid(if primary { t.primary } else { t.secondary }));
        b.rotate_last(rot);
    }
    b.text("Arc", (w - 300.0) / 2.0, 30.0, 300.0, "Save the date", t.b(20.0, t.primary.lerp(t.dark, 0.3)).bold().upper().ls(300.0));
    b.curve_last(60.0, 58.0);
    b.ctext("Names", 100.0, 440.0, "Lena & Marco", t.h(50.0, t.dark).italic().with(|s| s.bold = false));
    b.ctext("Date", 180.0, 440.0, "18 · 09 · 2027", t.h(34.0, t.primary.lerp(t.dark, 0.25)).ls(100.0));
    b.rect("Divider", (w - 40.0) / 2.0, 236.0, 40.0, 2.0, 1.0, solid(t.secondary));
    b.ctext("Place", 254.0, 440.0, "Lake Garda, Italy", t.b(16.0, t.dark).upper().ls(250.0));
    b.ctext("Note", 300.0, 440.0, "Invitation to follow", t.b(14.0, t.dark).italic());
}

fn kids_birthday(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.rect("Ground", 0.0, h - 112.0, w, 112.0, 0.0, solid(t.primary));
    b.icon("Tree left", "tree", 20.0, h - 220.0, 70.0, 110.0, solid(t.primary.lerp(t.dark, 0.3)));
    b.icon("Tree right", "tree", 395.0, h - 200.0, 60.0, 90.0, solid(t.primary.lerp(t.dark, 0.3)));
    for (i, (x, y, fill)) in
        [(40.0, 50.0, t.secondary), (380.0, 70.0, t.primary), (330.0, 30.0, t.secondary.lerp(t.light, 0.4))].into_iter().enumerate()
    {
        b.icon(&format!("Balloon {}", i + 1), "balloon", x, y, 64.0, 92.0, solid(fill));
    }
    b.ellipse("Age disc", 165.0, 70.0, 150.0, 150.0, solid(t.secondary));
    b.text("Age", 165.0, 95.0, 150.0, "5", t.h(100.0, t.dark).lh(1.0));
    b.ctext("Kicker", 250.0, 400.0, "Stomp on over!", t.b(18.0, t.primary.lerp(t.dark, 0.4)).bold().upper().ls(250.0));
    b.ctext("Headline", 282.0, 420.0, "Leo's dino\nparty", t.h(56.0, t.dark).lh(1.05));
    b.mark_last("dino", t.primary.lerp(t.dark, 0.25), true);
    b.ctext("Details", 420.0, 420.0, "Saturday 9 May · 2 – 5 pm\nAdventure Park, Fern Road", t.b(18.0, t.dark).lh(1.5));
    b.ctext("Extras", 490.0, 420.0, "Fossil hunt, cake and a bouncy volcano", t.b(15.0, t.dark).italic());
    b.ctext("RSVP", h - 80.0, 420.0, "RSVP to Mia · 555 0100 455", t.b(16.0, t.dark).bold());
}

fn graduation(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(lin(90.0, t.primary, t.dark));
    b.outline("Border", ShapeKind::Rect { corner_radius: 0.0 }, 20.0, 20.0, w - 40.0, h - 40.0, t.secondary, 2.0);
    b.text("Arc", 80.0, 50.0, 320.0, "Class of 2026", t.b(26.0, t.secondary).bold().upper().ls(250.0));
    b.curve_last(80.0, 82.0);
    b.icon("Cap", "graduation-cap", 170.0, 120.0, 140.0, 140.0, solid(t.light));
    b.ctext("Kicker", 290.0, 400.0, "Please join us in celebrating", t.b(14.0, t.light).upper().ls(200.0));
    b.ctext("Name", 320.0, 420.0, "Amara Okafor", t.h(48.0, t.light).italic().with(|s| s.bold = false));
    b.ctext("Degree", 390.0, 420.0, "BSc Marine Biology\nHarbourside University", t.b(17.0, t.secondary).lh(1.5));
    b.rect("Divider", (w - 60.0) / 2.0, 462.0, 60.0, 2.0, 1.0, solid(t.secondary));
    b.ctext("Party", 484.0, 420.0, "Garden party · Saturday 27 June · 3 pm\n41 Kestrel Close, Ashford", t.b(15.0, t.light).lh(1.6));
    b.ctext("RSVP", 580.0, 420.0, "RSVP by 15 June", t.b(14.0, t.secondary).bold().upper().ls(200.0));
}

fn card_gradient(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.name_page("Front");
    b.bg(lin(135.0, t.primary, t.secondary.lerp(t.primary, 0.2)));
    b.icon("Blob", "blob-1", 190.0, 0.0, 130.0, 130.0, solid(t.light.with_alpha(40)));
    b.text("Name", 24.0, 70.0, 260.0, "Noor Haddad", t.h(26.0, t.light).left());
    b.text("Role", 24.0, 104.0, 260.0, "Product engineer", t.b(13.0, t.light).bold().upper().ls(200.0).left());
    b.text("Company", 24.0, h - 40.0, 260.0, "Fieldnote Labs", t.b(12.0, t.light).left());

    b.new_page("Back");
    b.bg(solid(t.light));
    b.rect("Edge", 0.0, 0.0, 10.0, h, 0.0, lin(90.0, t.primary, t.secondary));
    let rows = [("envelope", "noor@fieldnote.example"), ("phone", "+44 555 0100 620"), ("globe", "fieldnote.example"), ("location-pin", "Leeds, UK")];
    for (i, (icon, value)) in rows.iter().enumerate() {
        let y = 36.0 + 36.0 * i as f64;
        b.ellipse(&format!("{icon} disc"), 30.0, y, 24.0, 24.0, solid(t.primary));
        b.icon(&format!("{icon} icon"), icon, 35.0, y + 5.0, 14.0, 14.0, solid(t.light));
        b.text(&format!("{icon} text"), 66.0, y + 4.0, w - 90.0, value, t.b(13.0, t.dark).left());
    }
}

fn card_photographer(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.name_page("Front");
    b.bg(solid(t.dark));
    b.outline("Lens ring", ShapeKind::Ellipse, 206.0, 38.0, 116.0, 116.0, t.primary, 3.0);
    b.ellipse("Lens", 228.0, 60.0, 72.0, 72.0, radial(t.secondary.lerp(t.dark, 0.4), t.dark));
    b.ellipse("Lens glint", 246.0, 74.0, 16.0, 16.0, solid(t.light.with_alpha(200)));
    b.text("Name", 24.0, 52.0, 200.0, "Sam Ferreira", t.h(24.0, t.light).left());
    b.text("Role", 24.0, 86.0, 200.0, "Photographer", t.b(12.0, t.primary).bold().upper().ls(250.0).left());
    b.text("Specialty", 24.0, h - 44.0, 200.0, "Weddings · Portraits · Food", t.b(10.0, t.secondary).left());

    b.new_page("Back");
    b.bg(solid(t.light));
    b.rect("Side", w - 130.0, 0.0, 130.0, h, 0.0, solid(t.primary));
    b.rect("QR card", w - 112.0, 40.0, 94.0, 94.0, 8.0, solid(t.light));
    b.qr("Portfolio QR code", "https://samferreira.example", w - 108.0, 44.0, 86.0, t.dark);
    b.text("Scan", w - 130.0, 144.0, 130.0, "See my work", t.b(11.0, t.dark).bold());
    b.text("Contact heading", 24.0, 36.0, 170.0, "Let's talk", t.h(20.0, t.dark).left());
    b.text("Contact", 24.0, 76.0, 176.0, "sam@ferreira.example\n555 0100 909\nPorto · travels anywhere", t.b(11.0, t.dark).lh(1.6).left());
}

fn loyalty_card(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.name_page("Front");
    b.bg(solid(t.primary));
    b.icon("Cup", "coffee", (w - 80.0) / 2.0, 30.0, 80.0, 80.0, solid(t.secondary));
    b.ctext("Name", 118.0, 280.0, "Little Bean", t.h(32.0, t.light));
    b.ctext("Tagline", 162.0, 280.0, "Coffee & cake · Mill Lane", t.b(11.0, t.light).upper().ls(200.0));

    b.new_page("Stamps");
    b.bg(solid(t.light));
    b.ctext("Heading", 20.0, 280.0, "Ten coffees, the last is on us", t.b(13.0, t.dark).bold());
    for i in 0..10 {
        let (col, row) = (i % 5, i / 5);
        let x = 30.0 + 56.0 * f64::from(col);
        let y = 56.0 + 60.0 * f64::from(row);
        if i == 9 {
            b.ellipse("Free stamp", x, y, 42.0, 42.0, solid(t.secondary));
            b.icon("Free stamp icon", "heart", x + 11.0, y + 12.0, 20.0, 20.0, solid(t.primary));
        } else {
            b.outline(&format!("Stamp {}", i + 1), ShapeKind::Ellipse, x, y, 42.0, 42.0, t.primary, 1.5);
            b.dash_last(t.primary, 1.5, &[4.0, 3.0]);
        }
    }
    b.ctext("Note", h - 34.0, 280.0, "One stamp per hot drink · no expiry", t.b(10.0, t.dark).italic());
}

fn new_year_card(b: &mut B) {
    let t = b.t.clone();
    let h = b.h;
    b.bg(radial(t.dark.lerp(t.primary, 0.15), t.dark));
    // Fireworks: rays of thin rounded bars around a centre.
    for (fi, (cx, cy, r, c)) in
        [(110.0, 110.0, 70.0, t.primary), (450.0, 90.0, 56.0, t.secondary), (290.0, 64.0, 32.0, t.primary)].into_iter().enumerate()
    {
        for k in 0..12 {
            let a = f64::from(k) * 30.0;
            let (x, y) = (cx + r * a.to_radians().sin(), cy - r * a.to_radians().cos());
            b.rect(&format!("Firework {} ray {}", fi + 1, k + 1), x - 2.0, y - 10.0, 4.0, 20.0, 2.0, solid(c));
            b.rotate_last(a);
        }
        b.ellipse(&format!("Firework {} core", fi + 1), cx - 6.0, cy - 6.0, 12.0, 12.0, solid(t.light));
    }
    b.ctext("Kicker", 150.0, 300.0, "Hello", t.b(18.0, t.secondary).bold().upper().ls(400.0));
    b.ctext("Year", 178.0, 300.0, "2027", t.h(90.0, t.light).lh(1.0));
    b.mark_last("27", t.primary, true);
    b.ctext("Message", 290.0, 400.0, "Wishing you a year of good surprises.", t.b(15.0, t.light));
    b.ctext("Signature", h - 54.0, 300.0, "Love, the Kaplans", t.b(14.0, t.secondary).bold());
}

fn award_certificate(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.rect("Top band", 0.0, 0.0, w, 24.0, 0.0, solid(t.primary));
    b.rect("Bottom band", 0.0, h - 24.0, w, 24.0, 0.0, solid(t.primary));
    b.outline("Frame", ShapeKind::Rect { corner_radius: 0.0 }, 50.0, 60.0, w - 100.0, h - 120.0, t.secondary, 2.0);
    // A seal with its title bent round the top.
    b.icon("Seal", "seal", 80.0, 100.0, 220.0, 220.0, solid(t.secondary));
    b.ellipse("Seal centre", 110.0, 130.0, 160.0, 160.0, solid(t.primary));
    b.text("Seal arc", 120.0, 140.0, 140.0, "Team award", t.b(14.0, t.light).bold().upper().ls(200.0));
    b.curve_last(130.0, 50.0);
    b.icon("Seal star", "star", 160.0, 190.0, 60.0, 60.0, solid(t.secondary));
    b.text("Seal year", 120.0, 254.0, 140.0, "2026", t.b(14.0, t.light).bold().ls(200.0));
    b.text("Kicker", 360.0, 120.0, 680.0, "Certificate of recognition", t.b(20.0, t.secondary.lerp(t.dark, 0.35)).bold().upper().ls(300.0).left());
    b.text("Title", 360.0, 164.0, 720.0, "Employee of the month", t.h(48.0, t.primary).left());
    b.text("Presented", 360.0, 260.0, 680.0, "Proudly presented to", t.b(20.0, t.dark).left());
    b.text("Recipient", 360.0, 300.0, 700.0, "Priya Raman", t.h(72.0, t.dark).italic().with(|s| s.bold = false).left());
    b.rect("Name line", 360.0, 395.0, 600.0, 2.0, 1.0, solid(t.secondary));
    b.text(
        "Reason",
        360.0,
        420.0,
        680.0,
        "For turning a hard quarter into our best one: calm under\npressure, generous with help and always one step ahead.",
        t.b(19.0, t.dark).lh(1.5).left(),
    );
    b.text("Month", 360.0, 510.0, 680.0, "October 2026 · Greenleaf Logistics", t.b(18.0, t.primary.lerp(t.dark, 0.2)).bold().left());
    b.rect("Manager line", 360.0, 640.0, 260.0, 2.0, 1.0, solid(t.dark));
    b.text("Manager label", 360.0, 650.0, 260.0, "Team lead", t.b(16.0, t.dark).left());
    b.rect("Director line", 700.0, 640.0, 260.0, 2.0, 1.0, solid(t.dark));
    b.text("Director label", 700.0, 650.0, 260.0, "Director", t.b(16.0, t.dark).left());
}
