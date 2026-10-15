//! Colours and spacing. Panels take colours from [`Tokens`], never literals.

use egui::{Color32, CornerRadius, Stroke, Visuals};

#[derive(Clone, Copy, Debug)]
pub struct Tokens {
    pub dark: bool,
    pub bg: Color32,
    pub panel: Color32,
    pub panel_alt: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    pub accent_soft: Color32,
    pub canvas: Color32,
    pub selection: Color32,
    pub handle_fill: Color32,
    pub guide: Color32,
    /// Distance hints on the canvas (gaps to neighbours and page edges).
    pub measure: Color32,
    pub danger: Color32,
    /// Present mode: the backdrop around the page and the text drawn on it.
    pub stage: Color32,
    pub stage_text: Color32,
    /// Home screen tiles and illustrations: (gradient from, gradient to, ink), five families
    /// (blue, violet, peach, pink, mint).
    pub tiles: [[Color32; 3]; 5],
    /// Paper in illustrations (pages, cards, photos).
    pub art_paper: Color32,
    /// Soft shadow under illustrated paper.
    pub art_shadow: Color32,
    /// Behind a thumbnail or a modal's content: the backdrop dimming the rest.
    pub scrim: Color32,
    /// Behind an object's picture in the layer strip: light for dark objects, dark for light ones
    /// (white text stays visible).
    pub thumb_light: Color32,
    pub thumb_dark: Color32,
}

impl Tokens {
    pub const LIGHT: Tokens = Tokens {
        dark: false,
        bg: Color32::from_rgb(0xf4, 0xf3, 0xf8),
        panel: Color32::from_rgb(0xff, 0xff, 0xff),
        panel_alt: Color32::from_rgb(0xf7, 0xf6, 0xfb),
        border: Color32::from_rgb(0xe2, 0xe0, 0xea),
        text: Color32::from_rgb(0x1c, 0x1b, 0x29),
        text_dim: Color32::from_rgb(0x6b, 0x69, 0x7c),
        accent: Color32::from_rgb(0x5b, 0x3c, 0xf5),
        accent_text: Color32::WHITE,
        accent_soft: Color32::from_rgb(0xec, 0xe8, 0xff),
        canvas: Color32::from_rgb(0xe9, 0xe7, 0xf0),
        selection: Color32::from_rgb(0x5b, 0x3c, 0xf5),
        handle_fill: Color32::WHITE,
        guide: Color32::from_rgb(0xff, 0x3d, 0x8b),
        measure: Color32::from_rgb(0xe8, 0x59, 0x0c),
        danger: Color32::from_rgb(0xd9, 0x30, 0x30),
        stage: Color32::BLACK,
        stage_text: Color32::from_gray(150),
        tiles: [
            [Color32::from_rgb(0xe3, 0xf1, 0xff), Color32::from_rgb(0xbc, 0xdb, 0xff), Color32::from_rgb(0x1f, 0x5f, 0xd6)],
            [Color32::from_rgb(0xef, 0xea, 0xff), Color32::from_rgb(0xd2, 0xc6, 0xff), Color32::from_rgb(0x5b, 0x3c, 0xf5)],
            [Color32::from_rgb(0xff, 0xee, 0xe2), Color32::from_rgb(0xff, 0xcd, 0xb0), Color32::from_rgb(0xd9, 0x55, 0x0b)],
            [Color32::from_rgb(0xff, 0xe8, 0xf2), Color32::from_rgb(0xff, 0xbf, 0xda), Color32::from_rgb(0xc8, 0x22, 0x6c)],
            [Color32::from_rgb(0xe0, 0xf8, 0xee), Color32::from_rgb(0xb2, 0xea, 0xd3), Color32::from_rgb(0x04, 0x84, 0x5f)],
        ],
        art_paper: Color32::WHITE,
        art_shadow: Color32::from_rgba_premultiplied(0x18, 0x10, 0x40, 0x1c),
        scrim: Color32::from_rgba_premultiplied(0x10, 0x0c, 0x24, 0x70),
        thumb_light: Color32::from_rgb(0xf7, 0xf6, 0xfb),
        thumb_dark: Color32::from_rgb(0x3a, 0x38, 0x48),
    };

