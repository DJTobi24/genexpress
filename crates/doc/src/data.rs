//! Data elements: charts (here) and tables ([`crate::TableEl`]).
//!
//! Both are pure data here; `genha-render` turns them into shapes and text fitted to the element's
//! frame. Every field has a default so older or hand-written files open, and [`ChartEl::sanitize`]
//! / [`crate::TableEl::sanitize`] cap what untrusted files and commands can ask for.
//!
//! A chart is a small table: category labels (the x axis, pie slices) and one or more series of
//! values, one value per category. Charts written before several series existed kept a single
//! list of `{label, value}` points under `data`; they open as one series.

use genha_color::Color;
use serde::{Deserialize, Serialize};

use crate::Fill;

/// Most categories (data points per series) in a chart.
pub const MAX_CHART_POINTS: usize = 200;
/// Most series in a chart.
pub const MAX_CHART_SERIES: usize = 20;
/// Longest label or cell text, in characters.
pub const MAX_CELL_CHARS: usize = 2_000;
/// Largest magnitude a chart value may have.
const MAX_VALUE: f64 = 1e15;

/// How a chart draws its data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChartKind {
    /// Vertical bars (columns): one per category, grouped or stacked by series.
    #[default]
    Bar,
    /// Horizontal bars, labels on the left.
    HorizontalBar,
    /// Points joined by a line, one line per series.
    Line,
    /// A line with the area under it filled.
    Area,
    /// Slices of a circle (the first series), starting at 12 o'clock and going clockwise.
    Pie,
    /// A pie with a hole ([`ChartEl::donut_hole`]).
    Donut,
    /// The first value as a share of [`ChartEl::max`] (100 when unset): a bar when the frame
    /// is at least twice as wide as it is tall, a ring otherwise.
    Progress,
}

impl ChartKind {
    pub const ALL: [ChartKind; 7] =
        [ChartKind::Bar, ChartKind::HorizontalBar, ChartKind::Line, ChartKind::Area, ChartKind::Pie, ChartKind::Donut, ChartKind::Progress];

    /// The id used in files and commands (`"horizontal-bar"`).
    pub fn id(self) -> &'static str {
        match self {
            ChartKind::Bar => "bar",
            ChartKind::HorizontalBar => "horizontal-bar",
            ChartKind::Line => "line",
            ChartKind::Area => "area",
            ChartKind::Pie => "pie",
            ChartKind::Donut => "donut",
            ChartKind::Progress => "progress",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ChartKind::Bar => "Bar",
            ChartKind::HorizontalBar => "Horizontal bar",
            ChartKind::Line => "Line",
            ChartKind::Area => "Area",
            ChartKind::Pie => "Pie",
            ChartKind::Donut => "Donut",
            ChartKind::Progress => "Progress",
        }
    }

    /// Parses an id (also accepts `column`, `hbar`, `ring`).
    pub fn from_id(id: &str) -> Option<ChartKind> {
        Some(match id.trim().to_ascii_lowercase().as_str() {
            "bar" | "column" | "columns" | "vertical-bar" => ChartKind::Bar,
            "horizontal-bar" | "hbar" | "horizontal" | "row" => ChartKind::HorizontalBar,
            "line" => ChartKind::Line,
            "area" => ChartKind::Area,
            "pie" => ChartKind::Pie,
            "donut" | "doughnut" => ChartKind::Donut,
            "progress" | "ring" | "gauge" => ChartKind::Progress,
            _ => return None,
        })
    }

    /// True for the kinds drawn against a value axis.
    pub fn has_axis(self) -> bool {
        matches!(self, ChartKind::Bar | ChartKind::HorizontalBar | ChartKind::Line | ChartKind::Area)
    }

    /// True for the kinds that draw every series (the others draw the first one).
    pub fn multi_series(self) -> bool {
        self.has_axis()
    }
}

