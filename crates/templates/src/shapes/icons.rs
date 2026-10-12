//! Icon drawings: original filled outlines in a 100 × 100 view box, built with [`P`].

use super::path::{P, Pt, at, rotate};

// ---------------------------------------------------------------- arrows

pub(super) fn chevron(deg: f64) -> String {
    let pts = [(30.0, 8.0), (48.0, 8.0), (82.0, 50.0), (48.0, 92.0), (30.0, 92.0), (64.0, 50.0)];
    P::new().poly(&rotate(&pts, (50.0, 50.0), deg), false).done()
}

pub(super) fn arrow_diagonal() -> String {
    let pts = [(10.0, 40.0), (58.0, 40.0), (58.0, 20.0), (90.0, 50.0), (58.0, 80.0), (58.0, 60.0), (10.0, 60.0)];
    P::new().poly(&rotate(&pts, (50.0, 50.0), -45.0), false).done()
}

pub(super) fn arrow_turn() -> String {
    // A band from 10 o'clock round to 3 o'clock, ending in a downward head.
    let (c, ro, ri) = ((50.0, 54.0), 36.0, 22.0);
    let (o0, i0) = (at(c.0, c.1, ro, 290.0), at(c.0, c.1, ri, 290.0));
    let (o1, i1) = (at(c.0, c.1, ro, 450.0), at(c.0, c.1, ri, 450.0));
    let d = format!(
        "M{:.2} {:.2} A{ro} {ro} 0 0 1 {:.2} {:.2} L96 {:.2} L79 {:.2} L62 {:.2} L{:.2} {:.2} A{ri} {ri} 0 0 0 {:.2} {:.2} Z",
        o0.0,
        o0.1,
        o1.0,
        o1.1,
        o1.1,
        o1.1 + 24.0,
        i1.1,
        i1.0,
        i1.1,
        i0.0,
        i0.1
    );
    P::new().raw(&d).done()
}

pub(super) fn arrows_swap() -> String {
    P::new()
        .poly(&[(8.0, 26.0), (66.0, 26.0), (66.0, 10.0), (92.0, 32.0), (66.0, 54.0), (66.0, 38.0), (8.0, 38.0)], false)
        .poly(&[(92.0, 62.0), (34.0, 62.0), (34.0, 58.0), (8.0, 74.0), (34.0, 90.0), (34.0, 74.0), (92.0, 74.0)], false)
        .done()
}

// ---------------------------------------------------------------- interface

pub(super) fn search() -> String {
    // A lens (ring) and handle as one outline, plus the lens hole.
    let c = (42.0, 42.0);
    let (r, hw): (f64, f64) = (30.0, 7.5);
    let u = (std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2);
    let nn = (-u.1, u.0);
    let t0 = (r * r - hw * hw).sqrt();
    let t1 = 54.0;
    let p = |t: f64, s: f64| (c.0 + u.0 * t + nn.0 * hw * s, c.1 + u.1 * t + nn.1 * hw * s);
    let (b, a) = (p(t0, -1.0), p(t0, 1.0));
    let (b1, a1) = (p(t1, -1.0), p(t1, 1.0));
    let d = format!(
        "M{:.2} {:.2} L{:.2} {:.2} A{hw} {hw} 0 0 1 {:.2} {:.2} L{:.2} {:.2} A{r} {r} 0 1 1 {:.2} {:.2} Z",
        b.0, b.1, b1.0, b1.1, a1.0, a1.1, a.0, a.1, b.0, b.1
    );
    P::new().raw(&d).circle(c.0, c.1, 19.0, true).done()
}

pub(super) fn gear() -> String {
    let mut pts: Vec<Pt> = vec![];
    for i in 0..8 {
        let a = f64::from(i) * 45.0;
        pts.extend([at(50.0, 50.0, 34.0, a - 15.0), at(50.0, 50.0, 45.0, a - 9.0), at(50.0, 50.0, 45.0, a + 9.0), at(50.0, 50.0, 34.0, a + 15.0)]);
    }
    P::new().poly(&pts, false).circle(50.0, 50.0, 14.0, true).done()
}

pub(super) fn user() -> String {
    P::new().circle(50.0, 28.0, 20.0, false).raw("M12 94 C12 70 28 56 50 56 C72 56 88 70 88 94 Z").done()
}

pub(super) fn users() -> String {
    P::new()
        .circle(30.0, 36.0, 14.0, false)
        .raw("M6 88 C6 70 16 58 30 58 C41 58 47 64 48 72 V88 Z")
        .circle(70.0, 30.0, 16.0, false)
        .raw("M52 88 C52 64 60 52 70 52 C84 52 94 64 94 88 Z")
        .done()
}

pub(super) fn calendar() -> String {
    let mut p = P::new().rrect(8.0, 12.0, 84.0, 80.0, 10.0, false).rrect(16.0, 34.0, 68.0, 50.0, 4.0, true);
    p = p.circle(30.0, 23.0, 4.0, true).circle(70.0, 23.0, 4.0, true);
    for row in 0..2 {
        for col in 0..3 {
            let (x, y) = (22.0 + f64::from(col) * 20.0, 40.0 + f64::from(row) * 20.0);
            p = p.rrect(x, y, 16.0, 14.0, 2.0, false);
        }
    }
    p.done()
}

pub(super) fn clock() -> String {
    P::new().ring(50.0, 50.0, 45.0, 37.0).poly(&[(46.0, 20.0), (54.0, 20.0), (54.0, 46.0), (72.0, 46.0), (72.0, 54.0), (46.0, 54.0)], false).done()
}

