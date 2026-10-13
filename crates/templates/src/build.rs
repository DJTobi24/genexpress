//! A small builder that keeps template code short: every element gets a fresh id and a name.

use genha_color::Color;
use genha_doc::{
    Animation, AnimationKind, BrandKit, ChartEl, ChartKind, ChartSeries, Document, Element, ElementKind, Fill, GroupEl, LoopKind, NodeId, Page,
    ShapeEl, ShapeKind, SpanStyle, StrokeStyle, TableEl, TextAlign, TextEffect, TextEl, TextFit, TextSpan, TextStyle,
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

    /// A chart in the body font, drawn in the role colours (so a brand kit re-colours it), with
    /// labels in `ink`; one `(name, values)` per series.
    pub fn chart(&self, kind: ChartKind, categories: &[&str], series: &[(&str, &[f64])], size: f64, ink: Color) -> ChartEl {
        ChartEl {
            kind,
            categories: categories.iter().map(|c| (*c).to_string()).collect(),
            series: series.iter().map(|(n, v)| ChartSeries::new(*n, v.to_vec())).collect(),
            colors: vec![self.primary, self.secondary, self.dark.lerp(self.light, 0.45), self.primary.lerp(self.light, 0.55)],
            show_legend: series.len() > 1,
            font_family: self.body.clone(),
            font_size: size,
            text_color: ink,
            ..ChartEl::default()
        }
    }

    /// A table in the body font: a header row in the primary colour, banded body rows.
    pub fn table(&self, rows: &[&[&str]], size: f64) -> TableEl {
        TableEl {
            rows: rows.iter().map(|r| r.iter().map(|c| (*c).to_string()).collect()).collect(),
            header_row: true,
            font_family: self.body.clone(),
            font_size: size,
            text_color: self.dark,
            header_fill: self.primary,
            header_text: self.light,
            row_fill: self.light,
            alt_row_fill: Some(self.primary.lerp(self.light, 0.9)),
            border: Some(StrokeStyle { color: self.dark.lerp(self.light, 0.8), width: 1.0, dash: vec![] }),
            padding: (size * 0.5).round(),
            ..TableEl::default()
        }
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

    /// A text element; its height is computed from the number of lines. Headings (bold) and short
    /// blocks (one or two lines: titles, values, labels, buttons) shrink to fit their box
    /// ([`TextFit::Shrink`]): in a wider font than the one they were designed with (Inter) they
    /// are set smaller instead of wrapping into a line the box has no room for. Longer body text
    /// keeps its size.
    pub fn text_el(&mut self, name: &str, x: f64, y: f64, w: f64, text: &str, style: TextStyle) -> Element {
        let lines = text.lines().count().max(1);
        let h = (lines as f64 * style.font_size * style.line_height + style.font_size * 0.15).ceil();
        let fit = if style.bold || lines <= 2 { TextFit::Shrink } else { TextFit::None };
        self.el(
            name,
            Frame::new(x, y, w, h),
            ElementKind::Text(TextEl { text: text.into(), style, effect: TextEffect::None, curve: 0.0, spans: vec![], fit }),
        )
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

    /// A library icon (see [`crate::shape_library`]) in the given frame; nothing if `id` is unknown.
    pub fn icon(&mut self, name: &str, id: &str, x: f64, y: f64, w: f64, h: f64, fill: Fill) -> Option<NodeId> {
        let shape = crate::shapes::library_shape(id);
        debug_assert!(shape.is_some(), "unknown library icon `{id}`");
        Some(self.shape(name, shape?, x, y, w, h, fill))
    }

    /// A rounded button: a pill of `fill` with `label` centred on it.
    pub fn pill(&mut self, name: &str, x: f64, y: f64, w: f64, h: f64, fill: Fill, label: &str, style: TextStyle) {
        self.rect(name, x, y, w, h, h / 2.0, fill);
        let style = TextStyle { line_height: 1.15, ..style };
        let ty = y + (h - style.font_size * style.line_height) / 2.0;
        self.text(&format!("{name} label"), x, ty, w, label, style);
    }

    /// Gives the last shape a dashed stroke.
    pub fn dash_last(&mut self, color: Color, width: f64, dash: &[f64]) {
        if let Some(e) = self.last()
            && let ElementKind::Shape(s) = &mut e.kind
        {
            s.stroke = Some(StrokeStyle { color, width, dash: dash.to_vec() });
        }
    }

    /// Gives the first `word` in the last text element its own style (a style run); nothing when
    /// the word is not there.
    pub fn span_last(&mut self, word: &str, style: SpanStyle) {
        if let Some(e) = self.last()
            && let ElementKind::Text(t) = &mut e.kind
            && let Some(byte) = t.text.find(word)
        {
            let style = style.reduced(&t.style);
            if style.is_empty() {
                return;
            }
            let start = t.text.get(..byte).map_or(0, |s| s.chars().count());
            let end = start + word.chars().count();
            t.spans.push(TextSpan::new(start..end, style));
            t.spans.sort_by_key(|s| s.start);
        }
    }

    /// Sets the first `word` of the last text element in `color` (and bold when `bold`).
    pub fn mark_last(&mut self, word: &str, color: Color, bold: bool) {
        self.span_last(word, SpanStyle { color: Some(color), bold: bold.then_some(true), ..SpanStyle::default() });
    }

    /// Bends the last text element along an arc of `degrees` (see `TextEl::curve`) and makes its
    /// box `height` tall, enough for the bent lines (they are centred in the box). Bent text
    /// shrinks to fit the box.
    pub fn curve_last(&mut self, degrees: f64, height: f64) {
        if let Some(e) = self.last()
            && let ElementKind::Text(t) = &mut e.kind
        {
            t.curve = degrees;
            t.fit = TextFit::Shrink;
            e.frame.height = height;
        }
    }

    /// Gives the last element an entrance animation.
    pub fn animate_last(&mut self, kind: AnimationKind, duration: f64, delay: f64) {
        if let Some(e) = self.last() {
            e.animation = Some(Animation::new(kind, duration, delay));
        }
    }

    /// Number of elements on the current page, to pass to [`B::animate_since`] later.
    pub fn mark(&self) -> usize {
        self.doc.pages.get(self.page).map_or(0, |p| p.elements.len())
    }

    /// Gives every element added since `mark` the same entrance.
    pub fn animate_since(&mut self, mark: usize, kind: AnimationKind, duration: f64, delay: f64) {
        if let Some(p) = self.page_mut() {
            for e in p.elements.iter_mut().skip(mark) {
                e.animation = Some(Animation::new(kind, duration, delay));
            }
        }
    }

    /// Repeats `looping` on the last element after its entrance.
    pub fn loop_last(&mut self, looping: LoopKind) {
        if let Some(e) = self.last()
            && let Some(a) = &mut e.animation
        {
            a.looping = Some(looping);
        }
    }

    /// How long the current page plays in a video or animated export, in seconds.
    pub fn duration(&mut self, seconds: f64) {
        if let Some(p) = self.page_mut() {
            p.duration = Some(seconds);
        }
    }

    pub fn chart(&mut self, name: &str, x: f64, y: f64, w: f64, h: f64, chart: ChartEl) -> NodeId {
        let e = self.el(name, Frame::new(x, y, w, h), ElementKind::Chart(chart));
        self.push(e)
    }

    pub fn table(&mut self, name: &str, x: f64, y: f64, w: f64, h: f64, table: TableEl) -> NodeId {
        let e = self.el(name, Frame::new(x, y, w, h), ElementKind::Table(table));
        self.push(e)
    }

    /// A QR code for `data` in a `size` square; nothing when it cannot be made.
    pub fn qr(&mut self, name: &str, data: &str, x: f64, y: f64, size: f64, color: Color) -> Option<NodeId> {
        let shape = crate::qr_code(data, color).ok();
        debug_assert!(shape.is_some(), "QR code for `{data}`");
        Some(self.shape(name, shape?, x, y, size, size, Fill::solid(color)))
    }

    /// Groups `children` (in page coordinates) under one element whose frame is their bounds.
    pub fn group(&mut self, name: &str, children: Vec<Element>) -> NodeId {
        let bounds = children.iter().map(|c| c.frame.bounds()).reduce(|a, b| a.union(b)).unwrap_or_default();
        let frame = Frame::new(bounds.x0, bounds.y0, bounds.width(), bounds.height());
        let e = self.el(name, frame, ElementKind::Group(GroupEl { children }));
        self.push(e)
    }
}
