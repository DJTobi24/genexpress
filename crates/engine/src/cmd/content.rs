//! Content and smart features: templates, libraries, smart resize, align/distribute, collages,
//! filters, background removal, QR codes, brand from a picture, and file-in/file-out quick actions.

use serde_json::{Value, json};

use super::{always, bad, cmd, color_param, f64_or, has_doc, has_multi, has_selection, ok, str_param, str_req, targets};
use crate::{CommandSpec, EngineError, Result, Session};
use genha_doc::{Asset, AssetId, BrandColor, Bytes, Color, Element, ElementKind, Fill, Frame, ImageEl, NodeId, ShapeEl, TextEl};
use genha_geom::Rect;

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(query "template.list", "Templates", [], None, "{category?, query?}", always, template_list),
        cmd!(
            "template.use",
            "Use Template",
            [],
            None,
            "{id, as_page?: bool — add its pages to the open design instead of opening a new one, brand?: bool — apply the design's brand kit}",
            always,
            template_use
        ),
        cmd!(query "library.list", "Shapes & Icons", [], None, "{category?}", always, library_list),
        cmd!("library.addShape", "Add Library Shape", [], None, "{id, x?, y?, width?, height?, fill?}", has_doc, library_add),
        cmd!(query "textPresets.list", "Text Styles", [], None, "{}", always, text_presets_list),
        cmd!("text.addPreset", "Add Styled Text", [], None, "{id, text?, x?, y?, width?}", has_doc, text_add_preset),
        cmd!(query "palettes.list", "Palettes", [], None, "{}", always, palettes_list),
        cmd!(query "filters.list", "Picture Filters", [], None, "{}", always, filters_list),
        cmd!(
            "page.resize",
            "Resize Design…",
            ["Page"],
            None,
            "{preset? | width, height, index?, copy?: bool — keep the page and add a resized copy after it} — re-lays out the content for the new size",
            has_doc,
            page_resize
        ),
        cmd!(
            "object.align",
            "Align",
            ["Arrange"],
            None,
            "{how: left|center-h|right|top|center-v|bottom, to?: page|selection (default: page for one object, selection for several), ids?}",
            has_selection,
            align
        ),
        cmd!("object.distribute", "Distribute", ["Arrange"], None, "{axis: horizontal|vertical, ids?: three or more}", has_multi, distribute),
        cmd!(
            "layout.collage",
            "Make Collage",
            ["Arrange"],
            None,
            "{layout: id from the collage layouts for this many pictures, gap?: px = 16, margin?: px = gap, ids?: pictures}",
            has_selection,
            collage
        ),
        cmd!("image.filter", "Picture Filter", [], None, "{ids?, preset: see filters.list, intensity?: 0–100 = 100}", has_selection, image_filter),
        cmd!(
            "image.removeBackground",
            "Remove Background",
            [],
            None,
            "{ids?, tolerance?: 0–100 = 30} — colour-based: removes the backdrop connected to the picture's edges (works best on plain backgrounds)",
            has_selection,
            remove_background
        ),
        cmd!(
            "image.changeBackground",
            "Change Background",
            [],
            None,
            "{ids?, color: #rrggbb, tolerance?: 0–100 = 30} — replaces the backdrop connected to the picture's edges with a colour",
            has_selection,
            change_background
        ),
        cmd!(
            "image.blurBackground",
            "Blur Background",
            [],
            None,
            "{ids?, amount?: px 0–100 = 12, tolerance?: 0–100 = 30} — keeps the subject sharp",
            has_selection,
            blur_background
        ),
        cmd!("image.placeAsset", "Place Picture Again", [], None, "{asset: id of a picture already in the design}", has_doc, place_asset),
        cmd!("element.qrCode", "Add QR Code", ["Insert"], None, "{text, color?: #rrggbb, x?, y?, size?}", has_doc, qr_code),
        cmd!(
            "brand.fromPicture",
            "Brand Colours from Picture",
            ["Brand"],
            None,
            "{id?: picture element, count?: 2–8 = 5}",
            has_selection,
            brand_from_picture
        ),
        cmd!("brand.usePalette", "Use Palette as Brand Colours", ["Brand"], None, "{id: see palettes.list}", has_doc, brand_use_palette),
        cmd!(
            "quick.image",
            "Quick Action",
            [],
            None,
            "{action: remove-background|change-background|blur-background|resize|convert|auto-crop|filter|pixelate|rotate|flip, input: path, output: path (format from its extension), scale?, width?, height?, fit?, preset?, tolerance?, color?, amount?, block?, quarter_turns?, horizontal?}",
            always,
            quick_image
        ),
    ]
}