pub(super) fn map() -> String {
    P::new()
        .poly(&[(6.0, 20.0), (34.0, 10.0), (34.0, 80.0), (6.0, 90.0)], false)
        .poly(&[(38.0, 10.0), (62.0, 20.0), (62.0, 90.0), (38.0, 80.0)], false)
        .poly(&[(66.0, 20.0), (94.0, 10.0), (94.0, 80.0), (66.0, 90.0)], false)
        .done()
}

pub(super) fn camera() -> String {
    P::new()
        .raw("M36 14 H64 L72 28 H84 C90 28 94 32 94 38 V78 C94 84 90 88 84 88 H16 C10 88 6 84 6 78 V38 C6 32 10 28 16 28 H28 Z")
        .circle(50.0, 57.0, 21.0, true)
        .circle(50.0, 57.0, 13.0, false)
        .rrect(76.0, 36.0, 10.0, 6.0, 2.0, true)
        .done()
}

pub(super) fn image() -> String {
    P::new()
        .rrect(6.0, 14.0, 88.0, 72.0, 8.0, false)
        .rrect(14.0, 22.0, 72.0, 56.0, 3.0, true)
        .poly(&[(20.0, 72.0), (42.0, 42.0), (56.0, 60.0), (66.0, 50.0), (80.0, 72.0)], false)
        .circle(67.0, 35.0, 7.0, false)
        .done()
}

pub(super) fn video() -> String {
    P::new().rrect(6.0, 24.0, 60.0, 52.0, 10.0, false).poly(&[(70.0, 50.0), (94.0, 30.0), (94.0, 70.0)], false).done()
}

pub(super) fn microphone() -> String {
    P::new()
        .rrect(36.0, 6.0, 28.0, 50.0, 14.0, false)
        .sector(50.0, 38.0, 22.0, 28.0, 90.0, 270.0, false)
        .rect(47.0, 66.0, 6.0, 16.0, false)
        .rrect(30.0, 82.0, 40.0, 8.0, 4.0, false)
        .done()
}

pub(super) fn headphones() -> String {
    P::new()
        .sector(50.0, 54.0, 36.0, 44.0, 270.0, 450.0, false)
        .rrect(6.0, 56.0, 22.0, 36.0, 8.0, false)
        .rrect(72.0, 56.0, 22.0, 36.0, 8.0, false)
        .done()
}

pub(super) fn share() -> String {
    let (a, b, c) = ((74.0, 20.0), (26.0, 50.0), (74.0, 80.0));
    let r = 14.0;
    let link = |p: P, from: Pt, to: Pt| {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        let len = dx.hypot(dy);
        let (ux, uy) = (dx / len, dy / len);
        let s = r + 3.0;
        p.bar((from.0 + ux * s, from.1 + uy * s), (to.0 - ux * s, to.1 - uy * s), 7.0, false)
    };
    let p = P::new().circle(a.0, a.1, r, false).circle(b.0, b.1, r, false).circle(c.0, c.1, r, false);
    link(link(p, b, a), b, c).done()
}

pub(super) fn bookmark() -> String {
    P::new().poly(&[(22.0, 6.0), (78.0, 6.0), (78.0, 94.0), (50.0, 72.0), (22.0, 94.0)], false).done()
}

pub(super) fn paper_plane() -> String {
    P::new().poly(&[(6.0, 46.0), (94.0, 8.0), (66.0, 92.0), (48.0, 62.0)], false).poly(&[(40.0, 66.0), (52.0, 72.0), (40.0, 90.0)], false).done()
}

pub(super) fn eye() -> String {
    P::new().raw("M4 50 C20 20 80 20 96 50 C80 80 20 80 4 50 Z").circle(50.0, 50.0, 19.0, true).circle(50.0, 50.0, 10.0, false).done()
}

fn tray(p: P) -> P {
    p.poly(&[(6.0, 62.0), (16.0, 62.0), (16.0, 82.0), (84.0, 82.0), (84.0, 62.0), (94.0, 62.0), (94.0, 92.0), (6.0, 92.0)], false)
}

pub(super) fn download() -> String {
    tray(P::new().poly(&[(42.0, 6.0), (58.0, 6.0), (58.0, 38.0), (74.0, 38.0), (50.0, 66.0), (26.0, 38.0), (42.0, 38.0)], false)).done()
}

pub(super) fn upload() -> String {
    tray(P::new().poly(&[(50.0, 6.0), (74.0, 32.0), (58.0, 32.0), (58.0, 70.0), (42.0, 70.0), (42.0, 32.0), (26.0, 32.0)], false)).done()
}

pub(super) fn check_circle() -> String {
    P::new().circle(50.0, 50.0, 45.0, false).poly(&[(24.0, 52.0), (32.0, 44.0), (44.0, 56.0), (68.0, 32.0), (76.0, 40.0), (44.0, 72.0)], true).done()
}

pub(super) fn info() -> String {
    P::new().circle(50.0, 50.0, 45.0, false).circle(50.0, 28.0, 7.0, true).rrect(43.0, 42.0, 14.0, 36.0, 3.0, true).done()
}

pub(super) fn warning() -> String {
    P::new()
        .raw("M44 10 C47 5 53 5 56 10 L94 80 C97 86 93 92 87 92 H13 C7 92 3 86 6 80 Z")
        .rrect(45.0, 34.0, 10.0, 32.0, 4.0, true)
        .circle(50.0, 77.0, 6.0, true)
        .done()
}

pub(super) fn question() -> String {
    P::new()
        .circle(50.0, 50.0, 45.0, false)
        .sector(50.0, 36.0, 9.0, 19.0, 270.0, 540.0, true)
        .rect(45.0, 57.0, 10.0, 10.0, true)
        .circle(50.0, 78.0, 6.0, true)
        .done()
}

