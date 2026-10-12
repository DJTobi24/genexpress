//! The shape and icon library. Icons are original drawings, filled outlines as SVG path data in a
//! 100 × 100 view box. Holes (the play button's triangle, the pin's dot…) run opposite to their
//! outline, so they show with both the non-zero and the even-odd fill rule; separate parts never
//! overlap.

use std::fmt::Write as _;

use genha_doc::ShapeKind;

mod icons;
pub(crate) mod path;

const ARROWS: &str = "Arrows";
const UI: &str = "Interface";
const PEOPLE: &str = "People & messages";
const MEDIA: &str = "Media";
const BUSINESS: &str = "Business & shopping";
const FOOD: &str = "Food & drink";
const NATURE: &str = "Nature & weather";
const TRAVEL: &str = "Travel & places";
const PARTY: &str = "Celebration";
const BADGES: &str = "Badges & awards";
const DECO: &str = "Decorative";

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

// Icons with fixed path data.
const HEART: &str = "M50 88 C20 66 6 50 6 32 C6 18 17 8 30 8 C39 8 46 13 50 20 C54 13 61 8 70 8 C83 8 94 18 94 32 C94 50 80 66 50 88 Z";
const ARROW_RIGHT: &str = "M6 38 H58 V16 L94 50 L58 84 V62 H6 Z";
const ARROW_LEFT: &str = "M94 38 H42 V16 L6 50 L42 84 V62 H94 Z";
const ARROW_UP: &str = "M38 94 V42 H16 L50 6 L84 42 H62 V94 Z";
const ARROW_DOWN: &str = "M38 6 V58 H16 L50 94 L84 58 H62 V6 Z";
const CHECK: &str = "M8 52 L22 38 L40 56 L78 18 L92 32 L40 84 Z";
const CROSS: &str = "M22 8 L50 36 L78 8 L92 22 L64 50 L92 78 L78 92 L50 64 L22 92 L8 78 L36 50 L8 22 Z";
const PLUS: &str = "M38 8 H62 V38 H92 V62 H62 V92 H38 V62 H8 V38 H38 Z";
const MINUS: &str = "M8 38 H92 V62 H8 Z";
const SPEECH_BUBBLE: &str = "M18 10 H82 C89 10 94 15 94 22 V62 C94 69 89 74 82 74 H44 L24 92 L28 74 H18 C11 74 6 69 6 62 V22 C6 15 11 10 18 10 Z";
const LIGHTNING: &str = "M60 4 L18 56 L44 56 L36 96 L82 40 L56 40 L64 4 Z";
const CLOUD: &str = "M28 78 C14 78 6 69 6 58 C6 47 14 39 25 38 C27 25 37 16 50 16 C62 16 71 24 74 35 C86 35 94 45 94 56 C94 68 85 78 72 78 Z";
const MOON: &str = "M64 8 C40 12 24 30 24 52 C24 76 44 94 66 94 C76 94 86 90 92 84 C64 84 46 66 46 44 C46 28 54 16 64 8 Z";
const MUSIC_NOTE: &str = "M56 14 L84 24 L84 38 L64 31 L64 74 C64 83 55 90 44 90 C33 90 26 84 26 76 C26 68 35 62 46 62 C50 62 53 63 56 64 Z";
const ENVELOPE: &str = "M6 18 H94 V82 H6 Z M14 26 L14 34 L50 62 L86 34 L86 26 L50 54 Z";
const PHONE: &str = "M28 8 L40 8 C43 8 45 10 46 13 L50 28 C51 31 50 34 47 36 L40 41 C45 52 52 59 62 64 L67 57 C69 54 72 53 75 54 L89 58 C92 59 94 61 94 64 L94 76 C94 86 86 94 76 94 C40 92 8 60 6 24 C6 14 14 8 24 8 Z";
const HOME: &str = "M50 8 L94 46 L84 46 L84 92 L60 92 L60 64 L40 64 L40 92 L16 92 L16 46 L6 46 Z";
const BELL: &str = "M50 6 C54 6 57 9 57 13 C70 17 78 28 78 42 V62 L88 74 V80 H12 V74 L22 62 V42 C22 28 30 17 43 13 C43 9 46 6 50 6 Z M40 84 H60 C60 90 55 94 50 94 C45 94 40 90 40 84 Z";
const SPARKLE: &str = "M50 4 C54 34 66 46 96 50 C66 54 54 66 50 96 C46 66 34 54 4 50 C34 46 46 34 50 4 Z";
const SHIELD: &str = "M50 4 L88 18 V46 C88 70 72 86 50 96 C28 86 12 70 12 46 V18 Z";