// ---------- templates and libraries ----------

fn template_list(_: &mut Session, p: &Value) -> Result<Value> {
    let cat = str_param(p, "category").map(str::to_lowercase);
    let q = str_param(p, "query").map(str::to_lowercase);
    let list: Vec<Value> = genha_templates::catalog()
        .into_iter()
        .filter(|t| cat.as_ref().is_none_or(|c| t.category.to_lowercase() == *c))
        .filter(|t| q.as_ref().is_none_or(|q| t.name.to_lowercase().contains(q) || t.tags.iter().any(|g| g.contains(q.as_str()))))
        .map(
            |t| json!({"id": t.id, "name": t.name, "category": t.category, "preset": t.preset, "width": t.width, "height": t.height, "tags": t.tags}),
        )
        .collect();
    Ok(json!(list))
}

fn template_use(s: &mut Session, p: &Value) -> Result<Value> {
    let id = str_req(p, "id", "template.use")?;
    let as_page = super::bool_or(p, "as_page", false) && s.active().is_some();
    let brand = if super::bool_or(p, "brand", false) { s.active().map(|d| d.doc.brand.clone()) } else { None };
    let tdoc = genha_templates::instantiate(id, brand.as_ref())
        .ok_or_else(|| bad("template.use", format!("unknown template `{id}` (see template.list)")))?;
    if !as_page {
        let index = s.open_document(tdoc);
        return Ok(ok(json!({"document": index})));
    }
    let first = s.edit("Add Template", |doc, cx| {
        let at = (cx.page + 1).min(doc.pages.len());
        let mut remap = std::collections::BTreeMap::new();
        for (aid, asset) in tdoc.assets {
            remap.insert(aid, doc.add_asset(asset));
        }
        for (i, mut page) in tdoc.pages.into_iter().enumerate() {
            doc.reassign_page_ids(&mut page);
            page.for_each_mut(&mut |e| {
                if let ElementKind::Image(img) = &mut e.kind
                    && let Some(n) = remap.get(&img.asset)
                {
                    img.asset = *n;
                }
            });
            doc.pages.insert(at + i, page);
        }
        cx.page = at;
        cx.selection.clear();
        Ok(at)
    })?;
    Ok(ok(json!({"page": first})))
}

fn library_list(_: &mut Session, p: &Value) -> Result<Value> {
    let cat = str_param(p, "category");
    Ok(json!(
        genha_templates::shape_library()
            .into_iter()
            .filter(|s| cat.is_none_or(|c| c.eq_ignore_ascii_case(s.category)))
            .map(|s| json!({"id": s.id, "label": s.label, "category": s.category}))
            .collect::<Vec<_>>()
    ))
}

fn library_add(s: &mut Session, p: &Value) -> Result<Value> {
    let id = str_req(p, "id", "library.addShape")?;
    let item = genha_templates::shape_library()
        .into_iter()
        .find(|x| x.id == id)
        .ok_or_else(|| bad("library.addShape", format!("unknown shape `{id}` (see library.list)")))?;
    let mut params = p.clone();
    if let Some(o) = params.as_object_mut() {
        o.insert("shape_json".into(), serde_json::to_value(&item.shape).map_err(|e| EngineError::Other(e.to_string()))?);
        o.insert("name".into(), json!(item.label));
    }
    super::create::run_shape_add(s, &params)
}

fn text_presets_list(_: &mut Session, _: &Value) -> Result<Value> {
    Ok(json!(
        genha_templates::text_presets()
            .into_iter()
            .map(|t| json!({"id": t.id, "label": t.label, "sample": t.sample, "style": t.style, "effect": t.effect}))
            .collect::<Vec<_>>()
    ))
}

