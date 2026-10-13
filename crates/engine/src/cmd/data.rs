//! Charts, tables and curved text.

use std::sync::Arc;

use serde_json::{Value, json};

use super::create::{add_element, frame_params, page_size, text_height};
use super::{bad, cmd, f64_req, has_doc, has_selection, merge, ok, str_param, targets, usize_param};
use crate::{CommandSpec, EngineError, Result, Session};
use genha_doc::{
    CellRange, ChartEl, ChartKind, ChartPoint, ChartSeries, Color, Element, ElementKind, LegendPosition, MAX_CELL_CHARS, MAX_CHART_POINTS,
    MAX_CHART_SERIES, MAX_CURVE, MAX_TABLE_COLS, MAX_TABLE_ROWS, NodeId, TableEl, TextAlign,
};
use genha_text::FontBook;

const RANGE_DOC: &str = "row, col (from 0; row 0 is the header), rows?: 1, cols?: 1";

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "chart.add",
            "Add Chart",
            ["Insert"],
            None,
            "{kind?: bar|horizontal-bar|line|area|pie|donut|progress|ring = bar, data/csv/rows/categories/series?: as chart.setData, x?, y?, width?, height?, colors?: [#rrggbb…], title?: name, …style fields of chart.setStyle} — sample data and brand colours by default; `ring` is a round progress chart",
            has_doc,
            chart_add
        ),
        cmd!(
            "chart.setData",
            "Chart Data",
            [],
            None,
            "{ids?, data?: [{label, value}…] | [[label, value]…] | \"Label: 10, Other: 20\" (one series; one item per line, `;` or `,`), csv?: \"…\" (comma- or tab-separated, \"quoted, cells\": first row the series names, first column the labels), rows?: [[cell…]…] (the same, as cells), categories?: [label…], series?: [{name, values: [number…], color?: #rrggbb|null}…]} — categories or series alone keep the other",
            has_selection,
            chart_set_data
        ),
        cmd!(
            "chart.setStyle",
            "Chart Style",
            [],
            None,
            "{ids?, …partial chart: kind, stacked, colors: [#rrggbb…], show_labels, show_values, show_legend, legend: none|auto|top|bottom|right, legend_position, gridlines, x_title, y_title, value_format: number|percent|currency, currency: symbol, decimals: 0–6|null, font_family, font_size, text_color, background: Fill JSON, donut_hole: 0–0.9, min: number|null, max: number|null}",
            has_selection,
            chart_set_style
        ),
        cmd!("chart.setKind", "Chart Type", [], None, "{ids?, kind: bar|horizontal-bar|line|area|pie|donut|progress}", has_selection, chart_set_kind),
        cmd!(
            "table.add",
            "Add Table",
            ["Insert"],
            None,
            "{rows?: [[cell…]…] | number of rows = 3, columns?: number = 3, csv?: \"a,b\\nc,d\" (tabs work too; \"quoted, cells\"), header?: bool = true, x?, y?, width?, height?}",
            has_doc,
            table_add
        ),
        cmd!(
            "table.setCell",
            "Edit Cell",
            [],
            None,
            "{id?, row, col, text} — rows and columns count from 0 (row 0 is the header); a cell under a merge edits the merge's text",
            has_selection,
            table_set_cell
        ),
        cmd!(
            "table.insertRow",
            "Insert Row",
            [],
            None,
            "{id?, index?: before this row (default: at the end)} — merges it falls inside grow",
            has_selection,
            table_insert_row
        ),
        cmd!(
            "table.deleteRow",
            "Delete Row",
            [],
            None,
            "{id?, index?: default the last row} — merges over it shrink",
            has_selection,
            table_delete_row
        ),
        cmd!(
            "table.insertColumn",
            "Insert Column",
            [],
            None,
            "{id?, index?: before this column (default: at the end)} — merges it falls inside grow",
            has_selection,
            table_insert_col
        ),
        cmd!(
            "table.deleteColumn",
            "Delete Column",
            [],
            None,
            "{id?, index?: default the last column} — merges over it shrink",
            has_selection,
            table_delete_col
        ),
        cmd!(
            "table.setStyle",
            "Table Style",
            [],
            None,
            "{ids?, …partial table: header_row, font_family, font_size, text_color, header_fill, header_text, row_fill, alt_row_fill: #rrggbb|null, border: {color, width}|null, borders: all|outer|inner|horizontal, outer_border: {color, width}|null (default: as border), padding, align: left|center|right, column_widths: [fractions], row_heights: [px, 0 = fit]}",
            has_selection,
            table_set_style
        ),
        cmd!(
            "table.setRows",
            "Table Contents",
            [],
            None,
            "{id?, rows: [[cell…]…] | csv: \"a,b\\nc,d\"} — merges and cell styles outside the new size go",
            has_selection,
            table_set_rows
        ),
        cmd!(
            "table.mergeCells",
            "Merge Cells",
            [],
            None,
            "{id?, row, col (from 0; row 0 is the header), rows?: 1, cols?: 1} — one cell spanning the range (merges it overlaps are taken in); the texts join in its top-left cell",
            has_selection,
            table_merge_cells
        ),
        cmd!(
            "table.splitCell",
            "Split Cells",
            [],
            None,
            "{id?, row, col, rows?: 1, cols?: 1} — undoes every merge overlapping the range",
            has_selection,
            table_split_cells
        ),
        cmd!(
            "table.setCellStyle",
            "Cell Style",
            [],
            None,
            "{id?, row, col (from 0; row 0 is the header), rows?: 1, cols?: 1, fill?: #rrggbb|null, text_color?: #rrggbb|null, bold?: bool|null, align?: left|center|right|null, font_size?: px|null, clear?: bool} — null (or clear) goes back to the table's look",
            has_selection,
            table_set_cell_style
        ),
        cmd!(
            "table.setColumnWidth",
            "Column Width",
            [],
            None,
            "{id?, col, width: px} — the other columns share the rest of the table's width; {auto: true} fits every column to its text, {equal: true} makes them equal",
            has_selection,
            table_set_column_width
        ),
        cmd!(
            "table.setRowHeight",
            "Row Height",
            [],
            None,
            "{id?, row?: default every row, height: px (0 = fit the text)} — rows still grow to fit their text",
            has_selection,
            table_set_row_height
        ),
        cmd!(
            "text.setCurve",
            "Curve Text",
            [],
            None,
            "{ids?, degrees: -360..360 — the arc the widest line spans; positive arches over the top of a circle, negative curves under it like a smile, 0 straightens; the frame's height follows}",
            has_selection,
            text_set_curve
        ),
    ]
}