pub(super) fn lock() -> String {
    P::new()
        .sector(50.0, 40.0, 16.0, 25.0, 270.0, 450.0, false)
        .rect(25.0, 40.0, 9.0, 6.0, false)
        .rect(66.0, 40.0, 9.0, 6.0, false)
        .rrect(14.0, 46.0, 72.0, 48.0, 8.0, false)
        .circle(50.0, 64.0, 7.0, true)
        .rrect(47.0, 72.0, 6.0, 12.0, 2.0, true)
        .done()
}

pub(super) fn key() -> String {
    P::new()
        .circle(28.0, 50.0, 22.0, false)
        .circle(28.0, 50.0, 9.0, true)
        .rect(50.5, 44.0, 44.0, 12.0, false)
        .rect(68.0, 56.0, 8.0, 12.0, false)
        .rect(84.0, 56.0, 10.0, 18.0, false)
        .done()
}

pub(super) fn shield_check() -> String {
    P::new()
        .raw("M50 4 L88 18 V46 C88 70 72 86 50 96 C28 86 12 70 12 46 V18 Z")
        .poly(&[(30.0, 50.0), (37.0, 43.0), (46.0, 52.0), (64.0, 34.0), (71.0, 41.0), (46.0, 66.0)], true)
        .done()
}

pub(super) fn wifi() -> String {
    P::new()
        .sector(50.0, 86.0, 54.0, 64.0, 315.0, 405.0, false)
        .sector(50.0, 86.0, 33.0, 43.0, 315.0, 405.0, false)
        .sector(50.0, 86.0, 13.0, 22.0, 315.0, 405.0, false)
        .circle(50.0, 86.0, 6.0, false)
        .done()
}

pub(super) fn battery() -> String {
    P::new()
        .rrect(4.0, 28.0, 84.0, 44.0, 8.0, false)
        .rrect(12.0, 36.0, 68.0, 28.0, 3.0, true)
        .rect(17.0, 41.0, 40.0, 18.0, false)
        .rrect(88.0, 40.0, 8.0, 20.0, 2.0, false)
        .done()
}

pub(super) fn globe() -> String {
    let (c, r_in) = (50.0, 38.0);
    let half = (r_in * r_in - 9.0f64).sqrt();
    let lens = 14.0 * (1.0 - (3.0f64 / r_in).powi(2)).sqrt();
    P::new()
        .ring(c, c, 45.0, r_in)
        .ellipse(c, c, 14.0, r_in, false)
        .rect(c - half + 0.3, 47.0, half - lens - 0.6, 6.0, false)
        .rect(c + lens + 0.3, 47.0, half - lens - 0.6, 6.0, false)
        .done()
}

// ---------------------------------------------------------------- communication

pub(super) fn chat() -> String {
    P::new()
        .raw("M16 6 H52 C58 6 62 10 62 16 V32 C62 38 58 42 52 42 H28 L14 54 L17 42 H16 C10 42 6 38 6 32 V16 C6 10 10 6 16 6 Z")
        .raw("M48 48 H84 C90 48 94 52 94 58 V74 C94 80 90 84 84 84 H83 L86 96 L72 84 H48 C42 84 38 80 38 74 V58 C38 52 42 48 48 48 Z")
        .done()
}

pub(super) fn megaphone() -> String {
    P::new()
        .poly(&[(20.0, 36.0), (72.0, 12.0), (72.0, 84.0), (20.0, 62.0)], false)
        .rrect(6.0, 36.0, 12.0, 26.0, 3.0, false)
        .poly(&[(28.0, 74.0), (40.0, 74.0), (46.0, 92.0), (34.0, 92.0)], false)
        .sector(72.0, 48.0, 12.0, 18.0, 50.0, 130.0, false)
        .done()
}

pub(super) fn hashtag() -> String {
    let pts = [
        (30.0, 8.0),
        (41.0, 8.0),
        (41.0, 28.0),
        (59.0, 28.0),
        (59.0, 8.0),
        (70.0, 8.0),
        (70.0, 28.0),
        (92.0, 28.0),
        (92.0, 39.0),
        (70.0, 39.0),
        (70.0, 61.0),
        (92.0, 61.0),
        (92.0, 72.0),
        (70.0, 72.0),
        (70.0, 92.0),
        (59.0, 92.0),
        (59.0, 72.0),
        (41.0, 72.0),
        (41.0, 92.0),
        (30.0, 92.0),
        (30.0, 72.0),
        (8.0, 72.0),
        (8.0, 61.0),
        (30.0, 61.0),
        (30.0, 39.0),
        (8.0, 39.0),
        (8.0, 28.0),
        (30.0, 28.0),
    ];
    let shear = |p: &[Pt]| -> Vec<Pt> { p.iter().map(|(x, y)| (x - (y - 50.0) * 0.12, *y)).collect() };
    P::new().poly(&shear(&pts), false).poly(&shear(&[(41.0, 39.0), (59.0, 39.0), (59.0, 61.0), (41.0, 61.0)]), true).done()
}

pub(super) fn at_sign() -> String {
    P::new().ring(48.0, 50.0, 19.0, 10.0).rrect(69.0, 31.0, 9.0, 30.0, 4.5, false).sector(50.0, 50.0, 36.0, 45.0, 150.0, 445.0, false).done()
}