fn text_add_preset(s: &mut Session, p: &Value) -> Result<Value> {
    let id = str_req(p, "id", "text.addPreset")?;
    let pr = genha_templates::text_presets()
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| bad("text.addPreset", format!("unknown text style `{id}`")))?;
    let d = s.doc()?;
    let (pw, ph) = d.doc.pages.get(d.page).map(|pg| (pg.width, pg.height)).unwrap_or((1080.0, 1080.0));
    // Presets are sized for a 1080 px page.
    let k = (pw.min(ph) / 1080.0).clamp(0.05, 20.0);
    let mut style = pr.style.clone();
    style.font_size = (style.font_size * k).max(4.0);
    let mut effect = pr.effect.clone();
    scale_effect(&mut effect, k);
    let text = str_param(p, "text").unwrap_or(pr.sample).to_string();
    let width = f64_or(p, "width", pw * 0.8);
    let h = super::create::text_height(s, &text, &style, width);
    let frame = Frame::new(f64_or(p, "x", (pw - width) / 2.0), f64_or(p, "y", (ph - h) / 2.0), width, h);
    let mut el = Element::new(NodeId(0), frame, ElementKind::Text(TextEl { text, style, effect, curve: 0.0 }));
    el.name = pr.label.to_string();
    super::create::add_element(s, "Add Text", el)
}

fn scale_effect(e: &mut genha_doc::TextEffect, k: f64) {
    use genha_doc::TextEffect::*;
    match e {
        Shadow { offset_x, offset_y, blur, .. } => {
            *offset_x *= k;
            *offset_y *= k;
            *blur *= k;
        }
        Outline { width, .. } => *width *= k,
        Highlight { padding, corner_radius, .. } => {
            *padding *= k;
            *corner_radius *= k;
        }
        None => {}
    }
}

fn palettes_list(_: &mut Session, _: &Value) -> Result<Value> {
    Ok(json!(genha_templates::palettes().into_iter().map(|p| json!({"id": p.id, "name": p.name, "colors": p.colors})).collect::<Vec<_>>()))
}

fn filters_list(_: &mut Session, _: &Value) -> Result<Value> {
    Ok(json!(
        genha_effects::filter_presets().into_iter().map(|f| json!({"id": f.id, "label": f.label, "adjustments": f.adjustments})).collect::<Vec<_>>()
    ))
}

// ---------- layout ----------

fn page_resize(s: &mut Session, p: &Value) -> Result<Value> {
    let d = s.doc()?;
    let i = super::usize_param(p, "index").unwrap_or(d.page);
    let page = d.doc.pages.get(i).cloned().ok_or_else(|| bad("page.resize", format!("no page {i}")))?;
    let (w, h) = super::page::size_params(p, "page.resize", (page.width, page.height))?;
    let copy = super::bool_or(p, "copy", false);
    if copy && d.doc.pages.len() >= genha_doc::MAX_PAGES {
        return Err(bad("page.resize", "too many pages"));
    }
    let preset_name = str_param(p, "preset").and_then(genha_doc::preset::find).map(|x| x.label);
    let index = s.edit(if copy { "Duplicate & Resize" } else { "Resize" }, |doc, cx| {
        let mut resized = genha_layout::smart_resize(&page, w, h);
        if copy {
            doc.reassign_page_ids(&mut resized);
            resized.name = match preset_name {
                Some(n) => format!("{} — {n}", page.name),
                None => format!("{} — {w:.0}×{h:.0}", page.name),
            };
            doc.pages.insert(i + 1, resized);
            cx.page = i + 1;
            cx.selection.clear();
            Ok(i + 1)
        } else {
            *doc.page_mut(i)? = resized;
            Ok(i)
        }
    })?;
    Ok(ok(json!({"index": index, "width": w, "height": h})))
}

fn frames_of(s: &Session, ids: &[NodeId]) -> Result<Vec<Frame>> {
    let d = s.doc()?;
    ids.iter().map(|id| d.doc.element(*id).map(|e| e.frame).map_err(EngineError::from)).collect()
}

fn apply_frames(s: &mut Session, label: &str, ids: &[NodeId], frames: Vec<Frame>) -> Result<Value> {
    s.edit(label, |doc, _| {
        for (id, f) in ids.iter().zip(frames) {
            let el = doc.element_mut(*id)?;
            if !el.locked {
                super::object::set_frame_moving_children(el, f);
            }
        }
        Ok(())
    })?;
    Ok(ok(json!({"changed": ids.len()})))
}