/// How values are written on a chart.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueFormat {
    /// The value itself (`1,250`).
    #[default]
    Number,
    /// Each value's share of its series' total (`25%`); a progress chart's share of its maximum.
    Percent,
    /// The value after [`ChartEl::currency`] (`$1,250`).
    Currency,
}

/// Where a chart's legend goes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LegendPosition {
    /// Under bar and line charts; at the right of a wide pie, else under it.
    #[default]
    Auto,
    Top,
    Bottom,
    Right,
}

impl LegendPosition {
    pub const ALL: [LegendPosition; 4] = [LegendPosition::Auto, LegendPosition::Top, LegendPosition::Bottom, LegendPosition::Right];

    pub fn id(self) -> &'static str {
        match self {
            LegendPosition::Auto => "auto",
            LegendPosition::Top => "top",
            LegendPosition::Bottom => "bottom",
            LegendPosition::Right => "right",
        }
    }
}

/// One labelled value (one category of a one-series chart).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ChartPoint {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub value: f64,
}

impl ChartPoint {
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        ChartPoint { label: label.into(), value }
    }
}

/// A named row of values, one per category.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ChartSeries {
    pub name: String,
    pub values: Vec<f64>,
    /// The series' colour; `None` takes the next chart colour.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}

impl ChartSeries {
    pub fn new(name: impl Into<String>, values: Vec<f64>) -> Self {
        ChartSeries { name: name.into(), values, color: None }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "ChartFile")]
