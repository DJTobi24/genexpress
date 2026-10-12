//! Cards and invitations: business cards, invitations, greeting cards, certificates, vouchers,
//! coupons and tickets.

use genha_doc::ShapeKind;
use genha_geom::Frame;

use super::{CARDS, Def, def, lin, solid};
use crate::build::{B, Sty, Theme, shape_el};

pub(super) const DEFS: &[Def] = &[
    def(
        "business-card",
        "Clean business card",
        CARDS,
        "business-card",
        &["business card", "contact", "print", "minimal"],
        || Theme::new(0x14213d, 0xfca311, 0x0b132b, 0xffffff, "Sans", "Sans"),
        business_card,
    ),
    def(
        "business-card-split",
        "Two-tone business card",
        CARDS,
        "business-card",
        &["business card", "contact", "print", "icons", "two-tone"],
        || Theme::new(0x2a9d8f, 0xe9c46a, 0x264653, 0xffffff, "Sans", "Sans"),
        card_split,
    ),
    def(
        "business-card-bold",
        "Bold double-sided business card",
        CARDS,
        "business-card-us",
        &["business card", "contact", "print", "dark", "double-sided", "monogram"],
        || Theme::new(0xfb5607, 0xffbe0b, 0x1a1a1a, 0xfafafa, "Display", "Sans"),
        card_bold,
    ),
    def(
        "business-card-monogram",
        "Monogram business card",
        CARDS,
        "business-card-us",
        &["business card", "contact", "print", "elegant", "monogram", "minimal"],
        || Theme::new(0x5f0f40, 0xfb8b24, 0x2b0a1e, 0xfdf8f3, "Serif", "Sans"),
        card_monogram,
    ),
    def(
        "birthday-invitation",
        "Confetti birthday invitation",
        CARDS,
        "invitation-5x7",
        &["invitation", "birthday", "party", "card"],
        || Theme::new(0xff8fab, 0xffc857, 0x3d2c3e, 0xfff5f7, "Serif", "Sans"),
        birthday_invitation,
    ),
    def(
        "wedding-invitation",
        "Garden wedding invitation",
        CARDS,
        "invitation-5x7",
        &["invitation", "wedding", "elegant", "leaves", "card", "rsvp"],
        || Theme::new(0x6b705c, 0xcb997e, 0x3f4238, 0xfffcf7, "Serif", "Sans"),
        wedding_invitation,
    ),
    def(
        "baby-shower-invitation",
        "Cloudy baby shower invitation",
        CARDS,
        "invitation-5x7",
        &["invitation", "baby shower", "baby", "clouds", "card", "pastel"],
        || Theme::new(0x48bfe3, 0xffafcc, 0x22333b, 0xf8fdff, "Serif", "Sans"),
        baby_shower,
    ),
    def(
        "party-invitation",
        "Rooftop party invitation",
        CARDS,
        "postcard",
        &["invitation", "party", "night", "confetti", "postcard", "landscape"],
        || Theme::new(0xff006e, 0xfcbf49, 0x0d0221, 0xfdf0ff, "Display", "Sans"),
        party_invitation,
    ),
    def(
        "thank-you-card",
        "Thank you card",
        CARDS,
        "postcard",
        &["greeting card", "thank you", "card", "postcard", "heart"],
        || Theme::new(0xf28482, 0x84a59d, 0x3d3b3c, 0xf7ede2, "Serif", "Sans"),
        thank_you,
    ),
    def(
        "holiday-card",
        "Snowy holiday card",
        CARDS,
        "invitation-5x7",
        &["greeting card", "holiday", "christmas", "winter", "season's greetings", "card"],
        || Theme::new(0xbc4749, 0x6a994e, 0x1b2d2a, 0xfaf6f0, "Serif", "Sans"),
        holiday_card,
    ),
    def(
        "achievement-certificate",
        "Achievement certificate",
        CARDS,
        "document-a4",
        &["certificate", "award", "school", "landscape"],
        || Theme::new(0x1f4e79, 0xc9a227, 0x1b1b1b, 0xfffdf7, "Serif", "Sans"),
        certificate,
    )
    .landscape(),
    def(
        "course-certificate",
        "Course completion certificate",
        CARDS,
        "document-a4",
        &["certificate", "course", "completion", "training", "modern", "landscape"],
        || Theme::new(0x0b6e4f, 0xf0a202, 0x1b1b1e, 0xffffff, "Sans", "Sans"),
        course_certificate,
    )
    .landscape(),
    def(
        "gift-voucher",
        "Spa gift voucher",
        CARDS,
        "postcard",
        &["gift voucher", "gift card", "voucher", "spa", "present", "landscape"],
        || Theme::new(0x6a4c93, 0xf9c74f, 0x231942, 0xfdfaff, "Serif", "Sans"),
        gift_voucher,
    ),
    def(
        "discount-coupon",
        "Twenty percent off coupon",
        CARDS,
        "postcard",
        &["coupon", "discount", "voucher", "sale", "shop", "code"],
        || Theme::new(0xf15bb5, 0xfee440, 0x2d0320, 0xfffbfe, "Display", "Sans"),
        coupon,
    ),
    def(
        "event-ticket",
        "Night market ticket",
        CARDS,
        "postcard",
        &["ticket", "event", "admission", "pass", "barcode", "landscape"],
        || Theme::new(0x00a6a6, 0xf49f0a, 0x0c1821, 0xf4f4f9, "Display", "Mono"),
        event_ticket,
    ),
];