// ---------- parsing ----------

/// Chart data from params: a list of points or pairs, or text like `"Label: 10, Other: 20"`.
pub fn parse_data(v: &Value, cmd: &str) -> Result<Vec<ChartPoint>> {
    let out = match v {
        Value::String(s) => parse_data_text(s, cmd)?,
        Value::Array(items) => {
            if items.len() > MAX_CHART_POINTS {
                return Err(bad(cmd, format!("at most {MAX_CHART_POINTS} data points")));
            }
            items
                .iter()
                .enumerate()
                .map(|(i, it)| match it {
                    Value::Object(o) => {
                        let label = o.get("label").map(cell_text).unwrap_or_default();
                        let value = o.get("value").and_then(number).ok_or_else(|| bad(cmd, format!("data[{i}]: missing number `value`")))?;
                        Ok(ChartPoint::new(label, value))
                    }
                    Value::Array(pair) => match pair.as_slice() {
                        [l, v] => Ok(ChartPoint::new(cell_text(l), number(v).ok_or_else(|| bad(cmd, format!("data[{i}]: not a number")))?)),
                        _ => Err(bad(cmd, format!("data[{i}]: expected [label, value]"))),
                    },
                    other => {
                        let value = number(other).ok_or_else(|| bad(cmd, format!("data[{i}]: expected {{label, value}}")))?;
                        Ok(ChartPoint::new(format!("{}", i + 1), value))
                    }
                })
                .collect::<Result<Vec<_>>>()?
        }
        _ => return Err(bad(cmd, "`data` must be a list of {label, value} or text like \"A: 10, B: 20\"")),
    };
    let mut c = ChartEl::with_points(out);
    c.sanitize();
    Ok(c.points())
}

fn number(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64().filter(|f| f.is_finite()),
        Value::String(s) => parse_number(s),
        _ => None,
    }
}

/// A number written by a person: `1,200`, `45%`, `$9.99`, `-3`.
fn parse_number(s: &str) -> Option<f64> {
    let t: String = s.chars().filter(|c| !matches!(c, ',' | '%' | '$' | '€' | '£' | '¥' | ' ' | '\u{a0}' | '_')).collect();
    t.parse::<f64>().ok().filter(|f| f.is_finite())
}

fn cell_text(v: &Value) -> String {
    let s = match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    };
    s.chars().take(MAX_CELL_CHARS).collect()
}

/// `"Label: 10, Other: 20"`, one item per line, or separated by `;` or `,`.
fn parse_data_text(s: &str, cmd: &str) -> Result<Vec<ChartPoint>> {
    if s.len() > 200_000 {
        return Err(bad(cmd, "data text is too long"));
    }
    let items: Vec<&str> = if s.contains('\n') {
        s.lines().collect()
    } else if s.contains(';') {
        s.split(';').collect()
    } else {
        s.split(',').collect()
    };
    let mut out = vec![];
    for (i, item) in items.iter().map(|x| x.trim()).filter(|x| !x.is_empty()).enumerate() {
        if out.len() >= MAX_CHART_POINTS {
            return Err(bad(cmd, format!("at most {MAX_CHART_POINTS} data points")));
        }
        let split = item.rfind([':', '=', '\t']).map(|k| (&item[..k], &item[k + 1..])).or_else(|| item.rsplit_once(' '));
        let p = match split {
            Some((l, v)) => parse_number(v).map(|v| ChartPoint::new(l.trim(), v)),
            None => parse_number(item).map(|v| ChartPoint::new(format!("{}", i + 1), v)),
        };
        out.push(p.ok_or_else(|| bad(cmd, format!("cannot read `{item}` (write label: number)")))?);
    }
    Ok(out)
}

/// The cells of one line of CSV (`sep` `,`) or TSV (`sep` tab): `"quoted, cells"` keep their
/// separators, `""` inside quotes is a quote.
fn split_line(line: &str, sep: char) -> Vec<String> {
    let mut cells = vec![];
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' if quoted => quoted = false,
            '"' if cur.trim().is_empty() => {
                cur.clear();
                quoted = true;
            }
            c if c == sep && !quoted => cells.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    cells.push(cur);
    cells.into_iter().map(|c| c.trim().chars().take(MAX_CELL_CHARS).collect()).collect()
}

/// Rows of cells from CSV or TSV text (tabs when there are any), blank lines left out.
fn split_text(csv: &str, max_rows: usize, max_cols: usize) -> Vec<Vec<String>> {
    let sep = if csv.contains('\t') { '\t' } else { ',' };
    csv.lines().filter(|l| !l.trim().is_empty()).take(max_rows).map(|l| split_line(l, sep).into_iter().take(max_cols).collect()).collect()
}

