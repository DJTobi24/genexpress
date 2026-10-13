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
            "{preset? | width, height, index?, copy?: bool — keep the page and add a resized copy after it, mode?: smart|scale|none = smart} — smart re-lays out the design for the new size (text reflowed, rows and grids re-arranged; `notes` lists anything that could not be kept readable), scale scales the content to fit and centres it, none changes the page size only",
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
            "{ids?, method?: auto|ml|color = auto, model?: u2netp|u2net, tolerance?: 0–100 = 30} — ml: a segmentation model finds the subject (works on busy photos; needs model.download once); color: removes the backdrop connected to the picture's edges (plain backgrounds); auto: ml when a model is installed, else color",
            has_selection,
            remove_background
        ),
        cmd!(
            "image.changeBackground",
            "Change Background",
            [],
            None,
            "{ids?, color: #rrggbb, method?: auto|ml|color = auto, model?, tolerance?: 0–100 = 30} — replaces the background with a colour (the subject is found as in image.removeBackground)",
            has_selection,
            change_background
        ),
        cmd!(
            "image.blurBackground",
            "Blur Background",
            [],
            None,
            "{ids?, amount?: px 0–100 = 12, method?: auto|ml|color = auto, model?, tolerance?: 0–100 = 30} — keeps the subject sharp (found as in image.removeBackground)",
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
            "{action: remove-background|change-background|blur-background|resize|convert|auto-crop|filter|pixelate|rotate|flip, input: path, output: path (format from its extension), scale?, width?, height?, fit?, preset?, method?: auto|ml|color (background actions), model?, tolerance?, color?, amount?, block?, quarter_turns?, horizontal?}",
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
    let mut el = Element::new(NodeId(0), frame, ElementKind::Text(TextEl { text, style, effect, curve: 0.0, spans: vec![] }));
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
    let mode = match str_param(p, "mode") {
        None => genha_layout::ResizeMode::Smart,
        Some(m) => genha_layout::ResizeMode::parse(m).ok_or_else(|| bad("page.resize", format!("unknown mode `{m}` (smart, scale, none)")))?,
    };
    let preset_name = str_param(p, "preset").and_then(genha_doc::preset::find).map(|x| x.label);
    let genha_layout::Resized { page: mut resized, notes } = genha_layout::resize_page(&page, w, h, mode, &s.fonts);
    let index = s.edit(if copy { "Duplicate & Resize" } else { "Resize" }, |doc, cx| {
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
    Ok(ok(json!({"index": index, "width": w, "height": h, "notes": notes})))
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
    run_background(s, "image.removeBackground", p)
}

fn change_background(s: &mut Session, p: &Value) -> Result<Value> {
    run_background(s, "image.changeBackground", p)
}

fn blur_background(s: &mut Session, p: &Value) -> Result<Value> {
    run_background(s, "image.blurBackground", p)
}

/// A background command in one go: [`background_job`], [`BackgroundJob::run`], [`apply_background`].
fn run_background(s: &mut Session, cmd: &str, p: &Value) -> Result<Value> {
    let job = background_job(s, cmd, p)?;
    let done = job.run(&mut |_, _| true)?;
    apply_background(s, done)
}

/// How the subject of a picture is told from its background.
#[derive(Clone, Debug)]
pub enum Finder {
    /// The classical colour-based flood fill from the edges (`tolerance` 0–100).
    Color { tolerance: f32 },
    /// A segmentation model; `bytes` holds the model file while it still has to be loaded.
    Ml { spec: &'static genha_segment::ModelSpec, model: Option<genha_segment::Segmenter>, bytes: Option<Vec<u8>> },
}

impl Finder {
    /// `"ml"` or `"color"`.
    pub fn method(&self) -> &'static str {
        match self {
            Finder::Color { .. } => "color",
            Finder::Ml { .. } => "ml",
        }
    }

    /// The model's name (ML only).
    pub fn model(&self) -> Option<&'static str> {
        match self {
            Finder::Ml { spec, .. } => Some(spec.name),
            Finder::Color { .. } => None,
        }
    }

    /// Loads the model when needed (slow the first time: the desktop app does it off the UI thread).
    fn ready(&mut self) -> Result<()> {
        if let Finder::Ml { spec, model, bytes } = self
            && model.is_none()
        {
            let b = bytes.take();
            *model = genha_segment::cached(spec, || b)?;
            if model.is_none() {
                return Err(EngineError::Other(format!("the {} model could not be read", spec.name)));
            }
        }
        Ok(())
    }
}

/// The finder `p` asks for (`method`: auto, ml or color; `model`; `tolerance`). Without a
/// session (file quick actions in the browser) only the colour method is available.
pub fn finder(s: Option<&Session>, p: &Value, cmd: &str) -> Result<Finder> {
    let tolerance = f64_or(p, "tolerance", 30.0).clamp(0.0, 100.0) as f32;
    let method = match p.get("method") {
        None | Some(Value::Null) => "auto",
        Some(Value::String(m)) => m.as_str(),
        Some(_) => return Err(bad(cmd, "`method` must be auto, ml or color")),
    };
    let named = match p.get("model") {
        None | Some(Value::Null) => None,
        Some(Value::String(n)) => Some(super::models::spec(n, cmd)?),
        Some(_) => return Err(bad(cmd, "`model` must be a model name (see model.status)")),
    };
    // The installed model to use, ready or with its file's bytes.
    let ml = |s: &Session| -> Option<Finder> {
        let spec = named.or_else(|| super::models::preferred(s))?;
        if !super::models::installed(s, spec) {
            return None;
        }
        let model = genha_segment::peek(spec);
        let bytes = if model.is_none() { Some(super::models::unloaded_bytes(s, spec)?) } else { None };
        Some(Finder::Ml { spec, model, bytes })
    };
    match method {
        "color" | "colour" => Ok(Finder::Color { tolerance }),
        "auto" => Ok(s.filter(|_| genha_segment::INFERENCE).and_then(ml).unwrap_or(Finder::Color { tolerance })),
        "ml" if !genha_segment::INFERENCE => Err(genha_segment::SegmentError::Unavailable.into()),
        "ml" => s.and_then(ml).ok_or_else(|| {
            let spec = named.or_else(|| s.and_then(super::models::preferred)).unwrap_or(&genha_segment::MODELS[0]);
            EngineError::Other(format!(
                "the background removal model is not installed: download it first (model.download {{\"name\": \"{}\"}}, {:.1} MB) or use method \"color\"",
                spec.name,
                spec.bytes as f64 / 1e6
            ))
        }),
        other => Err(bad(cmd, format!("unknown method `{}` (auto, ml or color)", other.chars().take(40).collect::<String>()))),
    }
}

/// What a background command does to each picture.
#[derive(Clone, Copy, Debug)]
pub enum BackgroundOp {
    Remove,
    Change(Color),
    Blur(f32),
}

impl BackgroundOp {
    /// `img` changed, its subject found by `finder` (loaded: [`Finder::ready`]).
    pub fn apply(&self, img: &genha_effects::RgbaImage, finder: &Finder) -> Result<genha_effects::RgbaImage> {
        use genha_effects as fx;
        let mask = match finder {
            // The colour method keeps its own exact results (and their tests).
            Finder::Color { tolerance } => {
                return Ok(match *self {
                    BackgroundOp::Remove => fx::remove_background(img, *tolerance),
                    BackgroundOp::Change(c) => fx::change_background(img, c, *tolerance),
                    BackgroundOp::Blur(r) => fx::blur_background(img, *tolerance, r),
                });
            }
            Finder::Ml { model: Some(m), .. } => m.mask(img)?,
            Finder::Ml { spec, .. } => return Err(EngineError::Other(format!("the {} model is not loaded", spec.name))),
        };
        Ok(match *self {
            BackgroundOp::Remove => fx::cut_out(img, &mask),
            BackgroundOp::Change(c) => fx::change_background_masked(img, &mask, c),
            BackgroundOp::Blur(r) => fx::blur_background_masked(img, &mask, r),
        })
    }
}

/// A background command's work, taken from the session so it can run on another thread (the
/// desktop app does, with progress): the pictures, the operation and how subjects are found.
#[derive(Debug)]
pub struct BackgroundJob {
    label: &'static str,
    suffix: &'static str,
    ids: Vec<NodeId>,
    sources: Vec<(AssetId, Asset)>,
    op: BackgroundOp,
    pub finder: Finder,
}

/// A finished [`BackgroundJob`], for [`apply_background`].
#[derive(Debug)]
pub struct BackgroundDone {
    label: &'static str,
    ids: Vec<NodeId>,
    assets: Vec<(AssetId, Asset)>,
    method: &'static str,
    model: Option<&'static str>,
}

/// Prepares `cmd` (image.removeBackground, image.changeBackground or image.blurBackground) with
/// params `p` on the selected pictures (or `ids`).
pub fn background_job(s: &Session, cmd: &str, p: &Value) -> Result<BackgroundJob> {
    let (op, label, suffix) = match cmd {
        "image.removeBackground" => (BackgroundOp::Remove, "Remove Background", "cutout"),
        "image.changeBackground" => (BackgroundOp::Change(super::color_req(p, "color", cmd)?), "Change Background", "background"),
        "image.blurBackground" => (BackgroundOp::Blur(f64_or(p, "amount", 12.0).clamp(0.0, 100.0) as f32), "Blur Background", "blurred"),
        other => return Err(bad(other, "not a background command")),
    };
    let ids = image_targets(s, p, cmd)?;
    let finder = finder(Some(s), p, cmd)?;
    let d = s.doc()?;
    let mut sources: Vec<(AssetId, Asset)> = vec![];
    for id in &ids {
        if let Ok(Element { kind: ElementKind::Image(img), .. }) = d.doc.element(*id)
            && !sources.iter().any(|(a, _)| *a == img.asset)
        {
            sources.push((img.asset, d.doc.asset(img.asset)?.clone()));
        }
    }
    Ok(BackgroundJob { label, suffix, ids, sources, op, finder })
}

impl BackgroundJob {
    /// How many pictures it changes.
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Does the work (the slow part): loads the model if needed, then decodes each picture, finds
    /// its subject and encodes the result. `progress(done, total)` is called before the first
    /// picture and after each one; returning false stops it (an error says so).
    pub fn run(mut self, progress: &mut dyn FnMut(usize, usize) -> bool) -> Result<BackgroundDone> {
        let total = self.sources.len();
        let cancelled = || EngineError::Other("cancelled".into());
        if !progress(0, total) {
            return Err(cancelled());
        }
        self.finder.ready()?;
        let mut assets = vec![];
        for (i, (id, src)) in self.sources.iter().enumerate() {
            let pixels = genha_effects::decode(&src.data)?;
            let out = self.op.apply(&pixels, &self.finder)?;
            let stem = src.name.rsplit_once('.').map(|(a, _)| a).unwrap_or(&src.name);
            assets.push((*id, png_asset(&format!("{stem}-{}.png", self.suffix), &out)?));
            if !progress(i + 1, total) {
                return Err(cancelled());
            }
        }
        Ok(BackgroundDone { label: self.label, ids: self.ids, assets, method: self.finder.method(), model: self.finder.model() })
    }
}

/// Puts a finished job's pictures in the design (one undo step). Pictures deleted or changed in
/// the meantime are left alone.
pub fn apply_background(s: &mut Session, done: BackgroundDone) -> Result<Value> {
    let ids = done.ids;
    let assets = done.assets;
    let changed = s.edit(done.label, |doc, _| {
        let mut map = std::collections::BTreeMap::new();
        for (old, asset) in assets {
            map.insert(old, doc.add_asset(asset));
        }
        let mut n = 0usize;
        for id in &ids {
            if let Ok(el) = doc.element_mut(*id)
                && let ElementKind::Image(img) = &mut el.kind
                && let Some(new) = map.get(&img.asset)
            {
                img.asset = *new;
                n += 1;
            }
        }
        Ok(n)
    })?;
    Ok(ok(json!({"changed": changed, "method": done.method, "model": done.model})))
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
fn quick_image(s: &mut Session, p: &Value) -> Result<Value> {
    str_req(p, "action", "quick.image")?;
    let input = str_req(p, "input", "quick.image")?;
    let output = str_req(p, "output", "quick.image")?;
    let ext = std::path::Path::new(output).extension().and_then(|e| e.to_str()).unwrap_or("png");
    quick_format(ext, p)?;
    let len = std::fs::metadata(input).map_err(|e| EngineError::Other(format!("cannot open {input}: {e}")))?.len();
    if len > super::create::MAX_PICTURE_BYTES as u64 {
        return Err(EngineError::Other(format!("{input} is too large")));
    }
    let bytes = std::fs::read(input).map_err(|e| EngineError::Other(format!("cannot read {input}: {e}")))?;
    let (data, width, height) = quick_image_with(Some(s), &bytes, ext, p)?;
    super::export::write(output, &data)?;
    Ok(ok(json!({"output": output, "width": width, "height": height, "bytes": data.len()})))
}

/// The output format of a quick action writing `ext` (png, jpg, webp, gif, bmp).
fn quick_format(ext: &str, p: &Value) -> Result<genha_effects::OutputFormat> {
    let mut format = genha_effects::OutputFormat::from_extension(ext)
        .ok_or_else(|| bad("quick.image", format!("unsupported output type `.{ext}` (png, jpg, webp, gif, bmp)")))?;
    if let genha_effects::OutputFormat::Jpeg { quality } = &mut format {
        *quality = f64_or(p, "quality", f64::from(*quality)).clamp(1.0, 100.0) as u8;
    }
    Ok(format)
}

/// A quick action on picture bytes (`quick.image`'s params without `input`/`output`): the result
/// encoded as `ext` (png, jpg, webp, gif, bmp), and its width and height. For hosts without a
/// file system (the web build). Background actions use the colour method here; see
/// [`quick_image_with`] for the ML one.
pub fn quick_image_bytes(bytes: &[u8], ext: &str, p: &Value) -> Result<(Vec<u8>, u32, u32)> {
    quick_image_with(None, bytes, ext, p)
}

/// [`quick_image_bytes`] with the session's installed models available to the background
/// actions (`method`: auto, ml or color).
pub fn quick_image_with(s: Option<&Session>, bytes: &[u8], ext: &str, p: &Value) -> Result<(Vec<u8>, u32, u32)> {
    use genha_effects as fx;
    let action = str_req(p, "action", "quick.image")?;
    let format = quick_format(ext, p)?;
    if bytes.len() > super::create::MAX_PICTURE_BYTES {
        return Err(EngineError::Other("the picture is too large".into()));
    }
    let img = fx::decode(bytes)?;
    let background = |op: BackgroundOp| -> Result<fx::RgbaImage> {
        let mut f = finder(s, p, "quick.image")?;
        f.ready()?;
        op.apply(&img, &f)
    };
    let out = match action {
        "remove-background" => background(BackgroundOp::Remove)?,
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
        "change-background" => background(BackgroundOp::Change(super::color_param(p, "color", "quick.image")?.unwrap_or(Color::WHITE)))?,
        "blur-background" => background(BackgroundOp::Blur(f64_or(p, "amount", 12.0) as f32))?,
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
    Ok((data, out.width(), out.height()))
}

#[cfg(target_arch = "wasm32")]
fn quick_image(_: &mut Session, _: &Value) -> Result<Value> {
    Err(EngineError::Other("file quick actions are not available in the browser".into()))
}
