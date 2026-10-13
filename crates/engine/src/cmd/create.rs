//! Adding shapes, text and pictures.

use base64::Engine as _;
use serde_json::{Value, json};

use super::{bad, cmd, color_param, f64_or, has_doc, merge, ok, parse, str_param};
use crate::{CommandSpec, EngineError, Result, Session};
use genha_doc::{Asset, Bytes, Color, Element, ElementKind, Fill, Frame, ImageEl, ShapeEl, ShapeKind, StrokeStyle, TextEffect, TextEl, TextStyle};

/// Largest picture file accepted.
pub const MAX_PICTURE_BYTES: usize = 100 << 20;

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "shape.add",
            "Add Shape",
            ["Insert"],
            None,
            "{shape: rect|rounded|ellipse|circle|triangle|polygon|star|line|path, x?, y?, width?, height?, rotation?, fill?: #rrggbb|none, stroke?: #rrggbb, stroke_width?, corner_radius?, sides?, points?, inner_ratio?, d?: SVG path data, view_width?, view_height?, name?} — or {shape_json: ShapeKind JSON, …}; centred on the page when no position is given",
            has_doc,
            shape_add
        ),
        cmd!(
            "text.add",
            "Add Text",
            ["Insert"],
            Some("T"),
            "{text, role?: heading|subheading|body, x?, y?, width?, height?, style?: partial TextStyle {font_family, font_size, bold, italic, color, align, line_height, letter_spacing, uppercase, underline}, effect?: TextEffect JSON}",
            has_doc,
            text_add
        ),
        cmd!(
            "image.place",
            "Place Picture…",
            ["Insert"],
            Some("Cmd+Shift+P"),
            "{path | data: base64, name?, x?, y?, width?, height?} — fitted to 60% of the page when no size is given",
            has_doc,
            image_place
        ),
    ]
}

pub(crate) fn page_size(s: &Session) -> Result<(f64, f64)> {
    let d = s.doc()?;
    Ok(d.doc.pages.get(d.page).map(|p| (p.width, p.height)).unwrap_or((1080.0, 1080.0)))
}

/// `x, y, width, height, rotation` from params, defaulting to a `dw × dh` box centred on the page.
pub(crate) fn frame_params(p: &Value, page: (f64, f64), dw: f64, dh: f64) -> Frame {
    let w = f64_or(p, "width", dw).max(1.0);
    let h = f64_or(p, "height", dh).max(1.0);
    let x = f64_or(p, "x", (page.0 - w) / 2.0);
    let y = f64_or(p, "y", (page.1 - h) / 2.0);
    Frame { x, y, width: w, height: h, rotation: f64_or(p, "rotation", 0.0) }.sanitized()
}

/// Adds `el` on top of the current page and selects it.
pub(crate) fn add_element(s: &mut Session, label: &str, mut el: Element) -> Result<Value> {
    let id = s.edit(label, |doc, cx| {
        el.id = doc.alloc_id();
        let id = el.id;
        doc.page_mut(cx.page)?.elements.push(el);
        cx.selection = vec![id];
        Ok(id)
    })?;
    Ok(ok(json!({"id": id})))
}

pub(crate) fn shape_kind(p: &Value, cmd: &str) -> Result<ShapeKind> {
    if p.get("shape_json").is_some() {
        return parse::<ShapeKind>(p, "shape_json", cmd);
    }
    let kind = str_param(p, "shape").unwrap_or("rect");
    Ok(match kind {
        "rect" | "square" => ShapeKind::Rect { corner_radius: f64_or(p, "corner_radius", 0.0).max(0.0) },
        "rounded" => ShapeKind::Rect { corner_radius: f64_or(p, "corner_radius", 24.0).max(0.0) },
        "ellipse" | "circle" => ShapeKind::Ellipse,
        "triangle" => ShapeKind::Triangle,
        "polygon" => ShapeKind::Polygon { sides: f64_or(p, "sides", 6.0).clamp(3.0, 64.0) as u32 },
        "star" => {
            ShapeKind::Star { points: f64_or(p, "points", 5.0).clamp(3.0, 64.0) as u32, inner_ratio: f64_or(p, "inner_ratio", 0.5).clamp(0.05, 1.0) }
        }
        "line" => ShapeKind::Line,
        "path" => {
            let d = str_param(p, "d").ok_or_else(|| bad(cmd, "a path shape needs `d` (SVG path data)"))?;
            if d.len() > 1 << 20 {
                return Err(bad(cmd, "path data is larger than 1 MB"));
            }
            ShapeKind::Path {
                d: d.to_string(),
                view_width: f64_or(p, "view_width", 100.0).max(1e-6),
                view_height: f64_or(p, "view_height", 100.0).max(1e-6),
            }
        }
        other => return Err(bad(cmd, format!("unknown shape `{other}`"))),
    })
}

fn shape_add(s: &mut Session, p: &Value) -> Result<Value> {
    run_shape_add(s, p)
}

