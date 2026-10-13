//! The template catalogue: original designs built only from shapes, gradients, library icons and
//! text, one module per category.
//!
//! Every template is drawn with four role colours (primary, secondary, dark, light) and two fonts
//! (heading, body), see [`Theme`]; tints are mixed from those, so applying a brand kit re-colours
//! the whole design consistently.

use genha_color::Color;
use genha_doc::{BrandKit, Document, Fill, GradientStop, preset};

use crate::build::{B, Theme};

mod brand;
mod cards;
mod decks;
mod documents;
mod events;
mod greetings;
mod marketing;
mod motion;
mod office;
mod posts;
mod print;
mod social;

/// A template in the catalogue.
#[derive(Clone, Debug, PartialEq)]
pub struct TemplateInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    /// The page-size preset it is designed for (see `genha_doc::preset`).
    pub preset: &'static str,
    /// Page size in pixels (the preset's, turned to landscape for landscape designs).
    pub width: f64,
    pub height: f64,
    /// Lower-case search words.
    pub tags: &'static [&'static str],
}

pub(crate) struct Def {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    preset: &'static str,
    landscape: bool,
    tags: &'static [&'static str],
    theme: fn() -> Theme,
    build: fn(&mut B),
}

/// A catalogue entry (portrait, or the preset's own orientation).
pub(crate) const fn def(
    id: &'static str,
    name: &'static str,
    category: &'static str,
    preset: &'static str,
    tags: &'static [&'static str],
    theme: fn() -> Theme,
    build: fn(&mut B),
) -> Def {
    Def { id, name, category, preset, landscape: false, tags, theme, build }
}

impl Def {
    /// The same entry on the preset turned to landscape.
    pub(crate) const fn landscape(self) -> Def {
        Def { landscape: true, ..self }
    }
}

pub(crate) const SOCIAL: &str = "Social";
pub(crate) const PRINT: &str = "Print";
pub(crate) const CARDS: &str = "Cards & invitations";
pub(crate) const DOCUMENTS: &str = "Documents";
pub(crate) const MARKETING: &str = "Marketing";
pub(crate) const LOGOS: &str = "Logos";
/// The categories in the order the catalogue lists them.
pub(crate) const CATEGORIES: [&str; 6] = [SOCIAL, PRINT, CARDS, DOCUMENTS, MARKETING, LOGOS];

fn defs() -> impl Iterator<Item = &'static Def> {
    [
        social::DEFS,
        posts::DEFS,
        motion::DEFS,
        print::DEFS,
        events::DEFS,
        cards::DEFS,
        greetings::DEFS,
        documents::DEFS,
        office::DEFS,
        decks::DEFS,
        marketing::DEFS,
        brand::DEFS,
    ]
    .into_iter()
    .flatten()
}

fn size(d: &Def) -> (f64, f64) {
    let (w, h) = preset::find(d.preset).map_or((1080.0, 1080.0), |p| (p.width, p.height));
    if d.landscape { (w.max(h), w.min(h)) } else { (w, h) }
}

/// Every template, grouped by category (in [`CATEGORIES`] order).
pub fn catalog() -> Vec<TemplateInfo> {
    let mut list: Vec<TemplateInfo> = defs()
        .map(|d| {
            let (width, height) = size(d);
            TemplateInfo { id: d.id, name: d.name, category: d.category, preset: d.preset, width, height, tags: d.tags }
        })
        .collect();
    list.sort_by_key(|t| CATEGORIES.iter().position(|c| *c == t.category).unwrap_or(CATEGORIES.len()));
    list
}

/// A new document from template `id`. With a brand kit, the template's primary, secondary, dark
/// and light colours become the kit's colours 0 to 3, its fonts the kit's heading and body fonts,
/// and the kit becomes the document's brand.
pub fn instantiate(id: &str, brand: Option<&BrandKit>) -> Option<Document> {
    let def = defs().find(|d| d.id == id)?;
    let (w, h) = size(def);
    let theme = (def.theme)().branded(brand);
    let mut b = B::new(def.name, w, h, theme);
    (def.build)(&mut b);
    let mut doc = b.finish();
    if let Some(kit) = brand {
        doc.brand = kit.clone();
    }
    Some(doc)
}

pub(crate) fn solid(c: Color) -> Fill {
    Fill::solid(c)
}

pub(crate) fn lin(angle: f64, a: Color, b: Color) -> Fill {
    Fill::linear(angle, a, b)
}

/// A radial gradient from `inner` at the centre to `outer` at the edge.
pub(crate) fn radial(inner: Color, outer: Color) -> Fill {
    Fill::Radial { stops: vec![GradientStop { offset: 0.0, color: inner }, GradientStop { offset: 1.0, color: outer }] }
}