// ---------------------------------------------------------------- business cards

fn business_card(b: &mut B) {
    let t = b.t.clone();
    let h = b.h;
    b.bg(solid(t.light));
    b.rect("Side band", 0.0, 0.0, 96.0, h, 0.0, solid(t.primary));
    let disc = b.el("Logo disc", Frame::new(24.0, 24.0, 48.0, 48.0), shape_el(ShapeKind::Ellipse, solid(t.secondary)));
    let letter = b.text_el("Logo letter", 24.0, 31.0, 48.0, "A", t.h(28.0, t.primary).lh(1.0));
    b.group("Logo", vec![disc, letter]);
    b.text("Name", 116.0, 40.0, 190.0, "Alex Morgan", t.h(20.0, t.dark).left());
    b.text("Role", 116.0, 68.0, 190.0, "Creative Director", t.b(12.0, t.dark.lerp(t.light, 0.35)).left());
    b.rect("Accent line", 116.0, 96.0, 36.0, 3.0, 1.5, solid(t.secondary));
    b.text("Contact", 116.0, 112.0, 190.0, "+1 555 0100 200\nalex@studio.example\nstudio.example", t.b(11.0, t.dark).lh(1.5).left());
}

fn card_split(b: &mut B) {
    let t = b.t.clone();
    let h = b.h;
    b.bg(solid(t.light));
    b.shape(
        "Colour panel",
        ShapeKind::Path { d: "M0 0 H140 L104 208 H0 Z".into(), view_width: 140.0, view_height: 208.0 },
        0.0,
        0.0,
        140.0,
        h,
        solid(t.primary),
    );
    b.ellipse("Logo disc", 30.0, 30.0, 50.0, 50.0, solid(t.secondary));
    b.icon("Logo leaf", "leaf", 42.0, 42.0, 26.0, 26.0, solid(t.primary));
    b.text("Company", 18.0, 96.0, 100.0, "Northleaf", t.b(12.0, t.light).bold().upper().ls(150.0).left());
    b.text("Name", 150.0, 26.0, 165.0, "Sam Okafor", t.h(20.0, t.dark).left());
    b.text("Role", 150.0, 54.0, 165.0, "Landscape architect", t.b(10.0, t.dark.lerp(t.light, 0.35)).left());
    for (i, (icon, value)) in
        [("phone", "555 0100 500"), ("envelope", "sam@northleaf.example"), ("location-pin", "8 Garden Row, Fernhill")].iter().enumerate()
    {
        let y = 92.0 + 24.0 * i as f64;
        b.icon(&format!("{icon} icon"), icon, 150.0, y, 12.0, 12.0, solid(t.primary));
        b.text(&format!("{icon} text"), 168.0, y - 1.0, 148.0, value, t.b(10.0, t.dark).left());
    }
    b.rect("Accent", 150.0, 176.0, 40.0, 3.0, 1.5, solid(t.secondary));
}