fn align(s: &mut Session, p: &Value) -> Result<Value> {
    let how_s = str_req(p, "how", "object.align")?;
    let how = genha_layout::Align::from_id(how_s).ok_or_else(|| bad("object.align", format!("unknown alignment `{how_s}`")))?;
    let ids = targets(s, p, "object.align")?;
    let frames = frames_of(s, &ids)?;
    let d = s.doc()?;
    let page = d.doc.pages.get(d.page).map(|pg| Rect::new(0.0, 0.0, pg.width, pg.height));
    let to_page = match str_param(p, "to") {
        Some("page") => true,
        Some("selection") => false,
        Some(o) => return Err(bad("object.align", format!("`to` must be page or selection, not `{o}`"))),
        None => ids.len() == 1,
    };
    let out = genha_layout::align(&frames, how, if to_page { page } else { None });
    apply_frames(s, "Align", &ids, out)
}

fn distribute(s: &mut Session, p: &Value) -> Result<Value> {
    let axis: genha_layout::Axis = super::parse(p, "axis", "object.distribute")?;
    let ids = targets(s, p, "object.distribute")?;
    if ids.len() < 3 {
        return Err(bad("object.distribute", "select three or more objects"));
    }
    let frames = frames_of(s, &ids)?;
    let out = genha_layout::distribute(&frames, axis);
    apply_frames(s, "Distribute", &ids, out)
}

fn collage(s: &mut Session, p: &Value) -> Result<Value> {
    let ids: Vec<NodeId> = {
        let all = targets(s, p, "layout.collage")?;
        let d = s.doc()?;
        all.into_iter().filter(|id| d.doc.element(*id).is_ok_and(|e| matches!(e.kind, ElementKind::Image(_)))).collect()
    };
    if ids.is_empty() {
        return Err(bad("layout.collage", "select pictures"));
    }
    let layout_id = str_req(p, "layout", "layout.collage")?;
    let layout = genha_layout::collage_layouts(ids.len())
        .into_iter()
        .find(|l| l.id == layout_id)
        .ok_or_else(|| bad("layout.collage", format!("no layout `{layout_id}` for {} pictures", ids.len())))?;
    let gap = f64_or(p, "gap", 16.0).clamp(0.0, 500.0);
    let margin = f64_or(p, "margin", gap).clamp(0.0, 2000.0);
    let d = s.doc()?;
    let (pw, ph) = d.doc.pages.get(d.page).map(|pg| (pg.width, pg.height)).unwrap_or((1080.0, 1080.0));
    let area = Rect::new(margin, margin, (pw - margin).max(margin + 1.0), (ph - margin).max(margin + 1.0));
    let cells = genha_layout::cells_in(area, &layout, gap);
    let n = ids.len().min(cells.len());
    s.edit("Collage", |doc, _| {
        for (id, f) in ids.iter().zip(cells.iter()).take(n) {
            let el = doc.element_mut(*id)?;
            el.frame = *f;
            if let ElementKind::Image(img) = &mut el.kind {
                img.fit = genha_doc::Fit::Cover;
            }
        }
        Ok(())
    })?;
    Ok(ok(json!({"placed": n})))
}

// ---------- pictures ----------

fn image_targets(s: &Session, p: &Value, cmd: &str) -> Result<Vec<NodeId>> {
    let ids = targets(s, p, cmd)?;
    let d = s.doc()?;
    let v: Vec<NodeId> =
        ids.into_iter().filter(|id| d.doc.element(*id).is_ok_and(|e| matches!(e.kind, ElementKind::Image(_)) && !e.locked)).collect();
    if v.is_empty() { Err(bad(cmd, "select a picture")) } else { Ok(v) }
}

fn image_filter(s: &mut Session, p: &Value) -> Result<Value> {
    let id = str_req(p, "preset", "image.filter")?;
    let preset = genha_effects::filter_preset(id).ok_or_else(|| bad("image.filter", format!("unknown filter `{id}` (see filters.list)")))?;
    let intensity = f64_or(p, "intensity", 100.0).clamp(0.0, 100.0) as f32;
    let adj = genha_effects::blend_preset(&preset, intensity);
    let ids = image_targets(s, p, "image.filter")?;
    s.edit("Filter", |doc, _| {
        for id in &ids {
            if let ElementKind::Image(img) = &mut doc.element_mut(*id)?.kind {
                img.adjustments = adj.clone();
                img.filter = (preset.id != "none").then(|| preset.id.to_string());
            }
        }
        Ok(())
    })?;
    Ok(ok(json!({"changed": ids.len()})))
}