pub(super) fn quote_marks() -> String {
    let mark = |dx: f64| {
        format!(
            "M{} 44 C{} 44 {} 51 {} 60 C{} 69 {} 76 {} 76 C{} 76 {} 69 {} 60 C{} 42 {} 28 {} 20 L{} 28 C{} 32 {} 38 {} 44.2 C{} 44.06 {} 44 {} 44 Z",
            30.0 + dx,
            39.0 + dx,
            46.0 + dx,
            46.0 + dx,
            46.0 + dx,
            39.0 + dx,
            30.0 + dx,
            21.0 + dx,
            14.0 + dx,
            14.0 + dx,
            14.0 + dx,
            24.0 + dx,
            42.0 + dx,
            44.0 + dx,
            35.0 + dx,
            29.5 + dx,
            28.4 + dx,
            29.0 + dx,
            29.5 + dx,
            30.0 + dx
        )
    };
    P::new().raw(&mark(0.0)).raw(&mark(40.0)).done()
}

pub(super) fn thumbs_up() -> String {
    P::new()
        .rrect(6.0, 44.0, 18.0, 48.0, 4.0, false)
        .raw("M30 44 L46 14 C48 8 58 8 60 16 L57 38 H84 C90 38 94 44 92 50 L86 82 C85 88 80 92 74 92 H30 Z")
        .done()
}

pub(super) fn smile() -> String {
    P::new()
        .circle(50.0, 50.0, 45.0, false)
        .ellipse(35.0, 38.0, 6.0, 8.0, true)
        .ellipse(65.0, 38.0, 6.0, 8.0, true)
        .sector(50.0, 52.0, 19.0, 27.0, 100.0, 260.0, true)
        .done()
}

pub(super) fn hand_wave() -> String {
    P::new()
        .rrect(24.0, 16.0, 12.0, 34.0, 6.0, false)
        .rrect(38.0, 8.0, 12.0, 42.0, 6.0, false)
        .rrect(52.0, 12.0, 12.0, 38.0, 6.0, false)
        .rrect(66.0, 22.0, 12.0, 28.0, 6.0, false)
        .raw("M22 52 H78 V72 C78 85 68 94 54 94 H44 C31 94 22 85 22 72 Z")
        .capsule((9.0, 48.0), (15.0, 66.0), 5.0, false)
        .sector(26.0, 22.0, 13.0, 18.0, 280.0, 340.0, false)
        .sector(82.0, 16.0, 6.0, 11.0, 20.0, 80.0, false)
        .done()
}

// ---------------------------------------------------------------- business

pub(super) fn shopping_bag() -> String {
    P::new().poly(&[(14.0, 34.0), (86.0, 34.0), (92.0, 94.0), (8.0, 94.0)], false).sector(50.0, 34.0, 13.0, 20.0, 270.0, 450.0, false).done()
}

pub(super) fn cart() -> String {
    P::new()
        .poly(&[(4.0, 8.0), (19.0, 8.0), (33.0, 62.0), (86.0, 62.0), (86.0, 70.0), (27.0, 70.0), (13.5, 16.0), (4.0, 16.0)], false)
        .poly(&[(28.0, 22.0), (94.0, 22.0), (86.0, 54.0), (36.0, 54.0)], false)
        .circle(38.0, 84.0, 8.0, false)
        .circle(78.0, 84.0, 8.0, false)
        .done()
}

pub(super) fn tag() -> String {
    P::new()
        .raw("M54 6 H88 C91 6 94 9 94 12 V46 C94 48 93 50 92 51 L52 91 C49 94 45 94 42 91 L9 58 C6 55 6 51 9 48 L49 8 C50 7 52 6 54 6 Z")
        .circle(75.0, 25.0, 7.0, true)
        .done()
}

pub(super) fn gift() -> String {
    P::new()
        .rect(8.0, 30.0, 36.0, 18.0, false)
        .rect(56.0, 30.0, 36.0, 18.0, false)
        .rect(46.0, 30.0, 8.0, 64.0, false)
        .rect(14.0, 52.0, 30.0, 42.0, false)
        .rect(56.0, 52.0, 30.0, 42.0, false)
        .raw("M46 26 C36 26 22 24 22 15 C22 6 34 6 46 26 Z")
        .raw("M54 26 C66 6 78 6 78 15 C78 24 64 26 54 26 Z")
        .done()
}

pub(super) fn trophy() -> String {
    P::new()
        .raw("M24 8 H76 V30 C76 48 66 62 50 62 C34 62 24 48 24 30 Z")
        .sector(24.0, 26.0, 9.0, 16.0, 180.0, 360.0, false)
        .sector(76.0, 26.0, 9.0, 16.0, 0.0, 180.0, false)
        .rect(45.0, 62.0, 10.0, 18.0, false)
        .rrect(28.0, 80.0, 44.0, 12.0, 3.0, false)
        .done()
}

pub(super) fn award_star() -> String {
    let star: Vec<Pt> = (0..10).map(|i| at(50.0, 62.0, if i % 2 == 0 { 18.0 } else { 8.0 }, f64::from(i) * 36.0)).collect();
    P::new()
        .poly(&[(24.0, 4.0), (40.0, 4.0), (50.0, 26.0), (36.0, 30.0)], false)
        .poly(&[(76.0, 4.0), (60.0, 4.0), (50.0, 26.0), (64.0, 30.0)], false)
        .circle(50.0, 62.0, 32.0, false)
        .poly(&star, true)
        .done()
}

pub(super) fn flag() -> String {
    P::new()
        .rrect(12.0, 6.0, 8.0, 88.0, 3.0, false)
        .raw("M24 10 C40 4 52 16 68 10 C76 7 84 8 90 10 V52 C84 50 76 49 68 52 C52 58 40 46 24 52 Z")
        .done()
}

