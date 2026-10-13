//! Charts, tables and curved text.

use std::sync::Arc;

use serde_json::{Value, json};

use super::create::{add_element, frame_params, page_size, text_el_height};
use super::{bad, cmd, f64_req, has_doc, has_selection, merge, ok, str_param, targets, usize_param};
use crate::{CommandSpec, EngineError, Result, Session};
use genha_doc::{
    ChartEl, ChartKind, ChartPoint, Color, Element, ElementKind, MAX_CELL_CHARS, MAX_CHART_POINTS, MAX_CURVE, MAX_TABLE_COLS, MAX_TABLE_ROWS, NodeId,
    TableEl,
};
use genha_text::FontBook;

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "chart.add",
            "Add Chart",
            ["Insert"],
            None,
            "{kind?: bar|horizontal-bar|line|area|pie|donut|progress|ring = bar, data?: [{label, value}…] | [[label, value]…] | \"Label: 10, Other: 20\" (one item per line, `;` or `,`), x?, y?, width?, height?, colors?: [#rrggbb…], title?: name} — sample data and brand colours by default; `ring` is a round progress chart",
            has_doc,
            chart_add
        ),
        cmd!("chart.setData", "Chart Data", [], None, "{ids?, data: as chart.add}", has_selection, chart_set_data),
        cmd!(
            "chart.setStyle",
            "Chart Style",
            [],
            None,
            "{ids?, …partial chart: kind, colors: [#rrggbb…], show_labels, show_values, show_legend, value_format: number|percent, font_family, font_size, text_color, background: Fill JSON, donut_hole: 0–0.9, max: number|null}",
            has_selection,
            chart_set_style
        ),
        cmd!("chart.setKind", "Chart Type", [], None, "{ids?, kind: bar|horizontal-bar|line|area|pie|donut|progress}", has_selection, chart_set_kind),
        cmd!(
            "table.add",
            "Add Table",
            ["Insert"],
            None,
            "{rows?: [[cell…]…] | number of rows = 3, columns?: number = 3, csv?: \"a,b\\nc,d\" (tabs work too), header?: bool = true, x?, y?, width?, height?}",
            has_doc,
            table_add
        ),
        cmd!(
            "table.setCell",
            "Edit Cell",
            [],
            None,
            "{id?, row, col, text} — rows and columns count from 0 (row 0 is the header)",
            has_selection,
            table_set_cell
        ),
        cmd!("table.insertRow", "Insert Row", [], None, "{id?, index?: before this row (default: at the end)}", has_selection, table_insert_row),
        cmd!("table.deleteRow", "Delete Row", [], None, "{id?, index?: default the last row}", has_selection, table_delete_row),
        cmd!(
            "table.insertColumn",
            "Insert Column",
            [],
            None,
            "{id?, index?: before this column (default: at the end)}",
            has_selection,
            table_insert_col
        ),
        cmd!("table.deleteColumn", "Delete Column", [], None, "{id?, index?: default the last column}", has_selection, table_delete_col),
        cmd!(
            "table.setStyle",
            "Table Style",
            [],
            None,
            "{ids?, …partial table: header_row, font_family, font_size, text_color, header_fill, header_text, row_fill, alt_row_fill: #rrggbb|null, border: {color, width}|null, padding, align: left|center|right, column_widths: [fractions]}",
            has_selection,
            table_set_style
        ),
        cmd!("table.setRows", "Table Contents", [], None, "{id?, rows: [[cell…]…] | csv: \"a,b\\nc,d\"}", has_selection, table_set_rows),
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
    let mut c = ChartEl { data: out, ..ChartEl::default() };
    c.sanitize();
    Ok(c.data)
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
    let t: String = s.chars().filter(|c| !matches!(c, ',' | '%' | '$' | '€' | '£' | ' ' | '\u{a0}' | '_')).collect();
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

