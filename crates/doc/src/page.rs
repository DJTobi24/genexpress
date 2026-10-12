//! Pages (artboards). Each page has its own size, so one project can hold a post, a story and a
//! banner side by side.

use serde::{Deserialize, Serialize};

use crate::{AssetId, Element, ElementKind, Fill, NodeId};
use genha_color::Color;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub id: NodeId,
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub background: Fill,
    /// Bottom first.
    #[serde(default)]
    pub elements: Vec<Element>,
    /// How long the page shows in a video or animated export, in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    /// Speaker notes (presentations).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

impl Page {
    pub fn new(id: NodeId, name: &str, width: f64, height: f64) -> Self {
        Page {
            id,
            name: name.into(),
            width,
            height,
            background: Fill::Solid { color: Color::WHITE },
            elements: vec![],
            duration: None,
            notes: String::new(),
        }
    }

    pub fn find(&self, id: NodeId) -> Option<&Element> {
        find_in(&self.elements, id)
    }

    pub fn find_mut(&mut self, id: NodeId) -> Option<&mut Element> {
        find_in_mut(&mut self.elements, id)
    }

    /// Removes `id` from the page or from a group on it.
    pub fn remove(&mut self, id: NodeId) -> Option<Element> {
        remove_in(&mut self.elements, id)
    }

    /// Index of a top-level element.
    pub fn index_of(&self, id: NodeId) -> Option<usize> {
        self.elements.iter().position(|e| e.id == id)
    }

    /// Every element id on the page, groups included, depth first.
    pub fn ids(&self) -> Vec<NodeId> {
        let mut v = vec![];
        self.for_each(&mut |e| v.push(e.id));
        v
    }

    pub fn for_each(&self, f: &mut dyn FnMut(&Element)) {
        fn walk(els: &[Element], f: &mut dyn FnMut(&Element)) {
            for e in els {
                f(e);
                if let ElementKind::Group(g) = &e.kind {
                    walk(&g.children, f);
                }
            }
        }
        walk(&self.elements, f);
    }

    pub fn for_each_mut(&mut self, f: &mut dyn FnMut(&mut Element)) {
        fn walk(els: &mut [Element], f: &mut dyn FnMut(&mut Element)) {
            for e in els {
                f(e);
                if let ElementKind::Group(g) = &mut e.kind {
                    walk(&mut g.children, f);
                }
            }
        }
        walk(&mut self.elements, f);
    }

    /// Assets the page's pictures use.
    pub fn asset_refs(&self) -> Vec<AssetId> {
        let mut v = vec![];
        self.for_each(&mut |e| {
            if let ElementKind::Image(i) = &e.kind {
                v.push(i.asset);
            }
        });
        v
    }
}

fn find_in(els: &[Element], id: NodeId) -> Option<&Element> {
    for e in els {
        if e.id == id {
            return Some(e);
        }
        if let ElementKind::Group(g) = &e.kind
            && let Some(f) = find_in(&g.children, id)
        {
            return Some(f);
        }
    }
    None
}

fn find_in_mut(els: &mut [Element], id: NodeId) -> Option<&mut Element> {
    for e in els {
        if e.id == id {
            return Some(e);
        }
        if let ElementKind::Group(g) = &mut e.kind
            && let Some(f) = find_in_mut(&mut g.children, id)
        {
            return Some(f);
        }
    }
    None
}

fn remove_in(els: &mut Vec<Element>, id: NodeId) -> Option<Element> {
    if let Some(i) = els.iter().position(|e| e.id == id) {
        return Some(els.remove(i));
    }
    els.iter_mut().find_map(|e| match &mut e.kind {
        ElementKind::Group(g) => remove_in(&mut g.children, id),
        _ => None,
    })
}
