//! A small builder that keeps template code short: every element gets a fresh id and a name.

use genha_color::Color;
use genha_doc::{
    BrandKit, Document, Element, ElementKind, Fill, GroupEl, NodeId, Page, ShapeEl, ShapeKind, StrokeStyle, TextAlign, TextEffect, TextEl, TextStyle,
};
use genha_geom::Frame;

/// The four role colours and two fonts a template is designed with; a brand kit replaces them.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub dark: Color,
    pub light: Color,
    pub heading: String,
    pub body: String,
}

impl Theme {
    pub fn new(primary: u32, secondary: u32, dark: u32, light: u32, heading: &str, body: &str) -> Theme {
        Theme { primary: hex(primary), secondary: hex(secondary), dark: hex(dark), light: hex(light), heading: heading.into(), body: body.into() }
    }

    /// The theme with the brand's colours 0..=3 (primary, secondary, dark, light) and fonts.
    pub fn branded(self, brand: Option<&BrandKit>) -> Theme {
        let Some(b) = brand else { return self };
        let font = |f: &str, d: String| if f.trim().is_empty() { d } else { f.to_string() };
        Theme {
            primary: b.color(0, self.primary),
            secondary: b.color(1, self.secondary),
            dark: b.color(2, self.dark),
            light: b.color(3, self.light),
            heading: font(&b.heading_font, self.heading),
            body: font(&b.body_font, self.body),
        }
    }

    /// A bold heading style.
    pub fn h(&self, size: f64, color: Color) -> TextStyle {
        TextStyle { font_family: self.heading.clone(), font_size: size, bold: true, color, line_height: 1.1, ..TextStyle::default() }
    }

    /// A regular body style.
    pub fn b(&self, size: f64, color: Color) -> TextStyle {
        TextStyle { font_family: self.body.clone(), font_size: size, color, line_height: 1.3, ..TextStyle::default() }
    }
}

