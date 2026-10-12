//! Brand kit: the colours, fonts and logos a project (or a team) designs with.

use genexpress_color::Color;
use serde::{Deserialize, Serialize};

use crate::AssetId;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BrandColor {
    pub name: String,
    pub color: Color,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BrandKit {
    pub name: String,
    #[serde(default)]
    pub colors: Vec<BrandColor>,
    pub heading_font: String,
    pub body_font: String,
    /// Logos stored in the document's assets.
    #[serde(default)]
    pub logos: Vec<AssetId>,
}

impl Default for BrandKit {
    fn default() -> Self {
        BrandKit {
            name: "My brand".into(),
            colors: vec![
                BrandColor { name: "Primary".into(), color: Color::rgb(0x5b, 0x3c, 0xf5) },
                BrandColor { name: "Secondary".into(), color: Color::rgb(0xff, 0x6b, 0x5b) },
                BrandColor { name: "Dark".into(), color: Color::rgb(0x1b, 0x1b, 0x2f) },
                BrandColor { name: "Light".into(), color: Color::rgb(0xf6, 0xf4, 0xff) },
            ],
            heading_font: "Sans".into(),
            body_font: "Sans".into(),
            logos: vec![],
        }
    }
}

impl BrandKit {
    /// The colour at `index`, or `fallback`.
    pub fn color(&self, index: usize, fallback: Color) -> Color {
        self.colors.get(index).map(|c| c.color).unwrap_or(fallback)
    }
}
