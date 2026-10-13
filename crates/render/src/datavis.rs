//! Charts and tables as drawing primitives (filled and stroked paths, laid-out text) in the
//! element's local box, shared by the raster renderer and the SVG writer so both draw the same.

use std::f64::consts::{PI, TAU};

use genha_color::Color;
use genha_doc::{
    BorderLines, BrandKit, CellRange, CellSpan, CellStyle, ChartEl, ChartKind, ChartPoint, Fill, LegendPosition, StrokeStyle, TableEl, TextAlign,
    TextStyle, ValueFormat,
};
use genha_geom::rounded_rect;
use genha_text::FontBook;
use kurbo::{Affine, Arc, BezPath, Circle, Point, Rect, Shape, Vec2};

/// Colours used when a chart has none and the brand offers too few.
pub const CHART_PALETTE: [Color; 8] = [
    Color::rgb(0x5b, 0x3c, 0xf5),
    Color::rgb(0xff, 0x6b, 0x5b),
    Color::rgb(0x14, 0xb8, 0xa6),
    Color::rgb(0xf5, 0xb8, 0x3c),
    Color::rgb(0x3c, 0x8d, 0xf5),
    Color::rgb(0xe8, 0x43, 0x93),
    Color::rgb(0x22, 0xc5, 0x5e),
    Color::rgb(0x8b, 0x5c, 0xf6),
];

/// One thing to draw.
#[derive(Clone, Debug, PartialEq)]
pub enum Prim {
    Fill {
        path: BezPath,
        color: Color,
    },
    /// An outline; `round` for round joins and caps (lines, progress arcs).
    Stroke {
        path: BezPath,
        color: Color,
        width: f64,
        round: bool,
    },
    Text(TextPrim),
}

/// Laid-out text: its glyph outlines and, for SVG `<text>`, its lines.
#[derive(Clone, Debug, PartialEq)]
pub struct TextPrim {
    pub path: BezPath,
    pub color: Color,
    /// Family, size, weight; `align` says which end of each line is its anchor.
    pub style: TextStyle,
    pub lines: Vec<TextLine>,
    /// Synthetic bold: width of the stroke drawn over the letters in `color` (0: none).
    pub embolden: f64,
    /// Where the lines go on the box (turned axis titles); `path` is already transformed.
    pub transform: Affine,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextLine {
    /// Left edge of the line.
    pub x: f64,
    pub width: f64,
    pub baseline: f64,
    pub text: String,
}

/// A chart or table ready to draw: an optional background over the whole box, then primitives
/// bottom to top.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    pub background: Option<Fill>,
    pub prims: Vec<Prim>,
}

#[derive(Clone, Copy)]
enum VAlign {
    Top,
    Middle,
    Bottom,
}

/// Collects primitives, laying text out with the fonts.
struct Builder<'a> {
    fonts: &'a FontBook,
    family: &'a str,
    prims: Vec<Prim>,
}

impl Builder<'_> {
    fn style(&self, size: f64, bold: bool, align: TextAlign) -> TextStyle {
        TextStyle { font_family: self.family.to_string(), font_size: size, bold, align, line_height: 1.2, ..TextStyle::default() }
    }

    /// Natural width of one line of `text`.
    fn measure(&self, text: &str, size: f64, bold: bool) -> f64 {
        genha_text::measure(self.fonts, text, &self.style(size, bold, TextAlign::Left)).0
    }

    fn fill(&mut self, path: BezPath, color: Color) {
        if color.a > 0 {
            self.prims.push(Prim::Fill { path, color });
        }
    }

    fn stroke(&mut self, path: BezPath, color: Color, width: f64, round: bool) {
        if color.a > 0 && width > 0.0 && width.is_finite() {
            self.prims.push(Prim::Stroke { path, color, width, round });
        }
    }

    /// Text anchored at (`x`, `y`): `h` picks which end of the lines sits at `x`, `v` which part
    /// of the block sits at `y`. Wraps at `wrap` (when positive) and keeps at most `max_lines`.
    /// Returns the block's box.
    #[allow(clippy::too_many_arguments)]
    fn text(&mut self, text: &str, size: f64, bold: bool, color: Color, at: Point, h: TextAlign, v: VAlign, wrap: f64, max_lines: usize) -> Rect {
        if text.trim().is_empty() || size <= 0.0 || !size.is_finite() {
            return Rect::from_origin_size(at, (0.0, 0.0));
        }
        let style = self.style(size, bold, h);
        let wrap = if wrap.is_finite() && wrap > 0.0 { wrap } else { f64::INFINITY };
        let (mut l, syn) = crate::layout_text(self.fonts, text, &style, wrap);
        l.lines.truncate(max_lines.max(1));
        let height = l.lines.iter().map(|ln| ln.height).sum::<f64>();
        let box_w = if wrap.is_finite() { wrap } else { l.width };
        let dx = at.x
            - box_w
                * match h {
                    TextAlign::Left => 0.0,
                    TextAlign::Center => 0.5,
                    TextAlign::Right => 1.0,
                };
        let dy = at.y
            - match v {
                VAlign::Top => 0.0,
                VAlign::Middle => height / 2.0,
                VAlign::Bottom => height,
            };
        let shift = Affine::translate((dx, dy));
        let mut path = BezPath::new();
        let mut lines = vec![];
        let (mut x0, mut x1) = (f64::INFINITY, f64::NEG_INFINITY);
        for ln in &l.lines {
            path.extend(shift * ln.path.clone());
            x0 = x0.min(ln.x + dx);
            x1 = x1.max(ln.x + ln.width + dx);
            lines.push(TextLine { x: ln.x + dx, width: ln.width, baseline: ln.baseline + dy, text: ln.text.trim_end().to_string() });
        }
        if color.a > 0 {
            self.prims.push(Prim::Text(TextPrim { path, color, style, lines, embolden: syn.embolden, transform: Affine::IDENTITY }));
        }
        if x0 > x1 {
            return Rect::from_origin_size(at, (0.0, 0.0));
        }
        Rect::new(x0, dy, x1, dy + height)
    }

    /// One line of text centred on `center`, turned a quarter to the left (reading upwards), at
    /// most `max_len` long.
    fn text_upwards(&mut self, text: &str, size: f64, color: Color, center: Point, max_len: f64) {
        let n = self.prims.len();
        self.text(text, size, false, color, center, TextAlign::Center, VAlign::Middle, max_len, 1);
        let xf = Affine::rotate_about(-PI / 2.0, center);
        if let Some(Prim::Text(tp)) = self.prims.get_mut(n) {
            tp.path.apply_affine(xf);
            tp.transform = xf;
        }
    }
}

fn sane(v: f64, lo: f64, hi: f64, default: f64) -> f64 {
    if v.is_finite() { v.clamp(lo, hi) } else { default }
}