/// Rows of cells from a JSON list of lists.
fn json_rows(v: &Value, cmd: &str, max_rows: usize, max_cols: usize) -> Result<Vec<Vec<String>>> {
    let Value::Array(rows) = v else { return Err(bad(cmd, "`rows` must be a list of rows of cells")) };
    if rows.len() > max_rows {
        return Err(bad(cmd, format!("at most {max_rows} rows")));
    }
    let mut out = vec![];
    for (i, r) in rows.iter().enumerate() {
        let Value::Array(cells) = r else { return Err(bad(cmd, format!("rows[{i}] must be a list of cells"))) };
        if cells.len() > max_cols {
            return Err(bad(cmd, format!("at most {max_cols} columns")));
        }
        out.push(cells.iter().map(cell_text).collect());
    }
    Ok(out)
}

/// Table rows from `rows` (cells) or `csv` text.
fn parse_rows(p: &Value, cmd: &str) -> Result<Option<Vec<Vec<String>>>> {
    if let Some(csv) = str_param(p, "csv") {
        if csv.len() > 1 << 20 {
            return Err(bad(cmd, "csv text is too long"));
        }
        return Ok(Some(split_text(csv, MAX_TABLE_ROWS, MAX_TABLE_COLS)));
    }
    match p.get("rows") {
        Some(v @ Value::Array(_)) => json_rows(v, cmd, MAX_TABLE_ROWS, MAX_TABLE_COLS).map(Some),
        _ => Ok(None),
    }
}

/// A chart's data from a grid of cells: the first column holds the category labels, the other
/// columns one series each, named by the first row (always: years like `2024` are names too;
/// empty names become `Series n`).
fn chart_grid(rows: &[Vec<String>], cmd: &str) -> Result<(Vec<String>, Vec<ChartSeries>)> {
    let width = rows.iter().map(Vec::len).max().unwrap_or(0);
    if width < 2 {
        return Err(bad(cmd, "write the labels in the first column and the values in the next ones"));
    }
    let n_series = width - 1;
    if n_series > MAX_CHART_SERIES {
        return Err(bad(cmd, format!("at most {MAX_CHART_SERIES} series")));
    }
    let (names, body) = match rows.split_first() {
        Some((h, rest)) => (h.iter().skip(1).cloned().collect::<Vec<_>>(), rest),
        None => (vec![], rows),
    };
    if body.len() > MAX_CHART_POINTS {
        return Err(bad(cmd, format!("at most {MAX_CHART_POINTS} data points")));
    }
    let mut series: Vec<ChartSeries> = (0..n_series)
        .map(|i| {
            let name = names.get(i).map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).unwrap_or_else(|| format!("Series {}", i + 1));
            ChartSeries::new(name, vec![])
        })
        .collect();
    let mut cats = vec![];
    for (r, row) in body.iter().enumerate() {
        cats.push(row.first().cloned().unwrap_or_default());
        for (i, s) in series.iter_mut().enumerate() {
            let cell = row.get(i + 1).map_or("", |c| c.trim());
            let v = if cell.is_empty() {
                0.0
            } else {
                parse_number(cell).ok_or_else(|| bad(cmd, format!("row {}, column {}: `{cell}` is not a number", r + 1, i + 2)))?
            };
            s.values.push(v);
        }
    }
    Ok((cats, series))
}

/// Series from a JSON list of `{name, values, color?}`.
fn json_series(v: &Value, cmd: &str) -> Result<Vec<ChartSeries>> {
    let Value::Array(items) = v else { return Err(bad(cmd, "`series` must be a list of {name, values}")) };
    if items.len() > MAX_CHART_SERIES {
        return Err(bad(cmd, format!("at most {MAX_CHART_SERIES} series")));
    }
    items
        .iter()
        .enumerate()
        .map(|(i, it)| {
            let Value::Object(o) = it else { return Err(bad(cmd, format!("series[{i}] must be {{name, values}}"))) };
            let name = o.get("name").map(cell_text).unwrap_or_else(|| format!("Series {}", i + 1));
            let values = match o.get("values") {
                None | Some(Value::Null) => vec![],
                Some(Value::Array(vals)) => {
                    if vals.len() > MAX_CHART_POINTS {
                        return Err(bad(cmd, format!("at most {MAX_CHART_POINTS} values in a series")));
                    }
                    vals.iter()
                        .enumerate()
                        .map(|(k, v)| match v {
                            Value::Null => Ok(0.0),
                            v => number(v).ok_or_else(|| bad(cmd, format!("series[{i}].values[{k}] is not a number"))),
                        })
                        .collect::<Result<Vec<_>>>()?
                }
                Some(_) => return Err(bad(cmd, format!("series[{i}].values must be a list of numbers"))),
            };
            let color = match o.get("color") {
                None | Some(Value::Null) => None,
                Some(Value::String(s)) => Some(Color::parse(s).ok_or_else(|| bad(cmd, format!("series[{i}].color: invalid colour `{s}`")))?),
                Some(_) => return Err(bad(cmd, format!("series[{i}].color must be a colour like \"#ff8800\""))),
            };
            Ok(ChartSeries { name, values, color })
        })
        .collect()
}

/// New chart data from params, applied to a chart by [`ChartData::apply`].
#[derive(Clone, Default)]
struct ChartData {
    /// One series of points (`data`).
    points: Option<Vec<ChartPoint>>,
    categories: Option<Vec<String>>,
    series: Option<Vec<ChartSeries>>,
}