pub(super) fn crown() -> String {
    P::new()
        .poly(&[(6.0, 28.0), (28.0, 50.0), (50.0, 16.0), (72.0, 50.0), (94.0, 28.0), (86.0, 76.0), (14.0, 76.0)], false)
        .rrect(14.0, 80.0, 72.0, 10.0, 3.0, false)
        .done()
}

pub(super) fn briefcase() -> String {
    P::new()
        .poly(&[(34.0, 28.0), (34.0, 12.0), (66.0, 12.0), (66.0, 28.0), (59.0, 28.0), (59.0, 19.0), (41.0, 19.0), (41.0, 28.0)], false)
        .rrect(6.0, 28.0, 88.0, 62.0, 8.0, false)
        .rrect(43.0, 48.0, 14.0, 12.0, 2.0, true)
        .done()
}

pub(super) fn building() -> String {
    let mut p = P::new().poly(&[(20.0, 6.0), (80.0, 6.0), (80.0, 94.0), (58.0, 94.0), (58.0, 78.0), (42.0, 78.0), (42.0, 94.0), (20.0, 94.0)], false);
    for row in 0..5 {
        for col in 0..3 {
            p = p.rect(28.0 + f64::from(col) * 16.0, 14.0 + f64::from(row) * 13.0, 12.0, 8.0, true);
        }
    }
    p.done()
}

pub(super) fn graduation_cap() -> String {
    P::new()
        .poly(&[(50.0, 12.0), (96.0, 32.0), (50.0, 52.0), (4.0, 32.0)], false)
        .raw("M24 46 L50 58 L76 46 V68 C76 74 64 80 50 80 C36 80 24 74 24 68 Z")
        .rect(86.0, 40.0, 3.0, 22.0, false)
        .rrect(82.0, 62.0, 11.0, 14.0, 3.0, false)
        .done()
}

pub(super) fn book() -> String {
    P::new().raw("M6 20 C20 16 36 18 47 26 V88 C36 80 20 78 6 82 Z").raw("M53 26 C64 18 80 16 94 20 V82 C80 78 64 80 53 88 Z").done()
}

pub(super) fn lightbulb() -> String {
    P::new()
        .raw("M50 4 C70 4 84 18 84 36 C84 50 75 58 68 66 V72 H32 V66 C25 58 16 50 16 36 C16 18 30 4 50 4 Z")
        .sector(50.0, 36.0, 19.0, 25.0, 285.0, 335.0, true)
        .rrect(34.0, 76.0, 32.0, 6.0, 3.0, false)
        .rrect(36.0, 85.0, 28.0, 6.0, 3.0, false)
        .done()
}

pub(super) fn rocket() -> String {
    P::new()
        .raw("M50 4 C66 16 72 36 70 62 H30 C28 36 34 16 50 4 Z")
        .circle(50.0, 34.0, 9.0, true)
        .poly(&[(27.0, 46.0), (11.0, 64.0), (11.0, 80.0), (27.0, 68.0)], false)
        .poly(&[(73.0, 46.0), (89.0, 64.0), (89.0, 80.0), (73.0, 68.0)], false)
        .poly(&[(40.0, 66.0), (60.0, 66.0), (56.0, 80.0), (50.0, 94.0), (44.0, 80.0)], false)
        .done()
}

pub(super) fn chart_bars() -> String {
    P::new()
        .rrect(10.0, 58.0, 15.0, 28.0, 3.0, false)
        .rrect(31.0, 40.0, 15.0, 46.0, 3.0, false)
        .rrect(52.0, 24.0, 15.0, 62.0, 3.0, false)
        .rrect(73.0, 8.0, 15.0, 78.0, 3.0, false)
        .rrect(6.0, 89.0, 88.0, 5.0, 2.5, false)
        .done()
}

pub(super) fn chart_pie() -> String {
    let off = at(0.0, 0.0, 5.0, 315.0);
    P::new().sector(48.0, 52.0, 0.0, 42.0, 0.0, 270.0, false).sector(50.0 + off.0 - 2.0, 50.0 + off.1 + 2.0, 0.0, 42.0, 270.0, 360.0, false).done()
}

pub(super) fn percent() -> String {
    P::new().ring(27.0, 27.0, 17.0, 8.0).ring(73.0, 73.0, 17.0, 8.0).poly(&[(76.0, 8.0), (88.0, 16.0), (24.0, 92.0), (12.0, 84.0)], false).done()
}

pub(super) fn dollar_coin() -> String {
    P::new()
        .circle(50.0, 50.0, 46.0, false)
        .circle(50.0, 50.0, 40.0, true)
        .circle(50.0, 50.0, 37.0, false)
        .sector(50.0, 40.0, 6.0, 12.0, 180.0, 420.0, true)
        .sector(50.0, 60.0, 6.0, 12.0, 0.0, 240.0, true)
        .rect(47.0, 21.0, 6.0, 7.0, true)
        .rect(47.0, 72.0, 6.0, 7.0, true)
        .done()
}

pub(super) fn euro_coin() -> String {
    // A C open to the right, crossed by two bars (drawn in three pieces so nothing overlaps).
    let (cx, ro, ri) = (57.0, 23.0, 15.0);
    let edge = |r: f64, dy: f64| cx - (r * r - dy * dy).sqrt();
    let mut p = P::new().circle(50.0, 50.0, 46.0, false).sector(cx, 50.0, ri, ro, 145.0, 395.0, true);
    for (y0, y1) in [(40.0f64, 46.0f64), (54.0, 60.0)] {
        let near = (50.0 - y0).abs().min((50.0 - y1).abs());
        let far = (50.0 - y0).abs().max((50.0 - y1).abs());
        p = p.rect(18.0, y0, edge(ro, near) - 18.5, y1 - y0, true);
        let x = edge(ri, far) + 0.5;
        p = p.rect(x, y0, 62.0 - x, y1 - y0, true);
    }
    p.done()
}