/// Every shape in the library: basic shapes first, then icons grouped by category (the panel shows
/// the categories in this order).
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
    let icons: Vec<(&'static str, &'static str, &'static str, String)> = vec![
        // Arrows
        ("arrow-right", "Arrow right", ARROWS, ARROW_RIGHT.into()),
        ("arrow-left", "Arrow left", ARROWS, ARROW_LEFT.into()),
        ("arrow-up", "Arrow up", ARROWS, ARROW_UP.into()),
        ("arrow-down", "Arrow down", ARROWS, ARROW_DOWN.into()),
        ("arrow-diagonal", "Arrow up right", ARROWS, icons::arrow_diagonal()),
        ("arrow-turn", "Turn arrow", ARROWS, icons::arrow_turn()),
        ("arrows-swap", "Swap arrows", ARROWS, icons::arrows_swap()),
        ("chevron-right", "Chevron right", ARROWS, icons::chevron(0.0)),
        ("chevron-left", "Chevron left", ARROWS, icons::chevron(180.0)),
        ("chevron-up", "Chevron up", ARROWS, icons::chevron(-90.0)),
        ("chevron-down", "Chevron down", ARROWS, icons::chevron(90.0)),
        // Interface
        ("check", "Check mark", UI, CHECK.into()),
        ("cross", "Cross", UI, CROSS.into()),
        ("plus", "Plus", UI, PLUS.into()),
        ("minus", "Minus", UI, MINUS.into()),
        ("check-circle", "Check in a circle", UI, icons::check_circle()),
        ("info", "Information", UI, icons::info()),
        ("warning", "Warning", UI, icons::warning()),
        ("question", "Question", UI, icons::question()),
        ("search", "Search", UI, icons::search()),
        ("settings", "Settings gear", UI, icons::gear()),
        ("home", "Home", UI, HOME.into()),
        ("bell", "Bell", UI, BELL.into()),
        ("calendar", "Calendar", UI, icons::calendar()),
        ("clock", "Clock", UI, icons::clock()),
        ("lock", "Lock", UI, icons::lock()),
        ("key", "Key", UI, icons::key()),
        ("bookmark", "Bookmark", UI, icons::bookmark()),
        ("download", "Download", UI, icons::download()),
        ("upload", "Upload", UI, icons::upload()),
        ("wifi", "Wi-Fi", UI, icons::wifi()),
        ("battery", "Battery", UI, icons::battery()),
        ("eye", "Eye", UI, icons::eye()),
        // People and communication
        ("user", "Person", PEOPLE, icons::user()),
        ("users", "People", PEOPLE, icons::users()),
        ("heart", "Heart", PEOPLE, HEART.into()),
        ("thumbs-up", "Thumbs up", PEOPLE, icons::thumbs_up()),
        ("smile", "Smiling face", PEOPLE, icons::smile()),
        ("hand-wave", "Waving hand", PEOPLE, icons::hand_wave()),
        ("speech-bubble", "Speech bubble", PEOPLE, SPEECH_BUBBLE.into()),
        ("chat", "Conversation", PEOPLE, icons::chat()),
        ("quote-marks", "Quote marks", PEOPLE, icons::quote_marks()),
        ("share", "Share", PEOPLE, icons::share()),
        ("hashtag", "Hashtag", PEOPLE, icons::hashtag()),
        ("at-sign", "At sign", PEOPLE, icons::at_sign()),
        ("megaphone", "Megaphone", PEOPLE, icons::megaphone()),
        ("paper-plane", "Paper plane", PEOPLE, icons::paper_plane()),
        ("envelope", "Envelope", PEOPLE, ENVELOPE.into()),
        ("phone", "Phone", PEOPLE, PHONE.into()),
        ("location-pin", "Location pin", PEOPLE, pin()),
        // Media
        ("play-button", "Play button", MEDIA, play()),
        ("music-note", "Music note", MEDIA, MUSIC_NOTE.into()),
        ("camera", "Camera", MEDIA, icons::camera()),
        ("image", "Picture", MEDIA, icons::image()),
        ("video", "Video camera", MEDIA, icons::video()),
        ("microphone", "Microphone", MEDIA, icons::microphone()),
        ("headphones", "Headphones", MEDIA, icons::headphones()),
        ("book", "Open book", MEDIA, icons::book()),
        // Business
        ("briefcase", "Briefcase", BUSINESS, icons::briefcase()),
        ("building", "Office building", BUSINESS, icons::building()),
        ("chart-bars", "Bar chart", BUSINESS, icons::chart_bars()),
        ("chart-pie", "Pie chart", BUSINESS, icons::chart_pie()),
        ("lightbulb", "Light bulb", BUSINESS, icons::lightbulb()),
        ("rocket", "Rocket", BUSINESS, icons::rocket()),
        ("shopping-bag", "Shopping bag", BUSINESS, icons::shopping_bag()),
        ("cart", "Shopping cart", BUSINESS, icons::cart()),
        ("tag", "Price tag", BUSINESS, icons::tag()),
        ("percent", "Percent", BUSINESS, icons::percent()),
        ("dollar-coin", "Dollar coin", BUSINESS, icons::dollar_coin()),
        ("euro-coin", "Euro coin", BUSINESS, icons::euro_coin()),
        ("graduation-cap", "Graduation cap", BUSINESS, icons::graduation_cap()),
        // Food and drink
        ("coffee", "Coffee cup", FOOD, icons::coffee()),
        ("pizza", "Pizza slice", FOOD, icons::pizza()),
        ("cake", "Cake", FOOD, icons::cake()),
        ("wine-glass", "Wine glass", FOOD, icons::wine_glass()),
        // Nature and weather
        ("leaf", "Leaf", NATURE, icons::leaf()),
        ("flower", "Flower", NATURE, icons::flower()),
        ("tree", "Tree", NATURE, icons::tree()),
        ("paw", "Paw print", NATURE, icons::paw()),
        ("sun", "Sun", NATURE, sun()),
        ("cloud", "Cloud", NATURE, CLOUD.into()),
        ("moon", "Moon", NATURE, MOON.into()),
        ("lightning", "Lightning", NATURE, LIGHTNING.into()),
        ("snowflake", "Snowflake", NATURE, icons::snowflake()),
        ("umbrella", "Umbrella", NATURE, icons::umbrella()),
        // Travel and places
        ("car", "Car", TRAVEL, icons::car()),
        ("plane", "Plane", TRAVEL, icons::plane()),
        ("bike", "Bicycle", TRAVEL, icons::bike()),
        ("map", "Map", TRAVEL, icons::map()),
        ("globe", "Globe", TRAVEL, icons::globe()),
        // Celebration
        ("gift", "Gift", PARTY, icons::gift()),
        ("balloon", "Balloon", PARTY, icons::balloon()),
        ("crown", "Crown", PARTY, icons::crown()),
        ("confetti", "Confetti", PARTY, icons::confetti()),
        ("sparkle", "Sparkle", PARTY, SPARKLE.into()),
        ("sparkles", "Sparkles", PARTY, icons::sparkles()),
        // Badges and awards
        ("badge", "Award badge", BADGES, medal()),
        ("seal", "Sale seal", BADGES, burst()),
        ("shield", "Shield", BADGES, SHIELD.into()),
        ("shield-check", "Shield with check", BADGES, icons::shield_check()),
        ("trophy", "Trophy", BADGES, icons::trophy()),
        ("award-star", "Star medal", BADGES, icons::award_star()),
        ("flag", "Flag", BADGES, icons::flag()),
        ("rosette", "Rosette", BADGES, icons::rosette()),
        ("ribbon-banner", "Ribbon banner", BADGES, icons::ribbon_banner()),
        // Decorative
        ("blob-1", "Blob 1", DECO, icons::blob_1()),
        ("blob-2", "Blob 2", DECO, icons::blob_2()),
        ("blob-3", "Blob 3", DECO, icons::blob_3()),
        ("wave-divider", "Wave divider", DECO, icons::wave_divider()),
        ("scallop-divider", "Scalloped divider", DECO, icons::scallop_divider()),
        ("brush-stroke", "Brush stroke", DECO, icons::brush_stroke()),
        ("frame-square", "Square frame", DECO, icons::frame_square()),
        ("frame-circle", "Round frame", DECO, icons::frame_circle()),
        ("frame-stamp", "Stamp frame", DECO, icons::frame_stamp()),
    ];
    v.extend(icons.into_iter().map(|(id, label, cat, d)| s(id, label, cat, path(d))));
    v
}

/// The library shape with id `id`.
pub(crate) fn library_shape(id: &str) -> Option<ShapeKind> {
    shape_library().into_iter().find(|s| s.id == id).map(|s| s.shape)
}