impl ChartData {
    fn parse(p: &Value, cmd: &str) -> Result<ChartData> {
        let mut d = ChartData::default();
        if let Some(v) = p.get("data").filter(|v| !v.is_null()) {
            d.points = Some(parse_data(v, cmd)?);
            return Ok(d);
        }
        let grid = match (str_param(p, "csv").or_else(|| str_param(p, "tsv")), p.get("rows")) {
            (Some(text), _) => {
                if text.len() > 1 << 20 {
                    return Err(bad(cmd, "csv text is too long"));
                }
                Some(split_text(text, MAX_CHART_POINTS + 2, MAX_CHART_SERIES + 2))
            }
            (None, Some(v @ Value::Array(_))) => Some(json_rows(v, cmd, MAX_CHART_POINTS + 1, MAX_CHART_SERIES + 1)?),
            _ => None,
        };
        if let Some(rows) = grid {
            let (cats, series) = chart_grid(&rows, cmd)?;
            d.categories = Some(cats);
            d.series = Some(series);
            return Ok(d);
        }
        match p.get("categories") {
            None | Some(Value::Null) => {}
            Some(Value::Array(a)) if a.len() <= MAX_CHART_POINTS => d.categories = Some(a.iter().map(cell_text).collect()),
            Some(Value::Array(_)) => return Err(bad(cmd, format!("at most {MAX_CHART_POINTS} categories"))),
            Some(_) => return Err(bad(cmd, "`categories` must be a list of labels")),
        }
        if let Some(v) = p.get("series").filter(|v| !v.is_null()) {
            d.series = Some(json_series(v, cmd)?);
        }
        Ok(d)
    }

    fn is_empty(&self) -> bool {
        self.points.is_none() && self.categories.is_none() && self.series.is_none()
    }

    fn apply(&self, c: &mut ChartEl) {
        if let Some(points) = &self.points {
            c.set_points(points.clone());
        }
        if let Some(cats) = &self.categories {
            c.categories = cats.clone();
        }
        if let Some(series) = &self.series {
            c.series = series.clone();
        }
        if self.categories.is_some() && self.series.is_none() {
            // New labels for the same values: series keep their values, squared up below.
            let n = c.categories.len();
            for s in &mut c.series {
                s.values.resize(n, 0.0);
            }
        } else if self.series.is_some() && self.categories.is_none() {
            let n = c.series.iter().map(|s| s.values.len()).max().unwrap_or(0);
            let have = c.categories.len();
            c.categories.truncate(n);
            c.categories.extend((have..n).map(|i| format!("{}", i + 1)));
        }
        c.sanitize();
    }
}

// ---------- helpers ----------

/// Runs `f` on each target that is a chart (or table…); fails when none is.
fn each(s: &mut Session, p: &Value, cmd: &str, label: &str, f: impl Fn(&mut Element, &FontBook) -> Result<bool>) -> Result<Value> {
    let ids: Vec<NodeId> = targets(s, p, cmd)?;
    let fonts = Arc::clone(&s.fonts);
    let n = s.edit(label, |doc, _| {
        let mut n = 0;
        for id in &ids {
            let el = doc.element_mut(*id)?;
            if !el.locked && f(el, &fonts)? {
                n += 1;
            }
        }
        if n == 0 {
            return Err(EngineError::Other(format!("{label} does not apply to the selected objects")));
        }
        Ok(n)
    })?;
    Ok(ok(json!({"changed": n})))
}

fn kind_param(p: &Value, cmd: &str) -> Result<Option<ChartKind>> {
    match p.get("kind") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(k)) => ChartKind::from_id(k)
            .map(Some)
            .ok_or_else(|| bad(cmd, format!("unknown chart kind `{k}` (bar, horizontal-bar, line, area, pie, donut, progress)"))),
        Some(_) => Err(bad(cmd, "`kind` must be a string")),
    }
}

fn colors_param(p: &Value, cmd: &str) -> Result<Option<Vec<Color>>> {
    match p.get("colors") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) => a
            .iter()
            .take(64)
            .map(|v| v.as_str().and_then(Color::parse).ok_or_else(|| bad(cmd, "`colors` must be colours like \"#ff8800\"")))
            .collect::<Result<Vec<_>>>()
            .map(Some),
        Some(_) => Err(bad(cmd, "`colors` must be a list of colours")),
    }
}

/// `p` without the keys that pick targets.
fn patch_of(p: &Value) -> Value {
    let mut patch = p.clone();
    if let Some(o) = patch.as_object_mut() {
        for k in ["ids", "id", "merge_key"] {
            o.remove(k);
        }
    }
    patch
}

/// Fits a table's frame to its rows after a change: a table at its natural height stays there,
/// a stretched one keeps its stretch, and nothing is cut off.
fn refit_table(el: &mut Element, before: f64, fonts: &FontBook) {
    let ElementKind::Table(t) = &el.kind else { return };
    let after = genha_render::table_geometry(fonts, t, el.frame.width, 0.0).natural_height;
    if after <= 0.0 || !after.is_finite() {
        return;
    }
    let h = if before > 0.0 && before.is_finite() { (el.frame.height * after / before).max(after) } else { after.max(el.frame.height) };
    el.frame.height = h.clamp(1.0, genha_doc::MAX_PAGE_SIDE * 4.0);
}

fn natural(el: &Element, fonts: &FontBook) -> f64 {
    match &el.kind {
        ElementKind::Table(t) => genha_render::table_geometry(fonts, t, el.frame.width, 0.0).natural_height,
        _ => 0.0,
    }
}

/// Applies `f` (given the table and its frame's width) to every targeted table, then refits its
/// height.
fn each_table_w(s: &mut Session, p: &Value, cmd: &str, label: &str, f: impl Fn(&mut TableEl, f64, &FontBook) -> Result<bool>) -> Result<Value> {
    each(s, p, cmd, label, |el, fonts| {
        let before = natural(el, fonts);
        let width = el.frame.width;
        let changed = match &mut el.kind {
            ElementKind::Table(t) => {
                let c = f(t, width, fonts)?;
                t.sanitize();
                c
            }
            _ => false,
        };
        if changed {
            refit_table(el, before, fonts);
        }
        Ok(changed)
    })
}