// ---------------------------------------------------------------- food and drink

pub(super) fn coffee() -> String {
    P::new()
        .raw("M14 32 H74 V58 C74 76 62 86 44 86 C26 86 14 76 14 58 Z")
        .sector(74.0, 50.0, 8.0, 15.0, 0.0, 180.0, false)
        .rrect(8.0, 89.0, 72.0, 6.0, 3.0, false)
        .capsule((30.0, 8.0), (30.0, 22.0), 3.0, false)
        .capsule((44.0, 6.0), (44.0, 22.0), 3.0, false)
        .capsule((58.0, 8.0), (58.0, 22.0), 3.0, false)
        .done()
}

pub(super) fn pizza() -> String {
    P::new()
        .raw("M10 14 C36 4 64 4 90 14 L50 94 Z")
        .sector(50.0, 160.0, 140.0, 146.0, 348.0, 372.0, true)
        .circle(38.0, 36.0, 7.0, true)
        .circle(62.0, 40.0, 6.0, true)
        .circle(50.0, 62.0, 6.0, true)
        .done()
}

pub(super) fn cake() -> String {
    P::new()
        .rrect(10.0, 58.0, 80.0, 34.0, 6.0, false)
        .rrect(22.0, 32.0, 56.0, 23.0, 6.0, false)
        .rect(46.0, 14.0, 8.0, 15.0, false)
        .raw("M50 0.5 C55 6 56 10 50 11 C44 10 45 6 50 0.5 Z")
        .raw("M10 68 V72 C16 78 22 78 26 72 C30 78 38 78 42 72 C46 78 54 78 58 72 C62 78 70 78 74 72 C78 78 86 78 90 72 V68 C86 74 78 74 74 68 C70 74 62 74 58 68 C54 74 46 74 42 68 C38 74 30 74 26 68 C22 74 16 74 10 68 Z")
        .done()
}

pub(super) fn wine_glass() -> String {
    P::new()
        .raw("M26 6 H74 C76 30 70 50 50 52 C30 50 24 30 26 6 Z")
        .rect(47.0, 52.0, 6.0, 30.0, false)
        .rrect(30.0, 82.0, 40.0, 8.0, 4.0, false)
        .done()
}

// ---------------------------------------------------------------- nature

pub(super) fn leaf() -> String {
    P::new().raw("M12 88 C12 42 40 12 90 10 C88 60 60 88 12 88 Z").bar((24.0, 76.0), (70.0, 30.0), 3.0, true).done()
}

pub(super) fn flower() -> String {
    let mut p = P::new();
    for i in 0..6 {
        let c = at(50.0, 50.0, 28.0, f64::from(i) * 60.0);
        p = p.circle(c.0, c.1, 13.5, false);
    }
    p.circle(50.0, 50.0, 12.0, false).done()
}

pub(super) fn tree() -> String {
    P::new()
        .poly(
            &[
                (50.0, 4.0),
                (74.0, 32.0),
                (63.0, 32.0),
                (82.0, 56.0),
                (70.0, 56.0),
                (90.0, 80.0),
                (10.0, 80.0),
                (30.0, 56.0),
                (18.0, 56.0),
                (37.0, 32.0),
                (26.0, 32.0),
            ],
            false,
        )
        .rect(44.0, 80.0, 12.0, 16.0, false)
        .done()
}

pub(super) fn paw() -> String {
    P::new()
        .raw("M50 52 C66 52 80 66 80 78 C80 88 72 92 64 92 C58 92 54 88 50 88 C46 88 42 92 36 92 C28 92 20 88 20 78 C20 66 34 52 50 52 Z")
        .ellipse(18.0, 44.0, 9.0, 12.0, false)
        .ellipse(38.0, 22.0, 10.0, 13.0, false)
        .ellipse(62.0, 22.0, 10.0, 13.0, false)
        .ellipse(82.0, 44.0, 9.0, 12.0, false)
        .done()
}

pub(super) fn snowflake() -> String {
    // One arm pointing up, with a pair of side branches; turned six times round the centre.
    let arm: Vec<Pt> = [
        (-3.5, -11.0),
        (-3.5, -24.0),
        (-13.0, -33.5),
        (-10.5, -36.0),
        (-3.5, -29.0),
        (-3.5, -44.0),
        (3.5, -44.0),
        (3.5, -29.0),
        (10.5, -36.0),
        (13.0, -33.5),
        (3.5, -24.0),
        (3.5, -11.0),
    ]
    .iter()
    .map(|(x, y)| (50.0 + x, 50.0 + y))
    .collect();
    let mut p = P::new().circle(50.0, 50.0, 7.0, false);
    for i in 0..6 {
        p = p.poly(&rotate(&arm, (50.0, 50.0), f64::from(i) * 60.0), false);
    }
    p.done()
}

pub(super) fn umbrella() -> String {
    P::new()
        .raw("M6 46 C8 22 28 8 50 8 C72 8 92 22 94 46 C88 42 80 42 72 46 C66 42 58 42 50 46 C42 42 34 42 28 46 C20 42 12 42 6 46 Z")
        .raw("M47 49 H53 V80 C53 88 47 94 39 94 C32 94 27 89 27 83 H33 C33 86 35 88 39 88 C43 88 47 85 47 80 Z")
        .done()
}

// ---------------------------------------------------------------- travel

