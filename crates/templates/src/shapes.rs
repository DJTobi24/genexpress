//! The shape and icon library. Icons are original drawings, filled outlines as SVG path data in a
//! 100 × 100 view box. Holes (the play button's triangle, the pin's dot…) run opposite to their
//! outline, so they show with both the non-zero and the even-odd fill rule; separate parts never
//! overlap.

use std::fmt::Write as _;

use genexpress_doc::ShapeKind;

/// One entry of the shape panel.
#[derive(Clone, Debug, PartialEq)]
pub struct LibraryShape {
    pub id: &'static str,
    pub label: &'static str,
    pub category: &'static str,
    pub shape: ShapeKind,
}

const VIEW: f64 = 100.0;

fn path(d: impl Into<String>) -> ShapeKind {
    ShapeKind::Path { d: d.into(), view_width: VIEW, view_height: VIEW }
}

/// A circle as two arcs; `clockwise` as seen on screen (y down).
fn circle(cx: f64, cy: f64, r: f64, clockwise: bool) -> String {
    let s = u8::from(clockwise);
    format!("M{} {cy} A{r} {r} 0 1 {s} {} {cy} A{r} {r} 0 1 {s} {} {cy} Z", cx + r, cx - r, cx + r)
}

/// A closed polygon through `points`.
fn poly(points: &[(f64, f64)]) -> String {
    let mut d = String::new();
    for (i, (x, y)) in points.iter().enumerate() {
        let _ = write!(d, "{}{:.2} {:.2} ", if i == 0 { 'M' } else { 'L' }, x, y);
    }
    d.push('Z');
    d
}

fn polar(r: f64, deg: f64) -> (f64, f64) {
    let a = (deg - 90.0).to_radians();
    (50.0 + r * a.cos(), 50.0 + r * a.sin())
}

fn sun() -> String {
    let mut d = circle(50.0, 50.0, 18.0, true);
    for i in 0..8 {
        let a = f64::from(i) * 45.0;
        d.push(' ');
        d.push_str(&poly(&[polar(46.0, a), polar(26.0, a + 12.0), polar(26.0, a - 12.0)]));
    }
    d
}

fn burst() -> String {
    let n = 16;
    let pts: Vec<(f64, f64)> = (0..n * 2).map(|i| polar(if i % 2 == 0 { 48.0 } else { 39.0 }, f64::from(i) * 180.0 / f64::from(n))).collect();
    poly(&pts)
}

fn play() -> String {
    format!("{} M40 30 L40 70 L72 50 Z", circle(50.0, 50.0, 44.0, true))
}

fn pin() -> String {
    format!("M50 94 C50 94 16 58 16 36 C16 17 31 4 50 4 C69 4 84 17 84 36 C84 58 50 94 50 94 Z {}", circle(50.0, 36.0, 13.0, false))
}

fn medal() -> String {
    format!("{} M32 62 L44 68 L38 96 L30 88 L20 92 Z M68 62 L80 92 L70 88 L62 96 L56 68 Z", circle(50.0, 36.0, 28.0, true))
}