/// The colours a chart draws with: its own, else the brand's (too-light and too-dark ones left
/// out) followed by the built-in palette.
pub fn chart_colors(c: &ChartEl, brand: &BrandKit) -> Vec<Color> {
    if !c.colors.is_empty() {
        return c.colors.clone();
    }
    let mut out: Vec<Color> = brand.colors.iter().map(|b| b.color).filter(|col| (0.03..0.75).contains(&col.luminance()) && col.a > 0).collect();
    for p in CHART_PALETTE {
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

/// The colour of series `i`: its own, else the next of the chart's colours (then the built-in
/// palette, so series differ even when the chart has a single colour).
pub fn series_color(c: &ChartEl, brand: &BrandKit, i: usize) -> Color {
    if let Some(col) = c.series.get(i).and_then(|s| s.color) {
        return col;
    }
    let mut colors = chart_colors(c, brand);
    for p in CHART_PALETTE {
        if !colors.contains(&p) {
            colors.push(p);
        }
    }
    colors.get(i % colors.len().max(1)).copied().unwrap_or(CHART_PALETTE[0])
}

/// `v` with thousands separators and at most two decimals.
pub fn format_number(v: f64) -> String {
    format_decimals(v, None)
}

/// `v` with thousands separators and `decimals` decimals (`None`: at most two, trailing zeros
/// dropped).
pub fn format_decimals(v: f64, decimals: Option<u8>) -> String {
    if !v.is_finite() {
        return "0".into();
    }
    if v.abs() >= 1e15 {
        return format!("{v:.2e}");
    }
    let d = usize::from(decimals.unwrap_or(2).min(6));
    let s = format!("{:.d$}", v.abs());
    let (int, frac) = s.split_once('.').unwrap_or((&s, ""));
    let frac = if decimals.is_none() { frac.trim_end_matches('0') } else { frac };
    let mut grouped = String::new();
    for (i, ch) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let neg = v < 0.0 && (grouped != "0" || frac.chars().any(|c| c != '0'));
    format!("{}{}{}{}", if neg { "-" } else { "" }, grouped, if frac.is_empty() { "" } else { "." }, frac)
}

/// A short axis label: `1.5k`, `2M`.
fn format_compact(v: f64) -> String {
    let a = v.abs();
    let (d, suffix) = if a >= 1e9 {
        (1e9, "B")
    } else if a >= 1e6 {
        (1e6, "M")
    } else if a >= 1e4 {
        (1e3, "k")
    } else {
        return format_number(v);
    };
    let n = format!("{:.1}", v / d);
    format!("{}{suffix}", n.trim_end_matches('0').trim_end_matches('.'))
}

fn format_percent(share: f64, decimals: Option<u8>) -> String {
    let p = share * 100.0;
    if !p.is_finite() {
        return "0%".into();
    }
    if let Some(d) = decimals {
        let d = usize::from(d.min(6));
        return format!("{p:.d$}%");
    }
    if p.abs() >= 10.0 || p == 0.0 { format!("{p:.0}%") } else { format!("{}%", format!("{p:.1}").trim_end_matches('0').trim_end_matches('.')) }
}

/// `text` (a formatted number) after the chart's currency symbol, its sign first.
fn with_currency(c: &ChartEl, text: &str) -> String {
    match text.strip_prefix('-') {
        Some(magnitude) => format!("-{}{magnitude}", c.currency),
        None => format!("{}{text}", c.currency),
    }
}

/// How a value is written on the chart (`total`: what a percentage is a share of).
pub fn value_text(c: &ChartEl, v: f64, total: f64) -> String {
    match c.value_format {
        ValueFormat::Number => format_decimals(v, c.decimals),
        ValueFormat::Percent => format_percent(if total.abs() > 0.0 { v / total } else { 0.0 }, c.decimals),
        ValueFormat::Currency => with_currency(c, &format_decimals(v, c.decimals)),
    }
}

/// A value on the axis.
fn tick_text(c: &ChartEl, v: f64) -> String {
    match c.value_format {
        ValueFormat::Currency => with_currency(c, &format_compact(v)),
        _ => format_compact(v),
    }
}

/// A round step of about `range / ticks`.
fn nice_step(range: f64, ticks: f64) -> f64 {
    let raw = range / ticks.max(1.0);
    if !raw.is_finite() || raw <= 0.0 {
        return 1.0;
    }
    let mag = 10f64.powf(raw.log10().floor());
    let norm = raw / mag;
    let nice = if norm <= 1.0 {
        1.0
    } else if norm <= 2.0 {
        2.0
    } else if norm <= 2.5 {
        2.5
    } else if norm <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * mag
}

/// The value scale of an axis chart: bottom, top and tick step. A set minimum or maximum is
/// kept as it is; otherwise the scale takes in zero and the data and ends on a round step.
fn value_scale(c: &ChartEl, values: &[f64], ticks: f64) -> (f64, f64, f64) {
    let mut lo = values.iter().copied().filter(|v| v.is_finite()).fold(0.0_f64, f64::min);
    let mut hi = values.iter().copied().filter(|v| v.is_finite()).fold(0.0_f64, f64::max);
    let fixed_lo = c.min.filter(|m| m.is_finite());
    if let Some(m) = fixed_lo {
        lo = m;
        hi = hi.max(lo);
    }
    let fixed_hi = c.max.filter(|m| m.is_finite() && *m > lo);
    if let Some(m) = fixed_hi {
        hi = m;
    }
    if hi - lo <= 0.0 || !(hi - lo).is_finite() {
        hi = lo + 1.0;
    }
    let step = nice_step(hi - lo, ticks);
    if fixed_lo.is_none() {
        lo = (lo / step).floor() * step;
    }
    if fixed_hi.is_none() {
        hi = (hi / step).ceil() * step;
    }
    if hi <= lo || !hi.is_finite() || !lo.is_finite() {
        hi = lo + step;
    }
    (lo, hi, step)
}

/// The ticks of an axis: the round values, and a set minimum or maximum that is not one of
/// them (unless a round value is close by).
fn axis_ticks(c: &ChartEl, lo: f64, hi: f64, step: f64) -> Vec<f64> {
    let mut ticks = tick_values(lo, hi, step);
    for (fixed, end) in [(c.min.is_some(), lo), (c.max.is_some(), hi)] {
        if fixed && end.is_finite() && ticks.iter().all(|t| (t - end).abs() > step * 0.45) {
            ticks.push(end);
        }
    }
    ticks
}

/// The round values from `lo` to `hi` that get a tick (at most 41).
fn tick_values(lo: f64, hi: f64, step: f64) -> Vec<f64> {
    if !(step > 0.0 && step.is_finite() && lo.is_finite() && hi.is_finite()) {
        return vec![];
    }
    let first = (lo / step - 1e-9).ceil();
    (0..=40).map(|k| (first + k as f64) * step).take_while(|v| *v <= hi + step * 1e-6).collect()
}

/// A bar with its far end rounded: `up` rounds the top corners, else the bottom ones (or, for
/// horizontal bars, `right` rounds the right end).
fn bar_path(r: Rect, radius: f64, end: BarEnd) -> BezPath {
    let rad = radius.max(0.0).min(r.width() / 2.0).min(r.height() / 2.0);
    if rad <= 0.01 {
        return r.to_path(0.1);
    }
    let k = 0.552_284_75 * rad;
    let mut p = BezPath::new();
    let (x0, y0, x1, y1) = (r.x0, r.y0, r.x1, r.y1);
    match end {
        BarEnd::Top => {
            p.move_to((x0, y1));
            p.line_to((x0, y0 + rad));
            p.curve_to((x0, y0 + rad - k), (x0 + rad - k, y0), (x0 + rad, y0));
            p.line_to((x1 - rad, y0));
            p.curve_to((x1 - rad + k, y0), (x1, y0 + rad - k), (x1, y0 + rad));
            p.line_to((x1, y1));
        }
        BarEnd::Bottom => {
            p.move_to((x0, y0));
            p.line_to((x1, y0));
            p.line_to((x1, y1 - rad));
            p.curve_to((x1, y1 - rad + k), (x1 - rad + k, y1), (x1 - rad, y1));
            p.line_to((x0 + rad, y1));
            p.curve_to((x0 + rad - k, y1), (x0, y1 - rad + k), (x0, y1 - rad));
        }
        BarEnd::Right => {
            p.move_to((x0, y0));
            p.line_to((x1 - rad, y0));
            p.curve_to((x1 - rad + k, y0), (x1, y0 + rad - k), (x1, y0 + rad));
            p.line_to((x1, y1 - rad));
            p.curve_to((x1, y1 - rad + k), (x1 - rad + k, y1), (x1 - rad, y1));
            p.line_to((x0, y1));
        }
        BarEnd::Left => {
            p.move_to((x1, y0));
            p.line_to((x1, y1));
            p.line_to((x0 + rad, y1));
            p.curve_to((x0 + rad - k, y1), (x0, y1 - rad + k), (x0, y1 - rad));
            p.line_to((x0, y0 + rad));
            p.curve_to((x0, y0 + rad - k), (x0 + rad - k, y0), (x0 + rad, y0));
        }
    }
    p.close_path();
    p
}

#[derive(Clone, Copy)]
enum BarEnd {
    Top,
    Bottom,
    Right,
    Left,
}

fn line(a: Point, b: Point) -> BezPath {
    let mut p = BezPath::new();
    p.move_to(a);
    p.line_to(b);
    p
}

/// An arc from `start` (radians, 0 = 3 o'clock, clockwise on the page) sweeping `sweep`.
fn arc(center: Point, r: f64, start: f64, sweep: f64) -> Arc {
    Arc { center, radii: Vec2::new(r, r), start_angle: start, sweep_angle: sweep, x_rotation: 0.0 }
}

fn arc_point(center: Point, r: f64, angle: f64) -> Point {
    center + Vec2::new(r * angle.cos(), r * angle.sin())
}

/// A pie slice (or ring segment when `inner` > 0).
fn slice_path(center: Point, outer: f64, inner: f64, start: f64, sweep: f64) -> BezPath {
    let mut p = BezPath::new();
    if sweep >= TAU - 1e-9 {
        // A whole circle (or ring: the inner circle wound the other way).
        p.move_to(arc_point(center, outer, start));
        p.extend(arc(center, outer, start, TAU).append_iter(0.1));
        p.close_path();
        if inner > 0.0 {
            p.move_to(arc_point(center, inner, start));
            p.extend(arc(center, inner, start, -TAU).append_iter(0.1));
            p.close_path();
        }
        return p;
    }
    p.move_to(arc_point(center, outer, start));
    p.extend(arc(center, outer, start, sweep).append_iter(0.1));
    if inner > 0.0 {
        p.line_to(arc_point(center, inner, start + sweep));
        p.extend(arc(center, inner, start + sweep, -sweep).append_iter(0.1));
    } else {
        p.line_to(center);
    }
    p.close_path();
    p
}

/// Legend entries laid out in rows under the chart (or in a column at its right).
struct Legend {
    items: Vec<(Color, String, f64)>,
    size: f64,
}

impl Legend {
    fn swatch(&self) -> f64 {
        self.size * 0.75
    }
    fn row_h(&self) -> f64 {
        self.size * 1.5
    }
    fn item_w(&self, text_w: f64) -> f64 {
        self.swatch() + self.size * 0.4 + text_w
    }

    /// Rows of item indices fitting `width`.
    fn rows(&self, width: f64) -> Vec<Vec<usize>> {
        let gap = self.size;
        let mut rows: Vec<Vec<usize>> = vec![];
        let mut x = 0.0;
        for (i, (_, _, tw)) in self.items.iter().enumerate() {
            let iw = self.item_w(*tw);
            match rows.last_mut() {
                Some(row) if x + gap + iw <= width => {
                    row.push(i);
                    x += gap + iw;
                }
                _ => {
                    rows.push(vec![i]);
                    x = iw;
                }
            }
        }
        rows
    }

    fn height_below(&self, width: f64) -> f64 {
        self.rows(width).len() as f64 * self.row_h()
    }

    fn column_width(&self) -> f64 {
        self.items.iter().map(|(_, _, tw)| self.item_w(*tw)).fold(0.0, f64::max)
    }

    /// Draws the rows centred in `area`'s width, from its top.
    fn draw_below(&self, b: &mut Builder, area: Rect, text: Color) {
        let gap = self.size;
        let rows = self.rows(area.width());
        let total_h = rows.len() as f64 * self.row_h();
        let mut y = area.y0 + (area.height() - total_h).max(0.0) / 2.0;
        for row in rows {
            let row_w: f64 = row.iter().filter_map(|i| self.items.get(*i)).map(|(_, _, tw)| self.item_w(*tw)).sum::<f64>()
                + gap * row.len().saturating_sub(1) as f64;
            let mut x = area.center().x - row_w / 2.0;
            for i in row {
                let Some((color, label, tw)) = self.items.get(i) else { continue };
                self.entry(b, Point::new(x, y + self.row_h() / 2.0), *color, label, *tw, text);
                x += self.item_w(*tw) + gap;
            }
            y += self.row_h();
        }
    }

    /// Draws the entries as a column vertically centred in `area`.
    fn draw_column(&self, b: &mut Builder, area: Rect, text: Color) {
        let total_h = self.items.len() as f64 * self.row_h();
        let mut y = area.y0 + (area.height() - total_h).max(0.0) / 2.0;
        for (color, label, tw) in &self.items {
            self.entry(b, Point::new(area.x0, y + self.row_h() / 2.0), *color, label, (*tw).min(area.width()), text);
            y += self.row_h();
        }
    }

    fn entry(&self, b: &mut Builder, left_mid: Point, color: Color, label: &str, text_w: f64, text: Color) {
        let s = self.swatch();
        let sq = Rect::new(left_mid.x, left_mid.y - s / 2.0, left_mid.x + s, left_mid.y + s / 2.0);
        b.fill(rounded_rect(sq, s * 0.25), color);
        let at = Point::new(sq.x1 + self.size * 0.4, left_mid.y);
        b.text(label, self.size, false, text, at, TextAlign::Left, VAlign::Middle, text_w + 1.0, 1);
    }
}

/// A chart's data ready to draw: the categories and the series this kind shows.
struct Data {
    cats: Vec<String>,
    series: Vec<SeriesView>,
    /// One series whose bars (or slices) each take the next chart colour.
    per_category: bool,
}

struct SeriesView {
    name: String,
    values: Vec<f64>,
    color: Color,
}

impl Data {
    fn of(c: &ChartEl, brand: &BrandKit) -> Data {
        let cats: Vec<String> = c.categories.iter().take(genha_doc::MAX_CHART_POINTS).cloned().collect();
        let n = cats.len();
        let series: Vec<SeriesView> = c
            .series
            .iter()
            .take(c.drawn_series().min(genha_doc::MAX_CHART_SERIES))
            .enumerate()
            .map(|(i, s)| SeriesView { name: s.name.clone(), values: (0..n).map(|k| c.value(i, k)).collect(), color: series_color(c, brand, i) })
            .collect();
        let per_category = series.len() == 1
            && c.series.first().is_some_and(|s| s.color.is_none())
            && matches!(c.kind, ChartKind::Bar | ChartKind::HorizontalBar | ChartKind::Pie | ChartKind::Donut);
        Data { cats, series, per_category }
    }

    fn value(&self, s: usize, k: usize) -> f64 {
        self.series.get(s).and_then(|s| s.values.get(k)).copied().unwrap_or(0.0)
    }

    /// The sum of a series (what its percentages are shares of).
    fn total(&self, s: usize) -> f64 {
        self.series.get(s).map_or(0.0, |s| s.values.iter().sum())
    }

    /// The values the value axis must hold: every value, or the stacks' tops and bottoms.
    fn extent(&self, stacked: bool) -> Vec<f64> {
        if !stacked {
            return self.series.iter().flat_map(|s| s.values.iter().copied()).collect();
        }
        (0..self.cats.len())
            .flat_map(|k| {
                let (mut pos, mut neg) = (0.0, 0.0);
                for s in 0..self.series.len() {
                    let v = self.value(s, k);
                    if v >= 0.0 { pos += v } else { neg += v }
                }
                [pos, neg]
            })
            .collect()
    }

    /// The legend: series names, or the categories of a one-series chart coloured per category.
    fn legend(&self, b: &Builder, size: f64, color_at: &dyn Fn(usize) -> Color) -> Legend {
        let items: Vec<(Color, String, f64)> = if self.per_category {
            self.cats.iter().enumerate().map(|(i, l)| (color_at(i), l.clone(), b.measure(l, size, false))).collect()
        } else {
            self.series.iter().map(|s| (s.color, s.name.clone(), b.measure(&s.name, size, false))).collect()
        };
        Legend { items, size }
    }
}

/// The chart `c` in a `w` × `h` box.
pub fn chart_scene(fonts: &FontBook, brand: &BrandKit, c: &ChartEl, w: f64, h: f64) -> Scene {
    let mut scene = Scene { background: (c.background != Fill::None).then(|| c.background.clone()), prims: vec![] };
    if !(w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0) {
        return scene;
    }
    let mut b = Builder { fonts, family: &c.font_family, prims: vec![] };
    let colors = chart_colors(c, brand);
    let color_at = |i: usize| colors.get(i % colors.len().max(1)).copied().unwrap_or(CHART_PALETTE[0]);
    let fs = sane(c.font_size, 1.0, 2000.0, 24.0).min(w.min(h) * 0.15).max(1.0);
    let pad = fs * 0.5;
    let area = Rect::new(pad, pad, w - pad, h - pad);
    let d = Data::of(c, brand);
    let text = c.text_color;
    match c.kind {
        ChartKind::Pie | ChartKind::Donut => {
            let mut plot = area;
            if c.show_legend && !d.cats.is_empty() {
                let legend = d.legend(&b, fs * 0.8, &color_at);
                plot = place_legend(&mut b, &legend, area, c.legend_position, true, fs, text);
            }
            let first = d.series.first().map(|s| s.color).unwrap_or(color_at(0));
            let slice_color = |i: usize| if d.per_category { color_at(i) } else { first };
            pie(&mut b, c, &c.points(), plot, fs, &slice_color, separator_color(c));
        }
        ChartKind::Progress => {
            // A bar's text follows its own height rather than the box's short side.
            let fs = if w >= h * 2.0 { sane(c.font_size, 1.0, 2000.0, 24.0).min(h * 0.4).max(1.0) } else { fs };
            let color = c.series.first().and_then(|s| s.color).unwrap_or(color_at(0));
            progress(&mut b, c, &c.points(), Rect::new(0.0, 0.0, w, h), fs, color);
        }
        ChartKind::Bar | ChartKind::HorizontalBar | ChartKind::Line | ChartKind::Area => {
            let mut plot_area = area;
            if c.show_legend && !d.series.is_empty() && !d.cats.is_empty() {
                let legend = d.legend(&b, fs * 0.8, &color_at);
                plot_area = place_legend(&mut b, &legend, area, c.legend_position, false, fs, text);
            }
            if c.kind == ChartKind::HorizontalBar {
                horizontal_bars(&mut b, c, &d, plot_area, fs, &color_at);
            } else {
                vertical(&mut b, c, &d, plot_area, fs, &color_at);
            }
        }
    }
    scene.prims = b.prims;
    scene
}

/// Draws the legend at `pos` in `area` and returns what is left for the plot.
fn place_legend(b: &mut Builder, legend: &Legend, area: Rect, pos: LegendPosition, pie: bool, fs: f64, text: Color) -> Rect {
    let col_w = legend.column_width();
    let fits_right = col_w < area.width() * 0.45;
    let pos = match pos {
        LegendPosition::Auto if pie && area.width() > area.height() * 1.3 && fits_right => LegendPosition::Right,
        LegendPosition::Auto => LegendPosition::Bottom,
        LegendPosition::Right if !fits_right => LegendPosition::Bottom,
        p => p,
    };
    let mut plot = area;
    match pos {
        LegendPosition::Right => {
            legend.draw_column(b, Rect::new(area.x1 - col_w, area.y0, area.x1, area.y1), text);
            plot.x1 -= col_w + fs;
        }
        LegendPosition::Top => {
            let lh = legend.height_below(area.width()).min(area.height() * 0.4);
            legend.draw_below(b, Rect::new(area.x0, area.y0, area.x1, area.y0 + lh), text);
            plot.y0 += lh + fs * 0.5;
        }
        _ => {
            let lh = legend.height_below(area.width()).min(area.height() * 0.4);
            legend.draw_below(b, Rect::new(area.x0, area.y1 - lh, area.x1, area.y1), text);
            plot.y1 -= lh + fs * 0.5;
        }
    }
    if plot.width() < 1.0 || plot.height() < 1.0 { area } else { plot }
}

/// The colour between pie slices: the chart's background, else white.
fn separator_color(c: &ChartEl) -> Color {
    match &c.background {
        Fill::Solid { color } if color.a == 255 => *color,
        _ => Color::WHITE,
    }
}

fn no_data(b: &mut Builder, area: Rect, fs: f64, text: Color) {
    b.text("No data", fs, false, text.with_alpha(140), area.center(), TextAlign::Center, VAlign::Middle, 0.0, 1);
}

/// One bar (or a stack's segment) along the value axis, placed within its category's slot.
struct BarSpec {
    cat: usize,
    series: usize,
    /// Where the bar starts and ends on the value axis.
    from: f64,
    to: f64,
    /// Its centre's offset from the slot's centre, and its thickness, in slots.
    offset: f64,
    thickness: f64,
    /// The far end is rounded (the top of a stack, or every bar side by side).
    rounded: bool,
    value: f64,
}

/// The bars of every series: side by side in each category, or stacked.
fn bar_specs(d: &Data, stacked: bool) -> Vec<BarSpec> {
    let m = d.series.len();
    let mut out = vec![];
    for k in 0..d.cats.len() {
        if stacked {
            let (mut pos, mut neg) = (0.0, 0.0);
            let last_pos = (0..m).rev().find(|s| d.value(*s, k) > 0.0);
            let last_neg = (0..m).rev().find(|s| d.value(*s, k) < 0.0);
            for s in 0..m {
                let v = d.value(s, k);
                if v == 0.0 {
                    continue;
                }
                let acc = if v > 0.0 { &mut pos } else { &mut neg };
                let from = *acc;
                *acc += v;
                let rounded = Some(s) == if v > 0.0 { last_pos } else { last_neg };
                out.push(BarSpec { cat: k, series: s, from, to: *acc, offset: 0.0, thickness: 0.62, rounded, value: v });
            }
        } else {
            let group = if m > 1 { 0.8 } else { 0.62 };
            let each = group / m.max(1) as f64;
            for s in 0..m {
                let v = d.value(s, k);
                let offset = -group / 2.0 + each * (s as f64 + 0.5);
                let thickness = if m > 1 { each * 0.88 } else { each };
                out.push(BarSpec { cat: k, series: s, from: 0.0, to: v, offset, thickness, rounded: true, value: v });
            }
        }
    }
    out
}

/// Room taken by the axis titles: (height under the plot, width beside it) and their size.
fn title_room(c: &ChartEl, fs: f64) -> (f64, f64, f64) {
    let size = fs * 0.85;
    let room = size * 1.25 + fs * 0.3;
    (if c.x_title.trim().is_empty() { 0.0 } else { room }, if c.y_title.trim().is_empty() { 0.0 } else { room }, size)
}

/// Draws the axis titles: the x title centred under `plot` at the bottom of `area`, the y title
/// reading upwards at the left of `area`, centred on `plot`.
fn axis_titles(b: &mut Builder, c: &ChartEl, area: Rect, plot: Rect, size: f64) {
    let ink = c.text_color.with_alpha(220);
    if !c.x_title.trim().is_empty() {
        b.text(&c.x_title, size, false, ink, Point::new(plot.center().x, area.y1), TextAlign::Center, VAlign::Bottom, area.width(), 1);
    }
    if !c.y_title.trim().is_empty() {
        b.text_upwards(&c.y_title, size, ink, Point::new(area.x0 + size * 0.625, plot.center().y), area.height());
    }
}

/// Vertical bars, lines and areas.
fn vertical(b: &mut Builder, c: &ChartEl, d: &Data, outer: Rect, fs: f64, color_at: &dyn Fn(usize) -> Color) {
    let text = c.text_color;
    let tick_fs = fs * 0.75;
    let n = d.cats.len();
    let m = d.series.len();
    let stacked = c.stacked && m > 1;
    let (x_title_h, y_title_w, title_fs) = title_room(c, fs);
    let area = Rect::new(outer.x0 + y_title_w, outer.y0, outer.x1, (outer.y1 - x_title_h).max(outer.y0 + 1.0));
    // Room for category labels under the plot.
    let label_fs = fs * 0.8;
    let mut label_lines = 0usize;
    if c.show_labels && n > 0 {
        let slot = (area.width() * 0.85) / n as f64;
        for l in &d.cats {
            let lines = if b.measure(l, label_fs, false) > slot * 0.95 { 2 } else { 1 };
            label_lines = label_lines.max(if l.trim().is_empty() { 0 } else { lines });
        }
    }
    let label_h = if label_lines > 0 { label_lines as f64 * label_fs * 1.2 + fs * 0.4 } else { 0.0 };
    let top_room = if c.show_values { fs * 1.3 } else { fs * 0.5 };
    let plot_h = (area.height() - label_h - top_room).max(1.0);
    let (lo, hi, step) = value_scale(c, &d.extent(stacked), (plot_h / (fs * 2.5)).clamp(2.0, 8.0));
    let ticks = axis_ticks(c, lo, hi, step);
    let tick_w = ticks.iter().map(|v| b.measure(&tick_text(c, *v), tick_fs, false)).fold(0.0, f64::max);
    let plot =
        Rect::new(area.x0 + tick_w + fs * 0.5, area.y0 + top_room, area.x1.max(area.x0 + tick_w + fs * 0.5 + 1.0), area.y0 + top_room + plot_h);
    let y_of = |v: f64| plot.y1 - (v.clamp(lo, hi) - lo) / (hi - lo) * plot.height();
    let grid_w = (fs * 0.05).max(1.0);
    for v in &ticks {
        let y = y_of(*v);
        if c.gridlines {
            b.stroke(line(Point::new(plot.x0, y), Point::new(plot.x1, y)), text.with_alpha(28), grid_w, false);
        }
        b.text(&tick_text(c, *v), tick_fs, false, text.with_alpha(150), Point::new(plot.x0 - fs * 0.4, y), TextAlign::Right, VAlign::Middle, 0.0, 1);
    }
    axis_titles(b, c, outer, plot, title_fs);
    let base_y = y_of(0.0);
    if n == 0 || m == 0 {
        no_data(b, plot, fs, text);
        return;
    }
    let slot = plot.width() / n as f64;
    let cx = |i: usize| plot.x0 + slot * (i as f64 + 0.5);
    let label_size = fs * 0.75;
    let bar_color = |s: usize, k: usize| if d.per_category { color_at(k) } else { d.series.get(s).map_or(color_at(s), |x| x.color) };
    match c.kind {
        ChartKind::Bar => {
            for bar in bar_specs(d, stacked) {
                let bw = (slot * bar.thickness).max(1.0);
                let x = cx(bar.cat) + bar.offset * slot;
                let (y0, y1) = (y_of(bar.from), y_of(bar.to));
                let r = Rect::new(x - bw / 2.0, y0.min(y1), x + bw / 2.0, y0.max(y1));
                let up = bar.to >= bar.from;
                let radius = if bar.rounded { (bw * 0.18).min(fs * 0.35) } else { 0.0 };
                let color = bar_color(bar.series, bar.cat);
                b.fill(bar_path(r, radius, if up { BarEnd::Top } else { BarEnd::Bottom }), color);
                if !c.show_values {
                    continue;
                }
                let label = value_text(c, bar.value, d.total(bar.series));
                let tw = b.measure(&label, label_size, true);
                if stacked {
                    // Inside the segment, when it fits.
                    if r.height() >= label_size * 1.2 && tw <= bw * 0.95 {
                        b.text(&label, label_size, true, color.readable_text(), r.center(), TextAlign::Center, VAlign::Middle, 0.0, 1);
                    }
                    continue;
                }
                let size = if m > 1 && tw > bw * 1.05 { label_size * bw * 1.05 / tw } else { label_size };
                if size < fs * 0.35 {
                    continue;
                }
                let (at, v) = if up { (Point::new(x, r.y0 - fs * 0.25), VAlign::Bottom) } else { (Point::new(x, r.y1 + fs * 0.2), VAlign::Top) };
                b.text(&label, size, true, text, at, TextAlign::Center, v, 0.0, 1);
            }
        }
        _ => lines_and_areas(b, c, d, stacked, &cx, &y_of, base_y, fs),
    }
    // The zero line, over the bars' feet.
    b.stroke(line(Point::new(plot.x0, base_y), Point::new(plot.x1, base_y)), text.with_alpha(90), grid_w * 1.5, false);
    if label_lines > 0 {
        for (i, l) in d.cats.iter().enumerate() {
            let at = Point::new(cx(i), plot.y1 + fs * 0.4);
            b.text(l, label_fs, false, text, at, TextAlign::Center, VAlign::Top, slot * 0.95, label_lines);
        }
    }
}

/// The lines (and the areas under them) of every series.
#[allow(clippy::too_many_arguments)]
fn lines_and_areas(
    b: &mut Builder,
    c: &ChartEl,
    d: &Data,
    stacked: bool,
    cx: &dyn Fn(usize) -> f64,
    y_of: &dyn Fn(f64) -> f64,
    base_y: f64,
    fs: f64,
) {
    let n = d.cats.len();
    let m = d.series.len();
    let lw = (fs * if m > 1 { 0.11 } else { 0.14 }).max(1.5);
    // Each series' heights: its values, or the running sum of the series up to it.
    let mut tops: Vec<Vec<f64>> = vec![];
    for s in 0..m {
        let row: Vec<f64> = (0..n)
            .map(|k| if stacked { tops.last().and_then(|p| p.get(k)).copied().unwrap_or(0.0) + d.value(s, k) } else { d.value(s, k) })
            .collect();
        tops.push(row);
    }
    let pts: Vec<Vec<Point>> = tops.iter().map(|row| row.iter().enumerate().map(|(k, v)| Point::new(cx(k), y_of(*v))).collect()).collect();
    if c.kind == ChartKind::Area {
        for (s, row) in pts.iter().enumerate() {
            let (Some(first), Some(last)) = (row.first(), row.last()) else { continue };
            let color = d.series.get(s).map_or(Color::BLACK, |x| x.color);
            let mut a = BezPath::new();
            a.move_to(*first);
            for p in row.iter().skip(1) {
                a.line_to(*p);
            }
            match pts.get(s.wrapping_sub(1)).filter(|_| stacked && s > 0) {
                // A band between this series and the one under it.
                Some(below) => {
                    for p in below.iter().rev() {
                        a.line_to(*p);
                    }
                }
                None => {
                    a.line_to((last.x, base_y));
                    a.line_to((first.x, base_y));
                }
            }
            a.close_path();
            let alpha = if stacked { 170 } else { 80 };
            b.fill(a, color.with_alpha(((u16::from(color.a) * alpha) / 255) as u8));
        }
    }
    let markers = n <= 60;
    for (s, row) in pts.iter().enumerate() {
        let color = d.series.get(s).map_or(Color::BLACK, |x| x.color);
        if row.len() > 1 {
            let mut l = BezPath::new();
            for (i, p) in row.iter().enumerate() {
                if i == 0 {
                    l.move_to(*p);
                } else {
                    l.line_to(*p);
                }
            }
            b.stroke(l, color, lw, true);
        }
        for (k, p) in row.iter().enumerate() {
            if markers || row.len() == 1 {
                b.fill(Circle::new(*p, lw * 1.9).to_path(0.05), color);
                b.fill(Circle::new(*p, lw * 0.9).to_path(0.05), Color::WHITE);
            }
            if !c.show_values {
                continue;
            }
            let label = value_text(c, d.value(s, k), d.total(s));
            match pts.get(s.wrapping_sub(1)).and_then(|below| below.get(k)).filter(|_| stacked && s > 0) {
                // Stacked: inside the band, under its own point, when there is room.
                Some(under) => {
                    if under.y - p.y >= fs * 0.75 * 1.6 + lw * 2.0 {
                        let at = Point::new(p.x, (p.y + under.y) / 2.0 + lw);
                        b.text(&label, fs * 0.75, true, c.text_color, at, TextAlign::Center, VAlign::Middle, 0.0, 1);
                    }
                }
                None if stacked => {
                    if base_y - p.y >= fs * 0.75 * 1.6 + lw * 2.0 {
                        let at = Point::new(p.x, (p.y + base_y) / 2.0 + lw);
                        b.text(&label, fs * 0.75, true, c.text_color, at, TextAlign::Center, VAlign::Middle, 0.0, 1);
                    }
                }
                None => {
                    let at = Point::new(p.x, p.y - lw * 1.9 - fs * 0.2);
                    b.text(&label, fs * 0.75, true, c.text_color, at, TextAlign::Center, VAlign::Bottom, 0.0, 1);
                }
            }
        }
    }
}

fn horizontal_bars(b: &mut Builder, c: &ChartEl, d: &Data, outer: Rect, fs: f64, color_at: &dyn Fn(usize) -> Color) {
    let text = c.text_color;
    let m = d.series.len();
    let stacked = c.stacked && m > 1;
    let (x_title_h, y_title_w, title_fs) = title_room(c, fs);
    let area = Rect::new(outer.x0 + y_title_w, outer.y0, outer.x1, (outer.y1 - x_title_h).max(outer.y0 + 1.0));
    let label_fs = fs * 0.8;
    let tick_fs = fs * 0.75;
    let value_fs = fs * 0.75;
    let label_w = if c.show_labels { d.cats.iter().map(|l| b.measure(l, label_fs, false)).fold(0.0, f64::max).min(area.width() * 0.3) } else { 0.0 };
    let value_w = if c.show_values && !stacked {
        let mut w: f64 = 0.0;
        for s in 0..m {
            for k in 0..d.cats.len() {
                w = w.max(b.measure(&value_text(c, d.value(s, k), d.total(s)), value_fs, true));
            }
        }
        w + fs * 0.4
    } else {
        fs * 0.3
    };
    let tick_h = tick_fs * 1.2 + fs * 0.3;
    let x0 = area.x0 + if label_w > 0.0 { label_w + fs * 0.5 } else { 0.0 };
    let plot = Rect::new(x0, area.y0, (area.x1 - value_w).max(x0 + 1.0), (area.y1 - tick_h).max(area.y0 + 1.0));
    let (lo, hi, step) = value_scale(c, &d.extent(stacked), (plot.width() / (fs * 4.0)).clamp(2.0, 8.0));
    let x_of = |v: f64| plot.x0 + (v.clamp(lo, hi) - lo) / (hi - lo) * plot.width();
    let grid_w = (fs * 0.05).max(1.0);
    for v in axis_ticks(c, lo, hi, step) {
        let x = x_of(v);
        if c.gridlines {
            b.stroke(line(Point::new(x, plot.y0), Point::new(x, plot.y1)), text.with_alpha(28), grid_w, false);
        }
        b.text(&tick_text(c, v), tick_fs, false, text.with_alpha(150), Point::new(x, plot.y1 + fs * 0.3), TextAlign::Center, VAlign::Top, 0.0, 1);
    }
    axis_titles(b, c, outer, plot, title_fs);
    let base_x = x_of(0.0);
    if d.cats.is_empty() || m == 0 {
        no_data(b, plot, fs, text);
        return;
    }
    let slot = plot.height() / d.cats.len() as f64;
    let cy = |k: usize| plot.y0 + slot * (k as f64 + 0.5);
    for bar in bar_specs(d, stacked) {
        let bh = (slot * bar.thickness).max(1.0);
        let y = cy(bar.cat) + bar.offset * slot;
        let (xa, xb) = (x_of(bar.from), x_of(bar.to));
        let r = Rect::new(xa.min(xb), y - bh / 2.0, xa.max(xb), y + bh / 2.0);
        let right = bar.to >= bar.from;
        let radius = if bar.rounded { (bh * 0.18).min(fs * 0.35) } else { 0.0 };
        let color = if d.per_category { color_at(bar.cat) } else { d.series.get(bar.series).map_or(color_at(bar.series), |x| x.color) };
        b.fill(bar_path(r, radius, if right { BarEnd::Right } else { BarEnd::Left }), color);
        if !c.show_values {
            continue;
        }
        let label = value_text(c, bar.value, d.total(bar.series));
        let size = if m > 1 { value_fs.min(bh * 0.8) } else { value_fs };
        if stacked {
            let tw = b.measure(&label, size, true);
            if r.width() >= tw + fs * 0.3 && bh >= size {
                b.text(&label, size, true, color.readable_text(), r.center(), TextAlign::Center, VAlign::Middle, 0.0, 1);
            }
            continue;
        }
        let (at, h) = if right { (Point::new(r.x1 + fs * 0.25, y), TextAlign::Left) } else { (Point::new(r.x0 - fs * 0.25, y), TextAlign::Right) };
        b.text(&label, size, true, text, at, h, VAlign::Middle, 0.0, 1);
    }
    if label_w > 0.0 {
        for (k, l) in d.cats.iter().enumerate() {
            b.text(l, label_fs, false, text, Point::new(plot.x0 - fs * 0.5, cy(k)), TextAlign::Right, VAlign::Middle, label_w + 1.0, 2);
        }
    }
    b.stroke(line(Point::new(base_x, plot.y0), Point::new(base_x, plot.y1)), text.with_alpha(90), grid_w * 1.5, false);
}

fn pie(b: &mut Builder, c: &ChartEl, data: &[ChartPoint], area: Rect, fs: f64, color_at: &dyn Fn(usize) -> Color, sep: Color) {
    let text = c.text_color;
    let plot = area;
    let center = plot.center();
    let r = (plot.width().min(plot.height()) / 2.0).max(1.0);
    let hole = if c.kind == ChartKind::Donut { sane(c.donut_hole, 0.0, 0.9, 0.55) } else { 0.0 };
    let inner = r * hole;
    let total: f64 = data.iter().map(|p| p.value.max(0.0)).sum();
    if total <= 0.0 || !total.is_finite() {
        b.fill(slice_path(center, r, inner, -PI / 2.0, TAU), text.with_alpha(25));
        no_data(b, Rect::from_center_size(center, (r * 2.0, r * 2.0)), fs, text);
        return;
    }
    let slices: Vec<(usize, f64, f64)> = {
        let mut a = -PI / 2.0;
        data.iter()
            .enumerate()
            .filter(|(_, p)| p.value > 0.0)
            .map(|(i, p)| {
                let sweep = p.value / total * TAU;
                let s = (i, a, sweep);
                a += sweep;
                s
            })
            .collect()
    };
    for &(i, start, sweep) in &slices {
        b.fill(slice_path(center, r, inner, start, sweep), color_at(i));
    }
    if slices.len() > 1 {
        let sw = (r * 0.015).max(1.0);
        for &(_, start, _) in &slices {
            let a = arc_point(center, inner, start);
            let z = arc_point(center, r + sw, start);
            b.stroke(line(a, z), sep, sw, false);
        }
    }
    for &(i, start, sweep) in &slices {
        let Some(p) = data.get(i) else { continue };
        let mid = start + sweep / 2.0;
        let rr = if inner > 0.0 { (r + inner) / 2.0 } else { r * 0.62 };
        let room = sweep * rr;
        let ring = if inner > 0.0 { r - inner } else { r };
        let label = c.show_labels && !p.label.trim().is_empty();
        let size = fs.min(ring * 0.32);
        if room < size * 1.6 || !(c.show_values || label) {
            continue;
        }
        let ink = color_at(i).readable_text();
        let at = arc_point(center, rr, mid);
        match (label, c.show_values) {
            (true, true) => {
                b.text(&p.label, size * 0.8, false, ink, at, TextAlign::Center, VAlign::Bottom, room.min(ring) * 1.2, 1);
                b.text(&value_text(c, p.value, total), size, true, ink, at, TextAlign::Center, VAlign::Top, 0.0, 1);
            }
            (true, false) => {
                b.text(&p.label, size * 0.85, true, ink, at, TextAlign::Center, VAlign::Middle, room.min(ring) * 1.2, 2);
            }
            _ => {
                b.text(&value_text(c, p.value, total), size, true, ink, at, TextAlign::Center, VAlign::Middle, 0.0, 1);
            }
        }
    }
}

fn progress(b: &mut Builder, c: &ChartEl, data: &[ChartPoint], bx: Rect, fs: f64, color: Color) {
    let text = c.text_color;
    let first = data.first().cloned().unwrap_or_default();
    let max = c.max.filter(|m| m.is_finite() && *m > 0.0).unwrap_or(100.0);
    let frac = (first.value / max).clamp(0.0, 1.0);
    let frac = if frac.is_finite() { frac } else { 0.0 };
    let shown = match c.value_format {
        ValueFormat::Percent => format_percent(frac, c.decimals),
        _ => value_text(c, first.value, max),
    };
    let track = color.with_alpha(((u16::from(color.a) * 50) / 255) as u8);
    let pad = fs * 0.5;
    if bx.width() >= bx.height() * 2.0 {
        // A bar: label and value above a rounded track.
        let area = Rect::new(bx.x0 + pad, bx.y0 + pad, (bx.x1 - pad).max(bx.x0 + pad + 1.0), (bx.y1 - pad).max(bx.y0 + pad + 1.0));
        let header = (c.show_labels && !first.label.trim().is_empty()) || c.show_values;
        let mut top = area.y0;
        if header {
            let hy = area.y0;
            if c.show_labels {
                b.text(&first.label, fs, false, text, Point::new(area.x0, hy), TextAlign::Left, VAlign::Top, area.width() * 0.7, 1);
            }
            if c.show_values {
                b.text(&shown, fs, true, text, Point::new(area.x1, hy), TextAlign::Right, VAlign::Top, 0.0, 1);
            }
            top += fs * 1.2 + fs * 0.3;
        }
        let avail = (area.y1 - top).max(1.0);
        let th = avail.min(fs * 1.1).max(2.0);
        let ty = top + (avail - th) / 2.0;
        let tr = Rect::new(area.x0, ty, area.x1, ty + th);
        b.fill(rounded_rect(tr, th / 2.0), track);
        if frac > 0.0 {
            let fw = (tr.width() * frac).max(th);
            b.fill(rounded_rect(Rect::new(tr.x0, tr.y0, tr.x0 + fw, tr.y1), th / 2.0), color);
        }
    } else {
        // A ring with the value in the middle.
        let center = bx.center();
        let outer = (bx.width().min(bx.height()) / 2.0 - pad).max(1.0);
        let thick = outer * 0.16;
        let rm = outer - thick / 2.0;
        b.stroke(Circle::new(center, rm).to_path(0.05), track, thick, false);
        if frac > 0.0 {
            let mut p = BezPath::new();
            p.move_to(arc_point(center, rm, -PI / 2.0));
            p.extend(arc(center, rm, -PI / 2.0, frac * TAU).append_iter(0.05));
            b.stroke(p, color, thick, frac < 1.0);
        }
        let big = (rm * 0.42).max(1.0);
        let label = c.show_labels && !first.label.trim().is_empty();
        if c.show_values {
            let at = if label { Point::new(center.x, center.y + big * 0.15) } else { center };
            b.text(&shown, big, true, text, at, TextAlign::Center, if label { VAlign::Bottom } else { VAlign::Middle }, 0.0, 1);
        }
        if label {
            let at = if c.show_values { Point::new(center.x, center.y + big * 0.2) } else { center };
            let size = fs.min(rm * 0.2);
            b.text(
                &first.label,
                size,
                false,
                text.with_alpha(190),
                at,
                TextAlign::Center,
                if c.show_values { VAlign::Top } else { VAlign::Middle },
                rm * 1.4,
                2,
            );
        }
    }
}

// ---------- tables ----------

/// Where a table's rows and columns fall in its box.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TableGeometry {
    /// Column edges, left to right (`columns + 1` values).
    pub xs: Vec<f64>,
    /// Row edges, top to bottom (`rows + 1` values).
    pub ys: Vec<f64>,
    /// The height the rows need (the box may be taller; rows then stretch).
    pub natural_height: f64,
}

impl TableGeometry {
    /// The box of the cells in `r` (cut to the table), `None` outside it.
    pub fn range_rect(&self, r: CellRange) -> Option<Rect> {
        let (x0, y0) = (*self.xs.get(r.col)?, *self.ys.get(r.row)?);
        let x1 = self.xs.get(r.end_col()).or(self.xs.last()).copied()?;
        let y1 = self.ys.get(r.end_row()).or(self.ys.last()).copied()?;
        Some(Rect::new(x0, y0, x1, y1))
    }
}

fn cell_style(t: &TableEl, header: bool) -> TextStyle {
    TextStyle {
        font_family: t.font_family.clone(),
        font_size: sane(t.font_size, 1.0, 2000.0, 24.0),
        bold: header,
        color: if header { t.header_text } else { t.text_color },
        align: t.align,
        line_height: 1.25,
        ..TextStyle::default()
    }
}

/// The text style of a cell: the table's (header or body), then the cell's own changes.
fn cell_text_style(t: &TableEl, header: bool, own: Option<&CellStyle>) -> TextStyle {
    let mut s = cell_style(t, header);
    if let Some(o) = own {
        if let Some(c) = o.text_color {
            s.color = c;
        }
        if let Some(b) = o.bold {
            s.bold = b;
        }
        if let Some(a) = o.align {
            s.align = a;
        }
        if let Some(f) = o.font_size {
            s.font_size = sane(f, 1.0, 2000.0, 24.0);
        }
    }
    s
}

/// The range a cell draws over: itself, or its whole merge when it is a merge's top-left cell;
/// `None` for cells hidden under a merge.
fn drawn_range(spans: &[Vec<CellSpan>], r: usize, c: usize) -> Option<CellRange> {
    match spans.get(r).and_then(|row| row.get(c)) {
        Some(CellSpan::Own) => Some(CellRange::cell(r, c)),
        Some(CellSpan::Anchor(m)) => Some(*m),
        _ => None,
    }
}

/// Rows and columns of `t` in a box `w` wide and `h` tall (`h` ≤ the natural height keeps the
/// natural row heights).
pub fn table_geometry(fonts: &FontBook, t: &TableEl, w: f64, h: f64) -> TableGeometry {
    let rows = t.rows.len().min(genha_doc::MAX_TABLE_ROWS);
    let cols = t.columns().min(genha_doc::MAX_TABLE_COLS);
    if rows == 0 || cols == 0 || !(w.is_finite() && w > 0.0) {
        return TableGeometry::default();
    }
    let pad = sane(t.padding, 0.0, 1000.0, 12.0);
    let mut xs = vec![0.0];
    let mut x = 0.0;
    for f in t.fractions().iter().take(cols) {
        x += f * w;
        xs.push(x);
    }
    let spans = t.spans();
    let styles = t.style_grid();
    let text_height = |r: usize, range: CellRange| -> f64 {
        let text = t.cell(range.row, range.col);
        if text.is_empty() {
            return 0.0;
        }
        let own = styles.get(r).and_then(|row| row.get(range.col)).copied().flatten();
        let style = cell_text_style(t, r == 0 && t.header_row, own);
        let x0 = xs.get(range.col).copied().unwrap_or(0.0);
        let x1 = xs.get(range.end_col()).or(xs.last()).copied().unwrap_or(w);
        genha_text::layout(fonts, text, &style, (x1 - x0 - 2.0 * pad).max(1.0)).height
    };
    let mut heights = vec![];
    let mut tall = vec![];
    for r in 0..rows {
        let style = cell_style(t, r == 0 && t.header_row);
        let mut rh = style.font_size * style.line_height;
        for c in 0..cols {
            match drawn_range(&spans, r, c) {
                Some(m) if m.rows > 1 => tall.push(m),
                Some(m) => rh = rh.max(text_height(r, m)),
                None => {}
            }
        }
        heights.push((rh + 2.0 * pad).max(t.row_height(r)));
    }
    // A merge over several rows makes its last row taller when its text needs the room.
    for m in tall {
        let need = text_height(m.row, m) + 2.0 * pad;
        let have: f64 = heights.iter().skip(m.row).take(m.rows).sum();
        if need > have
            && let Some(last) = heights.get_mut((m.end_row() - 1).min(rows - 1))
        {
            *last += need - have;
        }
    }
    let natural: f64 = heights.iter().sum();
    let extra = if h.is_finite() && h > natural { (h - natural) / rows as f64 } else { 0.0 };
    let mut ys = vec![0.0];
    let mut y = 0.0;
    for rh in heights {
        y += rh + extra;
        ys.push(y);
    }
    TableGeometry { xs, ys, natural_height: natural }
}

/// Column widths (fractions) that fit each column to its longest unwrapped text; cells merged
/// across columns do not count.
pub fn table_fit_columns(fonts: &FontBook, t: &TableEl) -> Vec<f64> {
    let cols = t.columns().min(genha_doc::MAX_TABLE_COLS);
    let pad = sane(t.padding, 0.0, 1000.0, 12.0);
    let spans = t.spans();
    let styles = t.style_grid();
    let mut widths = vec![sane(t.font_size, 1.0, 2000.0, 24.0) * 2.0; cols];
    for (r, row) in spans.iter().enumerate() {
        for (c, span) in row.iter().enumerate() {
            let one_col = match span {
                CellSpan::Own => true,
                CellSpan::Anchor(m) => m.cols == 1,
                CellSpan::Covered(_) => false,
            };
            let text = t.cell(r, c);
            if !one_col || text.is_empty() {
                continue;
            }
            let own = styles.get(r).and_then(|s| s.get(c)).copied().flatten();
            let style = cell_text_style(t, r == 0 && t.header_row, own);
            let w = text.lines().map(|l| genha_text::measure(fonts, l, &style).0).fold(0.0, f64::max);
            if let Some(slot) = widths.get_mut(c) {
                *slot = slot.max(w + 2.0 * pad + 1.0);
            }
        }
    }
    let sum: f64 = widths.iter().sum();
    if !(sum.is_finite() && sum > 0.0) {
        return vec![];
    }
    widths.iter().map(|w| w / sum).collect()
}

/// The table `t` in a `w` × `h` box.
pub fn table_scene(fonts: &FontBook, t: &TableEl, w: f64, h: f64) -> Scene {
    let mut b = Builder { fonts, family: &t.font_family, prims: vec![] };
    let g = table_geometry(fonts, t, w, h);
    let (Some(&right), Some(&bottom)) = (g.xs.last(), g.ys.last()) else { return Scene::default() };
    let pad = sane(t.padding, 0.0, 1000.0, 12.0);
    let spans = t.spans();
    let styles = t.style_grid();
    // Row fills.
    let mut fills = vec![];
    let mut body = 0usize;
    for r in 0..g.ys.len().saturating_sub(1) {
        let fill = if r == 0 && t.header_row {
            t.header_fill
        } else {
            body += 1;
            match t.alt_row_fill {
                Some(alt) if body.is_multiple_of(2) => alt,
                _ => t.row_fill,
            }
        };
        fills.push(fill);
        if let (Some(&y0), Some(&y1)) = (g.ys.get(r), g.ys.get(r + 1)) {
            b.fill(Rect::new(0.0, y0, right, y1).to_path(0.1), fill);
        }
    }
    // Cells: their own fill (a merge over several rows takes its top row's), then their text.
    for (r, row) in spans.iter().enumerate() {
        for c in 0..row.len() {
            let Some(range) = drawn_range(&spans, r, c) else { continue };
            let Some(cell) = g.range_rect(range) else { continue };
            let own = styles.get(r).and_then(|s| s.get(c)).copied().flatten();
            match own.and_then(|o| o.fill) {
                Some(f) => b.fill(cell.to_path(0.1), f),
                None if range.rows > 1 => b.fill(cell.to_path(0.1), fills.get(r).copied().unwrap_or(t.row_fill)),
                None => {}
            }
            let text = t.cell(r, c);
            if text.is_empty() {
                continue;
            }
            let style = cell_text_style(t, r == 0 && t.header_row, own);
            let cw = (cell.width() - 2.0 * pad).max(1.0);
            let (l, syn) = crate::layout_text(fonts, text, &style, cw);
            let dx = cell.x0 + pad;
            let dy = cell.y0 + (cell.height() - l.height) / 2.0;
            let shift = Affine::translate((dx, dy));
            let mut path = BezPath::new();
            let mut lines = vec![];
            for ln in &l.lines {
                path.extend(shift * ln.path.clone());
                lines.push(TextLine { x: ln.x + dx, width: ln.width, baseline: ln.baseline + dy, text: ln.text.trim_end().to_string() });
            }
            if style.color.a > 0 {
                let color = style.color;
                b.prims.push(Prim::Text(TextPrim { path, color, style, lines, embolden: syn.embolden, transform: Affine::IDENTITY }));
            }
        }
    }
    table_lines(&mut b, t, &g, &spans, right, bottom);
    Scene { background: None, prims: b.prims }
}

/// A usable line style: drawn, finite, at most 100 wide.
fn line_style(s: Option<&StrokeStyle>) -> Option<(Color, f64)> {
    let s = s?;
    (s.width.is_finite() && s.width > 0.0 && s.color.a > 0).then(|| (s.color, s.width.min(100.0)))
}

/// The lines between and around the cells, skipping the inside of merged cells.
fn table_lines(b: &mut Builder, t: &TableEl, g: &TableGeometry, spans: &[Vec<CellSpan>], right: f64, bottom: f64) {
    let merge_of = |r: usize, c: usize| match spans.get(r).and_then(|row| row.get(c)) {
        Some(CellSpan::Anchor(m) | CellSpan::Covered(m)) => Some(*m),
        _ => None,
    };
    let joined = |a: (usize, usize), z: (usize, usize)| merge_of(a.0, a.1).is_some_and(|m| Some(m) == merge_of(z.0, z.1));
    let rows = g.ys.len().saturating_sub(1);
    let cols = g.xs.len().saturating_sub(1);
    let (vertical, horizontal) = match t.borders {
        BorderLines::All | BorderLines::Inner => (true, true),
        BorderLines::Horizontal => (false, true),
        BorderLines::Outer => (false, false),
    };
    if let Some((color, width)) = line_style(t.border.as_ref()) {
        let mut grid = BezPath::new();
        // Runs of segments along each inner edge, broken where a merge spans it.
        let mut runs = |along: &[f64], at: f64, open: &dyn Fn(usize) -> bool, upright: bool| {
            let mut start: Option<usize> = None;
            for i in 0..=along.len().saturating_sub(1) {
                let drawn = i + 1 < along.len() && open(i);
                match (drawn, start) {
                    (true, None) => start = Some(i),
                    (false, Some(s)) => {
                        let (Some(&a), Some(&z)) = (along.get(s), along.get(i)) else { continue };
                        if upright {
                            grid.extend(line(Point::new(at, a), Point::new(at, z)));
                        } else {
                            grid.extend(line(Point::new(a, at), Point::new(z, at)));
                        }
                        start = None;
                    }
                    _ => {}
                }
            }
        };
        if vertical {
            for k in 1..cols {
                let Some(&x) = g.xs.get(k) else { continue };
                runs(&g.ys, x, &|r| !joined((r, k - 1), (r, k)), true);
            }
        }
        if horizontal {
            for k in 1..rows {
                let Some(&y) = g.ys.get(k) else { continue };
                runs(&g.xs, y, &|c| !joined((k - 1, c), (k, c)), false);
            }
        }
        b.stroke(grid, color, width, false);
    }
    let outer = line_style(t.outer_border.as_ref().or(t.border.as_ref()));
    if let Some((color, width)) = outer {
        match t.borders {
            BorderLines::All | BorderLines::Outer => b.stroke(Rect::new(0.0, 0.0, right, bottom).to_path(0.1), color, width, false),
            BorderLines::Horizontal => {
                let mut p = line(Point::new(0.0, 0.0), Point::new(right, 0.0));
                p.extend(line(Point::new(0.0, bottom), Point::new(right, bottom)));
                b.stroke(p, color, width, false);
            }
            BorderLines::Inner => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genha_doc::ChartSeries;

    #[test]
    fn numbers_read_well() {
        assert_eq!(format_number(1234567.0), "1,234,567");
        assert_eq!(format_number(-1234.5), "-1,234.5");
        assert_eq!(format_number(0.126), "0.13");
        assert_eq!(format_number(-0.0001), "0");
        assert_eq!(format_number(f64::NAN), "0");
        assert_eq!(format_compact(25_000.0), "25k");
        assert_eq!(format_compact(1_500_000.0), "1.5M");
        assert_eq!(format_percent(0.4, None), "40%");
        assert_eq!(format_percent(0.025, None), "2.5%");
        assert_eq!(format_percent(0.4, Some(1)), "40.0%");
        assert_eq!(nice_step(47.0, 5.0), 10.0);
        assert_eq!(nice_step(0.0, 5.0), 1.0);
        assert_eq!(format_decimals(1234.5, Some(2)), "1,234.50");
        assert_eq!(format_decimals(-0.004, Some(2)), "0.00");
        assert_eq!(format_decimals(2.75, Some(0)), "3");
        let mut c = ChartEl { value_format: ValueFormat::Currency, currency: "€".into(), ..ChartEl::default() };
        assert_eq!(value_text(&c, -1250.0, 0.0), "-€1,250");
        c.decimals = Some(2);
        assert_eq!(value_text(&c, 9.5, 0.0), "€9.50");
        assert_eq!(tick_text(&c, 25_000.0), "€25k");
    }

    #[test]
    fn scales_keep_a_set_minimum_and_maximum() {
        let mut c = ChartEl::default();
        assert_eq!(value_scale(&c, &[3.0, 47.0], 5.0), (0.0, 50.0, 10.0));
        c.min = Some(20.0);
        c.max = Some(55.0);
        let (lo, hi, step) = value_scale(&c, &[3.0, 47.0], 5.0);
        assert_eq!((lo, hi), (20.0, 55.0));
        assert_eq!(tick_values(lo, hi, step), vec![20.0, 30.0, 40.0, 50.0]);
        // A maximum under the minimum is ignored.
        c.max = Some(10.0);
        let (lo, hi, _) = value_scale(&c, &[3.0, 47.0], 5.0);
        assert!(lo == 20.0 && hi >= 47.0);
        assert!(tick_values(0.0, f64::INFINITY, 1.0).is_empty());
        // A set end that is not round gets its own tick.
        c.min = Some(-5.0);
        c.max = Some(10.0);
        let (lo, hi, step) = value_scale(&c, &[3.0], 1.5);
        assert_eq!(axis_ticks(&c, lo, hi, step), vec![0.0, 10.0, -5.0]);
    }

    #[test]
    fn bars_group_and_stack() {
        let mut c = ChartEl::with_points(vec![ChartPoint::new("a", 2.0), ChartPoint::new("b", -1.0)]);
        c.series.push(ChartSeries::new("two", vec![3.0, -4.0]));
        let d = Data::of(&c, &BrandKit::default());
        assert!(!d.per_category);
        assert_eq!(d.extent(true), vec![5.0, 0.0, 0.0, -5.0]);
        let side = bar_specs(&d, false);
        assert_eq!(side.len(), 4);
        assert!(side[0].offset < 0.0 && side[1].offset > 0.0 && side.iter().all(|b| b.rounded && b.from == 0.0));
        let stack = bar_specs(&d, true);
        let ends: Vec<(f64, f64, bool)> = stack.iter().map(|b| (b.from, b.to, b.rounded)).collect();
        assert_eq!(ends, vec![(0.0, 2.0, false), (2.0, 5.0, true), (0.0, -1.0, false), (-1.0, -5.0, true)]);
        // Series without a colour get different ones even when the chart has one colour.
        c.colors = vec![Color::BLACK];
        let d = Data::of(&c, &BrandKit::default());
        assert_ne!(d.series[0].color, d.series[1].color);
        // Pies draw the first series only.
        c.kind = ChartKind::Pie;
        assert_eq!(Data::of(&c, &BrandKit::default()).series.len(), 1);
    }
}