pub(super) fn car() -> String {
    P::new()
        .raw(
            "M8 62 V50 C8 46 10 43 14 42 L24 40 L34 24 C36 21 39 20 42 20 H64 C67 20 70 22 72 25 L80 40 L88 42 C91 43 92 46 92 50 V62 H86 A14 14 0 0 0 58 62 H42 A14 14 0 0 0 14 62 Z",
        )
        .poly(&[(38.0, 27.0), (47.0, 27.0), (47.0, 40.0), (30.0, 40.0)], true)
        .poly(&[(51.0, 27.0), (64.0, 27.0), (72.0, 40.0), (51.0, 40.0)], true)
        .circle(28.0, 63.0, 11.0, false)
        .circle(72.0, 63.0, 11.0, false)
        .done()
}

pub(super) fn plane() -> String {
    P::new()
        .poly(
            &[
                (50.0, 4.0),
                (56.0, 12.0),
                (56.0, 38.0),
                (94.0, 58.0),
                (94.0, 66.0),
                (56.0, 56.0),
                (56.0, 80.0),
                (68.0, 88.0),
                (68.0, 95.0),
                (50.0, 91.0),
                (32.0, 95.0),
                (32.0, 88.0),
                (44.0, 80.0),
                (44.0, 56.0),
                (6.0, 66.0),
                (6.0, 58.0),
                (44.0, 38.0),
                (44.0, 12.0),
            ],
            false,
        )
        .done()
}

pub(super) fn bike() -> String {
    let outer = [(42.0, 36.0), (72.0, 36.0), (55.0, 64.0)];
    let c = ((42.0 + 72.0 + 55.0) / 3.0, (36.0 + 36.0 + 64.0) / 3.0);
    let inner: Vec<Pt> = outer.iter().map(|(x, y)| (c.0 + (x - c.0) * 0.45, c.1 + (y - c.1) * 0.45)).collect();
    P::new()
        .ring(22.0, 68.0, 19.0, 12.0)
        .ring(78.0, 68.0, 19.0, 12.0)
        .circle(22.0, 68.0, 4.0, false)
        .circle(78.0, 68.0, 4.0, false)
        .poly(&outer, false)
        .poly(&inner, true)
        .rect(41.0, 29.0, 5.0, 7.0, false)
        .rrect(33.0, 24.0, 18.0, 5.0, 2.5, false)
        .rect(67.0, 25.5, 5.0, 10.5, false)
        .rrect(64.0, 20.0, 18.0, 5.0, 2.5, false)
        .done()
}

// ---------------------------------------------------------------- celebration

pub(super) fn balloon() -> String {
    P::new()
        .raw("M50 4 C70 4 82 20 82 38 C82 58 66 72 50 72 C34 72 18 58 18 38 C18 20 30 4 50 4 Z")
        .poly(&[(44.0, 79.0), (56.0, 79.0), (50.0, 72.0)], false)
        .raw("M49 79 H51 C52 84 48 88 50 96 H48 C46 88 50 84 49 79 Z")
        .sector(50.0, 38.0, 20.0, 26.0, 290.0, 330.0, true)
        .done()
}

pub(super) fn confetti() -> String {
    let mut p = P::new();
    let rects: [(f64, f64, f64); 5] = [(14.0, 18.0, 30.0), (66.0, 12.0, -20.0), (84.0, 52.0, 45.0), (30.0, 80.0, -35.0), (54.0, 46.0, 15.0)];
    for (x, y, r) in rects {
        let pts = [(x - 7.0, y - 3.5), (x + 7.0, y - 3.5), (x + 7.0, y + 3.5), (x - 7.0, y + 3.5)];
        p = p.poly(&rotate(&pts, (x, y), r), false);
    }
    for (x, y, r) in [(40.0, 22.0, 5.0), (88.0, 26.0, 4.0), (12.0, 54.0, 5.0), (70.0, 84.0, 5.0), (34.0, 50.0, 3.5)] {
        p = p.circle(x, y, r, false);
    }
    for (x, y, r) in [(62.0, 66.0, 20.0), (16.0, 88.0, -10.0), (86.0, 88.0, 40.0)] {
        let pts = [(x, y - 6.0), (x + 6.0, y + 5.0), (x - 6.0, y + 5.0)];
        p = p.poly(&rotate(&pts, (x, y), r), false);
    }
    p.done()
}

fn sparkle_at(p: P, cx: f64, cy: f64, r: f64) -> P {
    let k = r * 0.12;
    let d = format!(
        "M{cx} {} C{} {} {} {} {} {cy} C{} {} {} {} {cx} {} C{} {} {} {} {} {cy} C{} {} {} {} {cx} {} Z",
        cy - r,
        cx + k,
        cy - k,
        cx + k,
        cy - k,
        cx + r,
        cx + k,
        cy + k,
        cx + k,
        cy + k,
        cy + r,
        cx - k,
        cy + k,
        cx - k,
        cy + k,
        cx - r,
        cx - k,
        cy - k,
        cx - k,
        cy - k,
        cy - r
    );
    p.raw(&d)
}

pub(super) fn sparkles() -> String {
    let p = sparkle_at(P::new(), 40.0, 56.0, 36.0);
    let p = sparkle_at(p, 80.0, 20.0, 15.0);
    sparkle_at(p, 82.0, 78.0, 11.0).done()
}

// ---------------------------------------------------------------- badges and decorative

pub(super) fn rosette() -> String {
    let n = 18;
    let (cx, cy, r) = (50.0, 50.0, 40.0);
    let step = 360.0 / f64::from(n);
    let bump = 2.0 * r * (step / 2.0).to_radians().sin() / 2.0 + 0.5;
    let start = at(cx, cy, r, 0.0);
    let mut d = format!("M{:.2} {:.2}", start.0, start.1);
    for i in 1..=n {
        let p = at(cx, cy, r, f64::from(i) * step);
        d.push_str(&format!(" A{bump:.2} {bump:.2} 0 0 1 {:.2} {:.2}", p.0, p.1));
    }
    d.push_str(" Z");
    P::new().raw(&d).circle(cx, cy, 30.0, true).circle(cx, cy, 25.0, false).done()
}