fn card_bold(b: &mut B) {
    let t = b.t.clone();
    b.name_page("Front");
    b.bg(solid(t.dark));
    b.text("Name", 24.0, 52.0, 196.0, "Jordan Reyes", t.h(24.0, t.light).left());
    b.text("Role", 24.0, 88.0, 196.0, "Brand strategist", t.b(12.0, t.secondary).bold().upper().ls(150.0).left());
    b.rect("Accent", 24.0, 116.0, 30.0, 4.0, 2.0, solid(t.primary));
    b.text("Contact", 24.0, 132.0, 196.0, "555 0100 400\njordan@reyes.example", t.b(10.0, t.light.lerp(t.dark, 0.2)).lh(1.5).left());
    b.ellipse("Monogram disc", 214.0, 40.0, 112.0, 112.0, solid(t.primary));
    b.text("Monogram", 214.0, 72.0, 112.0, "JR", t.h(44.0, t.dark).lh(1.0));

    b.new_page("Back");
    b.bg(solid(t.primary));
    b.icon("Sparkle", "sparkle", 148.0, 38.0, 40.0, 40.0, solid(t.secondary));
    b.ctext("Studio", 92.0, 300.0, "Reyes Studio", t.h(28.0, t.dark));
    b.ctext("Website", 136.0, 300.0, "reyes.example", t.b(12.0, t.dark).bold().ls(100.0));
}

fn card_monogram(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.outline("Monogram ring", ShapeKind::Ellipse, 138.0, 18.0, 60.0, 60.0, t.primary, 2.0);
    b.text("Monogram", 138.0, 32.0, 60.0, "M", t.h(30.0, t.primary).lh(1.0));
    b.ctext("Name", 92.0, 300.0, "Maya Lindqvist", t.h(20.0, t.dark));
    b.ctext("Role", 122.0, 300.0, "Ceramic artist", t.b(10.0, t.secondary).bold().upper().ls(300.0));
    b.rect("Rule", 148.0, 145.0, 40.0, 1.5, 0.0, solid(t.secondary));
    b.ctext("Contact", 156.0, 310.0, "maya@clay.example · 555 0100 600", t.b(9.0, t.dark.lerp(t.light, 0.25)));
}

// ---------------------------------------------------------------- invitations