/// A PNG asset from pixels.
fn png_asset(name: &str, img: &genha_effects::RgbaImage) -> Result<Asset> {
    let data = genha_effects::encode(img, genha_effects::OutputFormat::Png)?;
    Ok(Asset { name: name.to_string(), mime: "image/png".into(), width: img.width(), height: img.height(), data: Bytes::new(data) })
}

fn remove_background(s: &mut Session, p: &Value) -> Result<Value> {
    let tolerance = f64_or(p, "tolerance", 30.0).clamp(0.0, 100.0) as f32;
    replace_pictures(s, p, "image.removeBackground", "Remove Background", "cutout", |img| genha_effects::remove_background(img, tolerance))
}

fn change_background(s: &mut Session, p: &Value) -> Result<Value> {
    let tolerance = f64_or(p, "tolerance", 30.0).clamp(0.0, 100.0) as f32;
    let color = super::color_req(p, "color", "image.changeBackground")?;
    replace_pictures(s, p, "image.changeBackground", "Change Background", "background", |img| genha_effects::change_background(img, color, tolerance))
}

fn blur_background(s: &mut Session, p: &Value) -> Result<Value> {
    let tolerance = f64_or(p, "tolerance", 30.0).clamp(0.0, 100.0) as f32;
    let amount = f64_or(p, "amount", 12.0).clamp(0.0, 100.0) as f32;
    replace_pictures(s, p, "image.blurBackground", "Blur Background", "blurred", |img| genha_effects::blur_background(img, tolerance, amount))
}

/// Replaces the selected pictures' data with `f` applied to it (one new asset per source picture).
fn replace_pictures(
    s: &mut Session,
    p: &Value,
    cmd: &str,
    label: &str,
    suffix: &str,
    f: impl Fn(&genha_effects::RgbaImage) -> genha_effects::RgbaImage,
) -> Result<Value> {
    let ids = image_targets(s, p, cmd)?;
    // Decode and cut out before the edit (the slow part), one new asset per source picture.
    let d = s.doc()?;
    let mut done: std::collections::BTreeMap<AssetId, Asset> = Default::default();
    for id in &ids {
        if let Ok(Element { kind: ElementKind::Image(img), .. }) = d.doc.element(*id)
            && !done.contains_key(&img.asset)
        {
            let src = d.doc.asset(img.asset)?;
            let pixels = genha_effects::decode(&src.data)?;
            let cut = f(&pixels);
            let stem = src.name.rsplit_once('.').map(|(a, _)| a).unwrap_or(&src.name);
            done.insert(img.asset, png_asset(&format!("{stem}-{suffix}.png"), &cut)?);
        }
    }
    s.edit(label, |doc, _| {
        let mut map = std::collections::BTreeMap::new();
        for (old, asset) in done {
            map.insert(old, doc.add_asset(asset));
        }
        for id in &ids {
            if let ElementKind::Image(img) = &mut doc.element_mut(*id)?.kind
                && let Some(n) = map.get(&img.asset)
            {
                img.asset = *n;
            }
        }
        Ok(())
    })?;
    Ok(ok(json!({"changed": ids.len()})))
}

fn place_asset(s: &mut Session, p: &Value) -> Result<Value> {
    let aid = p.get("asset").and_then(Value::as_u64).map(AssetId).ok_or_else(|| bad("image.placeAsset", "missing `asset`"))?;
    let d = s.doc()?;
    let a = d.doc.asset(aid)?;
    let (pw, ph) = d.doc.pages.get(d.page).map(|pg| (pg.width, pg.height)).unwrap_or((1080.0, 1080.0));
    let (iw, ih) = (f64::from(a.width.max(1)), f64::from(a.height.max(1)));
    let k = (pw * 0.6 / iw).min(ph * 0.6 / ih);
    let (w, h) = (iw * k, ih * k);
    let mut el = Element::new(NodeId(0), Frame::new((pw - w) / 2.0, (ph - h) / 2.0, w, h), ElementKind::Image(ImageEl::new(aid)));
    el.name = a.name.clone();
    super::create::add_element(s, "Place Picture", el)
}

