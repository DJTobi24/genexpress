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
    ("glacier-mint", "Glacier mint", [0x4fb3bf, 0xa8e6cf, 0xff8b94, 0x0b3c49, 0xf3fbfa]),
    ("saffron-market", "Saffron market", [0xf4a300, 0xd1495b, 0x00798c, 0x2e1f27, 0xfdf7e8]),
    ("plum-velvet", "Plum velvet", [0x6d2e46, 0xa26769, 0xd5b9b2, 0x2a0f1c, 0xfaf3f0]),
    ("electric-lime", "Electric lime", [0xc6f432, 0x2b59c3, 0xff5d73, 0x101418, 0xf9fff0]),
    ("ocean-depth", "Ocean depth", [0x05668d, 0x028090, 0x02c39a, 0x022b3a, 0xf0f3bd]),
    ("peach-fuzz", "Peach fuzz", [0xffb38a, 0xff7b54, 0x939b62, 0x3b2a20, 0xfff4ec]),
    ("cobalt-sun", "Cobalt sun", [0x1d4ed8, 0xfacc15, 0xf97316, 0x0b1437, 0xf8fafc]),
    ("moss-stone", "Moss and stone", [0x606c38, 0xdda15e, 0x8d99ae, 0x1f2416, 0xf5f3ea]),
    ("candy-shop", "Candy shop", [0xff70a6, 0x70d6ff, 0xffd670, 0x2b2d42, 0xfff7fb]),
    ("rust-slate", "Rust and slate", [0xb7472a, 0x5b8e7d, 0xf4b942, 0x22252c, 0xf7f3ee]),
    ("arctic-night", "Arctic night", [0x3d5a80, 0x98c1d9, 0xee6c4d, 0x141c2b, 0xe0fbfc]),
    ("tropical-punch", "Tropical punch", [0xff4365, 0x00d9c0, 0xffc145, 0x1b1b3a, 0xfffdf6]),
    ("sage-blush", "Sage and blush", [0x87a878, 0xe8a598, 0xc9cba3, 0x2d3a2e, 0xfbf7f4]),
    ("royal-gold", "Royal gold", [0x3c096c, 0xffb700, 0x9d4edd, 0x1a0033, 0xfff9ec]),
    ("cherry-cola", "Cherry cola", [0x9e0031, 0xff9f1c, 0x6d4c3d, 0x1f0a0f, 0xfff3e6]),
    ("skyline", "Skyline", [0x4361ee, 0x4cc9f0, 0xf72585, 0x0f1029, 0xf4f7ff]),
    ("olive-grove", "Olive grove", [0x6b8f71, 0xd9c5b2, 0xaa8f66, 0x1e2a22, 0xf8f6f0]),
    ("flamingo", "Flamingo", [0xf26a8d, 0xf49cbb, 0x3bb273, 0x2d1e2f, 0xfff0f5]),
    ("graphite", "Graphite", [0x3a3a3c, 0x8e8e93, 0x0a84ff, 0x111113, 0xf2f2f4]),
    ("honey-oat", "Honey oat", [0xe9b44c, 0x9b2915, 0x50a2a7, 0x2b1d0e, 0xfbf5e6]),
    ("deep-teal", "Deep teal", [0x0f4c5c, 0xe36414, 0x5f0f40, 0x081f26, 0xfdf8f0]),
    ("lilac-dusk", "Lilac dusk", [0x9b72cf, 0xf7b2bd, 0x56cbf9, 0x231a33, 0xfaf7ff]),
    ("citrus-grove", "Citrus grove", [0xf9c80e, 0xf86624, 0x43bccd, 0x1d1a05, 0xfffcf0]),
    ("pine-cabin", "Pine cabin", [0x2d6a4f, 0xb5838d, 0xe5989b, 0x10241b, 0xf6f1eb]),
];

/// The built-in palettes.
pub fn palettes() -> Vec<Palette> {
    PALETTES.iter().map(|(id, name, c)| Palette { id, name, colors: c.iter().map(|v| hex(*v)).collect() }).collect()
}