fn birthday_invitation(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.outline("Border", ShapeKind::Rect { corner_radius: 16.0 }, 24.0, 24.0, 432.0, 624.0, t.primary, 3.0);
    // name, shape, x, y, w, h, primary colour?, rotation
    type Confetti = (&'static str, ShapeKind, f64, f64, f64, f64, bool, f64);
    let confetti: [Confetti; 8] = [
        ("Confetti dot", ShapeKind::Ellipse, 60.0, 60.0, 18.0, 18.0, true, 0.0),
        ("Confetti dot", ShapeKind::Ellipse, 400.0, 80.0, 14.0, 14.0, false, 0.0),
        ("Confetti triangle", ShapeKind::Triangle, 90.0, 150.0, 20.0, 18.0, false, 20.0),
        ("Confetti star", ShapeKind::Star { points: 5, inner_ratio: 0.45 }, 380.0, 170.0, 24.0, 24.0, true, 0.0),
        ("Confetti dot", ShapeKind::Ellipse, 70.0, 580.0, 16.0, 16.0, false, 0.0),
        ("Confetti triangle", ShapeKind::Triangle, 392.0, 590.0, 20.0, 18.0, true, -15.0),
        ("Confetti star", ShapeKind::Star { points: 5, inner_ratio: 0.45 }, 60.0, 400.0, 18.0, 18.0, false, 0.0),
        ("Confetti dot", ShapeKind::Ellipse, 410.0, 420.0, 12.0, 12.0, true, 0.0),
    ];
    for (name, shape, x, y, w, h, primary, rot) in confetti {
        b.shape(name, shape, x, y, w, h, solid(if primary { t.primary } else { t.secondary }));
        b.rotate_last(rot);
    }
    b.ellipse("Balloon", 196.0, 64.0, 88.0, 106.0, lin(90.0, t.primary.lerp(t.light, 0.25), t.primary));
    b.shape("Balloon knot", ShapeKind::Triangle, 232.0, 168.0, 16.0, 12.0, solid(t.primary));
    b.rect("Balloon string", 239.0, 180.0, 2.0, 56.0, 1.0, solid(t.dark.lerp(t.light, 0.5)));
    b.ctext("Kicker", 256.0, 360.0, "You're invited to", t.b(16.0, t.dark).bold().upper().ls(300.0));
    b.ctext("Headline", 288.0, 400.0, "Mia's 30th\nBirthday", t.h(60.0, t.primary.lerp(t.dark, 0.35)).italic());
    b.rect("Divider", 210.0, 440.0, 60.0, 3.0, 1.5, solid(t.secondary));
    b.ctext("Details", 460.0, 400.0, "Saturday, 21 June · 7 pm\nThe Garden Room, 12 Elm Road", t.b(18.0, t.dark).lh(1.5));
    b.ctext("RSVP", 540.0, 360.0, "RSVP to Sam by 1 June", t.b(16.0, t.primary.lerp(t.dark, 0.35)).bold());
}

fn wedding_invitation(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.outline("Border", ShapeKind::Rect { corner_radius: 8.0 }, 20.0, 20.0, 440.0, 632.0, t.primary, 1.5);
    for (name, x, y, s, rot, primary) in [
        ("Leaf top left", 40.0, 44.0, 64.0, -20.0, false),
        ("Leaf top", 92.0, 36.0, 44.0, 30.0, true),
        ("Leaf bottom right", 376.0, 564.0, 64.0, 160.0, false),
        ("Leaf bottom", 344.0, 592.0, 44.0, 210.0, true),
    ] {
        b.icon(name, "leaf", x, y, s, s, solid(if primary { t.primary } else { t.secondary }));
        b.rotate_last(rot);
    }
    b.ctext("Kicker", 150.0, 420.0, "Together with their families", t.b(13.0, t.dark).upper().ls(250.0));
    b.ctext("Names", 190.0, 420.0, "Emma\n& Noah", t.h(64.0, t.primary).italic().with(|s| s.bold = false));
    b.ctext("Invite", 365.0, 420.0, "invite you to celebrate their wedding", t.b(15.0, t.dark));
    b.icon("Divider", "sparkle", 230.0, 402.0, 20.0, 20.0, solid(t.secondary));
    b.ctext("Date", 440.0, 440.0, "Saturday, 12 September 2026", t.h(20.0, t.dark));
    b.ctext("Place", 482.0, 420.0, "Four o'clock in the afternoon\nWillow Barn, Hollow Lane", t.b(15.0, t.dark.lerp(t.light, 0.2)).lh(1.6));
    b.ctext("RSVP", 566.0, 420.0, "Kindly reply by 1 August", t.b(13.0, t.secondary.lerp(t.dark, 0.2)).bold().upper().ls(200.0));
}

fn baby_shower(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.icon("Cloud", "cloud", 36.0, 60.0, 130.0, 86.0, solid(t.primary.lerp(t.light, 0.65)));
    b.icon("Small cloud", "cloud", 320.0, 140.0, 110.0, 74.0, solid(t.primary.lerp(t.light, 0.75)));
    b.icon("Balloon", "balloon", 180.0, 70.0, 120.0, 170.0, solid(t.primary));
    for (i, (x, y, s)) in [(70.0, 190.0, 24.0), (390.0, 60.0, 30.0), (120.0, 600.0, 22.0), (360.0, 590.0, 28.0)].into_iter().enumerate() {
        b.icon(&format!("Star {}", i + 1), "sparkle", x, y, s, s, solid(t.secondary));
    }
    b.ctext("Kicker", 280.0, 420.0, "Please join us for a", t.b(14.0, t.dark).upper().ls(250.0));
    b.ctext("Headline", 308.0, 440.0, "Baby shower", t.h(54.0, t.primary.lerp(t.dark, 0.35)));
    b.ctext("Honouring", 384.0, 400.0, "in honour of Priya", t.b(18.0, t.dark).italic());
    b.rect("Details card", 60.0, 440.0, 360.0, 120.0, 18.0, solid(t.secondary.lerp(t.light, 0.6)));
    b.ctext("Details", 466.0, 340.0, "Sunday, 9 August · 2 pm\n14 Maple Court, Brookside", t.b(16.0, t.dark).lh(1.6));
    b.ctext("RSVP", 584.0, 400.0, "RSVP to Lena · 555 0100 700", t.b(13.0, t.dark).bold());
}

fn party_invitation(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.dark));
    b.ellipse("Disco ball", 390.0, 40.0, 130.0, 130.0, lin(135.0, t.primary, t.secondary));
    b.icon("Sparkles", "sparkles", 486.0, 24.0, 50.0, 50.0, solid(t.light));
    b.icon("Confetti", "confetti", 400.0, 230.0, 140.0, 140.0, solid(t.secondary));
    b.text("Kicker", 40.0, 96.0, 330.0, "You're invited", t.b(14.0, t.secondary).bold().upper().ls(300.0).left());
    b.text("Headline", 40.0, 124.0, 360.0, "Rooftop\nparty", t.h(64.0, t.light).upper().lh(0.95).left());
    b.text(
        "Details",
        40.0,
        272.0,
        340.0,
        "Friday 18 July · 9 pm till late\nThe Highline, 88 Harbour St",
        t.b(14.0, t.light.lerp(t.dark, 0.2)).lh(1.5).left(),
    );
    b.pill("RSVP", 40.0, 330.0, 160.0, 38.0, solid(t.primary), "RSVP by 10 July", t.b(13.0, t.light).bold());
}