/// Applies `f` to every targeted table, then refits its height.
fn each_table(s: &mut Session, p: &Value, cmd: &str, label: &str, f: impl Fn(&mut TableEl) -> Result<bool>) -> Result<Value> {
    each_table_w(s, p, cmd, label, |t, _, _| f(t))
}

// ---------- charts ----------

fn chart_add(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "chart.add";
    let kind = kind_param(p, cmd)?.unwrap_or(ChartKind::Bar);
    let ring = str_param(p, "kind").is_some_and(|k| matches!(k.trim(), "ring" | "gauge"));
    let mut c = ChartEl::sample(kind);
    let data = ChartData::parse(p, cmd)?;
    if !data.is_empty() {
        data.apply(&mut c);
    }
    let brand = s.doc()?.doc.brand.clone();
    let primary = brand.color(0, Color::rgb(0x5b, 0x3c, 0xf5));
    c.colors = match colors_param(p, cmd)? {
        Some(cols) => cols,
        None if matches!(kind, ChartKind::Pie | ChartKind::Donut) || c.series.len() > 1 => {
            genha_render::chart_colors(&ChartEl::default(), &brand).into_iter().take(8).collect()
        }
        None => vec![primary],
    };
    let page = page_size(s)?;
    let side = page.0.min(page.1);
    c.font_family = brand.body_font.clone();
    c.font_size = (side * 0.026).round().clamp(10.0, 200.0);
    c.text_color = brand.color(2, Color::rgb(0x1b, 0x1b, 0x2f));
    if c.series.len() > 1 {
        // Several series need a key.
        c.show_legend = true;
    }
    // Style fields given with the chart (titles, stacking, legend…).
    let mut style = style_patch(p, cmd)?;
    if let Some(o) = style.as_object_mut() {
        for k in ["x", "y", "width", "height", "title", "colors", "kind", "rows", "csv", "tsv"] {
            o.remove(k);
        }
    }
    if style.as_object().is_some_and(|o| !o.is_empty()) {
        c = merge(&c, &style, cmd)?;
    }
    c.sanitize();
    let (dw, dh) = match kind {
        ChartKind::Progress if ring => (side * 0.4, side * 0.4),
        ChartKind::Progress => (page.0 * 0.6, (side * 0.11).max(40.0)),
        ChartKind::Pie | ChartKind::Donut => (side * 0.6, side * 0.6),
        _ => (page.0 * 0.6, side * 0.45),
    };
    let frame = frame_params(p, page, dw, dh);
    let mut el = Element::new(NodeId(0), frame, ElementKind::Chart(c));
    if let Some(n) = str_param(p, "title") {
        el.name = n.chars().take(200).collect();
    }
    add_element(s, "Add Chart", el)
}

fn chart_set_data(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "chart.setData";
    let data = ChartData::parse(p, cmd)?;
    if data.is_empty() {
        return Err(bad(cmd, "give `data`, `csv`, `rows`, `categories` or `series`"));
    }
    each(s, p, cmd, "Chart Data", |el, _| match &mut el.kind {
        ElementKind::Chart(c) => {
            data.apply(c);
            Ok(true)
        }
        _ => Ok(false),
    })
}

/// The style fields of `p` as a chart patch: `legend` becomes `show_legend` and
/// `legend_position`, kinds may use the friendly ids, data keys are left out.
fn style_patch(p: &Value, cmd: &str) -> Result<Value> {
    let mut patch = patch_of(p);
    let legend = match p.get("legend") {
        None | Some(Value::Null) => None,
        Some(Value::Bool(false)) => Some((false, None)),
        Some(Value::Bool(true)) => Some((true, None)),
        Some(Value::String(l)) if l == "none" || l == "off" => Some((false, None)),
        Some(Value::String(l)) => {
            let pos = LegendPosition::ALL
                .into_iter()
                .find(|x| x.id() == l.trim().to_ascii_lowercase())
                .ok_or_else(|| bad(cmd, format!("unknown legend place `{l}` (none, auto, top, bottom, right)")))?;
            Some((true, Some(pos)))
        }
        Some(_) => return Err(bad(cmd, "`legend` must be none, auto, top, bottom or right")),
    };
    if let Some(o) = patch.as_object_mut() {
        for k in ["data", "categories", "series", "legend"] {
            o.remove(k);
        }
        if let Some(k) = kind_param(p, cmd)? {
            o.insert("kind".into(), json!(k));
        }
        if let Some((show, pos)) = legend {
            o.insert("show_legend".into(), json!(show));
            if let Some(pos) = pos {
                o.insert("legend_position".into(), json!(pos));
            }
        }
    }
    Ok(patch)
}

fn chart_set_style(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "chart.setStyle";
    let patch = style_patch(p, cmd)?;
    let _: ChartEl = merge(&ChartEl::default(), &patch, cmd)?;
    each(s, p, cmd, "Chart Style", |el, _| match &mut el.kind {
        ElementKind::Chart(c) => {
            let mut next: ChartEl = merge(c, &patch, cmd)?;
            next.sanitize();
            *c = next;
            Ok(true)
        }
        _ => Ok(false),
    })
}

fn chart_set_kind(s: &mut Session, p: &Value) -> Result<Value> {
    let kind = kind_param(p, "chart.setKind")?.ok_or_else(|| bad("chart.setKind", "missing `kind`"))?;
    each(s, p, "chart.setKind", "Chart Type", |el, _| match &mut el.kind {
        ElementKind::Chart(c) => {
            let was_pie = matches!(c.kind, ChartKind::Pie | ChartKind::Donut);
            let is_pie = matches!(kind, ChartKind::Pie | ChartKind::Donut);
            c.kind = kind;
            if was_pie != is_pie {
                // Shares read best on pies, plain numbers on bars and lines; several series need
                // a key.
                c.value_format = if is_pie { genha_doc::ValueFormat::Percent } else { genha_doc::ValueFormat::Number };
                c.show_legend = is_pie || c.series.len() > 1;
            }
            if kind == ChartKind::Progress {
                c.value_format = genha_doc::ValueFormat::Percent;
            }
            Ok(true)
        }
        _ => Ok(false),
    })
}