    pub const DARK: Tokens = Tokens {
        dark: true,
        bg: Color32::from_rgb(0x17, 0x16, 0x1f),
        panel: Color32::from_rgb(0x21, 0x20, 0x2c),
        panel_alt: Color32::from_rgb(0x2a, 0x29, 0x37),
        border: Color32::from_rgb(0x3a, 0x38, 0x4a),
        text: Color32::from_rgb(0xee, 0xec, 0xf6),
        text_dim: Color32::from_rgb(0xa3, 0xa0, 0xb8),
        accent: Color32::from_rgb(0x8b, 0x74, 0xff),
        accent_text: Color32::WHITE,
        accent_soft: Color32::from_rgb(0x34, 0x2d, 0x5c),
        canvas: Color32::from_rgb(0x12, 0x11, 0x18),
        selection: Color32::from_rgb(0x9d, 0x89, 0xff),
        handle_fill: Color32::from_rgb(0x21, 0x20, 0x2c),
        guide: Color32::from_rgb(0xff, 0x5c, 0xa0),
        measure: Color32::from_rgb(0xff, 0x92, 0x2b),
        danger: Color32::from_rgb(0xff, 0x6b, 0x6b),
        stage: Color32::BLACK,
        stage_text: Color32::from_gray(150),
        tiles: [
            [Color32::from_rgb(0x1d, 0x2f, 0x4d), Color32::from_rgb(0x23, 0x44, 0x75), Color32::from_rgb(0x8d, 0xc2, 0xff)],
            [Color32::from_rgb(0x2a, 0x24, 0x52), Color32::from_rgb(0x3b, 0x2f, 0x7a), Color32::from_rgb(0xb4, 0xa5, 0xff)],
            [Color32::from_rgb(0x46, 0x2a, 0x1e), Color32::from_rgb(0x6a, 0x38, 0x22), Color32::from_rgb(0xff, 0xb0, 0x7a)],
            [Color32::from_rgb(0x46, 0x1f, 0x34), Color32::from_rgb(0x6b, 0x26, 0x4b), Color32::from_rgb(0xff, 0x9c, 0xc8)],
            [Color32::from_rgb(0x15, 0x3b, 0x31), Color32::from_rgb(0x1a, 0x57, 0x45), Color32::from_rgb(0x7d, 0xe8, 0xbf)],
        ],
        art_paper: Color32::from_rgb(0xf4, 0xf2, 0xfb),
        art_shadow: Color32::from_rgba_premultiplied(0, 0, 0, 0x50),
        scrim: Color32::from_rgba_premultiplied(0, 0, 0, 0x90),
        thumb_light: Color32::from_rgb(0xe6, 0xe4, 0xee),
        thumb_dark: Color32::from_rgb(0x2a, 0x29, 0x37),
    };

    pub fn get(ctx: &egui::Context) -> Tokens {
        if ctx.global_style().visuals.dark_mode { Tokens::DARK } else { Tokens::LIGHT }
    }
}

/// Colours offered for project folders (kept with the folder; the same in light and dark mode).
pub const FOLDER_COLORS: [Color32; 8] = [
    Color32::from_rgb(0x5b, 0x3c, 0xf5),
    Color32::from_rgb(0x25, 0x63, 0xeb),
    Color32::from_rgb(0x0e, 0x9f, 0x8e),
    Color32::from_rgb(0x3f, 0xa3, 0x3c),
    Color32::from_rgb(0xe8, 0xa3, 0x17),
    Color32::from_rgb(0xea, 0x58, 0x0c),
    Color32::from_rgb(0xdb, 0x27, 0x77),
    Color32::from_rgb(0x6b, 0x69, 0x7c),
];

/// Applies the GenHa look to egui's widgets.
pub fn apply(ctx: &egui::Context, dark: bool) {
    let t = if dark { Tokens::DARK } else { Tokens::LIGHT };
    let mut v = if dark { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = t.panel;
    v.window_fill = t.panel;
    v.extreme_bg_color = t.panel_alt;
    v.faint_bg_color = t.panel_alt;
    v.selection.bg_fill = t.accent;
    v.selection.stroke = Stroke::new(1.0, t.accent_text);
    v.hyperlink_color = t.accent;
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(10);
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.corner_radius = CornerRadius::same(8);
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, t.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, t.text);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, t.text);
    v.widgets.inactive.weak_bg_fill = t.panel_alt;
    v.widgets.hovered.weak_bg_fill = t.accent_soft;
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        s.spacing.button_padding = egui::vec2(10.0, 5.0);
        s.spacing.interact_size.y = 26.0;
    });
}

pub fn to_egui(c: genha_color::Color) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)
}

pub fn from_egui(c: Color32) -> genha_color::Color {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    genha_color::Color::rgba(r, g, b, a)
}

/// How far a hover has faded in at `rect` (0 to 1 over 0.12 s), keyed by the rect's place.
pub fn hover_fade(ctx: &egui::Context, rect: egui::Rect, hovered: bool) -> f32 {
    let id = egui::Id::new(("hover_fade", rect.min.x.round() as i32, rect.min.y.round() as i32, rect.width().round() as i32));
    ctx.animate_bool_with_time(id, hovered, 0.12)
}
