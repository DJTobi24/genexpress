//! Tables: rows of cell text, merged cells, per-cell styles, column widths and row heights.

use genha_color::Color;
use serde::{Deserialize, Serialize};

use crate::data::{MAX_CELL_CHARS, finite_or, truncate_chars};
use crate::{StrokeStyle, TextAlign};

/// Most rows in a table.
pub const MAX_TABLE_ROWS: usize = 100;
/// Most columns in a table.
pub const MAX_TABLE_COLS: usize = 30;
/// Tallest row height a table may ask for, in page pixels.
const MAX_ROW_HEIGHT: f64 = 100_000.0;

fn one() -> usize {
    1
}

/// A block of cells: `rows` × `cols` from (`row`, `col`), counting from 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CellRange {
    pub row: usize,
    pub col: usize,
    #[serde(default = "one")]
    pub rows: usize,
    #[serde(default = "one")]
    pub cols: usize,
}

impl CellRange {
    pub fn new(row: usize, col: usize, rows: usize, cols: usize) -> Self {
        CellRange { row, col, rows: rows.max(1), cols: cols.max(1) }
    }

    /// One cell.
    pub fn cell(row: usize, col: usize) -> Self {
        CellRange::new(row, col, 1, 1)
    }

    /// The row after the last one.
    pub fn end_row(&self) -> usize {
        self.row.saturating_add(self.rows)
    }

    /// The column after the last one.
    pub fn end_col(&self) -> usize {
        self.col.saturating_add(self.cols)
    }

    pub fn contains(&self, row: usize, col: usize) -> bool {
        (self.row..self.end_row()).contains(&row) && (self.col..self.end_col()).contains(&col)
    }

    pub fn intersects(&self, o: &CellRange) -> bool {
        self.row < o.end_row() && o.row < self.end_row() && self.col < o.end_col() && o.col < self.end_col()
    }

    /// The smallest range holding both.
    pub fn union(&self, o: &CellRange) -> CellRange {
        let (r0, c0) = (self.row.min(o.row), self.col.min(o.col));
        CellRange::new(r0, c0, self.end_row().max(o.end_row()) - r0, self.end_col().max(o.end_col()) - c0)
    }

    /// This range cut to a `rows` × `cols` table; `None` when nothing is left.
    pub fn clipped(&self, rows: usize, cols: usize) -> Option<CellRange> {
        if self.row >= rows || self.col >= cols {
            return None;
        }
        Some(CellRange::new(self.row, self.col, self.end_row().min(rows) - self.row, self.end_col().min(cols) - self.col))
    }

    pub fn is_single(&self) -> bool {
        self.rows <= 1 && self.cols <= 1
    }

    /// Every cell, row by row.
    pub fn cells(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        (self.row..self.end_row()).flat_map(move |r| (self.col..self.end_col()).map(move |c| (r, c)))
    }
}

/// How one cell differs from the table's look; `None` fields follow the table.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CellStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_color: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<TextAlign>,
    /// Text size in page pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
}

impl CellStyle {
    pub fn is_empty(&self) -> bool {
        *self == CellStyle::default()
    }
}

/// A cell's own style.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CellFormat {
    pub row: usize,
    pub col: usize,
    #[serde(default)]
    pub style: CellStyle,
}

/// Which lines a table draws (with [`TableEl::border`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BorderLines {
    /// Around the table and between every cell.
    #[default]
    All,
    /// Only around the table.
    Outer,
    /// Only between cells.
    Inner,
    /// Between rows, and along the top and bottom.
    Horizontal,
}

impl BorderLines {
    pub const ALL: [BorderLines; 4] = [BorderLines::All, BorderLines::Outer, BorderLines::Inner, BorderLines::Horizontal];

    pub fn id(self) -> &'static str {
        match self {
            BorderLines::All => "all",
            BorderLines::Outer => "outer",
            BorderLines::Inner => "inner",
            BorderLines::Horizontal => "horizontal",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            BorderLines::All => "All",
            BorderLines::Outer => "Outside",
            BorderLines::Inner => "Inside",
            BorderLines::Horizontal => "Rows",
        }
    }

    pub fn outer(self) -> bool {
        matches!(self, BorderLines::All | BorderLines::Outer)
    }
}