// ---------- tables ----------

fn table_add(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "table.add";
    let n_rows = usize_param(p, "rows").unwrap_or(3);
    let n_cols = usize_param(p, "columns").unwrap_or(3);
    if n_rows == 0 || n_cols == 0 || n_rows > MAX_TABLE_ROWS || n_cols > MAX_TABLE_COLS {
        return Err(bad(cmd, format!("a table has 1–{MAX_TABLE_ROWS} rows and 1–{MAX_TABLE_COLS} columns")));
    }
    let mut t = TableEl::blank(n_rows, n_cols);
    if let Some(rows) = parse_rows(p, cmd)? {
        if rows.is_empty() || rows.iter().all(Vec::is_empty) {
            return Err(bad(cmd, "the table has no cells"));
        }
        t.rows = rows;
    }
    t.header_row = super::bool_or(p, "header", true);
    let brand = s.doc()?.doc.brand.clone();
    let primary = brand.color(0, Color::rgb(0x5b, 0x3c, 0xf5));
    let page = page_size(s)?;
    let side = page.0.min(page.1);
    t.font_family = brand.body_font.clone();
    t.font_size = (side * 0.022).round().clamp(8.0, 200.0);
    t.padding = (t.font_size * 0.5).round();
    t.text_color = brand.color(2, Color::rgb(0x1b, 0x1b, 0x2f));
    t.header_fill = primary;
    t.header_text = primary.readable_text();
    t.alt_row_fill = Some(primary.lerp(Color::WHITE, 0.92));
    t.sanitize();
    let width = super::f64_or(p, "width", page.0 * 0.7).max(1.0);
    let h = genha_render::table_geometry(&s.fonts, &t, width, 0.0).natural_height.max(1.0);
    let frame = frame_params(p, page, width, h);
    add_element(s, "Add Table", Element::new(NodeId(0), frame, ElementKind::Table(t)))
}

fn index_param(p: &Value, key: &str, cmd: &str) -> Result<Option<usize>> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_u64()
            .and_then(|i| usize::try_from(i).ok())
            .map(Some)
            .ok_or_else(|| bad(cmd, format!("`{key}` must be a row or column number (0 or more)"))),
    }
}

/// A count of rows or columns (at least 1).
fn span_param(p: &Value, key: &str, cmd: &str, max: usize) -> Result<usize> {
    match index_param(p, key, cmd)? {
        None => Ok(1),
        Some(n) if (1..=max).contains(&n) => Ok(n),
        Some(_) => Err(bad(cmd, format!("`{key}` must be 1 to {max}"))),
    }
}

/// The cell range of `row`, `col`, `rows`, `cols`.
fn range_param(p: &Value, cmd: &str) -> Result<CellRange> {
    let row = index_param(p, "row", cmd)?.ok_or_else(|| bad(cmd, format!("missing `row` ({RANGE_DOC})")))?;
    let col = index_param(p, "col", cmd)?.ok_or_else(|| bad(cmd, format!("missing `col` ({RANGE_DOC})")))?;
    Ok(CellRange::new(row, col, span_param(p, "rows", cmd, MAX_TABLE_ROWS)?, span_param(p, "cols", cmd, MAX_TABLE_COLS)?))
}

fn table_set_cell(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "table.setCell";
    let row = index_param(p, "row", cmd)?.ok_or_else(|| bad(cmd, "missing `row`"))?;
    let col = index_param(p, "col", cmd)?.ok_or_else(|| bad(cmd, "missing `col`"))?;
    let text: String = str_param(p, "text").ok_or_else(|| bad(cmd, "missing `text`"))?.chars().take(MAX_CELL_CHARS).collect();
    each_table(s, p, cmd, "Edit Cell", |t| {
        // A cell under a merge stands for the merge, whose text is in its top-left cell.
        let (r, c) = t.merge_at(row, col).map_or((row, col), |m| (m.row, m.col));
        let cell = t.rows.get_mut(r).and_then(|x| x.get_mut(c)).ok_or_else(|| bad(cmd, format!("no cell at row {row}, column {col}")))?;
        *cell = text.clone();
        Ok(true)
    })
}

fn table_insert_row(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "table.insertRow";
    let index = index_param(p, "index", cmd)?;
    each_table(s, p, cmd, "Insert Row", |t| {
        if t.rows.len() >= MAX_TABLE_ROWS {
            return Err(bad(cmd, format!("a table has at most {MAX_TABLE_ROWS} rows")));
        }
        t.insert_row(index.unwrap_or(t.rows.len()));
        Ok(true)
    })
}

fn table_delete_row(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "table.deleteRow";
    let index = index_param(p, "index", cmd)?;
    each_table(s, p, cmd, "Delete Row", |t| {
        if t.rows.len() <= 1 {
            return Err(bad(cmd, "a table keeps at least one row (delete the table instead)"));
        }
        let at = index.unwrap_or(t.rows.len() - 1);
        if at >= t.rows.len() {
            return Err(bad(cmd, format!("no row {at}")));
        }
        t.delete_row(at);
        Ok(true)
    })
}

fn table_insert_col(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "table.insertColumn";
    let index = index_param(p, "index", cmd)?;
    each_table(s, p, cmd, "Insert Column", |t| {
        let n = t.columns();
        if n >= MAX_TABLE_COLS {
            return Err(bad(cmd, format!("a table has at most {MAX_TABLE_COLS} columns")));
        }
        t.insert_col(index.unwrap_or(n));
        Ok(true)
    })
}