pub struct ChartEl {
    pub kind: ChartKind,
    /// Category labels: the x axis of bars and lines, the slices of a pie.
    pub categories: Vec<String>,
    /// The values, one series per row; pies and progress charts use the first.
    pub series: Vec<ChartSeries>,
    /// Bars, lines and areas of several series stack on each other instead of standing side by
    /// side (bars) or overlapping (lines, areas).
    pub stacked: bool,
    /// Chart colours, used in turn (per category in a one-series bar or pie chart, else per
    /// series without its own colour); empty means the brand colours (or a built-in palette).
    pub colors: Vec<Color>,
    /// Category names (axis labels; names inside pie slices).
    pub show_labels: bool,
    /// Values on the bars, points and slices.
    pub show_values: bool,
    /// A key of colours and names.
    pub show_legend: bool,
    pub legend_position: LegendPosition,
    /// Lines across the plot at each value step.
    pub gridlines: bool,
    /// Title under the horizontal axis.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub x_title: String,
    /// Title along the vertical axis.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub y_title: String,
    pub value_format: ValueFormat,
    /// Symbol written before currency values.
    pub currency: String,
    /// Decimals shown in values (0–6); `None` shows up to two, trailing zeros dropped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decimals: Option<u8>,
    pub font_family: String,
    /// Size of labels, in page pixels.
    pub font_size: f64,
    pub text_color: Color,
    /// Behind the whole chart.
    pub background: Fill,
    /// A donut's hole as a fraction of its radius, `0..=0.9`.
    pub donut_hole: f64,
    /// The bottom of the value scale (bars, lines); `None` starts at zero or the lowest value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// The top of the value scale (bars, lines) or a progress chart's 100%; `None` fits the data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

impl Default for ChartEl {
    fn default() -> Self {
        ChartEl {
            kind: ChartKind::Bar,
            categories: vec![],
            series: vec![],
            stacked: false,
            colors: vec![],
            show_labels: true,
            show_values: true,
            show_legend: false,
            legend_position: LegendPosition::Auto,
            gridlines: true,
            x_title: String::new(),
            y_title: String::new(),
            value_format: ValueFormat::Number,
            currency: "$".into(),
            decimals: None,
            font_family: "Sans".into(),
            font_size: 24.0,
            text_color: Color::rgb(0x1b, 0x1b, 0x2f),
            background: Fill::None,
            donut_hole: 0.55,
            min: None,
            max: None,
        }
    }
}

/// A chart as stored in files: the current fields plus the single-series `data` of files from
/// before several series existed, folded into [`ChartEl::categories`] and [`ChartEl::series`].
#[derive(Deserialize)]
#[serde(default)]
struct ChartFile {
    kind: ChartKind,
    categories: Vec<String>,
    series: Vec<ChartSeries>,
    stacked: bool,
    colors: Vec<Color>,
    show_labels: bool,
    show_values: bool,
    show_legend: bool,
    legend_position: LegendPosition,
    gridlines: bool,
    x_title: String,
    y_title: String,
    value_format: ValueFormat,
    currency: String,
    decimals: Option<u8>,
    font_family: String,
    font_size: f64,
    text_color: Color,
    background: Fill,
    donut_hole: f64,
    min: Option<f64>,
    max: Option<f64>,
    /// Before M5.1: one series as `{label, value}` points.
    data: Vec<ChartPoint>,
}

impl Default for ChartFile {
    fn default() -> Self {
        let c = ChartEl::default();
        ChartFile {
            kind: c.kind,
            categories: c.categories,
            series: c.series,
            stacked: c.stacked,
            colors: c.colors,
            show_labels: c.show_labels,
            show_values: c.show_values,
            show_legend: c.show_legend,
            legend_position: c.legend_position,
            gridlines: c.gridlines,
            x_title: c.x_title,
            y_title: c.y_title,
            value_format: c.value_format,
            currency: c.currency,
            decimals: c.decimals,
            font_family: c.font_family,
            font_size: c.font_size,
            text_color: c.text_color,
            background: c.background,
            donut_hole: c.donut_hole,
            min: c.min,
            max: c.max,
            data: vec![],
        }
    }
}

impl From<ChartFile> for ChartEl {
    fn from(f: ChartFile) -> Self {
        let mut c = ChartEl {
            kind: f.kind,
            categories: f.categories,
            series: f.series,
            stacked: f.stacked,
            colors: f.colors,
            show_labels: f.show_labels,
            show_values: f.show_values,
            show_legend: f.show_legend,
            legend_position: f.legend_position,
            gridlines: f.gridlines,
            x_title: f.x_title,
            y_title: f.y_title,
            value_format: f.value_format,
            currency: f.currency,
            decimals: f.decimals,
            font_family: f.font_family,
            font_size: f.font_size,
            text_color: f.text_color,
            background: f.background,
            donut_hole: f.donut_hole,
            min: f.min,
            max: f.max,
        };
        if c.series.is_empty() && c.categories.is_empty() && !f.data.is_empty() {
            c.set_points(f.data.into_iter().take(MAX_CHART_POINTS).collect());
        }
        c
    }
}

impl ChartEl {
    /// A chart of `kind` with sample data (one series).
    pub fn sample(kind: ChartKind) -> Self {
        let data = match kind {
            ChartKind::Pie | ChartKind::Donut => {
                vec![ChartPoint::new("Design", 40.0), ChartPoint::new("Video", 25.0), ChartPoint::new("Photo", 20.0), ChartPoint::new("Other", 15.0)]
            }
            ChartKind::Progress => vec![ChartPoint::new("Progress", 72.0)],
            ChartKind::Line | ChartKind::Area => vec![
                ChartPoint::new("Jan", 12.0),
                ChartPoint::new("Feb", 19.0),
                ChartPoint::new("Mar", 15.0),
                ChartPoint::new("Apr", 27.0),
                ChartPoint::new("May", 24.0),
                ChartPoint::new("Jun", 33.0),
            ],
            _ => vec![ChartPoint::new("Q1", 24.0), ChartPoint::new("Q2", 38.0), ChartPoint::new("Q3", 31.0), ChartPoint::new("Q4", 46.0)],
        };
        let pie = matches!(kind, ChartKind::Pie | ChartKind::Donut);
        let mut c = ChartEl {
            kind,
            show_legend: pie,
            value_format: if pie || kind == ChartKind::Progress { ValueFormat::Percent } else { ValueFormat::Number },
            ..ChartEl::default()
        };
        c.set_points(data);
        c
    }

