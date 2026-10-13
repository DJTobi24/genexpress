//! Properties of charts and tables, and the curve of text (part of the properties panel).

use serde_json::json;

use super::{Actions, color_row, section};
use crate::theme::Tokens;
use genha_color::Color;
use genha_doc::{
    BorderLines, BrandColor, BrandKit, CellRange, CellSpan, ChartEl, ChartKind, ChartSeries, LegendPosition, TableEl, TextAlign, TextEl, ValueFormat,
};

/// A brand kit holding just these colours (what the chart colours fall back to).
fn kit(brand: &[Color]) -> BrandKit {
    BrandKit { colors: brand.iter().map(|c| BrandColor { name: String::new(), color: *c }).collect(), ..BrandKit::default() }
}

/// A small colour button; returns the new colour when changed.
fn swatch(ui: &mut egui::Ui, color: Color) -> Option<Color> {
    let mut e = crate::theme::to_egui(color);
    ui.color_edit_button_srgba(&mut e).changed().then(|| crate::theme::from_egui(e))
}

pub(super) fn chart(ui: &mut egui::Ui, t: &Tokens, id: u64, c: &ChartEl, brand: &[Color], acts: &mut Actions) {
    let key = |what: &str| format!("{what}:{id}");
    section(ui, t, "CHART TYPE");
    ui.horizontal_wrapped(|ui| {
        for k in ChartKind::ALL {
            if ui.add(egui::Button::new(k.label()).selected(c.kind == k)).clicked() {
                acts.run("chart.setKind", json!({"kind": k.id()}));
            }
        }
    });

    chart_data(ui, t, id, c, brand, acts);

    if !c.kind.multi_series() || c.series.len() <= 1 {
        section(ui, t, "COLOURS");
        let shown: Vec<Color> = if c.colors.is_empty() { genha_render::CHART_PALETTE.to_vec() } else { c.colors.clone() };
        let mut colors = shown.clone();
        let mut colors_changed = false;
        ui.horizontal_wrapped(|ui| {
            for col in colors.iter_mut() {
                if let Some(n) = swatch(ui, *col) {
                    *col = n;
                    colors_changed = true;
                }
            }
        });
        if colors_changed {
            acts.run("chart.setStyle", json!({"colors": colors.iter().map(|c| c.to_hex()).collect::<Vec<_>>(), "merge_key": key("chartcolors")}));
        }
        ui.horizontal_wrapped(|ui| {
            // Very light and very dark brand colours (backgrounds, text) make poor series colours.
            let usable: Vec<String> = brand.iter().filter(|c| (0.03..0.75).contains(&c.luminance())).map(|c| c.to_hex()).collect();
            if ui.add_enabled(!usable.is_empty(), egui::Button::new("Brand colours")).on_hover_text("Use the brand kit's colours").clicked() {
                acts.run("chart.setStyle", json!({"colors": usable}));
            }
            if ui.button("Add colour").clicked() {
                let mut more = shown.clone();
                more.push(genha_render::CHART_PALETTE.get(more.len() % genha_render::CHART_PALETTE.len()).copied().unwrap_or(Color::BLACK));
                acts.run("chart.setStyle", json!({"colors": more.iter().map(|c| c.to_hex()).collect::<Vec<_>>()}));
            }
            if shown.len() > 1 && ui.button("One colour").clicked() {
                acts.run("chart.setStyle", json!({"colors": [shown.first().copied().unwrap_or(Color::BLACK).to_hex()]}));
            }
        });
    }

    if c.kind.has_axis() {
        section(ui, t, "AXES");
        ui.horizontal_wrapped(|ui| {
            let mut stacked = c.stacked;
            if ui.add_enabled(c.series.len() > 1, egui::Checkbox::new(&mut stacked, "Stacked")).on_hover_text("Series on top of each other").changed()
            {
                acts.run("chart.setStyle", json!({"stacked": stacked}));
            }
            let mut grid = c.gridlines;
            if ui.checkbox(&mut grid, "Gridlines").changed() {
                acts.run("chart.setStyle", json!({"gridlines": grid}));
            }
        });
        egui::Grid::new(("chart_axes", id)).num_columns(2).spacing([6.0, 4.0]).show(ui, |ui| {
            for (label, field, value) in [("X title", "x_title", &c.x_title), ("Y title", "y_title", &c.y_title)] {
                ui.label(label);
                let mut v = value.clone();
                if ui.add(egui::TextEdit::singleline(&mut v).id_salt((field, id)).desired_width(170.0).hint_text("none")).changed() {
                    acts.run("chart.setStyle", json!({field: v, "merge_key": key(field)}));
                }
                ui.end_row();
            }
            let top = c.series.iter().flat_map(|s| s.values.iter().copied()).fold(1.0_f64, f64::max);
            for (label, field, value, guess) in [("Min", "min", c.min, 0.0), ("Max", "max", c.max, top.ceil())] {
                ui.label(label);
                ui.horizontal(|ui| {
                    let mut auto = value.is_none();
                    if ui.checkbox(&mut auto, "Auto").changed() {
                        acts.run("chart.setStyle", json!({field: if auto { None } else { Some(guess) }}));
                    }
                    if let Some(mut m) = value
                        && ui.add(egui::DragValue::new(&mut m).range(-1e12..=1e12).speed(0.5)).changed()
                    {
                        acts.run("chart.setStyle", json!({field: m, "merge_key": key(field)}));
                    }
                });
                ui.end_row();
            }
        });
    }

    section(ui, t, "LEGEND");
    ui.horizontal_wrapped(|ui| {
        if ui.add(egui::Button::new("None").selected(!c.show_legend)).clicked() {
            acts.run("chart.setStyle", json!({"legend": "none"}));
        }
        for (label, pos) in
            [("Auto", LegendPosition::Auto), ("Top", LegendPosition::Top), ("Bottom", LegendPosition::Bottom), ("Right", LegendPosition::Right)]
        {
            if ui.add(egui::Button::new(label).selected(c.show_legend && c.legend_position == pos)).clicked() {
                acts.run("chart.setStyle", json!({"legend": pos.id()}));
            }
        }
    });

    section(ui, t, "SHOW");
    ui.horizontal_wrapped(|ui| {
        for (label, field, on) in [("Labels", "show_labels", c.show_labels), ("Values", "show_values", c.show_values)] {
            let mut v = on;
            if ui.checkbox(&mut v, label).changed() {
                acts.run("chart.setStyle", json!({field: v}));
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("Values as");
        for (label, f) in [("Number", ValueFormat::Number), ("Percent", ValueFormat::Percent), ("Currency", ValueFormat::Currency)] {
            if ui.add(egui::Button::new(label).selected(c.value_format == f)).clicked() {
                acts.run("chart.setStyle", json!({"value_format": f}));
            }
        }
    });
    ui.horizontal(|ui| {
        if c.value_format == ValueFormat::Currency {
            let mut sym = c.currency.clone();
            if ui.add(egui::TextEdit::singleline(&mut sym).id_salt(("currency", id)).desired_width(36.0)).on_hover_text("Currency symbol").changed() {
                acts.run("chart.setStyle", json!({"currency": sym, "merge_key": key("currency")}));
            }
        }
        let mut auto = c.decimals.is_none();
        if ui.checkbox(&mut auto, "Auto decimals").changed() {
            acts.run("chart.setStyle", json!({"decimals": if auto { None } else { Some(2) }}));
        }
        if let Some(mut d) = c.decimals
            && ui.add(egui::DragValue::new(&mut d).range(0..=6).prefix("Decimals ")).changed()
        {
            acts.run("chart.setStyle", json!({"decimals": d}));
        }
    });
    if c.kind == ChartKind::Donut {
        let mut hole = c.donut_hole;
        if ui.add(egui::Slider::new(&mut hole, 0.1..=0.9).text("Hole")).changed() {
            acts.run("chart.setStyle", json!({"donut_hole": hole, "merge_key": key("hole")}));
        }
    }
    if c.kind == ChartKind::Progress {
        ui.horizontal(|ui| {
            let mut auto = c.max.is_none();
            if ui.checkbox(&mut auto, "Out of 100").changed() {
                let top = c.value(0, 0).max(1.0);
                acts.run("chart.setStyle", json!({"max": if auto { None } else { Some(top.ceil()) }}));
            }
            if let Some(mut m) = c.max
                && ui.add(egui::DragValue::new(&mut m).range(0.001..=1e12).prefix("Max ")).changed()
            {
                acts.run("chart.setStyle", json!({"max": m, "merge_key": key("max")}));
            }
        });
    }

    section(ui, t, "TEXT");
    let mut size = c.font_size;
    if ui.add(egui::DragValue::new(&mut size).range(1.0..=2000.0).speed(0.5).prefix("Size ")).changed() {
        acts.run("chart.setStyle", json!({"font_size": size, "merge_key": key("chartsize")}));
    }
    if let Some(col) = color_row(ui, c.text_color, brand) {
        acts.run("chart.setStyle", json!({"text_color": col.to_hex(), "merge_key": key("charttext")}));
    }
}

/// The data grid: one row per category, one column per series (one series for pies and progress
/// charts), and a box for pasting a table.
fn chart_data(ui: &mut egui::Ui, t: &Tokens, id: u64, c: &ChartEl, brand: &[Color], acts: &mut Actions) {
    section(ui, t, "DATA");
    let kit = kit(brand);
    let multi = c.kind.multi_series();
    let shown = if multi { c.series.len() } else { c.series.len().min(1) };
    let mut cats = c.categories.clone();
    let mut series: Vec<ChartSeries> = c.series.clone();
    if series.is_empty() {
        series.push(ChartSeries::new("Series 1", vec![0.0; cats.len()]));
    }
    let mut typed = false;
    let mut changed = false;
    let mut remove_series = None;
    let mut remove_cat = None;
    if multi {
        // Each series: its colour, its name, and a button to remove it.
        for (i, s) in series.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                if let Some(col) = swatch(ui, genha_render::series_color(c, &kit, i)) {
                    s.color = Some(col);
                    changed = true;
                }
                typed |= ui.add(egui::TextEdit::singleline(&mut s.name).id_salt(("series_name", id, i)).desired_width(130.0)).changed();
                if shown > 1 && ui.small_button("x").on_hover_text("Remove this series").clicked() {
                    remove_series = Some(i);
                }
            });
        }
    }
    egui::ScrollArea::horizontal().id_salt(("chart_grid_scroll", id)).show(ui, |ui| {
        egui::Grid::new(("chart_grid", id)).spacing([4.0, 4.0]).show(ui, |ui| {
            ui.label(egui::RichText::new("Label").color(t.text_dim));
            for s in series.iter_mut().take(shown.max(1)) {
                if multi {
                    let name: String = s.name.chars().take(8).collect();
                    ui.label(egui::RichText::new(name).color(t.text_dim));
                } else {
                    typed |= ui.add(egui::TextEdit::singleline(&mut s.name).id_salt(("series_name", id, 0)).desired_width(56.0)).changed();
                }
            }
            ui.end_row();
            for (k, label) in cats.iter_mut().enumerate() {
                typed |= ui.add(egui::TextEdit::singleline(label).id_salt(("chart_label", id, k)).desired_width(64.0)).changed();
                for s in series.iter_mut().take(shown.max(1)) {
                    if let Some(v) = s.values.get_mut(k) {
                        typed |= ui.add_sized([56.0, 20.0], egui::DragValue::new(v).speed(0.5).max_decimals(2)).changed();
                    }
                }
                if ui.small_button("x").on_hover_text("Remove this row").clicked() {
                    remove_cat = Some(k);
                }
                ui.end_row();
            }
        });
    });
    if let Some(i) = remove_series
        && i < series.len()
    {
        series.remove(i);
        changed = true;
    }
    if let Some(k) = remove_cat
        && k < cats.len()
    {
        cats.remove(k);
        for s in &mut series {
            if k < s.values.len() {
                s.values.remove(k);
            }
        }
        changed = true;
    }
    ui.horizontal_wrapped(|ui| {
        if ui.add_enabled(cats.len() < genha_doc::MAX_CHART_POINTS, egui::Button::new("Add row")).clicked() {
            cats.push(format!("Item {}", cats.len() + 1));
            for s in &mut series {
                let last = s.values.last().copied().unwrap_or(10.0);
                s.values.push(last);
            }
            changed = true;
        }
        if multi && ui.add_enabled(series.len() < genha_doc::MAX_CHART_SERIES, egui::Button::new("Add series")).clicked() {
            let n = series.len();
            let values = series.last().map_or(vec![10.0; cats.len()], |s| s.values.iter().map(|v| (v * 0.7).round()).collect());
            let color = genha_render::series_color(c, &kit, n);
            series.push(ChartSeries { name: format!("Series {}", n + 1), values, color: Some(color) });
            changed = true;
        }
    });
    if changed || typed {
        let mut p = json!({"categories": cats, "series": series});
        if typed && !changed {
            p["merge_key"] = json!(format!("chartdata:{id}"));
        }
        acts.run("chart.setData", p);
    }
    egui::CollapsingHeader::new("Paste a table").id_salt(("chart_paste", id)).show(ui, |ui| {
        ui.label(
            egui::RichText::new("Rows from a spreadsheet or CSV: first row the series names, first column the labels.").small().color(t.text_dim),
        );
        let mem = egui::Id::new(("chart_paste_text", id));
        let mut text: String = ui.data(|d| d.get_temp(mem)).unwrap_or_default();
        let edit = egui::TextEdit::multiline(&mut text)
            .id_salt(("chart_paste_edit", id))
            .desired_rows(4)
            .desired_width(f32::INFINITY)
            .hint_text("Month, Sales, Costs\nJan, 12, 8\nFeb, 19, 11");
        if ui.add(edit).changed() {
            ui.data_mut(|d| d.insert_temp(mem, text.clone()));
        }
        if ui.add_enabled(!text.trim().is_empty(), egui::Button::new("Use this data")).clicked() {
            acts.run("chart.setData", json!({"csv": text}));
        }
    });
}