fn qr_code(s: &mut Session, p: &Value) -> Result<Value> {
    let text = str_req(p, "text", "element.qrCode")?;
    let color = color_param(p, "color", "element.qrCode")?.unwrap_or(Color::BLACK);
    let shape = genha_templates::qr_code(text, color).map_err(|e| bad("element.qrCode", e.to_string()))?;
    let d = s.doc()?;
    let (pw, ph) = d.doc.pages.get(d.page).map(|pg| (pg.width, pg.height)).unwrap_or((1080.0, 1080.0));
    let size = f64_or(p, "size", pw.min(ph) * 0.3).max(16.0);
    let frame = Frame::new(f64_or(p, "x", (pw - size) / 2.0), f64_or(p, "y", (ph - size) / 2.0), size, size);
    // A white plate behind the code keeps it scannable on any background.
    let mut el = Element::new(NodeId(0), frame, ElementKind::Shape(ShapeEl { shape, fill: Fill::solid(color), stroke: None, shadow: None }));
    el.name = format!("QR code: {}", text.chars().take(40).collect::<String>());
    let plate = Element::new(
        NodeId(0),
        frame,
        ElementKind::Shape(ShapeEl {
            shape: genha_doc::ShapeKind::Rect { corner_radius: size * 0.04 },
            fill: Fill::solid(Color::WHITE),
            stroke: None,
            shadow: None,
        }),
    );
    let gid = s.edit("Add QR Code", |doc, cx| {
        let mut plate = plate;
        let mut code = el;
        plate.id = doc.alloc_id();
        plate.name = "QR background".into();
        code.id = doc.alloc_id();
        let gid = doc.alloc_id();
        let mut g = Element::new(gid, frame, ElementKind::Group(genha_doc::GroupEl { children: vec![plate, code] }));
        g.name = "QR code".into();
        doc.page_mut(cx.page)?.elements.push(g);
        cx.selection = vec![gid];
        Ok(gid)
    })?;
    Ok(ok(json!({"id": gid})))
}

// ---------- brand ----------

fn brand_from_picture(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = image_targets(s, p, "brand.fromPicture")?;
    let count = f64_or(p, "count", 5.0).clamp(2.0, 8.0) as usize;
    let d = s.doc()?;
    let Some(ElementKind::Image(img)) = ids.first().and_then(|id| d.doc.element(*id).ok()).map(|e| &e.kind) else {
        return Err(bad("brand.fromPicture", "select a picture"));
    };
    let pixels = genha_effects::decode(&d.doc.asset(img.asset)?.data)?;
    let mut colors = genha_effects::dominant_colors(&pixels, count);
    // Darkest last-but-one and lightest last, so templates' "dark"/"light" slots read well.
    colors.sort_by(|a, b| b.luminance().total_cmp(&a.luminance()));
    let names = ["Primary", "Secondary", "Accent", "Accent 2", "Accent 3", "Accent 4", "Dark", "Light"];
    let mut ordered = colors.clone();
    if let (Some(light), Some(dark)) = (colors.first().copied(), colors.last().copied())
        && colors.len() >= 4
    {
        ordered = colors.iter().skip(1).take(colors.len() - 2).copied().collect();
        ordered.insert(2.min(ordered.len()), dark);
        ordered.insert(3.min(ordered.len()), light);
    }
    let list: Vec<BrandColor> =
        ordered.iter().enumerate().map(|(i, c)| BrandColor { name: names.get(i).copied().unwrap_or("Colour").to_string(), color: *c }).collect();
    let hex: Vec<String> = list.iter().map(|c| c.color.to_hex()).collect();
    s.edit("Brand Colours from Picture", |doc, _| {
        doc.brand.colors = list;
        Ok(())
    })?;
    Ok(ok(json!({"colors": hex})))
}

fn brand_use_palette(s: &mut Session, p: &Value) -> Result<Value> {
    let id = str_req(p, "id", "brand.usePalette")?;
    let pal =
        genha_templates::palettes().into_iter().find(|x| x.id == id).ok_or_else(|| bad("brand.usePalette", format!("unknown palette `{id}`")))?;
    let names = ["Primary", "Secondary", "Dark", "Light", "Accent", "Accent 2", "Accent 3", "Accent 4"];
    let list: Vec<BrandColor> =
        pal.colors.iter().enumerate().map(|(i, c)| BrandColor { name: names.get(i).copied().unwrap_or("Colour").to_string(), color: *c }).collect();
    s.edit("Use Palette", |doc, _| {
        doc.brand.colors = list;
        Ok(())
    })?;
    Ok(ok(json!({})))
}