pub(crate) const fn hex(v: u32) -> Color {
    Color::rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// Chainable tweaks for text styles.
pub(crate) trait Sty: Sized {
    fn with(self, f: impl FnOnce(&mut TextStyle)) -> Self;
    fn left(self) -> Self {
        self.with(|s| s.align = TextAlign::Left)
    }
    fn right(self) -> Self {
        self.with(|s| s.align = TextAlign::Right)
    }
    fn bold(self) -> Self {
        self.with(|s| s.bold = true)
    }
    fn italic(self) -> Self {
        self.with(|s| s.italic = true)
    }
    fn upper(self) -> Self {
        self.with(|s| s.uppercase = true)
    }
    /// Letter spacing in thousandths of an em.
    fn ls(self, v: f64) -> Self {
        self.with(|s| s.letter_spacing = v)
    }
    fn lh(self, v: f64) -> Self {
        self.with(|s| s.line_height = v)
    }
}

impl Sty for TextStyle {
    fn with(mut self, f: impl FnOnce(&mut TextStyle)) -> Self {
        f(&mut self);
        self
    }
}

/// Builds one document page by page.
pub(crate) struct B {
    pub doc: Document,
    page: usize,
    pub w: f64,
    pub h: f64,
    pub t: Theme,
}

pub(crate) fn shape_el(shape: ShapeKind, fill: Fill) -> ElementKind {
    ElementKind::Shape(ShapeEl { shape, fill, stroke: None, shadow: None })
}

#[allow(clippy::too_many_arguments)]
impl B {
    pub fn new(title: &str, w: f64, h: f64, t: Theme) -> B {
        let mut doc = Document::new(title, w, h);
        if let Some(p) = doc.pages.first_mut() {
            p.name = "Page 1".into();
        }
        B { doc, page: 0, w, h, t }
    }

    pub fn finish(self) -> Document {
        self.doc
    }

    fn page_mut(&mut self) -> Option<&mut Page> {
        self.doc.pages.get_mut(self.page)
    }

    /// Starts a new page of the same size.
    pub fn new_page(&mut self, name: &str) {
        let id = self.doc.alloc_id();
        self.doc.pages.push(Page::new(id, name, self.w, self.h));
        self.page = self.doc.pages.len() - 1;
    }

    pub fn name_page(&mut self, name: &str) {
        if let Some(p) = self.page_mut() {
            p.name = name.into();
        }
    }

    pub fn notes(&mut self, notes: &str) {
        if let Some(p) = self.page_mut() {
            p.notes = notes.into();
        }
    }

    pub fn bg(&mut self, fill: Fill) {
        if let Some(p) = self.page_mut() {
            p.background = fill;
        }
    }

    /// A named element with a fresh id (not yet on a page).
    pub fn el(&mut self, name: &str, frame: Frame, kind: ElementKind) -> Element {
        let id = self.doc.alloc_id();
        let mut e = Element::new(id, frame, kind);
        e.name = name.into();
        e
    }

    pub fn push(&mut self, e: Element) -> NodeId {
        let id = e.id;
        if let Some(p) = self.page_mut() {
            p.elements.push(e);
        }
        id
    }

    /// The last element added to the current page.
    pub fn last(&mut self) -> Option<&mut Element> {
        self.page_mut().and_then(|p| p.elements.last_mut())
    }

    pub fn rotate_last(&mut self, degrees: f64) {
        if let Some(e) = self.last() {
            e.frame.rotation = degrees;
        }
    }

    pub fn effect_last(&mut self, effect: TextEffect) {
        if let Some(e) = self.last()
            && let ElementKind::Text(t) = &mut e.kind
        {
            t.effect = effect;
        }
    }

    pub fn shape(&mut self, name: &str, shape: ShapeKind, x: f64, y: f64, w: f64, h: f64, fill: Fill) -> NodeId {
        let e = self.el(name, Frame::new(x, y, w, h), shape_el(shape, fill));
        self.push(e)
    }

    pub fn rect(&mut self, name: &str, x: f64, y: f64, w: f64, h: f64, radius: f64, fill: Fill) -> NodeId {
        self.shape(name, ShapeKind::Rect { corner_radius: radius }, x, y, w, h, fill)
    }

    pub fn ellipse(&mut self, name: &str, x: f64, y: f64, w: f64, h: f64, fill: Fill) -> NodeId {
        self.shape(name, ShapeKind::Ellipse, x, y, w, h, fill)
    }

    /// An outline only shape.
    pub fn outline(&mut self, name: &str, shape: ShapeKind, x: f64, y: f64, w: f64, h: f64, color: Color, width: f64) -> NodeId {
        let kind = ElementKind::Shape(ShapeEl { shape, fill: Fill::None, stroke: Some(StrokeStyle { color, width, dash: vec![] }), shadow: None });
        let e = self.el(name, Frame::new(x, y, w, h), kind);
        self.push(e)
    }

    /// A text element; its height is computed from the number of lines.
    pub fn text_el(&mut self, name: &str, x: f64, y: f64, w: f64, text: &str, style: TextStyle) -> Element {
        let lines = text.lines().count().max(1) as f64;
        let h = (lines * style.font_size * style.line_height + style.font_size * 0.15).ceil();
        self.el(name, Frame::new(x, y, w, h), ElementKind::Text(TextEl { text: text.into(), style, effect: TextEffect::None, curve: 0.0 }))
    }

    pub fn text(&mut self, name: &str, x: f64, y: f64, w: f64, text: &str, style: TextStyle) -> NodeId {
        let e = self.text_el(name, x, y, w, text, style);
        self.push(e)
    }

    /// A text box of width `w` centred horizontally on the page.
    pub fn ctext(&mut self, name: &str, y: f64, w: f64, text: &str, style: TextStyle) -> NodeId {
        let x = (self.w - w) / 2.0;
        self.text(name, x, y, w, text, style)
    }

    /// Groups `children` (in page coordinates) under one element whose frame is their bounds.
    pub fn group(&mut self, name: &str, children: Vec<Element>) -> NodeId {
        let bounds = children.iter().map(|c| c.frame.bounds()).reduce(|a, b| a.union(b)).unwrap_or_default();
        let frame = Frame::new(bounds.x0, bounds.y0, bounds.width(), bounds.height());
        let e = self.el(name, frame, ElementKind::Group(GroupEl { children }));
        self.push(e)
    }
}
