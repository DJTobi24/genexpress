//! The GenHa document model (layer L1): pure data + serde.
//!
//! A [`Document`] (a "project") holds pages of possibly different sizes. Each [`Page`] has a
//! background and a z-ordered list of [`Element`]s (bottom first): shapes, text, images and groups.
//! Pictures and other binary data live once in [`Document::assets`] and are shared by id. The
//! [`BrandKit`] carries the project's colours, fonts and logos.
//!
//! Every object has a document-unique [`NodeId`]. Documents are cheap to clone (asset bytes are
//! reference counted), which the engine's undo history relies on.
#![forbid(unsafe_code)]

mod asset;
mod brand;
mod element;
mod page;
pub mod preset;

use std::collections::BTreeMap;

pub use asset::{Asset, AssetId, Bytes};
pub use brand::{BrandColor, BrandKit};
pub use element::{
    Adjustments, Animation, AnimationKind, CropRect, Element, ElementKind, Fill, GradientStop, GroupEl, ImageEl, Shadow, ShapeEl, ShapeKind,
    StrokeStyle, TextAlign, TextEffect, TextEl, TextStyle,
};
pub use genha_color::Color;
pub use genha_geom::{Fit, Frame};
pub use page::Page;
pub use preset::{PRESETS, PagePreset};
use serde::{Deserialize, Serialize};

/// Largest page side accepted, in pixels (untrusted input is capped to this).
pub const MAX_PAGE_SIDE: f64 = 16_384.0;
/// Most pages a document may have.
pub const MAX_PAGES: usize = 500;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum DocError {
    #[error("no such object {0}")]
    NoNode(NodeId),
    #[error("no page {0}")]
    NoPage(usize),
    #[error("no asset {0}")]
    NoAsset(AssetId),
    #[error("{0}")]
    Invalid(String),
}

/// A document-unique object id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub u64);

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub title: String,
    pub pages: Vec<Page>,
    #[serde(default)]
    pub brand: BrandKit,
    #[serde(default)]
    pub assets: BTreeMap<AssetId, Asset>,
    /// The next id [`Document::alloc_id`] hands out (ids and asset ids share the counter).
    pub next_id: u64,
}

impl Default for Document {
    fn default() -> Self {
        Document::new("Untitled", 1080.0, 1080.0)
    }
}

impl Document {
    /// A document with one blank white page of the given size.
    pub fn new(title: &str, width: f64, height: f64) -> Self {
        let mut doc = Document { title: title.to_string(), pages: vec![], brand: BrandKit::default(), assets: BTreeMap::new(), next_id: 1 };
        let id = doc.alloc_id();
        doc.pages.push(Page::new(id, "Page 1", width, height));
        doc
    }

    pub fn alloc_id(&mut self) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    pub fn page(&self, index: usize) -> Result<&Page, DocError> {
        self.pages.get(index).ok_or(DocError::NoPage(index))
    }

    pub fn page_mut(&mut self, index: usize) -> Result<&mut Page, DocError> {
        self.pages.get_mut(index).ok_or(DocError::NoPage(index))
    }

    /// Index of the page with id `id`.
    pub fn page_index(&self, id: NodeId) -> Option<usize> {
        self.pages.iter().position(|p| p.id == id)
    }

    /// The element `id` (searching groups too) and the index of its page.
    pub fn find(&self, id: NodeId) -> Option<(usize, &Element)> {
        self.pages.iter().enumerate().find_map(|(i, p)| p.find(id).map(|e| (i, e)))
    }

    pub fn find_mut(&mut self, id: NodeId) -> Option<&mut Element> {
        self.pages.iter_mut().find_map(|p| p.find_mut(id))
    }

    pub fn element(&self, id: NodeId) -> Result<&Element, DocError> {
        self.find(id).map(|(_, e)| e).ok_or(DocError::NoNode(id))
    }

    pub fn element_mut(&mut self, id: NodeId) -> Result<&mut Element, DocError> {
        self.find_mut(id).ok_or(DocError::NoNode(id))
    }

