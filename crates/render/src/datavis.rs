//! Charts and tables as drawing primitives (filled and stroked paths, laid-out text) in the
//! element's local box, shared by the raster renderer and the SVG writer so both draw the same.

use std::f64::consts::{PI, TAU};

use genha_color::Color;
use genha_doc::{BrandKit, ChartEl, ChartKind, ChartPoint, Fill, TableEl, TextAlign, TextStyle, ValueFormat};
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
        let mut l = genha_text::layout(self.fonts, text, &style, wrap);
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
            self.prims.push(Prim::Text(TextPrim { path, color, style, lines }));
        }
        if x0 > x1 {
            return Rect::from_origin_size(at, (0.0, 0.0));
        }
        Rect::new(x0, dy, x1, dy + height)
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

/// `v` with thousands separators and at most two decimals.
pub fn format_number(v: f64) -> String {
    if !v.is_finite() {
        return "0".into();
    }
    if v.abs() >= 1e15 {
        return format!("{v:.2e}");
    }
    let s = format!("{:.2}", v.abs());
    let (int, frac) = s.split_once('.').unwrap_or((&s, ""));
    let frac = frac.trim_end_matches('0');
    let mut grouped = String::new();
    for (i, ch) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let neg = v < 0.0 && (grouped != "0" || !frac.is_empty());
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

fn format_percent(share: f64) -> String {
    let p = share * 100.0;
    if p.abs() >= 10.0 || p == 0.0 { format!("{p:.0}%") } else { format!("{}%", format!("{p:.1}").trim_end_matches('0').trim_end_matches('.')) }
}

/// How a value is written on the chart.
fn value_text(c: &ChartEl, v: f64, total: f64) -> String {
    match c.value_format {
        ValueFormat::Number => format_number(v),
        ValueFormat::Percent => format_percent(if total.abs() > 0.0 { v / total } else { 0.0 }),
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

/// The value scale of an axis chart: bottom, top and tick step.
fn value_scale(c: &ChartEl, values: &[f64], ticks: f64) -> (f64, f64, f64) {
    let mut lo = values.iter().copied().fold(0.0_f64, f64::min);
    let mut hi = values.iter().copied().fold(0.0_f64, f64::max);
    if let Some(m) = c.max.filter(|m| m.is_finite() && *m > lo) {
        hi = m;
    }
    if hi - lo <= 0.0 || !(hi - lo).is_finite() {
        hi = lo + 1.0;
    }
    let step = nice_step(hi - lo, ticks);
    lo = (lo / step).floor() * step;
    if c.max.is_none() {
        hi = (hi / step).ceil() * step;
    }
    if hi <= lo {
        hi = lo + step;
    }
    (lo, hi, step)
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
    let data: Vec<ChartPoint> = c.data.iter().take(genha_doc::MAX_CHART_POINTS).cloned().collect();
    let text = c.text_color;
    match c.kind {
        ChartKind::Pie | ChartKind::Donut => pie(&mut b, c, &data, area, fs, &color_at, separator_color(c)),
        ChartKind::Progress => {
            // A bar's text follows its own height rather than the box's short side.
            let fs = if w >= h * 2.0 { sane(c.font_size, 1.0, 2000.0, 24.0).min(h * 0.4).max(1.0) } else { fs };
            progress(&mut b, c, &data, Rect::new(0.0, 0.0, w, h), fs, color_at(0));
        }
        ChartKind::Bar | ChartKind::HorizontalBar | ChartKind::Line | ChartKind::Area => {
            let mut plot_area = area;
            if c.show_legend && matches!(c.kind, ChartKind::Bar | ChartKind::HorizontalBar) && !data.is_empty() {
                let legend = legend_for(&b, &data, fs * 0.8, &color_at);
                let lh = legend.height_below(area.width()).min(area.height() * 0.4);
                legend.draw_below(&mut b, Rect::new(area.x0, area.y1 - lh, area.x1, area.y1), text);
                plot_area.y1 -= lh + fs * 0.4;
            }
            if c.kind == ChartKind::HorizontalBar {
                horizontal_bars(&mut b, c, &data, plot_area, fs, &color_at);
            } else {
                vertical(&mut b, c, &data, plot_area, fs, &color_at);
            }
        }
    }
    scene.prims = b.prims;
    scene
}

/// The colour between pie slices: the chart's background, else white.
fn separator_color(c: &ChartEl) -> Color {
    match &c.background {
        Fill::Solid { color } if color.a == 255 => *color,
        _ => Color::WHITE,
    }
}

fn legend_for(b: &Builder, data: &[ChartPoint], size: f64, color_at: &dyn Fn(usize) -> Color) -> Legend {
    let items = data.iter().enumerate().map(|(i, p)| (color_at(i), p.label.clone(), b.measure(&p.label, size, false))).collect();
    Legend { items, size }
}

fn no_data(b: &mut Builder, area: Rect, fs: f64, text: Color) {
    b.text("No data", fs, false, text.with_alpha(140), area.center(), TextAlign::Center, VAlign::Middle, 0.0, 1);
}

/// Vertical bars, lines and areas.
fn vertical(b: &mut Builder, c: &ChartEl, data: &[ChartPoint], area: Rect, fs: f64, color_at: &dyn Fn(usize) -> Color) {
    let text = c.text_color;
    let values: Vec<f64> = data.iter().map(|p| p.value).collect();
    let total: f64 = values.iter().sum();
    let tick_fs = fs * 0.75;
    let n = data.len();
    // Room for category labels under the plot.
    let label_fs = fs * 0.8;
    let mut label_lines = 0usize;
    if c.show_labels && n > 0 {
        let slot = (area.width() * 0.85) / n as f64;
        for p in data {
            let lines = if b.measure(&p.label, label_fs, false) > slot * 0.95 { 2 } else { 1 };
            label_lines = label_lines.max(if p.label.trim().is_empty() { 0 } else { lines });
        }
    }
    let label_h = if label_lines > 0 { label_lines as f64 * label_fs * 1.2 + fs * 0.4 } else { 0.0 };
    let top_room = if c.show_values { fs * 1.3 } else { fs * 0.5 };
    let plot_h = (area.height() - label_h - top_room).max(1.0);
    let (lo, hi, step) = value_scale(c, &values, (plot_h / (fs * 2.5)).clamp(2.0, 8.0));
    let ticks: Vec<f64> = (0..=40).map(|k| lo + k as f64 * step).take_while(|v| *v <= hi + step * 1e-6).collect();
    let tick_w = ticks.iter().map(|v| b.measure(&format_compact(*v), tick_fs, false)).fold(0.0, f64::max);
    let plot = Rect::new(area.x0 + tick_w + fs * 0.5, area.y0 + top_room, area.x1, area.y0 + top_room + plot_h);
    let y_of = |v: f64| plot.y1 - (v - lo) / (hi - lo) * plot.height();
    let grid_w = (fs * 0.05).max(1.0);
    for v in &ticks {
        let y = y_of(*v);
        b.stroke(line(Point::new(plot.x0, y), Point::new(plot.x1, y)), text.with_alpha(28), grid_w, false);
        b.text(
            &format_compact(*v),
            tick_fs,
            false,
            text.with_alpha(150),
            Point::new(plot.x0 - fs * 0.4, y),
            TextAlign::Right,
            VAlign::Middle,
            0.0,
            1,
        );
    }
    let base_v = 0.0_f64.clamp(lo, hi);
    let base_y = y_of(base_v);
    if n == 0 {
        no_data(b, plot, fs, text);
        return;
    }
    let slot = plot.width() / n as f64;
    let cx = |i: usize| plot.x0 + slot * (i as f64 + 0.5);
    let label_size = fs * 0.75;
    match c.kind {
        ChartKind::Bar => {
            let bw = (slot * 0.62).max(1.0);
            for (i, p) in data.iter().enumerate() {
                let y = y_of(p.value.clamp(lo, hi));
                let r = Rect::new(cx(i) - bw / 2.0, y.min(base_y), cx(i) + bw / 2.0, y.max(base_y));
                let up = p.value >= 0.0;
                b.fill(bar_path(r, (bw * 0.18).min(fs * 0.35), if up { BarEnd::Top } else { BarEnd::Bottom }), color_at(i));
                if c.show_values {
                    let (at, v) =
                        if up { (Point::new(cx(i), r.y0 - fs * 0.25), VAlign::Bottom) } else { (Point::new(cx(i), r.y1 + fs * 0.2), VAlign::Top) };
                    b.text(&value_text(c, p.value, total), label_size, true, text, at, TextAlign::Center, v, 0.0, 1);
                }
            }
        }
        _ => {
            let color = color_at(0);
            let pts: Vec<Point> = data.iter().enumerate().map(|(i, p)| Point::new(cx(i), y_of(p.value.clamp(lo, hi)))).collect();
            let lw = (fs * 0.14).max(1.5);
            if c.kind == ChartKind::Area
                && let (Some(first), Some(last)) = (pts.first(), pts.last())
            {
                let mut a = BezPath::new();
                a.move_to((first.x, base_y));
                for p in &pts {
                    a.line_to(*p);
                }
                a.line_to((last.x, base_y));
                a.close_path();
                b.fill(a, color.with_alpha(((u16::from(color.a) * 80) / 255) as u8));
            }
            if pts.len() > 1 {
                let mut l = BezPath::new();
                for (i, p) in pts.iter().enumerate() {
                    if i == 0 {
                        l.move_to(*p);
                    } else {
                        l.line_to(*p);
                    }
                }
                b.stroke(l, color, lw, true);
            }
            for (i, p) in pts.iter().enumerate() {
                b.fill(Circle::new(*p, lw * 1.9).to_path(0.05), color);
                b.fill(Circle::new(*p, lw * 0.9).to_path(0.05), Color::WHITE);
                if c.show_values
                    && let Some(d) = data.get(i)
                {
                    let at = Point::new(p.x, p.y - lw * 1.9 - fs * 0.2);
                    b.text(&value_text(c, d.value, total), label_size, true, text, at, TextAlign::Center, VAlign::Bottom, 0.0, 1);
                }
            }
        }
    }
    // The zero line, over the bars' feet.
    b.stroke(line(Point::new(plot.x0, base_y), Point::new(plot.x1, base_y)), text.with_alpha(90), grid_w * 1.5, false);
    if label_lines > 0 {
        for (i, p) in data.iter().enumerate() {
            let at = Point::new(cx(i), plot.y1 + fs * 0.4);
            b.text(&p.label, label_fs, false, text, at, TextAlign::Center, VAlign::Top, slot * 0.95, label_lines);
        }
    }
}

fn horizontal_bars(b: &mut Builder, c: &ChartEl, data: &[ChartPoint], area: Rect, fs: f64, color_at: &dyn Fn(usize) -> Color) {
    let text = c.text_color;
    let values: Vec<f64> = data.iter().map(|p| p.value).collect();
    let total: f64 = values.iter().sum();
    let label_fs = fs * 0.8;
    let tick_fs = fs * 0.75;
    let label_w =
        if c.show_labels { data.iter().map(|p| b.measure(&p.label, label_fs, false)).fold(0.0, f64::max).min(area.width() * 0.3) } else { 0.0 };
    let value_w = if c.show_values {
        data.iter().map(|p| b.measure(&value_text(c, p.value, total), fs * 0.75, true)).fold(0.0, f64::max) + fs * 0.4
    } else {
        fs * 0.3
    };
    let tick_h = tick_fs * 1.2 + fs * 0.3;
    let x0 = area.x0 + if label_w > 0.0 { label_w + fs * 0.5 } else { 0.0 };
    let plot = Rect::new(x0, area.y0, (area.x1 - value_w).max(x0 + 1.0), (area.y1 - tick_h).max(area.y0 + 1.0));
    let (lo, hi, step) = value_scale(c, &values, (plot.width() / (fs * 4.0)).clamp(2.0, 8.0));
    let x_of = |v: f64| plot.x0 + (v - lo) / (hi - lo) * plot.width();
    let grid_w = (fs * 0.05).max(1.0);
    let mut v = lo;
    for _ in 0..=40 {
        if v > hi + step * 1e-6 {
            break;
        }
        let x = x_of(v);
        b.stroke(line(Point::new(x, plot.y0), Point::new(x, plot.y1)), text.with_alpha(28), grid_w, false);
        b.text(&format_compact(v), tick_fs, false, text.with_alpha(150), Point::new(x, plot.y1 + fs * 0.3), TextAlign::Center, VAlign::Top, 0.0, 1);
        v += step;
    }
    let base_x = x_of(0.0_f64.clamp(lo, hi));
    if data.is_empty() {
        no_data(b, plot, fs, text);
        return;
    }
    let slot = plot.height() / data.len() as f64;
    let bh = (slot * 0.62).max(1.0);
    for (i, p) in data.iter().enumerate() {
        let cy = plot.y0 + slot * (i as f64 + 0.5);
        let x = x_of(p.value.clamp(lo, hi));
        let r = Rect::new(x.min(base_x), cy - bh / 2.0, x.max(base_x), cy + bh / 2.0);
        let right = p.value >= 0.0;
        b.fill(bar_path(r, (bh * 0.18).min(fs * 0.35), if right { BarEnd::Right } else { BarEnd::Left }), color_at(i));
        if c.show_values {
            let (at, h) =
                if right { (Point::new(r.x1 + fs * 0.25, cy), TextAlign::Left) } else { (Point::new(r.x0 - fs * 0.25, cy), TextAlign::Right) };
            b.text(&value_text(c, p.value, total), fs * 0.75, true, text, at, h, VAlign::Middle, 0.0, 1);
        }
        if label_w > 0.0 {
            b.text(&p.label, label_fs, false, text, Point::new(plot.x0 - fs * 0.5, cy), TextAlign::Right, VAlign::Middle, label_w + 1.0, 2);
        }
    }
    b.stroke(line(Point::new(base_x, plot.y0), Point::new(base_x, plot.y1)), text.with_alpha(90), grid_w * 1.5, false);
}

fn pie(b: &mut Builder, c: &ChartEl, data: &[ChartPoint], area: Rect, fs: f64, color_at: &dyn Fn(usize) -> Color, sep: Color) {
    let text = c.text_color;
    let mut plot = area;
    if c.show_legend && !data.is_empty() {
        let legend = legend_for(b, data, fs * 0.8, color_at);
        let col_w = legend.column_width();
        if area.width() > area.height() * 1.3 && col_w < area.width() * 0.45 {
            let lw = col_w + fs;
            legend.draw_column(b, Rect::new(area.x1 - col_w, area.y0, area.x1, area.y1), text);
            plot.x1 -= lw;
        } else {
            let lh = legend.height_below(area.width()).min(area.height() * 0.4);
            legend.draw_below(b, Rect::new(area.x0, area.y1 - lh, area.x1, area.y1), text);
            plot.y1 -= lh + fs * 0.5;
        }
    }
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
        ValueFormat::Percent => format_percent(frac),
        ValueFormat::Number => format_number(first.value),
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
    let mut heights = vec![];
    for (r, row) in t.rows.iter().take(rows).enumerate() {
        let style = cell_style(t, r == 0 && t.header_row);
        let one = style.font_size * style.line_height;
        let mut rh = one;
        for c in 0..cols {
            let text = row.get(c).map_or("", String::as_str);
            if text.is_empty() {
                continue;
            }
            let cw = (xs.get(c + 1).copied().unwrap_or(w) - xs.get(c).copied().unwrap_or(0.0) - 2.0 * pad).max(1.0);
            rh = rh.max(genha_text::layout(fonts, text, &style, cw).height);
        }
        heights.push(rh + 2.0 * pad);
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

/// The table `t` in a `w` × `h` box.
pub fn table_scene(fonts: &FontBook, t: &TableEl, w: f64, h: f64) -> Scene {
    let mut b = Builder { fonts, family: &t.font_family, prims: vec![] };
    let g = table_geometry(fonts, t, w, h);
    let (Some(&right), Some(&bottom)) = (g.xs.last(), g.ys.last()) else { return Scene::default() };
    let pad = sane(t.padding, 0.0, 1000.0, 12.0);
    let mut body = 0usize;
    for (r, row) in t.rows.iter().enumerate() {
        let (Some(&y0), Some(&y1)) = (g.ys.get(r), g.ys.get(r + 1)) else { break };
        let header = r == 0 && t.header_row;
        let fill = if header {
            t.header_fill
        } else {
            body += 1;
            match t.alt_row_fill {
                Some(alt) if body.is_multiple_of(2) => alt,
                _ => t.row_fill,
            }
        };
        b.fill(Rect::new(0.0, y0, right, y1).to_path(0.1), fill);
        let style = cell_style(t, header);
        for (c, text) in row.iter().enumerate() {
            let (Some(&x0), Some(&x1)) = (g.xs.get(c), g.xs.get(c + 1)) else { break };
            if text.is_empty() {
                continue;
            }
            let cw = (x1 - x0 - 2.0 * pad).max(1.0);
            let l = genha_text::layout(fonts, text, &style, cw);
            let dx = x0 + pad;
            let dy = y0 + ((y1 - y0) - l.height) / 2.0;
            let shift = Affine::translate((dx, dy));
            let mut path = BezPath::new();
            let mut lines = vec![];
            for ln in &l.lines {
                path.extend(shift * ln.path.clone());
                lines.push(TextLine { x: ln.x + dx, width: ln.width, baseline: ln.baseline + dy, text: ln.text.trim_end().to_string() });
            }
            if style.color.a > 0 {
                b.prims.push(Prim::Text(TextPrim { path, color: style.color, style: style.clone(), lines }));
            }
        }
    }
    if let Some(border) = &t.border
        && border.width.is_finite()
        && border.width > 0.0
    {
        let bw = border.width.min(100.0);
        let mut grid = BezPath::new();
        for x in g.xs.iter().skip(1).take(g.xs.len().saturating_sub(2)) {
            grid.extend(line(Point::new(*x, 0.0), Point::new(*x, bottom)));
        }
        for y in g.ys.iter().skip(1).take(g.ys.len().saturating_sub(2)) {
            grid.extend(line(Point::new(0.0, *y), Point::new(right, *y)));
        }
        grid.extend(Rect::new(0.0, 0.0, right, bottom).to_path(0.1));
        b.stroke(grid, border.color, bw, false);
    }
    Scene { background: None, prims: b.prims }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_well() {
        assert_eq!(format_number(1234567.0), "1,234,567");
        assert_eq!(format_number(-1234.5), "-1,234.5");
        assert_eq!(format_number(0.126), "0.13");
        assert_eq!(format_number(-0.0001), "0");
        assert_eq!(format_number(f64::NAN), "0");
        assert_eq!(format_compact(25_000.0), "25k");
        assert_eq!(format_compact(1_500_000.0), "1.5M");
        assert_eq!(format_percent(0.4), "40%");
        assert_eq!(format_percent(0.025), "2.5%");
        assert_eq!(nice_step(47.0, 5.0), 10.0);
        assert_eq!(nice_step(0.0, 5.0), 1.0);
    }
}