// ---------------------------------------------------------------- greeting cards

fn thank_you(b: &mut B) {
    let t = b.t.clone();
    let h = b.h;
    b.bg(solid(t.light));
    b.rect("Colour panel", 0.0, 0.0, 230.0, h, 0.0, solid(t.primary));
    b.icon("Heart", "heart", 65.0, 148.0, 100.0, 100.0, solid(t.light));
    b.text("Headline", 258.0, 100.0, 290.0, "Thank you", t.h(52.0, t.dark).italic().with(|s| s.bold = false).left());
    b.rect("Accent", 260.0, 180.0, 50.0, 3.0, 1.5, solid(t.secondary));
    b.text(
        "Message",
        260.0,
        200.0,
        280.0,
        "for being part of our\nlittle celebration.\nIt meant the world.",
        t.b(16.0, t.dark.lerp(t.light, 0.2)).lh(1.6).left(),
    );
    b.text("Signature", 260.0, 300.0, 280.0, "With love, Ana & Theo", t.b(15.0, t.secondary.lerp(t.dark, 0.35)).bold().left());
}

fn holiday_card(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.dark));
    for (i, (x, y, s)) in
        [(40.0, 40.0, 36.0), (400.0, 60.0, 44.0), (90.0, 290.0, 28.0), (380.0, 280.0, 30.0), (220.0, 30.0, 22.0)].into_iter().enumerate()
    {
        b.icon(&format!("Snowflake {}", i + 1), "snowflake", x, y, s, s, solid(t.light.with_alpha(if i % 2 == 0 { 200 } else { 120 })));
    }
    b.ctext("Headline", 110.0, 440.0, "Warm wishes", t.h(50.0, t.light));
    b.ctext("Subline", 190.0, 440.0, "and a bright new year", t.b(18.0, t.light).upper().ls(250.0));
    b.rect("Accent", 210.0, 240.0, 60.0, 4.0, 2.0, solid(t.primary));
    b.icon("Tree left", "tree", 50.0, 410.0, 120.0, 190.0, solid(t.secondary));
    b.icon("Tree middle", "tree", 175.0, 360.0, 130.0, 240.0, solid(t.secondary.lerp(t.dark, 0.2)));
    b.icon("Tree right", "tree", 310.0, 430.0, 110.0, 170.0, solid(t.secondary));
    for (i, (x, y)) in [(222.0, 450.0), (256.0, 490.0), (205.0, 520.0), (270.0, 545.0)].into_iter().enumerate() {
        b.ellipse(&format!("Bauble {}", i + 1), x, y, 12.0, 12.0, solid(t.primary));
    }
    b.rect("Snow", 0.0, 590.0, w, h - 590.0, 0.0, solid(t.light));
    b.ctext("Signature", 620.0, 400.0, "From the Rivera family", t.b(16.0, t.dark).bold());
}