pub(crate) fn run_shape_add(s: &mut Session, p: &Value) -> Result<Value> {
    let shape = shape_kind(p, "shape.add")?;
    let page = page_size(s)?;
    let side = page.0.min(page.1) * 0.3;
    let is_line = matches!(shape, ShapeKind::Line);
    let frame = frame_params(p, page, side, if is_line { 4.0 } else { side });
    let primary = s.doc()?.doc.brand.color(0, Color::rgb(0x5b, 0x3c, 0xf5));
    let fill = match p.get("fill") {
        Some(Value::String(v)) if v == "none" => Fill::None,
        Some(Value::Object(_)) => parse::<Fill>(p, "fill", "shape.add")?,
        _ => match color_param(p, "fill", "shape.add")? {
            Some(c) => Fill::solid(c),
            None if is_line => Fill::None,
            None => Fill::solid(primary),
        },
    };
    let stroke = match color_param(p, "stroke", "shape.add")? {
        Some(color) => Some(StrokeStyle { color, width: f64_or(p, "stroke_width", 4.0).max(0.0), dash: vec![] }),
        None if is_line => Some(StrokeStyle { color: primary, width: f64_or(p, "stroke_width", 6.0).max(0.0), dash: vec![] }),
        None => None,
    };
    let mut el = Element::new(genha_doc::NodeId(0), frame, ElementKind::Shape(ShapeEl { shape, fill, stroke, shadow: None }));
    if let Some(n) = str_param(p, "name") {
        el.name = n.to_string();
    }
    add_element(s, "Add Shape", el)
}

/// Rough text height before layout: lines × size × line height (the text crate measures exactly).
pub(crate) fn estimate_text_size(text: &str, style: &TextStyle, max_width: f64) -> (f64, f64) {
    let em = style.font_size.max(1.0);
    let char_w = em * 0.55;
    let mut lines = 0.0;
    let mut widest: f64 = 0.0;
    for line in text.split('\n') {
        let w = line.chars().count() as f64 * char_w;
        widest = widest.max(w.min(max_width));
        lines += (w / max_width.max(1.0)).ceil().max(1.0);
    }
    (widest, lines * em * style.line_height.max(0.5))
}

/// The height `text` needs in a box `width` wide (laid out with the session's fonts).
pub(crate) fn text_height(s: &Session, text: &str, style: &TextStyle, width: f64) -> f64 {
    if s.fonts.is_empty() {
        return estimate_text_size(text, style, width).1;
    }
    genha_text::layout(&s.fonts, text, style, width).height.max(style.font_size)
}

/// The height a text element (with its style runs) needs in a box `width` wide.
pub(crate) fn text_el_height(s: &Session, t: &genha_doc::TextEl, width: f64) -> f64 {
    if t.spans.is_empty() {
        return text_height(s, &t.text, &t.style, width);
    }
    genha_text::layout_el(&s.fonts, t, width).height.max(t.style.font_size)
}

fn text_add(s: &mut Session, p: &Value) -> Result<Value> {
    let text = str_param(p, "text").unwrap_or("Add your text").to_string();
    if text.len() > 100_000 {
        return Err(bad("text.add", "text longer than 100000 bytes"));
    }
    let brand = s.doc()?.doc.brand.clone();
    let page = page_size(s)?;
    let mut style = TextStyle { color: brand.color(2, Color::rgb(0x1b, 0x1b, 0x2f)), ..TextStyle::default() };
    let base = page.0.min(page.1);
    match str_param(p, "role").unwrap_or("body") {
        "heading" => {
            style.font_family = brand.heading_font.clone();
            style.font_size = (base * 0.09).round().max(8.0);
            style.bold = true;
        }
        "subheading" => {
            style.font_family = brand.heading_font.clone();
            style.font_size = (base * 0.05).round().max(8.0);
            style.bold = true;
        }
        "body" => {
            style.font_family = brand.body_font.clone();
            style.font_size = (base * 0.035).round().max(8.0);
        }
        other => return Err(bad("text.add", format!("unknown role `{other}` (heading, subheading, body)"))),
    }
    if let Some(patch) = p.get("style") {
        style = merge(&style, patch, "text.add")?;
    }
    if !style.font_size.is_finite() || !(1.0..=2000.0).contains(&style.font_size) {
        return Err(bad("text.add", "font_size must be 1–2000"));
    }
    let effect = if p.get("effect").is_some() { parse::<TextEffect>(p, "effect", "text.add")? } else { TextEffect::None };
    let width = f64_or(p, "width", page.0 * 0.8);
    let h = text_height(s, &text, &style, width);
    let frame = frame_params(p, page, width, h);
    let el = Element::new(
        genha_doc::NodeId(0),
        frame,
        ElementKind::Text(TextEl { text, style, effect, curve: 0.0, spans: vec![], fit: Default::default() }),
    );
    add_element(s, "Add Text", el)
}