// ---------- quick actions (file in, file out) ----------

#[cfg(not(target_arch = "wasm32"))]
fn quick_image(_: &mut Session, p: &Value) -> Result<Value> {
    use genha_effects as fx;
    let action = str_req(p, "action", "quick.image")?;
    let input = str_req(p, "input", "quick.image")?;
    let output = str_req(p, "output", "quick.image")?;
    let ext = std::path::Path::new(output).extension().and_then(|e| e.to_str()).unwrap_or("png");
    let mut format = fx::OutputFormat::from_extension(ext)
        .ok_or_else(|| bad("quick.image", format!("unsupported output type `.{ext}` (png, jpg, webp, gif, bmp)")))?;
    if let fx::OutputFormat::Jpeg { quality } = &mut format {
        *quality = f64_or(p, "quality", f64::from(*quality)).clamp(1.0, 100.0) as u8;
    }
    let len = std::fs::metadata(input).map_err(|e| EngineError::Other(format!("cannot open {input}: {e}")))?.len();
    if len > super::create::MAX_PICTURE_BYTES as u64 {
        return Err(EngineError::Other(format!("{input} is too large")));
    }
    let bytes = std::fs::read(input).map_err(|e| EngineError::Other(format!("cannot read {input}: {e}")))?;
    let img = fx::decode(&bytes)?;
    let out = match action {
        "remove-background" => fx::remove_background(&img, f64_or(p, "tolerance", 30.0).clamp(0.0, 100.0) as f32),
        "resize" => {
            let (w, h) = match (p.get("width").and_then(Value::as_f64), p.get("height").and_then(Value::as_f64)) {
                (Some(w), Some(h)) => (w, h),
                (Some(w), None) => (w, w * f64::from(img.height()) / f64::from(img.width().max(1))),
                (None, Some(h)) => (h * f64::from(img.width()) / f64::from(img.height().max(1)), h),
                (None, None) => {
                    let k = f64_or(p, "scale", 0.5).clamp(0.01, 16.0);
                    (f64::from(img.width()) * k, f64::from(img.height()) * k)
                }
            };
            if !w.is_finite() || !h.is_finite() {
                return Err(bad("quick.image", "invalid size"));
            }
            let fit: genha_geom::Fit = if p.get("fit").is_some() { super::parse(p, "fit", "quick.image")? } else { genha_geom::Fit::Stretch };
            fx::resize(&img, w.round().clamp(1.0, 16384.0) as u32, h.round().clamp(1.0, 16384.0) as u32, fit)?
        }
        "convert" => img,
        "change-background" => {
            let color = super::color_param(p, "color", "quick.image")?.unwrap_or(Color::WHITE);
            fx::change_background(&img, color, f64_or(p, "tolerance", 30.0).clamp(0.0, 100.0) as f32)
        }
        "blur-background" => fx::blur_background(&img, f64_or(p, "tolerance", 30.0).clamp(0.0, 100.0) as f32, f64_or(p, "amount", 12.0) as f32),
        "pixelate" => fx::pixelate(&img, f64_or(p, "block", 16.0).clamp(2.0, 256.0) as u32),
        "auto-crop" => fx::auto_crop(&img),
        "rotate" => fx::rotate(&img, f64_or(p, "quarter_turns", 1.0).clamp(-4.0, 4.0) as i32),
        "flip" => fx::flip(&img, super::bool_or(p, "horizontal", true)),
        "filter" => {
            let id = str_param(p, "preset").unwrap_or("mono");
            let preset = fx::filter_preset(id).ok_or_else(|| bad("quick.image", format!("unknown filter `{id}`")))?;
            let mut img = img;
            fx::apply_adjustments(&mut img, &fx::blend_preset(&preset, f64_or(p, "intensity", 100.0).clamp(0.0, 100.0) as f32));
            img
        }
        other => return Err(bad("quick.image", format!("unknown action `{other}`"))),
    };
    let data = fx::encode(&out, format)?;
    super::export::write(output, &data)?;
    Ok(ok(json!({"output": output, "width": out.width(), "height": out.height(), "bytes": data.len()})))
}

#[cfg(target_arch = "wasm32")]
fn quick_image(_: &mut Session, _: &Value) -> Result<Value> {
    Err(EngineError::Other("file quick actions are not available in the browser".into()))
}
