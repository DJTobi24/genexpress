//! Page size presets for common destinations. Sizes are the platforms' published recommendations
//! (pixels) or standard paper sizes at 96 px per inch (print exports scale up to 300 dpi).

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct PagePreset {
    pub id: &'static str,
    pub label: &'static str,
    pub category: &'static str,
    pub width: f64,
    pub height: f64,
}

const fn p(id: &'static str, label: &'static str, category: &'static str, width: f64, height: f64) -> PagePreset {
    PagePreset { id, label, category, width, height }
}

/// Paper inches → pixels at 96 px/in.
const IN: f64 = 96.0;
/// Millimetres → pixels at 96 px/in.
const MM: f64 = 96.0 / 25.4;

pub const PRESETS: &[PagePreset] = &[
    // Social
    p("instagram-post", "Instagram post (square)", "Social", 1080.0, 1080.0),
    p("instagram-portrait", "Instagram post (portrait)", "Social", 1080.0, 1350.0),
    p("instagram-story", "Instagram story / reel", "Social", 1080.0, 1920.0),
    p("facebook-post", "Facebook post", "Social", 1200.0, 630.0),
    p("facebook-cover", "Facebook cover", "Social", 1640.0, 624.0),
    p("facebook-story", "Facebook story", "Social", 1080.0, 1920.0),
    p("tiktok-video", "TikTok video", "Social", 1080.0, 1920.0),
    p("x-post", "X post", "Social", 1600.0, 900.0),
    p("x-header", "X header", "Social", 1500.0, 500.0),
    p("linkedin-post", "LinkedIn post", "Social", 1200.0, 627.0),
    p("linkedin-banner", "LinkedIn banner", "Social", 1584.0, 396.0),
    p("pinterest-pin", "Pinterest pin", "Social", 1000.0, 1500.0),
    p("youtube-thumbnail", "YouTube thumbnail", "Social", 1280.0, 720.0),
    p("youtube-banner", "YouTube channel banner", "Social", 2560.0, 1440.0),
    // Video
    p("video-landscape", "Video 16:9 (Full HD)", "Video", 1920.0, 1080.0),
    p("video-portrait", "Video 9:16", "Video", 1080.0, 1920.0),
    p("video-square", "Video 1:1", "Video", 1080.0, 1080.0),
    // Print
    p("flyer-a4", "Flyer A4", "Print", 210.0 * MM, 297.0 * MM),
    p("flyer-a5", "Flyer A5", "Print", 148.0 * MM, 210.0 * MM),
    p("flyer-letter", "Flyer US Letter", "Print", 8.5 * IN, 11.0 * IN),
    p("poster-a3", "Poster A3", "Print", 297.0 * MM, 420.0 * MM),
    p("poster-18x24", "Poster 18×24 in", "Print", 18.0 * IN, 24.0 * IN),
    p("business-card", "Business card 85×55 mm", "Print", 85.0 * MM, 55.0 * MM),
    p("business-card-us", "Business card 3.5×2 in", "Print", 3.5 * IN, 2.0 * IN),
    p("invitation-5x7", "Invitation 5×7 in", "Print", 5.0 * IN, 7.0 * IN),
    p("postcard", "Postcard A6", "Print", 148.0 * MM, 105.0 * MM),
    // Marketing
    p("logo", "Logo", "Marketing", 500.0, 500.0),
    p("banner-leaderboard", "Web banner 728×90", "Marketing", 728.0, 90.0),
    p("banner-rectangle", "Web banner 300×250", "Marketing", 300.0, 250.0),
    p("email-header", "Email header", "Marketing", 600.0, 200.0),
    // Documents
    p("presentation", "Presentation 16:9", "Documents", 1920.0, 1080.0),
    p("presentation-4x3", "Presentation 4:3", "Documents", 1024.0, 768.0),
    p("document-a4", "Document A4", "Documents", 210.0 * MM, 297.0 * MM),
    p("resume", "Resume (A4)", "Documents", 210.0 * MM, 297.0 * MM),
    p("webpage", "Web page", "Documents", 1440.0, 2400.0),
];

pub fn find(id: &str) -> Option<&'static PagePreset> {
    PRESETS.iter().find(|p| p.id == id)
}

/// Categories in display order.
pub fn categories() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = vec![];
    for p in PRESETS {
        if !v.contains(&p.category) {
            v.push(p.category);
        }
    }
    v
}