fn table_delete_col(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "table.deleteColumn";
    let index = index_param(p, "index", cmd)?;
    each_table(s, p, cmd, "Delete Column", |t| {
        let n = t.columns();
        if n <= 1 {
            return Err(bad(cmd, "a table keeps at least one column (delete the table instead)"));
        }
        let at = index.unwrap_or(n - 1);
        if at >= n {
            return Err(bad(cmd, format!("no column {at}")));
        }
        t.delete_col(at);
        Ok(true)
    })
}

fn table_set_style(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "table.setStyle";
    let patch = patch_of(p);
    let _: TableEl = merge(&TableEl::default(), &patch, cmd)?;
    each_table(s, p, cmd, "Table Style", |t| {
        *t = merge(t, &patch, cmd)?;
        Ok(true)
    })
}

fn table_set_rows(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "table.setRows";
    let rows = parse_rows(p, cmd)?.ok_or_else(|| bad(cmd, "give `rows` (a list of rows of cells) or `csv`"))?;
    if rows.is_empty() || rows.iter().all(Vec::is_empty) {
        return Err(bad(cmd, "the table has no cells"));
    }
    each_table(s, p, cmd, "Table Contents", |t| {
        t.rows = rows.clone();
        Ok(true)
    })
}

fn table_merge_cells(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "table.mergeCells";
    let range = range_param(p, cmd)?;
    if range.is_single() {
        return Err(bad(cmd, "merge at least two cells (give `rows` or `cols`)"));
    }
    each_table(s, p, cmd, "Merge Cells", |t| {
        t.merge_cells(range).map(|_| true).ok_or_else(|| bad(cmd, format!("no cells at row {}, column {}", range.row, range.col)))
    })
}

fn table_split_cells(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "table.splitCell";
    let range = range_param(p, cmd)?;
    each_table(s, p, cmd, "Split Cells", |t| {
        if t.split_cells(range) == 0 {
            return Err(bad(cmd, "no merged cells there"));
        }
        Ok(true)
    })
}

/// A colour field that may be `null` (back to the table's colour): `None` when not given.
fn opt_color(p: &Value, key: &str, cmd: &str) -> Result<Option<Option<Color>>> {
    match p.get(key) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(_) => super::color_param(p, key, cmd).map(Some),
    }
}

fn table_set_cell_style(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "table.setCellStyle";
    let range = range_param(p, cmd)?;
    let clear = super::bool_or(p, "clear", false);
    let fill = opt_color(p, "fill", cmd)?;
    let text_color = opt_color(p, "text_color", cmd)?;
    let bold = match p.get("bold") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(Value::Bool(b)) => Some(Some(*b)),
        Some(_) => return Err(bad(cmd, "`bold` must be true, false or null")),
    };
    let align = match p.get("align") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(v) => Some(Some(serde_json::from_value::<TextAlign>(v.clone()).map_err(|_| bad(cmd, "`align` must be left, center or right"))?)),
    };
    let font_size = match p.get("font_size") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(v) => {
            Some(Some(v.as_f64().filter(|f| f.is_finite() && *f > 0.0).ok_or_else(|| bad(cmd, "`font_size` must be a size in pixels"))?.min(2000.0)))
        }
    };
    if !clear && fill.is_none() && text_color.is_none() && bold.is_none() && align.is_none() && font_size.is_none() {
        return Err(bad(cmd, "give fill, text_color, bold, align, font_size or clear"));
    }
    each_table(s, p, cmd, "Cell Style", |t| {
        let n = t.style_cells(range, |st| {
            if clear {
                *st = genha_doc::CellStyle::default();
            }
            if let Some(v) = fill {
                st.fill = v;
            }
            if let Some(v) = text_color {
                st.text_color = v;
            }
            if let Some(v) = bold {
                st.bold = v;
            }
            if let Some(v) = align {
                st.align = v;
            }
            if let Some(v) = font_size {
                st.font_size = v;
            }
        });
        if n == 0 {
            return Err(bad(cmd, format!("no cells at row {}, column {}", range.row, range.col)));
        }
        Ok(true)
    })
}

fn table_set_column_width(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "table.setColumnWidth";
    let auto = super::bool_or(p, "auto", false);
    let equal = super::bool_or(p, "equal", false);
    let col = index_param(p, "col", cmd)?;
    let width = p.get("width").and_then(Value::as_f64).filter(|w| w.is_finite() && *w > 0.0);
    if !auto && !equal && (col.is_none() || width.is_none()) {
        return Err(bad(cmd, "give `col` and `width` (px), or `auto: true`, or `equal: true`"));
    }
    each_table_w(s, p, cmd, "Column Width", |t, table_w, fonts| {
        if equal {
            t.column_widths.clear();
            return Ok(true);
        }
        if auto {
            t.column_widths = genha_render::table_fit_columns(fonts, t);
            return Ok(true);
        }
        let (Some(col), Some(width)) = (col, width) else { return Ok(false) };
        let n = t.columns();
        if col >= n {
            return Err(bad(cmd, format!("no column {col}")));
        }
        if n == 1 {
            return Ok(false);
        }
        let mut f = t.fractions();
        let target = (width / table_w.max(1.0)).clamp(0.02, 1.0 - 0.02 * (n - 1) as f64);
        let rest: f64 = f.iter().enumerate().filter(|(i, _)| *i != col).map(|(_, w)| w).sum();
        for (i, w) in f.iter_mut().enumerate() {
            *w = if i == col {
                target
            } else if rest > 0.0 {
                *w / rest * (1.0 - target)
            } else {
                (1.0 - target) / (n - 1) as f64
            };
        }
        t.column_widths = f;
        Ok(true)
    })
}