    /// Removes the element `id` wherever it is; returns it with its page index.
    pub fn remove(&mut self, id: NodeId) -> Option<(usize, Element)> {
        self.pages.iter_mut().enumerate().find_map(|(i, p)| p.remove(id).map(|e| (i, e)))
    }

    /// Stores `asset`, returning its new id.
    pub fn add_asset(&mut self, asset: Asset) -> AssetId {
        let id = AssetId(self.alloc_id().0);
        self.assets.insert(id, asset);
        id
    }

    pub fn asset(&self, id: AssetId) -> Result<&Asset, DocError> {
        self.assets.get(&id).ok_or(DocError::NoAsset(id))
    }

    /// Gives `el` (and its children) fresh ids, for duplicates and pastes.
    pub fn reassign_ids(&mut self, el: &mut Element) {
        el.id = self.alloc_id();
        if let ElementKind::Group(g) = &mut el.kind {
            for c in &mut g.children {
                self.reassign_ids(c);
            }
        }
    }

    /// Gives `page` and everything on it fresh ids.
    pub fn reassign_page_ids(&mut self, page: &mut Page) {
        page.id = self.alloc_id();
        for e in &mut page.elements {
            self.reassign_ids(e);
        }
    }

    /// Makes a document read from an untrusted source safe to work with: at least one page, page
    /// sizes and frames finite and capped, unique ids and an id counter past all of them, and
    /// assets referenced by images present (missing ones are reported, not fatal). Returns
    /// human-readable notes on what was repaired.
    pub fn repair(&mut self) -> Vec<String> {
        let mut notes = vec![];
        if self.pages.len() > MAX_PAGES {
            notes.push(format!("dropped {} pages past the limit of {MAX_PAGES}", self.pages.len() - MAX_PAGES));
            self.pages.truncate(MAX_PAGES);
        }
        let mut max = self.assets.keys().map(|a| a.0).max().unwrap_or(0);
        let mut seen = std::collections::HashSet::new();
        let mut dupes = 0usize;
        for p in &mut self.pages {
            p.width = clamp_side(p.width);
            p.height = clamp_side(p.height);
            for id in std::iter::once(p.id).chain(p.ids()) {
                max = max.max(id.0);
            }
        }
        let mut next = max.saturating_add(1).max(self.next_id);
        let mut fix = |id: &mut NodeId| {
            if !seen.insert(*id) {
                *id = NodeId(next);
                next = next.saturating_add(1);
                seen.insert(*id);
                dupes += 1;
            }
        };
        for p in &mut self.pages {
            fix(&mut p.id);
            p.for_each_mut(&mut |e| {
                fix(&mut e.id);
                e.frame = e.frame.sanitized();
                if !e.opacity.is_finite() {
                    e.opacity = 1.0;
                }
                e.opacity = e.opacity.clamp(0.0, 1.0);
            });
        }
        if dupes > 0 {
            notes.push(format!("gave {dupes} objects with duplicate ids new ids"));
        }
        self.next_id = next;
        if self.pages.is_empty() {
            let id = self.alloc_id();
            self.pages.push(Page::new(id, "Page 1", 1080.0, 1080.0));
            notes.push("the document had no pages; added a blank one".into());
        }
        let missing: Vec<AssetId> = self.pages.iter().flat_map(|p| p.asset_refs()).filter(|a| !self.assets.contains_key(a)).collect();
        if !missing.is_empty() {
            notes.push(format!("{} pictures refer to missing data", missing.len()));
        }
        notes
    }

    /// Asset ids no element or brand logo refers to.
    pub fn unused_assets(&self) -> Vec<AssetId> {
        let used: std::collections::HashSet<AssetId> =
            self.pages.iter().flat_map(|p| p.asset_refs()).chain(self.brand.logos.iter().copied()).collect();
        self.assets.keys().filter(|a| !used.contains(a)).copied().collect()
    }
}

fn clamp_side(v: f64) -> f64 {
    if v.is_finite() { v.clamp(1.0, MAX_PAGE_SIDE) } else { 1080.0 }
}

#[cfg(test)]
mod tests;
