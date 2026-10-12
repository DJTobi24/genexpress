//! Properties of charts and tables, and the curve of text (part of the properties panel).

use serde_json::json;

use super::{Actions, color_row, section};
use crate::theme::Tokens;
use genha_color::Color;
use genha_doc::{ChartEl, ChartKind, ChartPoint, TableEl, TextAlign, TextEl, ValueFormat};

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

    section(ui, t, "DATA");
    let mut data: Vec<ChartPoint> = c.data.clone();
    let mut changed = false;
    let mut remove = None;
    egui::Grid::new(("chart_data", id)).num_columns(3).spacing([6.0, 4.0]).show(ui, |ui| {
        for (i, p) in data.iter_mut().enumerate() {
            changed |= ui.add(egui::TextEdit::singleline(&mut p.label).id_salt(("chart_label", id, i)).desired_width(120.0)).changed();
            changed |= ui.add(egui::DragValue::new(&mut p.value).speed(0.5).max_decimals(2)).changed();
            if ui.small_button("Remove").on_hover_text("Remove this row").clicked() {
                remove = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = remove
        && i < data.len()
    {
        data.remove(i);
        acts.run("chart.setData", json!({"data": data}));
    } else if changed {
        acts.run("chart.setData", json!({"data": data, "merge_key": key("chartdata")}));
    }
    if ui.add_enabled(c.data.len() < genha_doc::MAX_CHART_POINTS, egui::Button::new("Add row")).clicked() {
        let mut data = c.data.clone();
        let last = data.last().map_or(10.0, |p| p.value);
        data.push(ChartPoint::new(format!("Item {}", data.len() + 1), last));
        acts.run("chart.setData", json!({"data": data}));
    }

    section(ui, t, "COLOURS");
    let shown: Vec<Color> = if c.colors.is_empty() { genha_render::CHART_PALETTE.to_vec() } else { c.colors.clone() };
    let mut colors = shown.clone();
    let mut colors_changed = false;
    ui.horizontal_wrapped(|ui| {
        for col in colors.iter_mut() {
            let mut e = crate::theme::to_egui(*col);
            if ui.color_edit_button_srgba(&mut e).changed() {
                *col = crate::theme::from_egui(e);
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

    section(ui, t, "SHOW");
    ui.horizontal_wrapped(|ui| {
        for (label, field, on) in
            [("Labels", "show_labels", c.show_labels), ("Values", "show_values", c.show_values), ("Legend", "show_legend", c.show_legend)]
        {
            let mut v = on;
            if ui.checkbox(&mut v, label).changed() {
                acts.run("chart.setStyle", json!({field: v}));
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("Values as");
        for (label, f) in [("Number", ValueFormat::Number), ("Percent", ValueFormat::Percent)] {
            if ui.add(egui::Button::new(label).selected(c.value_format == f)).clicked() {
                acts.run("chart.setStyle", json!({"value_format": f}));
            }
        }
    });
    if c.kind == ChartKind::Donut {
        let mut hole = c.donut_hole;
        if ui.add(egui::Slider::new(&mut hole, 0.1..=0.9).text("Hole")).changed() {
            acts.run("chart.setStyle", json!({"donut_hole": hole, "merge_key": key("hole")}));
        }
    }
    if c.kind.has_axis() || c.kind == ChartKind::Progress {
        ui.horizontal(|ui| {
            let mut auto = c.max.is_none();
            if ui.checkbox(&mut auto, if c.kind == ChartKind::Progress { "Out of 100" } else { "Auto scale" }).changed() {
                let top = c.data.iter().map(|p| p.value).fold(1.0_f64, f64::max);
                acts.run("chart.setStyle", json!({"max": if auto { None } else { Some(top.ceil()) }}));
            }
            if let Some(m) = c.max {
                let mut m = m;
                if ui.add(egui::DragValue::new(&mut m).range(0.001..=1e12).prefix("Max ")).changed() {
                    acts.run("chart.setStyle", json!({"max": m, "merge_key": key("max")}));
                }
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

pub(super) fn table(ui: &mut egui::Ui, t: &Tokens, id: u64, tb: &TableEl, brand: &[Color], acts: &mut Actions) {
    let key = |what: &str| format!("{what}:{id}");
    section(ui, t, "CELLS");
    let cols = tb.columns();
    egui::ScrollArea::horizontal().id_salt(("table_cells", id)).show(ui, |ui| {
        egui::Grid::new(("table_grid", id)).spacing([4.0, 4.0]).show(ui, |ui| {
            for (r, row) in tb.rows.iter().enumerate() {
                for c in 0..cols {
                    let mut text = row.get(c).cloned().unwrap_or_default();
                    let edit = egui::TextEdit::singleline(&mut text).id_salt(("cell", id, r, c)).desired_width(80.0);
                    if ui.add(edit).changed() {
                        acts.run("table.setCell", json!({"row": r, "col": c, "text": text, "merge_key": format!("cell:{id}:{r}:{c}")}));
                    }
                }
                ui.end_row();
            }
        });
    });
    ui.horizontal_wrapped(|ui| {
        if ui.button("Add row").clicked() {
            acts.run("table.insertRow", json!({}));
        }
        if ui.add_enabled(tb.rows.len() > 1, egui::Button::new("Remove row")).clicked() {
            acts.run("table.deleteRow", json!({}));
        }
        if ui.button("Add column").clicked() {
            acts.run("table.insertColumn", json!({}));
        }
        if ui.add_enabled(cols > 1, egui::Button::new("Remove column")).clicked() {
            acts.run("table.deleteColumn", json!({}));
        }
    });
    let mut header = tb.header_row;
    if ui.checkbox(&mut header, "Header row").changed() {
        acts.run("table.setStyle", json!({"header_row": header}));
    }

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
    section(ui, t, "GRID LINES");
    let mut lines = tb.border.is_some();
    if ui.checkbox(&mut lines, "Grid lines").changed() {
        acts.run("style.stroke", if lines { json!({"width": 1.5, "color": "#d9d6e8"}) } else { json!({"none": true}) });
    }
    if let Some(b) = &tb.border {
        if let Some(c) = color_row(ui, b.color, brand) {
            acts.run("style.stroke", json!({"color": c.to_hex(), "merge_key": key("tableline")}));
        }
        let mut w = b.width;
        if ui.add(egui::Slider::new(&mut w, 0.5..=12.0).text("Width")).changed() {
            acts.run("style.stroke", json!({"width": w, "merge_key": key("tablelinew")}));
        }
    }
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