/// What a cell is in a table with merged cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellSpan {
    /// An ordinary cell.
    Own,
    /// The top-left cell of a merge: it holds the text and covers the range.
    Anchor(CellRange),
    /// Hidden under the merge of this range.
    Covered(CellRange),
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
    /// Every other body row (banded rows); `None` keeps them all [`TableEl::row_fill`]. Written
    /// as `null` when off (a missing field means the default, which is on).
    pub alt_row_fill: Option<Color>,
    /// Lines around and between the cells (`null` when off, as above).
    pub border: Option<StrokeStyle>,
    /// Which of the lines are drawn.
    pub borders: BorderLines,
    /// A different line around the table; `None` uses [`TableEl::border`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outer_border: Option<StrokeStyle>,
    /// Space between a cell's edge and its text, in page pixels.
    pub padding: f64,
    pub align: TextAlign,
    /// Column widths as fractions of the table's width (normalised); empty means equal columns.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub column_widths: Vec<f64>,
    /// Least height of each row in page pixels (0 or missing: as tall as its text); rows still
    /// grow to fit their text.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub row_heights: Vec<f64>,
    /// Merged cells; the top-left cell of each holds its text.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub merges: Vec<CellRange>,
    /// Cells with their own fill, text colour, weight, alignment or size.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cell_styles: Vec<CellFormat>,
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
            borders: BorderLines::All,
            outer_border: None,
            padding: 12.0,
            align: TextAlign::Left,
            column_widths: vec![],
            row_heights: vec![],
            merges: vec![],
            cell_styles: vec![],
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

    /// The least height asked for row `r` (0: as tall as its text).
    pub fn row_height(&self, r: usize) -> f64 {
        self.row_heights.get(r).copied().filter(|h| h.is_finite() && *h > 0.0).unwrap_or(0.0)
    }

    /// What every cell is (own, merge anchor, covered by a merge), row by row; merges that
    /// overlap an earlier one or leave the table are ignored.
    pub fn spans(&self) -> Vec<Vec<CellSpan>> {
        let (rows, cols) = (self.rows.len().min(MAX_TABLE_ROWS), self.columns().min(MAX_TABLE_COLS));
        let mut g = vec![vec![CellSpan::Own; cols]; rows];
        for m in &self.merges {
            let Some(m) = m.clipped(rows, cols) else { continue };
            if m.is_single() || m.cells().any(|(r, c)| !matches!(g.get(r).and_then(|row| row.get(c)), Some(CellSpan::Own))) {
                continue;
            }
            for (r, c) in m.cells() {
                if let Some(cell) = g.get_mut(r).and_then(|row| row.get_mut(c)) {
                    *cell = if (r, c) == (m.row, m.col) { CellSpan::Anchor(m) } else { CellSpan::Covered(m) };
                }
            }
        }
        g
    }

    /// The merge covering a cell, if any.
    pub fn merge_at(&self, row: usize, col: usize) -> Option<CellRange> {
        self.merges.iter().find(|m| m.contains(row, col)).copied()
    }

    /// A cell's own style (`None`: the table's look).
    pub fn style_at(&self, row: usize, col: usize) -> Option<&CellStyle> {
        self.cell_styles.iter().rev().find(|f| f.row == row && f.col == col).map(|f| &f.style)
    }

    /// Cell styles as a grid, row by row (for drawing).
    pub fn style_grid(&self) -> Vec<Vec<Option<&CellStyle>>> {
        let (rows, cols) = (self.rows.len().min(MAX_TABLE_ROWS), self.columns().min(MAX_TABLE_COLS));
        let mut g = vec![vec![None; cols]; rows];
        for f in &self.cell_styles {
            if let Some(cell) = g.get_mut(f.row).and_then(|r| r.get_mut(f.col)) {
                *cell = Some(&f.style);
            }
        }
        g
    }

    /// Changes the style of every cell in `range` with `f`; cells left with no style of their own
    /// lose their entry. Returns the number of cells in the range.
    pub fn style_cells(&mut self, range: CellRange, f: impl Fn(&mut CellStyle)) -> usize {
        let Some(range) = range.clipped(self.rows.len(), self.columns()) else { return 0 };
        let mut n = 0;
        for (r, c) in range.cells() {
            n += 1;
            match self.cell_styles.iter_mut().find(|x| x.row == r && x.col == c) {
                Some(x) => f(&mut x.style),
                None => {
                    let mut style = CellStyle::default();
                    f(&mut style);
                    self.cell_styles.push(CellFormat { row: r, col: c, style });
                }
            }
        }
        self.cell_styles.retain(|x| !x.style.is_empty());
        self.cell_styles.sort_by_key(|x| (x.row, x.col));
        n
    }

    /// Merges `range` into one cell, also taking in any merge it overlaps. The texts of the cells
    /// join (with spaces) in the top-left one. Returns the merged range, `None` when it is a single
    /// cell or outside the table.
    pub fn merge_cells(&mut self, range: CellRange) -> Option<CellRange> {
        let mut range = range.clipped(self.rows.len(), self.columns())?;
        loop {
            let grown = self.merges.iter().filter(|m| m.intersects(&range)).fold(range, |a, m| a.union(m));
            let grown = grown.clipped(self.rows.len(), self.columns())?;
            if grown == range {
                break;
            }
            range = grown;
        }
        if range.is_single() {
            return None;
        }
        self.merges.retain(|m| !m.intersects(&range));
        let texts: Vec<String> = range.cells().map(|(r, c)| self.cell(r, c).trim().to_string()).filter(|t| !t.is_empty()).collect();
        let mut joined = texts.join(" ");
        truncate_chars(&mut joined, MAX_CELL_CHARS);
        for (r, c) in range.cells() {
            if let Some(cell) = self.rows.get_mut(r).and_then(|row| row.get_mut(c)) {
                cell.clear();
            }
        }
        if let Some(cell) = self.rows.get_mut(range.row).and_then(|row| row.get_mut(range.col)) {
            *cell = joined;
        }
        self.merges.push(range);
        Some(range)
    }

    /// Splits every merge that overlaps `range`. Returns how many were split.
    pub fn split_cells(&mut self, range: CellRange) -> usize {
        let before = self.merges.len();
        self.merges.retain(|m| !m.intersects(&range));
        before - self.merges.len()
    }

    /// Inserts an empty row before row `at` (at the end when past it); merges it falls inside grow.
    pub fn insert_row(&mut self, at: usize) {
        let at = at.min(self.rows.len());
        let cols = self.columns().max(1);
        self.rows.insert(at, vec![String::new(); cols]);
        for m in &mut self.merges {
            if at <= m.row {
                m.row += 1;
            } else if at < m.end_row() {
                m.rows += 1;
            }
        }
        for f in &mut self.cell_styles {
            if f.row >= at {
                f.row += 1;
            }
        }
        if !self.row_heights.is_empty() {
            self.row_heights.resize(self.rows.len() - 1, 0.0);
            self.row_heights.insert(at, 0.0);
        }
    }

    /// Removes row `at` (nothing when there is no such row); merges over it shrink, and a merge
    /// whose top row goes keeps its text in its new top row.
    pub fn delete_row(&mut self, at: usize) {
        if at >= self.rows.len() {
            return;
        }
        for m in &self.merges.clone() {
            if m.row == at && m.rows > 1 {
                self.move_anchor(m.row, m.col, m.row + 1, m.col);
            }
        }
        self.rows.remove(at);
        for m in &mut self.merges {
            if m.row > at {
                m.row -= 1;
            } else if at < m.end_row() {
                m.rows -= 1;
            }
        }
        self.merges.retain(|m| m.rows > 0 && !m.is_single());
        self.cell_styles.retain(|f| f.row != at);
        for f in &mut self.cell_styles {
            if f.row > at {
                f.row -= 1;
            }
        }
        if at < self.row_heights.len() {
            self.row_heights.remove(at);
        }
    }

    /// Inserts an empty column before column `at` (at the end when past it); merges it falls
    /// inside grow. Set column widths give the new column their mean.
    pub fn insert_col(&mut self, at: usize) {
        let n = self.columns();
        let at = at.min(n);
        for r in &mut self.rows {
            r.resize(n, String::new());
            r.insert(at, String::new());
        }
        if !self.column_widths.is_empty() {
            let mean = self.column_widths.iter().sum::<f64>() / self.column_widths.len() as f64;
            self.column_widths.resize(n, mean);
            self.column_widths.insert(at.min(self.column_widths.len()), mean);
        }
        for m in &mut self.merges {
            if at <= m.col {
                m.col += 1;
            } else if at < m.end_col() {
                m.cols += 1;
            }
        }
        for f in &mut self.cell_styles {
            if f.col >= at {
                f.col += 1;
            }
        }
    }

    /// Removes column `at` (nothing when there is no such column); merges over it shrink, and a
    /// merge whose left column goes keeps its text in its new left column.
    pub fn delete_col(&mut self, at: usize) {
        if at >= self.columns() {
            return;
        }
        for m in &self.merges.clone() {
            if m.col == at && m.cols > 1 {
                self.move_anchor(m.row, m.col, m.row, m.col + 1);
            }
        }
        for r in &mut self.rows {
            if at < r.len() {
                r.remove(at);
            }
        }
        if at < self.column_widths.len() {
            self.column_widths.remove(at);
        }
        for m in &mut self.merges {
            if m.col > at {
                m.col -= 1;
            } else if at < m.end_col() {
                m.cols -= 1;
            }
        }
        self.merges.retain(|m| m.cols > 0 && !m.is_single());
        self.cell_styles.retain(|f| f.col != at);
        for f in &mut self.cell_styles {
            if f.col > at {
                f.col -= 1;
            }
        }
    }

    /// Moves a merge's text and style from its anchor to the cell that becomes its anchor.
    fn move_anchor(&mut self, r0: usize, c0: usize, r1: usize, c1: usize) {
        let text = self.rows.get_mut(r0).and_then(|r| r.get_mut(c0)).map(std::mem::take).unwrap_or_default();
        if let Some(cell) = self.rows.get_mut(r1).and_then(|r| r.get_mut(c1))
            && cell.is_empty()
        {
            *cell = text;
        }
        if let Some(style) = self.style_at(r0, c0).cloned()
            && self.style_at(r1, c1).is_none()
        {
            self.cell_styles.push(CellFormat { row: r1, col: c1, style });
        }
    }

    /// Caps rows, columns and text, squares the grid up, drops merges and cell styles outside the
    /// table (and merges overlapping an earlier one), makes numbers finite. Returns true when
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
        let rows = self.rows.len();
        if self.column_widths.len() > cols {
            self.column_widths.truncate(cols);
        }
        for w in &mut self.column_widths {
            *w = finite_or(*w, 0.0).clamp(0.0, 1e6);
        }
        self.row_heights.truncate(rows);
        for h in &mut self.row_heights {
            *h = finite_or(*h, 0.0).clamp(0.0, MAX_ROW_HEIGHT);
        }
        if self.row_heights.iter().all(|h| *h == 0.0) {
            self.row_heights.clear();
        }
        // Merges: inside the table, more than one cell, no overlaps (the first one wins).
        self.merges.truncate(MAX_TABLE_ROWS * MAX_TABLE_COLS);
        let mut taken = vec![false; rows * cols];
        let mut kept = vec![];
        for m in &self.merges {
            let Some(m) = m.clipped(rows, cols) else { continue };
            if m.is_single() || m.cells().any(|(r, c)| taken.get(r * cols + c).copied().unwrap_or(true)) {
                continue;
            }
            for (r, c) in m.cells() {
                if let Some(t) = taken.get_mut(r * cols + c) {
                    *t = true;
                }
            }
            kept.push(m);
        }
        self.merges = kept;
        // Cell styles: inside the table, one per cell (the last one wins), not empty, in order.
        self.cell_styles.truncate(MAX_TABLE_ROWS * MAX_TABLE_COLS * 4);
        let mut grid: Vec<Option<CellStyle>> = vec![None; rows * cols];
        for f in self.cell_styles.drain(..) {
            if f.row < rows
                && f.col < cols
                && let Some(slot) = grid.get_mut(f.row * cols + f.col)
            {
                *slot = Some(f.style);
            }
        }
        for (i, style) in grid.into_iter().enumerate() {
            let Some(mut style) = style else { continue };
            style.font_size = style.font_size.filter(|s| s.is_finite()).map(|s| s.clamp(1.0, 2000.0));
            if !style.is_empty() {
                self.cell_styles.push(CellFormat { row: i / cols.max(1), col: i % cols.max(1), style });
            }
        }
        self.font_size = finite_or(self.font_size, 24.0).clamp(1.0, 2000.0);
        self.padding = finite_or(self.padding, 12.0).clamp(0.0, 1000.0);
        for b in [&mut self.border, &mut self.outer_border].into_iter().flatten() {
            b.width = finite_or(b.width, 1.0).clamp(0.0, 1000.0);
            b.dash.truncate(16);
        }
        truncate_chars(&mut self.font_family, 200);
        *self != before
    }
}
