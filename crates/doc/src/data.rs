//! Data elements: charts and tables.
//!
//! Both are pure data here; `genha-render` turns them into shapes and text fitted to the element's
//! frame. Every field has a default so older or hand-written files open, and [`ChartEl::sanitize`]
//! / [`TableEl::sanitize`] cap what untrusted files and commands can ask for.

use genha_color::Color;
use serde::{Deserialize, Serialize};

use crate::{Fill, StrokeStyle, TextAlign};

/// Most data points in a chart.
pub const MAX_CHART_POINTS: usize = 200;
/// Most rows in a table.
pub const MAX_TABLE_ROWS: usize = 100;
/// Most columns in a table.
pub const MAX_TABLE_COLS: usize = 30;
/// Longest label or cell text, in characters.
pub const MAX_CELL_CHARS: usize = 2_000;

/// How a chart draws its data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChartKind {
    /// Vertical bars (columns), one per data point.
    #[default]
    Bar,
    /// Horizontal bars, labels on the left.
    HorizontalBar,
    /// Points joined by a line.
    Line,
    /// A line with the area under it filled.
    Area,
    /// Slices of a circle, starting at 12 o'clock and going clockwise.
    Pie,
    /// A pie with a hole ([`ChartEl::donut_hole`]).
    Donut,
    /// The first data point as a share of [`ChartEl::max`] (100 when unset): a bar when the frame
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
}

/// How values are written on a chart.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueFormat {
    /// The value itself (`1,250`).
    #[default]
    Number,
    /// Each value's share of the total (`25%`); a progress chart's share of its maximum.
    Percent,
}