/// A picture asset from file bytes (checked to decode-able size).
pub fn asset_from_bytes(name: &str, bytes: &[u8]) -> Result<Asset> {
    if bytes.len() > MAX_PICTURE_BYTES {
        return Err(EngineError::Other(format!("{name} is larger than {} MB", MAX_PICTURE_BYTES >> 20)));
    }
    let info = genha_effects::probe(bytes)?;
    Ok(Asset { name: name.to_string(), mime: info.mime.to_string(), width: info.width, height: info.height, data: Bytes::new(bytes.to_vec()) })
}

fn read_picture(p: &Value) -> Result<(String, Vec<u8>)> {
    if let Some(data) = str_param(p, "data") {
        if data.len() > MAX_PICTURE_BYTES * 4 / 3 + 16 {
            return Err(bad("image.place", "picture data too large"));
        }
        let bytes =
            base64::engine::general_purpose::STANDARD.decode(data.trim()).map_err(|e| bad("image.place", format!("`data` is not base64: {e}")))?;
        return Ok((str_param(p, "name").unwrap_or("picture").to_string(), bytes));
    }
    let path = str_param(p, "path").ok_or_else(|| bad("image.place", "give `path` or `data`"))?;
    read_file(path)
}

#[cfg(not(target_arch = "wasm32"))]
fn read_file(path: &str) -> Result<(String, Vec<u8>)> {
    let path = std::path::Path::new(path);
    let len = std::fs::metadata(path).map_err(|e| EngineError::Other(format!("cannot open {}: {e}", path.display())))?.len();
    if len > MAX_PICTURE_BYTES as u64 {
        return Err(EngineError::Other(format!("{} is larger than {} MB", path.display(), MAX_PICTURE_BYTES >> 20)));
    }
    let bytes = std::fs::read(path).map_err(|e| EngineError::Other(format!("cannot read {}: {e}", path.display())))?;
    Ok((path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "picture".into()), bytes))
}

#[cfg(target_arch = "wasm32")]
fn read_file(_: &str) -> Result<(String, Vec<u8>)> {
    Err(EngineError::Other("reading files by path is not available in the browser".into()))
}

/// The shapes of an SVG file as one element (a group when there are several), in the SVG's own
/// coordinates.
pub(crate) fn svg_element(name: &str, bytes: &[u8]) -> Result<(Element, f64, f64)> {
    let imported = genha_svg::import::import(bytes).map_err(|e| EngineError::Other(format!("{name}: {e}")))?;
    let (w, h) = (imported.width, imported.height);
    let mut els = imported.elements;
    let mut el = if els.len() == 1
        && let Some(one) = els.pop()
    {
        one
    } else {
        let mut g = Element::new(genha_doc::NodeId(0), Frame::default(), ElementKind::Group(genha_doc::GroupEl { children: els }));
        super::object::fit_group_frame(&mut g);
        g
    };
    el.name = name.to_string();
    Ok((el, w, h))
}

/// Places an SVG file's shapes on the current page, fitted to 60% of it unless a frame is given.
pub fn place_svg(s: &mut Session, name: &str, bytes: &[u8], p: &Value) -> Result<Value> {
    let (mut el, _, _) = svg_element(name, bytes)?;
    let page = page_size(s)?;
    let (iw, ih) = (el.frame.width.max(1e-3), el.frame.height.max(1e-3));
    let k = (page.0 * 0.6 / iw).min(page.1 * 0.6 / ih);
    let target = frame_params(p, page, iw * k, ih * k);
    super::object::set_frame_moving_children(&mut el, target);
    let id = s.edit("Place SVG", |doc, cx| {
        doc.reassign_ids(&mut el);
        let id = el.id;
        doc.page_mut(cx.page)?.elements.push(el);
        cx.selection = vec![id];
        Ok(id)
    })?;
    Ok(ok(json!({"id": id})))
}

/// Places picture bytes on the current page (SVG files become editable shapes).
pub fn place_picture(s: &mut Session, name: &str, bytes: &[u8], p: &Value) -> Result<Value> {
    if genha_svg::import::is_svg(bytes) {
        return place_svg(s, name, bytes, p);
    }
    let asset = asset_from_bytes(name, bytes)?;
    let page = page_size(s)?;
    let (iw, ih) = (f64::from(asset.width.max(1)), f64::from(asset.height.max(1)));
    let scale = (page.0 * 0.6 / iw).min(page.1 * 0.6 / ih);
    let (dw, dh) = (iw * scale, ih * scale);
    let frame = frame_params(p, page, dw, dh);
    let id = s.edit("Place Picture", |doc, cx| {
        let existing = doc.assets.iter().find(|(_, a)| a.data == asset.data).map(|(id, _)| *id);
        let aid = existing.unwrap_or_else(|| doc.add_asset(asset.clone()));
        let id = doc.alloc_id();
        let mut el = Element::new(id, frame, ElementKind::Image(ImageEl::new(aid)));
        el.name = asset.name.clone();
        doc.page_mut(cx.page)?.elements.push(el);
        cx.selection = vec![id];
        Ok(id)
    })?;
    Ok(ok(json!({"id": id})))
}

fn image_place(s: &mut Session, p: &Value) -> Result<Value> {
    let (name, bytes) = read_picture(p)?;
    place_picture(s, &name, &bytes, p)
}