/// Table rows from `rows` (cells) or `csv` text.
fn parse_rows(p: &Value, cmd: &str) -> Result<Option<Vec<Vec<String>>>> {
    if let Some(csv) = str_param(p, "csv") {
        if csv.len() > 1 << 20 {
            return Err(bad(cmd, "csv text is too long"));
        }
        let sep = if csv.contains('\t') { '\t' } else { ',' };
        let rows: Vec<Vec<String>> = csv
            .lines()
            .filter(|l| !l.trim().is_empty())
            .take(MAX_TABLE_ROWS)
            .map(|l| l.split(sep).take(MAX_TABLE_COLS).map(|c| c.trim().chars().take(MAX_CELL_CHARS).collect()).collect())
            .collect();
        return Ok(Some(rows));
    }
    match p.get("rows") {
        Some(Value::Array(rows)) => {
            if rows.len() > MAX_TABLE_ROWS {
                return Err(bad(cmd, format!("at most {MAX_TABLE_ROWS} rows")));
            }
            let mut out = vec![];
            for (i, r) in rows.iter().enumerate() {
                let Value::Array(cells) = r else { return Err(bad(cmd, format!("rows[{i}] must be a list of cells"))) };
                if cells.len() > MAX_TABLE_COLS {
                    return Err(bad(cmd, format!("at most {MAX_TABLE_COLS} columns")));
                }
                out.push(cells.iter().map(cell_text).collect());
            }
            Ok(Some(out))
        }
        _ => Ok(None),
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

/// Applies `f` to every targeted table, then refits its height.
fn each_table(s: &mut Session, p: &Value, cmd: &str, label: &str, f: impl Fn(&mut TableEl) -> Result<bool>) -> Result<Value> {
    each(s, p, cmd, label, |el, fonts| {
        let before = natural(el, fonts);
        let changed = match &mut el.kind {
            ElementKind::Table(t) => {
                let c = f(t)?;
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

// ---------- charts ----------

fn chart_add(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "chart.add";
    let kind = kind_param(p, cmd)?.unwrap_or(ChartKind::Bar);
    let ring = str_param(p, "kind").is_some_and(|k| matches!(k.trim(), "ring" | "gauge"));
    let mut c = ChartEl::sample(kind);
    if let Some(d) = p.get("data") {
        c.data = parse_data(d, cmd)?;
    }
    let brand = s.doc()?.doc.brand.clone();
    let primary = brand.color(0, Color::rgb(0x5b, 0x3c, 0xf5));
    c.colors = match colors_param(p, cmd)? {
        Some(cols) => cols,
        None if matches!(kind, ChartKind::Pie | ChartKind::Donut) => {
            genha_render::chart_colors(&ChartEl::default(), &brand).into_iter().take(8).collect()
        }
        None => vec![primary],
    };
    let page = page_size(s)?;
    let side = page.0.min(page.1);
    c.font_family = brand.body_font.clone();
    c.font_size = (side * 0.026).round().clamp(10.0, 200.0);
    c.text_color = brand.color(2, Color::rgb(0x1b, 0x1b, 0x2f));
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
    let data = parse_data(p.get("data").ok_or_else(|| bad("chart.setData", "missing `data`"))?, "chart.setData")?;
    each(s, p, "chart.setData", "Chart Data", |el, _| match &mut el.kind {
        ElementKind::Chart(c) => {
            c.data = data.clone();
            Ok(true)
        }
        _ => Ok(false),
    })
}

fn chart_set_style(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "chart.setStyle";
    let mut patch = patch_of(p);
    if let Some(o) = patch.as_object_mut() {
        // Data has its own command; kinds may use the friendly ids.
        o.remove("data");
        if let Some(k) = kind_param(p, cmd)? {
            o.insert("kind".into(), json!(k));
        }
    }
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
                // Shares read best on pies, plain numbers on bars and lines.
                c.value_format = if is_pie { genha_doc::ValueFormat::Percent } else { genha_doc::ValueFormat::Number };
                c.show_legend = is_pie;
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

fn table_set_cell(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "table.setCell";
    let row = index_param(p, "row", cmd)?.ok_or_else(|| bad(cmd, "missing `row`"))?;
    let col = index_param(p, "col", cmd)?.ok_or_else(|| bad(cmd, "missing `col`"))?;
    let text: String = str_param(p, "text").ok_or_else(|| bad(cmd, "missing `text`"))?.chars().take(MAX_CELL_CHARS).collect();
    each_table(s, p, cmd, "Edit Cell", |t| {
        let cell = t.rows.get_mut(row).and_then(|r| r.get_mut(col)).ok_or_else(|| bad(cmd, format!("no cell at row {row}, column {col}")))?;
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
        let at = index.unwrap_or(t.rows.len()).min(t.rows.len());
        t.rows.insert(at, vec![String::new(); t.columns().max(1)]);
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
        t.rows.remove(at);
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
        let at = index.unwrap_or(n).min(n);
        for r in &mut t.rows {
            r.resize(n, String::new());
            r.insert(at, String::new());
        }
        if !t.column_widths.is_empty() {
            let mean = t.column_widths.iter().sum::<f64>() / t.column_widths.len() as f64;
            t.column_widths.resize(n, mean);
            t.column_widths.insert(at.min(t.column_widths.len()), mean);
        }
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
        for r in &mut t.rows {
            if at < r.len() {
                r.remove(at);
            }
        }
        if at < t.column_widths.len() {
            t.column_widths.remove(at);
        }
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
                straight_heights.push((*id, text_el_height(s, t, frame.width)));
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
}