    /// A chart from points: one series (keeping the first series' name and colour).
    pub fn with_points(points: Vec<ChartPoint>) -> Self {
        let mut c = ChartEl::default();
        c.set_points(points);
        c
    }

    /// Replaces the data with one series of `points` (the first series keeps its name and colour).
    pub fn set_points(&mut self, points: Vec<ChartPoint>) {
        let first = self.series.first().cloned().unwrap_or_else(|| ChartSeries::new("Series 1", vec![]));
        self.categories = points.iter().map(|p| p.label.clone()).collect();
        self.series = vec![ChartSeries { values: points.iter().map(|p| p.value).collect(), ..first }];
    }

    /// The first series as labelled points (what pies and progress charts draw).
    pub fn points(&self) -> Vec<ChartPoint> {
        let values = self.series.first().map_or(&[][..], |s| s.values.as_slice());
        self.categories.iter().enumerate().map(|(i, l)| ChartPoint::new(l.clone(), values.get(i).copied().unwrap_or(0.0))).collect()
    }

    /// The value of series `s` at category `c` (0 when missing).
    pub fn value(&self, s: usize, c: usize) -> f64 {
        self.series.get(s).and_then(|s| s.values.get(c)).copied().filter(|v| v.is_finite()).unwrap_or(0.0)
    }

    /// Number of series this chart's kind draws.
    pub fn drawn_series(&self) -> usize {
        if self.kind.multi_series() { self.series.len() } else { self.series.len().min(1) }
    }

    /// Caps the data, squares series up with the categories, makes numbers finite and in range.
    /// Returns true when something changed.
    pub fn sanitize(&mut self) -> bool {
        let before = self.clone();
        self.series.truncate(MAX_CHART_SERIES);
        let longest = self.series.iter().map(|s| s.values.len()).max().unwrap_or(0);
        let n = self.categories.len().max(longest).min(MAX_CHART_POINTS);
        if !self.series.is_empty() || n == 0 {
            self.categories.resize(n, String::new());
        } else {
            self.categories.truncate(n);
        }
        for l in &mut self.categories {
            truncate_chars(l, MAX_CELL_CHARS);
        }
        for s in &mut self.series {
            s.values.resize(n, 0.0);
            for v in &mut s.values {
                *v = finite_or(*v, 0.0).clamp(-MAX_VALUE, MAX_VALUE);
            }
            truncate_chars(&mut s.name, MAX_CELL_CHARS);
        }
        self.colors.truncate(64);
        self.font_size = finite_or(self.font_size, 24.0).clamp(1.0, 2000.0);
        self.donut_hole = finite_or(self.donut_hole, 0.55).clamp(0.0, 0.9);
        self.max = self.max.filter(|m| m.is_finite()).map(|m| m.clamp(-MAX_VALUE, MAX_VALUE));
        self.min = self.min.filter(|m| m.is_finite()).map(|m| m.clamp(-MAX_VALUE, MAX_VALUE));
        if let (Some(lo), Some(hi)) = (self.min, self.max)
            && lo >= hi
        {
            self.min = None;
        }
        self.decimals = self.decimals.map(|d| d.min(6));
        truncate_chars(&mut self.currency, 8);
        truncate_chars(&mut self.x_title, 200);
        truncate_chars(&mut self.y_title, 200);
        truncate_chars(&mut self.font_family, 200);
        *self != before
    }
}

pub(crate) fn finite_or(v: f64, d: f64) -> f64 {
    if v.is_finite() { v } else { d }
}

pub(crate) fn truncate_chars(s: &mut String, max: usize) {
    if let Some((i, _)) = s.char_indices().nth(max) {
        s.truncate(i);
    }
}