fn table_set_row_height(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "table.setRowHeight";
    let row = index_param(p, "row", cmd)?;
    let auto = super::bool_or(p, "auto", false);
    let height = if auto {
        0.0
    } else {
        p.get("height")
            .and_then(Value::as_f64)
            .filter(|h| h.is_finite() && *h >= 0.0)
            .ok_or_else(|| bad(cmd, "give `height` in pixels (0 fits the text) or `auto: true`"))?
            .min(100_000.0)
    };
    each_table(s, p, cmd, "Row Height", |t| {
        let n = t.rows.len();
        t.row_heights.resize(n, 0.0);
        match row {
            Some(r) => *t.row_heights.get_mut(r).ok_or_else(|| bad(cmd, format!("no row {r}")))? = height,
            None => t.row_heights.iter_mut().for_each(|h| *h = height),
        }
        Ok(true)
    })
}

// ---------- curved text ----------

fn text_set_curve(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "text.setCurve";
    let deg = f64_req(p, "degrees", cmd)?.clamp(-MAX_CURVE, MAX_CURVE);
    let ids: Vec<NodeId> = targets(s, p, cmd)?;
    // Heights for straightened text come from the session's text layout.
    let mut straight_heights = vec![];
    if let Ok(d) = s.doc() {
        for id in &ids {
            if let Ok(Element { kind: ElementKind::Text(t), frame, .. }) = d.doc.element(*id) {
                straight_heights.push((*id, text_height(s, &t.text, &t.style, frame.width)));
            }
        }
    }
    let fonts = Arc::clone(&s.fonts);
    let n = s.edit("Curve Text", |doc, _| {
        let mut n = 0;
        for id in &ids {
            let el = doc.element_mut(*id)?;
            if el.locked {
                continue;
            }
            let ElementKind::Text(t) = &mut el.kind else { continue };
            t.curve = deg;
            let height = match genha_render::curved_text_size(&fonts, t, el.frame.width) {
                Some((_, h)) => Some(h),
                None => straight_heights.iter().find(|(i, _)| i == id).map(|(_, h)| *h),
            };
            if let Some(h) = height.filter(|h| h.is_finite() && *h > 0.0) {
                let cy = el.frame.y + el.frame.height / 2.0;
                el.frame.height = h.ceil();
                el.frame.y = cy - el.frame.height / 2.0;
            }
            n += 1;
        }
        if n == 0 {
            return Err(EngineError::Other("Curve Text applies to text".into()));
        }
        Ok(n)
    })?;
    Ok(ok(json!({"changed": n})))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_text_parses() {
        let d = parse_data(&json!("Apples: 10, Pears: 2.5, Plums = -3"), "t").unwrap();
        assert_eq!(d, vec![ChartPoint::new("Apples", 10.0), ChartPoint::new("Pears", 2.5), ChartPoint::new("Plums", -3.0)]);
        let d = parse_data(&json!("North 1,200\nSouth\t45%\n\n7"), "t").unwrap();
        assert_eq!(d, vec![ChartPoint::new("North", 1200.0), ChartPoint::new("South", 45.0), ChartPoint::new("3", 7.0)]);
        let d = parse_data(&json!([{"label": "a", "value": 1}, ["b", "2"], 3]), "t").unwrap();
        assert_eq!(d.iter().map(|p| p.value).collect::<Vec<_>>(), [1.0, 2.0, 3.0]);
        assert!(parse_data(&json!("a: lots"), "t").is_err());
        assert!(parse_data(&json!([{"label": "a"}]), "t").is_err());
        assert!(parse_data(&json!(vec![1; 500]), "t").is_err());
        assert!(parse_data(&json!(5), "t").is_err());
    }

    #[test]
    fn csv_cells_and_chart_grids_parse() {
        assert_eq!(split_line(r#"a, "b, c" ,"say ""hi""",,"#, ','), vec!["a", "b, c", "say \"hi\"", "", ""]);
        assert_eq!(split_line("x\t 1,200 \t", '\t'), vec!["x", "1,200", ""]);
        let rows = split_text("Quarter,North,South\nQ1,10,\"1,200\"\n\nQ2,5\n", 10, 10);
        let (cats, series) = chart_grid(&rows, "t").unwrap();
        assert_eq!(cats, vec!["Q1", "Q2"]);
        assert_eq!(series, vec![ChartSeries::new("North", vec![10.0, 5.0]), ChartSeries::new("South", vec![1200.0, 0.0])]);
        // The first row names the series even when it holds numbers (years); empty names are numbered.
        let (cats, series) = chart_grid(&split_text("Team\t2023\t\nb\t3\t4", 10, 10), "t").unwrap();
        assert_eq!(
            (cats, series[0].name.as_str(), series[1].name.as_str(), series[1].values.clone()),
            (vec!["b".to_string()], "2023", "Series 2", vec![4.0])
        );
        assert!(chart_grid(&split_text("a\nb", 10, 10), "t").is_err());
        assert!(chart_grid(&split_text("x,y\na,lots", 10, 10), "t").is_err());
        let wide = vec![vec!["x".to_string(); MAX_CHART_SERIES + 2]];
        assert!(chart_grid(&wide, "t").is_err());
        // Series from JSON, with colours and gaps.
        let s = json_series(&json!([{"name": "A", "values": [1, "2", null], "color": "#ff0000"}, {"values": []}]), "t").unwrap();
        assert_eq!((s[0].values.clone(), s[0].color, s[1].name.as_str()), (vec![1.0, 2.0, 0.0], Some(Color::rgb(255, 0, 0)), "Series 2"));
        assert!(json_series(&json!([{"values": ["x"]}]), "t").is_err());
        assert!(json_series(&json!([{"color": 3}]), "t").is_err());
        assert!(json_series(&json!({"a": 1}), "t").is_err());
    }
}