/// One labelled value.
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ChartEl {
    pub kind: ChartKind,
    pub data: Vec<ChartPoint>,
    /// Series colours, used in turn; empty means the brand colours (or a built-in palette).
    pub colors: Vec<Color>,
    /// Category names (axis labels; names inside pie slices).
    pub show_labels: bool,
    /// Values on the bars, points and slices.
    pub show_values: bool,
    /// A key of colours and names (bar and pie charts).
    pub show_legend: bool,
    pub value_format: ValueFormat,
    pub font_family: String,
    /// Size of labels, in page pixels.
    pub font_size: f64,
    pub text_color: Color,
    /// Behind the whole chart.
    pub background: Fill,
    /// A donut's hole as a fraction of its radius, `0..=0.9`.
    pub donut_hole: f64,
    /// The top of the value scale (bars, lines) or a progress chart's 100%; `None` fits the data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

impl Default for ChartEl {
    fn default() -> Self {
        ChartEl {
            kind: ChartKind::Bar,
            data: vec![],
            colors: vec![],
            show_labels: true,
            show_values: true,
            show_legend: false,
            value_format: ValueFormat::Number,
            font_family: "Sans".into(),
            font_size: 24.0,
            text_color: Color::rgb(0x1b, 0x1b, 0x2f),
            background: Fill::None,
            donut_hole: 0.55,
            max: None,
        }
    }
}

impl ChartEl {
    /// A chart of `kind` with sample data.
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
        ChartEl {
            kind,
            data,
            show_legend: pie,
            value_format: if pie || kind == ChartKind::Progress { ValueFormat::Percent } else { ValueFormat::Number },
            ..ChartEl::default()
        }
    }

    /// Caps the data, makes numbers finite and in range. Returns true when something changed.
    pub fn sanitize(&mut self) -> bool {
        let before = self.clone();
        self.data.truncate(MAX_CHART_POINTS);
        for p in &mut self.data {
            if !p.value.is_finite() {
                p.value = 0.0;
            }
            p.value = p.value.clamp(-1e15, 1e15);
            truncate_chars(&mut p.label, MAX_CELL_CHARS);
        }
        self.colors.truncate(64);
        self.font_size = finite_or(self.font_size, 24.0).clamp(1.0, 2000.0);
        self.donut_hole = finite_or(self.donut_hole, 0.55).clamp(0.0, 0.9);
        self.max = self.max.filter(|m| m.is_finite() && *m > 0.0).map(|m| m.min(1e15));
        truncate_chars(&mut self.font_family, 200);
        *self != before
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TableEl {
    /// Cell text, row by row; the first row is the header when [`TableEl::header_row`] is set.
    pub rows: Vec<Vec<String>>,
    pub header_row: bool,
    pub font_family: String,
    /// Size of the cell text, in page pixels.
    pub font_size: f64,
    pub text_color: Color,
    pub header_fill: Color,
    pub header_text: Color,
    pub row_fill: Color,
    /// Every other body row (banded rows); `None` keeps them all [`TableEl::row_fill`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_row_fill: Option<Color>,
    /// Lines around and between the cells.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<StrokeStyle>,
    /// Space between a cell's edge and its text, in page pixels.
    pub padding: f64,
    pub align: TextAlign,
    /// Column widths as fractions of the table's width (normalised); empty means equal columns.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub column_widths: Vec<f64>,
}

impl Default for TableEl {
    fn default() -> Self {
        TableEl {
            rows: vec![],
            header_row: true,
            font_family: "Sans".into(),
            font_size: 24.0,
            text_color: Color::rgb(0x1b, 0x1b, 0x2f),
            header_fill: Color::rgb(0x5b, 0x3c, 0xf5),
            header_text: Color::WHITE,
            row_fill: Color::WHITE,
            alt_row_fill: Some(Color::rgb(0xf3, 0xf1, 0xfe)),
            border: Some(StrokeStyle { color: Color::rgb(0xd9, 0xd6, 0xe8), width: 1.5, dash: vec![] }),
            padding: 12.0,
            align: TextAlign::Left,
            column_widths: vec![],
        }
    }
}

impl TableEl {
    /// A `rows` × `cols` table with a header row and empty cells.
    pub fn blank(rows: usize, cols: usize) -> Self {
        let (rows, cols) = (rows.clamp(1, MAX_TABLE_ROWS), cols.clamp(1, MAX_TABLE_COLS));
        let mut t = TableEl { rows: vec![vec![String::new(); cols]; rows], ..TableEl::default() };
        if let Some(head) = t.rows.first_mut() {
            for (i, c) in head.iter_mut().enumerate() {
                *c = format!("Column {}", i + 1);
            }
        }
        t
    }

    /// Number of columns (the longest row).
    pub fn columns(&self) -> usize {
        self.rows.iter().map(Vec::len).max().unwrap_or(0)
    }

    /// The text in a cell ("" outside the table).
    pub fn cell(&self, row: usize, col: usize) -> &str {
        self.rows.get(row).and_then(|r| r.get(col)).map_or("", String::as_str)
    }

    /// Column widths as fractions summing to 1, one per column.
    pub fn fractions(&self) -> Vec<f64> {
        let n = self.columns();
        if n == 0 {
            return vec![];
        }
        let given: Vec<f64> = (0..n).map(|i| self.column_widths.get(i).copied().filter(|w| w.is_finite() && *w > 0.0).unwrap_or(0.0)).collect();
        let sum: f64 = given.iter().sum();
        if self.column_widths.is_empty() || sum <= 0.0 || given.contains(&0.0) {
            return vec![1.0 / n as f64; n];
        }
        given.iter().map(|w| w / sum).collect()
    }

    /// Caps rows, columns and text, squares the grid up, makes numbers finite. Returns true when
    /// something changed.
    pub fn sanitize(&mut self) -> bool {
        let before = self.clone();
        self.rows.truncate(MAX_TABLE_ROWS);
        let cols = self.columns().clamp(if self.rows.is_empty() { 0 } else { 1 }, MAX_TABLE_COLS);
        for r in &mut self.rows {
            r.resize(cols, String::new());
            for c in r.iter_mut() {
                truncate_chars(c, MAX_CELL_CHARS);
            }
        }
        if self.column_widths.len() > cols {
            self.column_widths.truncate(cols);
        }
        for w in &mut self.column_widths {
            *w = finite_or(*w, 0.0).clamp(0.0, 1e6);
        }
        self.font_size = finite_or(self.font_size, 24.0).clamp(1.0, 2000.0);
        self.padding = finite_or(self.padding, 12.0).clamp(0.0, 1000.0);
        if let Some(b) = &mut self.border {
            b.width = finite_or(b.width, 1.0).clamp(0.0, 1000.0);
        }
        truncate_chars(&mut self.font_family, 200);
        *self != before
    }
}

fn finite_or(v: f64, d: f64) -> f64 {
    if v.is_finite() { v } else { d }
}

fn truncate_chars(s: &mut String, max: usize) {
    if let Some((i, _)) = s.char_indices().nth(max) {
        s.truncate(i);
    }
}