// ---------------------------------------------------------------- certificates

fn certificate(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.outline("Outer border", ShapeKind::Rect { corner_radius: 0.0 }, 30.0, 30.0, w - 60.0, h - 60.0, t.primary, 10.0);
    b.outline("Inner border", ShapeKind::Rect { corner_radius: 0.0 }, 52.0, 52.0, w - 104.0, h - 104.0, t.secondary, 2.0);
    for (name, cx, cy) in [
        ("Corner top left", 52.0, 52.0),
        ("Corner top right", w - 52.0, 52.0),
        ("Corner bottom left", 52.0, h - 52.0),
        ("Corner bottom right", w - 52.0, h - 52.0),
    ] {
        b.rect(name, cx - 16.0, cy - 16.0, 32.0, 32.0, 0.0, solid(t.secondary));
        b.rotate_last(45.0);
    }
    b.ctext("Title", 120.0, 800.0, "Certificate", t.h(72.0, t.primary).upper().ls(150.0));
    b.ctext("Subtitle", 212.0, 600.0, "Of achievement", t.b(24.0, t.secondary).bold().upper().ls(400.0));
    b.ctext("Presented to", 290.0, 700.0, "This certificate is proudly presented to", t.b(22.0, t.dark.lerp(t.light, 0.3)));
    b.ctext("Recipient", 335.0, 800.0, "Jordan Lee", t.h(80.0, t.dark).italic().with(|s| s.bold = false));
    b.rect("Name line", (w - 500.0) / 2.0, 450.0, 500.0, 2.0, 1.0, solid(t.secondary));
    b.ctext("Reason", 478.0, 760.0, "for outstanding dedication and creativity\nin the 2026 Young Designers Programme", t.b(22.0, t.dark).lh(1.5));
    b.shape("Seal", ShapeKind::Star { points: 20, inner_ratio: 0.86 }, (w - 120.0) / 2.0, 570.0, 120.0, 120.0, solid(t.secondary));
    b.ellipse("Seal centre", (w - 80.0) / 2.0, 590.0, 80.0, 80.0, solid(t.primary));
    b.rect("Date line", 160.0, 650.0, 260.0, 2.0, 1.0, solid(t.dark));
    b.text("Date label", 160.0, 662.0, 260.0, "Date", t.b(18.0, t.dark.lerp(t.light, 0.3)));
    b.rect("Signature line", w - 420.0, 650.0, 260.0, 2.0, 1.0, solid(t.dark));
    b.text("Signature label", w - 420.0, 662.0, 260.0, "Signature", t.b(18.0, t.dark.lerp(t.light, 0.3)));
}

