//! Curated colour palettes (original combinations).

use genha_color::Color;

use crate::build::hex;

#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    pub id: &'static str,
    pub name: &'static str,
    /// Five colours, roughly: main, accent, contrast, dark, light.
    pub colors: Vec<Color>,
}

const PALETTES: &[(&str, &str, [u32; 5])] = &[
    ("harbor-dawn", "Harbor dawn", [0x2b4c7e, 0xf2a65a, 0x567ebb, 0x1b2a41, 0xf6f1e9]),
    ("citrus-pop", "Citrus pop", [0xff8c42, 0xffd23f, 0x3bceac, 0x2d2a32, 0xfffbf0]),
    ("forest-floor", "Forest floor", [0x386641, 0xa7c957, 0xbc4749, 0x1f2a1d, 0xf2e8cf]),
    ("midnight-neon", "Midnight neon", [0x7b2ff7, 0x00e5ff, 0xff3cac, 0x0d0221, 0xf1edff]),
    ("terracotta", "Terracotta", [0xc8553d, 0xf28f3b, 0x588b8b, 0x2f1b14, 0xfbf3ea]),
    ("lagoon", "Lagoon", [0x0081a7, 0x00afb9, 0xf07167, 0x073642, 0xfdfcdc]),
    ("berry-jam", "Berry jam", [0x9d174d, 0xf472b6, 0xfbbf24, 0x3b0a1f, 0xfff1f5]),
    ("desert-sage", "Desert sage", [0x8a9a5b, 0xd4a373, 0x6b705c, 0x2e2f26, 0xfaedcd]),
    ("nordic-frost", "Nordic frost", [0x5e81ac, 0x88c0d0, 0xbf616a, 0x2e3440, 0xeceff4]),
    ("retro-diner", "Retro diner", [0xe63946, 0x48cae4, 0xffb703, 0x22223b, 0xfdf6e3]),
    ("lavender-field", "Lavender field", [0x7f5af0, 0xc3b1e1, 0x2cb67d, 0x16161a, 0xf8f5ff]),
    ("espresso", "Espresso", [0x6f4e37, 0xc69c72, 0xe9c46a, 0x231a14, 0xf7efe5]),
    ("coral-reef", "Coral reef", [0xff6f61, 0x2ec4b6, 0xffbf69, 0x14213d, 0xfff8f0]),
    ("mono-ink", "Mono ink", [0x111111, 0x555555, 0xe63b2e, 0x000000, 0xf5f5f5]),
    ("sunset-strip", "Sunset strip", [0xf25f5c, 0xffe066, 0x247ba0, 0x1d1e2c, 0xfff7e6]),
    ("mint-studio", "Mint studio", [0x2a9d8f, 0x99e2b4, 0xe76f51, 0x173f3a, 0xf1faf6]),
];

/// The built-in palettes.
pub fn palettes() -> Vec<Palette> {
    PALETTES.iter().map(|(id, name, c)| Palette { id, name, colors: c.iter().map(|v| hex(*v)).collect() }).collect()
}