/// The cells picked in a table's grid: the range and the cell a shift-click extends from.
#[derive(Clone, Copy)]
struct Pick {
    range: CellRange,
    from: (usize, usize),
}

pub(super) fn table(ui: &mut egui::Ui, t: &Tokens, id: u64, tb: &TableEl, width: f64, brand: &[Color], acts: &mut Actions) {
    let key = |what: &str| format!("{what}:{id}");
    let cols = tb.columns();
    let rows = tb.rows.len();
    let pick_id = egui::Id::new(("table_pick", id));
    let mut pick: Pick = ui.data(|d| d.get_temp(pick_id)).unwrap_or(Pick { range: CellRange::cell(0, 0), from: (0, 0) });
    pick.range = pick.range.clipped(rows, cols).unwrap_or(CellRange::cell(0, 0));
    section(ui, t, "CELLS");
    ui.label(egui::RichText::new("Click a cell to pick it, shift-click to pick a range.").small().color(t.text_dim));
    let spans = tb.spans();
    let shift = ui.input(|i| i.modifiers.shift);
    egui::ScrollArea::horizontal().id_salt(("table_cells", id)).show(ui, |ui| {
        egui::Grid::new(("table_grid", id)).spacing([4.0, 4.0]).show(ui, |ui| {
            for (r, row) in tb.rows.iter().enumerate() {
                for c in 0..cols {
                    let span = spans.get(r).and_then(|s| s.get(c)).copied().unwrap_or(CellSpan::Own);
                    let picked = pick.range.contains(r, c);
                    if let CellSpan::Covered(m) = span {
                        let mut empty = String::new();
                        let edit = egui::TextEdit::singleline(&mut empty).id_salt(("cell", id, r, c)).desired_width(80.0).hint_text("merged");
                        let edit = if picked { edit.background_color(t.accent_soft) } else { edit };
                        ui.add_enabled(false, edit).on_disabled_hover_text(format!(
                            "Part of the merged cell at row {}, column {}",
                            m.row + 1,
                            m.col + 1
                        ));
                        continue;
                    }
                    let mut text = row.get(c).cloned().unwrap_or_default();
                    let mut edit = egui::TextEdit::singleline(&mut text).id_salt(("cell", id, r, c)).desired_width(80.0);
                    if picked {
                        edit = edit.background_color(t.accent_soft);
                    }
                    let resp = ui.add(edit);
                    // A click picks (shift: extends); reaching a cell with the keyboard picks it
                    // unless it is already picked (focus also arrives a frame after a click).
                    if resp.clicked() || (resp.gained_focus() && !picked) {
                        let here = match span {
                            CellSpan::Anchor(m) => m,
                            _ => CellRange::cell(r, c),
                        };
                        pick = if shift && resp.clicked() {
                            Pick { range: CellRange::cell(pick.from.0, pick.from.1).union(&here), from: pick.from }
                        } else {
                            Pick { range: here, from: (r, c) }
                        };
                    }
                    if resp.changed() {
                        acts.run("table.setCell", json!({"row": r, "col": c, "text": text, "merge_key": format!("cell:{id}:{r}:{c}")}));
                    }
                }
                ui.end_row();
            }
        });
    });
    ui.horizontal_wrapped(|ui| {
        let r = pick.range;
        if ui.button("Row above").on_hover_text("Insert a row above the picked cells").clicked() {
            acts.run("table.insertRow", json!({"index": r.row}));
        }
        if ui.button("Row below").clicked() {
            acts.run("table.insertRow", json!({"index": r.end_row()}));
        }
        if ui.add_enabled(rows > 1, egui::Button::new("Delete row")).clicked() {
            acts.run("table.deleteRow", json!({"index": r.row}));
        }
        if ui.button("Column left").clicked() {
            acts.run("table.insertColumn", json!({"index": r.col}));
        }
        if ui.button("Column right").clicked() {
            acts.run("table.insertColumn", json!({"index": r.end_col()}));
        }
        if ui.add_enabled(cols > 1, egui::Button::new("Delete column")).clicked() {
            acts.run("table.deleteColumn", json!({"index": r.col}));
        }
    });
    let mut header = tb.header_row;
    if ui.checkbox(&mut header, "Header row").changed() {
        acts.run("table.setStyle", json!({"header_row": header}));
    }

    picked_cells(ui, t, id, tb, width, &mut pick, brand, acts);
    ui.data_mut(|d| d.insert_temp(pick_id, pick));

    section(ui, t, "TEXT");
    ui.horizontal(|ui| {
        let mut size = tb.font_size;
        if ui.add(egui::DragValue::new(&mut size).range(1.0..=2000.0).speed(0.5).prefix("Size ")).changed() {
            acts.run("table.setStyle", json!({"font_size": size, "merge_key": key("tablesize")}));
        }
        let mut pad = tb.padding;
        if ui.add(egui::DragValue::new(&mut pad).range(0.0..=200.0).speed(0.5).prefix("Padding ")).changed() {
            acts.run("table.setStyle", json!({"padding": pad, "merge_key": key("tablepad")}));
        }
    });
    ui.horizontal(|ui| {
        ui.label("Align");
        for (label, a) in [("Left", TextAlign::Left), ("Center", TextAlign::Center), ("Right", TextAlign::Right)] {
            if ui.add(egui::Button::new(label).selected(tb.align == a)).clicked() {
                acts.run("table.setStyle", json!({"align": a}));
            }
        }
    });
    if let Some(c) = color_row(ui, tb.text_color, brand) {
        acts.run("table.setStyle", json!({"text_color": c.to_hex(), "merge_key": key("tabletext")}));
    }

    section(ui, t, "HEADER");
    if let Some(c) = color_row(ui, tb.header_fill, brand) {
        acts.run("table.setStyle", json!({"header_fill": c.to_hex(), "header_text": c.readable_text().to_hex(), "merge_key": key("tablehead")}));
    }
    section(ui, t, "ROWS");
    if let Some(c) = color_row(ui, tb.row_fill, brand) {
        acts.run("table.setStyle", json!({"row_fill": c.to_hex(), "merge_key": key("tablerow")}));
    }
    let mut banded = tb.alt_row_fill.is_some();
    if ui.checkbox(&mut banded, "Banded rows").changed() {
        let alt = tb.header_fill.lerp(Color::WHITE, 0.9);
        acts.run("table.setStyle", json!({"alt_row_fill": if banded { Some(alt.to_hex()) } else { None }}));
    }
    if let Some(alt) = tb.alt_row_fill
        && let Some(c) = color_row(ui, alt, brand)
    {
        acts.run("table.setStyle", json!({"alt_row_fill": c.to_hex(), "merge_key": key("tablealt")}));
    }
    section(ui, t, "LINES");
    let mut lines = tb.border.is_some();
    if ui.checkbox(&mut lines, "Lines").changed() {
        acts.run("style.stroke", if lines { json!({"width": 1.5, "color": "#d9d6e8"}) } else { json!({"none": true}) });
    }
    if let Some(b) = &tb.border {
        ui.horizontal_wrapped(|ui| {
            for mode in BorderLines::ALL {
                if ui.add(egui::Button::new(mode.label()).selected(tb.borders == mode)).clicked() {
                    acts.run("table.setStyle", json!({"borders": mode}));
                }
            }
        });
        if let Some(c) = color_row(ui, b.color, brand) {
            acts.run("style.stroke", json!({"color": c.to_hex(), "merge_key": key("tableline")}));
        }
        let mut w = b.width;
        if ui.add(egui::Slider::new(&mut w, 0.5..=12.0).text("Width")).changed() {
            acts.run("style.stroke", json!({"width": w, "merge_key": key("tablelinew")}));
        }
        if tb.borders.outer() {
            let mut own = tb.outer_border.is_some();
            if ui.checkbox(&mut own, "Different outside line").changed() {
                let o = json!({"color": b.color.to_hex(), "width": (b.width * 2.0).min(12.0)});
                acts.run("table.setStyle", json!({"outer_border": if own { o } else { serde_json::Value::Null }}));
            }
            if let Some(o) = &tb.outer_border {
                if let Some(c) = color_row(ui, o.color, brand) {
                    acts.run("table.setStyle", json!({"outer_border": {"color": c.to_hex(), "width": o.width}, "merge_key": key("tableouter")}));
                }
                let mut w = o.width;
                if ui.add(egui::Slider::new(&mut w, 0.5..=24.0).text("Outside width")).changed() {
                    acts.run("table.setStyle", json!({"outer_border": {"color": o.color.to_hex(), "width": w}, "merge_key": key("tableouterw")}));
                }
            }
        }
    }
}