/// Icons with fixed path data.
const ICONS: &[(&str, &str, &str, &str)] = &[
    (
        "heart",
        "Heart",
        "Symbols",
        "M50 88 C20 66 6 50 6 32 C6 18 17 8 30 8 C39 8 46 13 50 20 C54 13 61 8 70 8 C83 8 94 18 94 32 C94 50 80 66 50 88 Z",
    ),
    ("arrow-right", "Arrow right", "Arrows", "M6 38 H58 V16 L94 50 L58 84 V62 H6 Z"),
    ("arrow-left", "Arrow left", "Arrows", "M94 38 H42 V16 L6 50 L42 84 V62 H94 Z"),
    ("arrow-up", "Arrow up", "Arrows", "M38 94 V42 H16 L50 6 L84 42 H62 V94 Z"),
    ("arrow-down", "Arrow down", "Arrows", "M38 6 V58 H16 L50 94 L84 58 H62 V6 Z"),
    ("check", "Check mark", "Symbols", "M8 52 L22 38 L40 56 L78 18 L92 32 L40 84 Z"),
    ("cross", "Cross", "Symbols", "M22 8 L50 36 L78 8 L92 22 L64 50 L92 78 L78 92 L50 64 L22 92 L8 78 L36 50 L8 22 Z"),
    ("plus", "Plus", "Symbols", "M38 8 H62 V38 H92 V62 H62 V92 H38 V62 H8 V38 H38 Z"),
    ("minus", "Minus", "Symbols", "M8 38 H92 V62 H8 Z"),
    (
        "speech-bubble",
        "Speech bubble",
        "Symbols",
        "M18 10 H82 C89 10 94 15 94 22 V62 C94 69 89 74 82 74 H44 L24 92 L28 74 H18 C11 74 6 69 6 62 V22 C6 15 11 10 18 10 Z",
    ),
    ("lightning", "Lightning", "Symbols", "M60 4 L18 56 L44 56 L36 96 L82 40 L56 40 L64 4 Z"),
    (
        "cloud",
        "Cloud",
        "Weather",
        "M28 78 C14 78 6 69 6 58 C6 47 14 39 25 38 C27 25 37 16 50 16 C62 16 71 24 74 35 C86 35 94 45 94 56 C94 68 85 78 72 78 Z",
    ),
    ("moon", "Moon", "Weather", "M64 8 C40 12 24 30 24 52 C24 76 44 94 66 94 C76 94 86 90 92 84 C64 84 46 66 46 44 C46 28 54 16 64 8 Z"),
    (
        "music-note",
        "Music note",
        "Symbols",
        "M56 14 L84 24 L84 38 L64 31 L64 74 C64 83 55 90 44 90 C33 90 26 84 26 76 C26 68 35 62 46 62 C50 62 53 63 56 64 Z",
    ),
    ("envelope", "Envelope", "Contact", "M6 18 H94 V82 H6 Z M14 26 L14 34 L50 62 L86 34 L86 26 L50 54 Z"),
    (
        "phone",
        "Phone",
        "Contact",
        "M28 8 L40 8 C43 8 45 10 46 13 L50 28 C51 31 50 34 47 36 L40 41 C45 52 52 59 62 64 L67 57 C69 54 72 53 75 54 L89 58 C92 59 94 61 94 64 L94 76 C94 86 86 94 76 94 C40 92 8 60 6 24 C6 14 14 8 24 8 Z",
    ),
    ("home", "Home", "Symbols", "M50 8 L94 46 L84 46 L84 92 L60 92 L60 64 L40 64 L40 92 L16 92 L16 46 L6 46 Z"),
    (
        "bell",
        "Bell",
        "Symbols",
        "M50 6 C54 6 57 9 57 13 C70 17 78 28 78 42 V62 L88 74 V80 H12 V74 L22 62 V42 C22 28 30 17 43 13 C43 9 46 6 50 6 Z M40 84 H60 C60 90 55 94 50 94 C45 94 40 90 40 84 Z",
    ),
    ("sparkle", "Sparkle", "Symbols", "M50 4 C54 34 66 46 96 50 C66 54 54 66 50 96 C46 66 34 54 4 50 C34 46 46 34 50 4 Z"),
    ("shield", "Shield", "Badges", "M50 4 L88 18 V46 C88 70 72 86 50 96 C28 86 12 70 12 46 V18 Z"),
];

/// Every shape in the library: basic shapes first, then icons.
pub fn shape_library() -> Vec<LibraryShape> {
    let s = |id, label, category, shape| LibraryShape { id, label, category, shape };
    let basic = "Basic shapes";
    let mut v = vec![
        s("rect", "Rectangle", basic, ShapeKind::Rect { corner_radius: 0.0 }),
        s("rounded-rect", "Rounded rectangle", basic, ShapeKind::Rect { corner_radius: 24.0 }),
        s("circle", "Circle", basic, ShapeKind::Ellipse),
        s("triangle", "Triangle", basic, ShapeKind::Triangle),
        s("diamond", "Diamond", basic, ShapeKind::Polygon { sides: 4 }),
        s("pentagon", "Pentagon", basic, ShapeKind::Polygon { sides: 5 }),
        s("hexagon", "Hexagon", basic, ShapeKind::Polygon { sides: 6 }),
        s("octagon", "Octagon", basic, ShapeKind::Polygon { sides: 8 }),
        s("star", "Star", basic, ShapeKind::Star { points: 5, inner_ratio: 0.45 }),
        s("line", "Line", basic, ShapeKind::Line),
    ];
    v.extend(ICONS.iter().map(|(id, label, cat, d)| s(id, label, cat, path(*d))));
    v.push(s("sun", "Sun", "Weather", path(sun())));
    v.push(s("location-pin", "Location pin", "Contact", path(pin())));
    v.push(s("play-button", "Play button", "Symbols", path(play())));
    v.push(s("badge", "Award badge", "Badges", path(medal())));
    v.push(s("seal", "Sale seal", "Badges", path(burst())));
    v
}

/// The library shape with id `id`.
pub(crate) fn library_shape(id: &str) -> Option<ShapeKind> {
    shape_library().into_iter().find(|s| s.id == id).map(|s| s.shape)
}