pub(super) fn ribbon_banner() -> String {
    P::new()
        .poly(
            &[
                (16.0, 26.0),
                (84.0, 26.0),
                (84.0, 36.0),
                (98.0, 36.0),
                (90.0, 52.0),
                (98.0, 68.0),
                (72.0, 68.0),
                (72.0, 60.0),
                (28.0, 60.0),
                (28.0, 68.0),
                (2.0, 68.0),
                (10.0, 52.0),
                (2.0, 36.0),
                (16.0, 36.0),
            ],
            false,
        )
        .poly(&[(72.0, 60.0), (84.0, 60.0), (72.0, 66.0)], true)
        .poly(&[(16.0, 60.0), (28.0, 60.0), (28.0, 66.0)], true)
        .done()
}

/// A smooth blob through points at `radii` (evenly spaced angles) round the centre.
fn blob(radii: &[f64], turn: f64) -> String {
    let step = 360.0 / radii.len() as f64;
    let pts: Vec<Pt> = radii.iter().enumerate().map(|(i, r)| at(50.0, 50.0, *r, turn + i as f64 * step)).collect();
    P::new().smooth(&pts, false).done()
}

pub(super) fn blob_1() -> String {
    blob(&[42.0, 34.0, 44.0, 38.0, 42.0, 30.0, 40.0, 36.0], 0.0)
}

pub(super) fn blob_2() -> String {
    blob(&[44.0, 28.0, 40.0, 42.0, 30.0, 44.0], 15.0)
}

pub(super) fn blob_3() -> String {
    blob(&[40.0, 44.0, 36.0, 44.0, 32.0, 42.0, 38.0, 44.0, 34.0, 42.0], 0.0)
}

pub(super) fn wave_divider() -> String {
    P::new().raw("M0 40 C17 22 33 22 50 40 C67 58 83 58 100 40 V80 H0 Z").done()
}

pub(super) fn scallop_divider() -> String {
    let mut d = String::from("M0 30 H100 V62");
    for i in (0..5).rev() {
        let x = f64::from(i) * 20.0;
        d.push_str(&format!(" A10 10 0 0 1 {x} 62"));
    }
    d.push_str(" Z");
    P::new().raw(&d).done()
}

pub(super) fn brush_stroke() -> String {
    // A rough-edged band; the jitter is a fixed table so the shape never changes.
    let top = [30.0, 27.0, 29.0, 25.0, 28.0, 26.0, 29.0, 26.0, 28.0, 25.0, 27.0, 30.0];
    let bottom = [70.0, 73.0, 71.0, 74.0, 70.0, 73.0, 72.0, 75.0, 71.0, 74.0, 72.0, 69.0];
    let steps = top.len() - 1;
    let x = |i: usize| 6.0 + 88.0 * i as f64 / steps as f64;
    let mut pts: Vec<Pt> = vec![(2.0, 34.0)];
    pts.extend(top.iter().enumerate().map(|(i, y)| (x(i), *y)));
    pts.extend([(97.0, 40.0), (94.0, 50.0), (98.0, 60.0)]);
    pts.extend(bottom.iter().enumerate().rev().map(|(i, y)| (x(i), *y)));
    pts.extend([(3.0, 64.0), (6.0, 50.0)]);
    P::new().poly(&pts, false).done()
}

pub(super) fn frame_square() -> String {
    P::new().rrect(4.0, 4.0, 92.0, 92.0, 4.0, false).rrect(12.0, 12.0, 76.0, 76.0, 2.0, true).done()
}

pub(super) fn frame_circle() -> String {
    P::new().ring(50.0, 50.0, 46.0, 39.0).done()
}

pub(super) fn frame_stamp() -> String {
    // A postage-stamp edge: straight runs with half-round bites, then a window.
    let (x0, y0, x1, y1, r) = (4.0, 4.0, 96.0, 96.0, 3.0);
    let bites = 8;
    let step = (x1 - x0) / f64::from(bites);
    let mut d = format!("M{x0} {y0}");
    // Top: left to right.
    for i in 0..bites {
        let c = x0 + step * (f64::from(i) + 0.5);
        d.push_str(&format!(" L{:.2} {y0} A{r} {r} 0 0 0 {:.2} {y0}", c - r, c + r));
    }
    d.push_str(&format!(" L{x1} {y0}"));
    for i in 0..bites {
        let c = y0 + step * (f64::from(i) + 0.5);
        d.push_str(&format!(" L{x1} {:.2} A{r} {r} 0 0 0 {x1} {:.2}", c - r, c + r));
    }
    d.push_str(&format!(" L{x1} {y1}"));
    for i in (0..bites).rev() {
        let c = x0 + step * (f64::from(i) + 0.5);
        d.push_str(&format!(" L{:.2} {y1} A{r} {r} 0 0 0 {:.2} {y1}", c + r, c - r));
    }
    d.push_str(&format!(" L{x0} {y1}"));
    for i in (0..bites).rev() {
        let c = y0 + step * (f64::from(i) + 0.5);
        d.push_str(&format!(" L{x0} {:.2} A{r} {r} 0 0 0 {x0} {:.2}", c + r, c - r));
    }
    d.push_str(" Z");
    P::new().raw(&d).rect(14.0, 14.0, 72.0, 72.0, true).done()
}