fn course_certificate(b: &mut B) {
    let t = b.t.clone();
    let h = b.h;
    b.bg(solid(t.light));
    b.rect("Side band", 0.0, 0.0, 300.0, h, 0.0, lin(90.0, t.primary, t.primary.lerp(t.dark, 0.35)));
    b.icon("Rosette", "rosette", 70.0, 100.0, 160.0, 160.0, solid(t.secondary));
    b.icon("Rosette star", "star", 125.0, 155.0, 50.0, 50.0, solid(t.secondary));
    b.text("Academy", 30.0, 300.0, 240.0, "Northwind\nAcademy", t.b(24.0, t.light).bold().lh(1.3));
    for i in 0..3 {
        b.rect(&format!("Stripe {}", i + 1), 40.0 + 80.0 * f64::from(i), h - 80.0, 60.0, 6.0, 3.0, solid(t.light.with_alpha(120)));
    }
    b.text("Kicker", 360.0, 110.0, 700.0, "Certificate of completion", t.b(20.0, t.primary).bold().upper().ls(300.0).left());
    b.text("Certifies", 360.0, 190.0, 700.0, "This certifies that", t.b(22.0, t.dark.lerp(t.light, 0.3)).left());
    b.text("Recipient", 360.0, 230.0, 700.0, "Riley Chen", t.h(80.0, t.dark).left());
    b.rect("Name line", 360.0, 340.0, 560.0, 2.0, 1.0, solid(t.secondary));
    b.text("Completed", 360.0, 370.0, 700.0, "has successfully completed the eight-week course", t.b(22.0, t.dark).left());
    b.text("Course", 360.0, 410.0, 700.0, "Introduction to Visual Design", t.h(36.0, t.primary).left());
    b.text("Details", 360.0, 465.0, 700.0, "with 40 hours of guided practice and a final project", t.b(20.0, t.dark.lerp(t.light, 0.25)).left());
    b.rect("Lead line", 360.0, 640.0, 260.0, 2.0, 1.0, solid(t.dark));
    b.text("Lead label", 360.0, 652.0, 260.0, "Course lead", t.b(16.0, t.dark.lerp(t.light, 0.3)).left());
    b.rect("Date line", 680.0, 640.0, 260.0, 2.0, 1.0, solid(t.dark));
    b.text("Date label", 680.0, 652.0, 260.0, "Date", t.b(16.0, t.dark.lerp(t.light, 0.3)).left());
    b.icon("Seal", "seal", 970.0, 560.0, 110.0, 110.0, solid(t.secondary));
    b.icon("Seal check", "check", 1000.0, 590.0, 50.0, 50.0, solid(t.light));
}

// ---------------------------------------------------------------- vouchers, coupons, tickets

/// A vertical dashed line at `x` from `y0` to `y1`.
fn dashed_vline(b: &mut B, name: &str, x: f64, y0: f64, y1: f64, color: genha_color::Color, width: f64) {
    let len = y1 - y0;
    b.shape(name, ShapeKind::Line, x - len / 2.0, (y0 + y1) / 2.0 - 2.0, len, 4.0, genha_doc::Fill::None);
    b.dash_last(color, width, &[6.0, 6.0]);
    b.rotate_last(90.0);
}

fn gift_voucher(b: &mut B) {
    let t = b.t.clone();
    let h = b.h;
    b.bg(solid(t.light));
    b.rect("Voucher panel", 0.0, 0.0, 400.0, h, 0.0, lin(135.0, t.primary, t.primary.lerp(t.dark, 0.4)));
    b.icon("Gift", "gift", 300.0, 30.0, 70.0, 70.0, solid(t.secondary));
    b.text("Kicker", 36.0, 50.0, 260.0, "Gift voucher", t.b(14.0, t.secondary).bold().upper().ls(300.0).left());
    b.text("Amount", 36.0, 86.0, 300.0, "$50", t.h(110.0, t.light).lh(1.0).left());
    b.text("For", 36.0, 226.0, 330.0, "for a relaxing day at", t.b(15.0, t.light).left());
    b.text("Venue", 36.0, 250.0, 330.0, "Juniper Spa", t.h(36.0, t.light).left());
    b.text("Validity", 36.0, 340.0, 330.0, "Valid until 31 December 2026", t.b(12.0, t.light.lerp(t.dark, 0.3)).left());
    dashed_vline(b, "Perforation", 400.0, 8.0, h - 8.0, t.dark.with_alpha(120), 1.5);
    b.text("Number label", 410.0, 50.0, 140.0, "No.", t.b(12.0, t.dark).upper().ls(200.0));
    b.text("Number", 410.0, 72.0, 140.0, "GV-2048", t.h(20.0, t.dark));
    for (i, label) in ["To", "From"].iter().enumerate() {
        let y = 140.0 + 70.0 * i as f64;
        b.text(&format!("{label} label"), 425.0, y, 120.0, label, t.b(12.0, t.dark.lerp(t.light, 0.3)).left());
        b.rect(&format!("{label} line"), 425.0, y + 38.0, 110.0, 1.5, 0.0, solid(t.dark.lerp(t.light, 0.5)));
    }
    b.icon("Sparkle", "sparkle", 455.0, 300.0, 50.0, 50.0, solid(t.secondary));
}