/// The picked cells: their place, merging, their own look, and their column's and row's size.
#[allow(clippy::too_many_arguments)]
fn picked_cells(ui: &mut egui::Ui, t: &Tokens, id: u64, tb: &TableEl, width: f64, pick: &mut Pick, brand: &[Color], acts: &mut Actions) {
    let key = |what: &str| format!("{what}:{id}");
    let (rows, cols) = (tb.rows.len(), tb.columns());
    let r = pick.range;
    section(ui, t, "PICKED CELLS");
    ui.horizontal_wrapped(|ui| {
        let mut v = [r.row + 1, r.col + 1, r.rows, r.cols];
        let mut moved = false;
        for (i, (label, max)) in [("Row ", rows), ("Col ", cols), ("Rows ", rows), ("Cols ", cols)].into_iter().enumerate() {
            if let Some(x) = v.get_mut(i) {
                moved |= ui.add(egui::DragValue::new(x).range(1..=max.max(1)).prefix(label)).changed();
            }
        }
        if moved {
            let [row, col, n, m] = v;
            pick.range = CellRange::new(row.saturating_sub(1), col.saturating_sub(1), n, m).clipped(rows, cols).unwrap_or(r);
            pick.from = (pick.range.row, pick.range.col);
        }
    });
    let at = |p: serde_json::Value| {
        let mut p = p;
        p["row"] = json!(r.row);
        p["col"] = json!(r.col);
        p["rows"] = json!(r.rows);
        p["cols"] = json!(r.cols);
        p
    };
    ui.horizontal_wrapped(|ui| {
        if ui.add_enabled(!r.is_single(), egui::Button::new("Merge cells")).clicked() {
            acts.run("table.mergeCells", at(json!({})));
        }
        let merged = tb.merges.iter().any(|m| m.intersects(&r));
        if ui.add_enabled(merged, egui::Button::new("Split cells")).clicked() {
            acts.run("table.splitCell", at(json!({})));
        }
        let styled = tb.cell_styles.iter().any(|f| r.contains(f.row, f.col));
        if ui.add_enabled(styled, egui::Button::new("Clear cell style")).clicked() {
            acts.run("table.setCellStyle", at(json!({"clear": true})));
        }
    });
    let own = tb.style_at(r.row, r.col).cloned().unwrap_or_default();
    let header = r.row == 0 && tb.header_row;
    ui.label("Fill");
    let fill_now = own.fill.unwrap_or(if header { tb.header_fill } else { tb.row_fill });
    ui.horizontal(|ui| {
        if let Some(c) = color_row(ui, fill_now, brand) {
            acts.run("table.setCellStyle", at(json!({"fill": c.to_hex(), "merge_key": key("cellfill")})));
        }
        if own.fill.is_some() && ui.small_button("Reset").clicked() {
            acts.run("table.setCellStyle", at(json!({"fill": null})));
        }
    });
    ui.label("Text colour");
    let ink_now = own.text_color.unwrap_or(if header { tb.header_text } else { tb.text_color });
    ui.horizontal(|ui| {
        if let Some(c) = color_row(ui, ink_now, brand) {
            acts.run("table.setCellStyle", at(json!({"text_color": c.to_hex(), "merge_key": key("cellink")})));
        }
        if own.text_color.is_some() && ui.small_button("Reset").clicked() {
            acts.run("table.setCellStyle", at(json!({"text_color": null})));
        }
    });
    ui.horizontal_wrapped(|ui| {
        let mut bold = own.bold.unwrap_or(header);
        if ui.checkbox(&mut bold, "Bold").changed() {
            acts.run("table.setCellStyle", at(json!({"bold": bold})));
        }
        let align = own.align.unwrap_or(tb.align);
        for (label, a) in [("Left", TextAlign::Left), ("Center", TextAlign::Center), ("Right", TextAlign::Right)] {
            if ui.add(egui::Button::new(label).selected(align == a)).clicked() {
                acts.run("table.setCellStyle", at(json!({"align": a})));
            }
        }
    });
    ui.horizontal(|ui| {
        let mut size = own.font_size.unwrap_or(tb.font_size);
        if ui.add(egui::DragValue::new(&mut size).range(1.0..=2000.0).speed(0.5).prefix("Text size ")).changed() {
            acts.run("table.setCellStyle", at(json!({"font_size": size, "merge_key": key("cellsize")})));
        }
        if own.font_size.is_some() && ui.small_button("Reset").clicked() {
            acts.run("table.setCellStyle", at(json!({"font_size": null})));
        }
    });
    section(ui, t, "COLUMN AND ROW SIZE");
    ui.horizontal_wrapped(|ui| {
        let frac = tb.fractions().get(r.col).copied().unwrap_or(0.0);
        let mut w = (frac * width).round();
        if ui.add(egui::DragValue::new(&mut w).range(1.0..=width.max(1.0)).speed(1.0).prefix(format!("Column {} width ", r.col + 1))).changed() {
            acts.run("table.setColumnWidth", json!({"col": r.col, "width": w, "merge_key": key("colw")}));
        }
        if ui.button("Fit columns").on_hover_text("Each column as wide as its text").clicked() {
            acts.run("table.setColumnWidth", json!({"auto": true}));
        }
        if ui.button("Equal columns").clicked() {
            acts.run("table.setColumnWidth", json!({"equal": true}));
        }
    });
    ui.horizontal_wrapped(|ui| {
        let set = tb.row_height(r.row);
        let mut auto = set <= 0.0;
        if ui.checkbox(&mut auto, format!("Row {} fits its text", r.row + 1)).changed() {
            let h = if auto { 0.0 } else { (tb.font_size * 3.0).round() };
            acts.run("table.setRowHeight", json!({"row": r.row, "height": h}));
        }
        if !auto {
            let mut h = set;
            if ui.add(egui::DragValue::new(&mut h).range(1.0..=10_000.0).speed(1.0).prefix("Least height ")).changed() {
                acts.run("table.setRowHeight", json!({"row": r.row, "height": h, "merge_key": key("rowh")}));
            }
        }
    });
}

/// The curve slider for text.
pub(super) fn curve(ui: &mut egui::Ui, t: &Tokens, id: u64, text: &TextEl, acts: &mut Actions) {
    section(ui, t, "CURVE");
    let mut deg = text.curve;
    ui.horizontal(|ui| {
        if ui.add(egui::Slider::new(&mut deg, -360.0..=360.0).step_by(1.0).suffix(" deg").text("Curve")).changed() {
            acts.run("text.setCurve", json!({"degrees": deg, "merge_key": format!("curve:{id}")}));
        }
        if text.curve != 0.0 && ui.button("Straight").clicked() {
            acts.run("text.setCurve", json!({"degrees": 0.0}));
        }
    });
}
