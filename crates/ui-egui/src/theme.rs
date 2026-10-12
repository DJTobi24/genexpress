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
    pub danger: Color32,
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
        danger: Color32::from_rgb(0xd9, 0x30, 0x30),
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
        danger: Color32::from_rgb(0xff, 0x6b, 0x6b),
    };

    pub fn get(ctx: &egui::Context) -> Tokens {
        if ctx.global_style().visuals.dark_mode { Tokens::DARK } else { Tokens::LIGHT }
    }
}

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