fn coupon(b: &mut B) {
    let t = b.t.clone();
    let (w, h) = (b.w, b.h);
    b.bg(solid(t.light));
    b.outline("Dashed border", ShapeKind::Rect { corner_radius: 20.0 }, 16.0, 16.0, w - 32.0, h - 32.0, t.primary, 3.0);
    b.dash_last(t.primary, 3.0, &[12.0, 8.0]);
    b.text("Discount", 50.0, 60.0, 300.0, "20%", t.h(120.0, t.primary).lh(1.0).left());
    b.text("Offer", 50.0, 196.0, 300.0, "off your next order", t.b(22.0, t.dark).left());
    b.pill("Code", 50.0, 245.0, 240.0, 54.0, solid(t.secondary), "Code: HELLO20", t.b(18.0, t.dark).bold().ls(80.0));
    b.text("Terms", 50.0, 322.0, 460.0, "Valid until 30 November · online and in store", t.b(12.0, t.dark.lerp(t.light, 0.3)).left());
    b.ellipse("Badge", 355.0, 56.0, 170.0, 170.0, solid(t.primary));
    b.icon("Percent", "percent", 395.0, 96.0, 90.0, 90.0, solid(t.light));
    b.text("Shop", 330.0, 240.0, 220.0, "Pip & Pine", t.h(24.0, t.dark));
    b.text("Shop website", 330.0, 276.0, 220.0, "pipandpine.example", t.b(12.0, t.dark));
}

fn event_ticket(b: &mut B) {
    let t = b.t.clone();
    b.bg(solid(t.light));
    b.rect("Ticket", 20.0, 60.0, 519.0, 277.0, 18.0, solid(t.primary));
    b.ellipse("Top notch", 388.0, 48.0, 24.0, 24.0, solid(t.light));
    b.ellipse("Bottom notch", 388.0, 325.0, 24.0, 24.0, solid(t.light));
    dashed_vline(b, "Perforation", 400.0, 84.0, 313.0, t.light.with_alpha(180), 1.5);
    b.text("Kicker", 44.0, 82.0, 330.0, "Admit one", t.b(14.0, t.light).bold().upper().ls(300.0).left());
    b.text("Title", 44.0, 108.0, 340.0, "Night\nmarket", t.h(56.0, t.light).upper().lh(0.95).left());
    for (i, (label, value)) in [("Date", "25 OCT"), ("Gate", "B"), ("Time", "6 PM")].iter().enumerate() {
        let x = 44.0 + 110.0 * i as f64;
        b.text(&format!("{label} label"), x, 240.0, 100.0, label, t.b(11.0, t.dark).bold().upper().ls(150.0).left());
        b.text(&format!("{label} value"), x, 258.0, 100.0, value, t.b(16.0, t.light).bold().left());
    }
    b.text("Venue", 44.0, 300.0, 340.0, "Harbour Square · market.example", t.b(11.0, t.light).left());
    b.text("Seat label", 410.0, 84.0, 120.0, "Seat", t.b(11.0, t.secondary).bold().upper().ls(150.0));
    b.text("Seat", 410.0, 100.0, 120.0, "12", t.h(32.0, t.light).lh(1.0));
    let bars = [3.0, 1.0, 2.0, 1.0, 3.0, 2.0, 1.0, 1.0, 3.0, 1.0, 2.0, 2.0, 1.0, 3.0, 1.0, 2.0, 1.0, 3.0];
    let mut x = 432.0;
    for (i, bw) in bars.iter().enumerate() {
        b.rect(&format!("Barcode bar {}", i + 1), x, 150.0, *bw * 1.6, 110.0, 0.0, solid(t.dark));
        x += *bw * 1.6 + 2.2;
    }
    b.text("Ticket number", 410.0, 270.0, 120.0, "004213", t.b(14.0, t.light).bold().ls(100.0));
}
