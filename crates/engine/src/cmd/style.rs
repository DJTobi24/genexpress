//! Fill, stroke, shadow, text style and effects, picture adjustments, copy/paste style.

use serde_json::{Value, json};

use super::{bad, cmd, color_param, f64_or, has_selection, merge, ok, parse, str_param, targets};
use crate::{CommandSpec, EngineError, Result, Session};
use genexpress_doc::{Adjustments, CropRect, Element, ElementKind, Fill, Fit, NodeId, Shadow, ShapeKind, StrokeStyle, TextEffect, TextStyle};

/// What Copy Style takes from an object.
#[derive(Clone, Debug)]
pub struct CopiedStyle {
    pub fill: Option<Fill>,
    pub stroke: Option<Option<StrokeStyle>>,
    pub shadow: Option<Option<Shadow>>,
    pub text: Option<(TextStyle, TextEffect)>,
    pub adjustments: Option<(Adjustments, Option<String>)>,
    pub opacity: f32,
}

fn has_style(s: &Session) -> std::result::Result<(), String> {
    has_selection(s)?;
    if s.style_clipboard.is_none() { Err("copy a style first".into()) } else { Ok(()) }
}

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "style.fill",
            "Fill",
            [],
            None,
            "{ids?, color?: #rrggbb | \"none\", fill?: Fill JSON} — shapes fill; text sets its colour",
            has_selection,
            fill
        ),
        cmd!(
            "style.stroke",
            "Border",
            [],
            None,
            "{ids?, color?: #rrggbb, width?, dash?: [len…], none?: bool} — shapes and pictures",
            has_selection,
            stroke
        ),
        cmd!("style.shadow", "Shadow", [], None, "{ids?, color?, offset_x?, offset_y?, blur?, none?: bool} — shapes", has_selection, shadow),
        cmd!("style.cornerRadius", "Corner Radius", [], None, "{ids?, radius} — rectangles and pictures", has_selection, corner_radius),
        cmd!("shape.setKind", "Change Shape", [], None, "{ids?, shape: …same as shape.add}", has_selection, set_shape),
        cmd!("text.setText", "Edit Text", [], None, "{id?, text}", has_selection, set_text),
        cmd!(
            "text.setStyle",
            "Text Style",
            [],
            None,
            "{ids?, …partial TextStyle: font_family, font_size, bold, italic, underline, uppercase, color, align: left|center|right, line_height, letter_spacing}",
            has_selection,
            set_text_style
        ),
        cmd!("text.setEffect", "Text Effect", [], None, "{ids?, effect: {type: none|shadow|outline|highlight, …}}", has_selection, set_text_effect),
        cmd!(
            "image.adjust",
            "Adjust Picture",
            [],
            None,
            "{ids?, …partial Adjustments: brightness, contrast, saturation, warmth (-100..100), sharpen, blur, grayscale, sepia (0..100), invert; reset?: bool}",
            has_selection,
            adjust
        ),
        cmd!("image.crop", "Crop Picture", [], None, "{ids?, x, y, width, height: fractions 0..1 of the picture; reset?: bool}", has_selection, crop),
        cmd!("image.setFit", "Picture Fit", [], None, "{ids?, fit: cover|contain|stretch}", has_selection, set_fit),
        cmd!("image.mask", "Crop to Shape", [], None, "{ids?, shape?: circle|star|… (as shape.add), none?: bool}", has_selection, mask),
        cmd!("style.copy", "Copy Style", ["Edit"], Some("Cmd+Alt+C"), "{id?}", has_selection, copy_style),
        cmd!("style.paste", "Paste Style", ["Edit"], Some("Cmd+Alt+V"), "{ids?}", has_style, paste_style),
    ]
}

fn each_kind(s: &mut Session, p: &Value, cmd: &str, label: &str, f: impl Fn(&mut Element) -> bool) -> Result<Value> {
    let ids: Vec<NodeId> = targets(s, p, cmd)?;
    let n = s.edit(label, |doc, _| {
        let mut n = 0;
        for id in &ids {
            let el = doc.element_mut(*id)?;
            if !el.locked && apply_deep(el, &f) {
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

/// Applies `f` to an element, or to every child of a group.
fn apply_deep(el: &mut Element, f: &impl Fn(&mut Element) -> bool) -> bool {
    if let ElementKind::Group(g) = &mut el.kind {
        let mut any = false;
        for c in &mut g.children {
            any |= apply_deep(c, f);
        }
        return any;
    }
    f(el)
}

fn fill(s: &mut Session, p: &Value) -> Result<Value> {
    let fill = match p.get("color") {
        Some(Value::String(v)) if v == "none" => Fill::None,
        _ => match color_param(p, "color", "style.fill")? {
            Some(c) => Fill::solid(c),
            None => parse::<Fill>(p, "fill", "style.fill")?,
        },
    };
    each_kind(s, p, "style.fill", "Fill", |e| match &mut e.kind {
        ElementKind::Shape(sh) => {
            sh.fill = fill.clone();
            true
        }
        ElementKind::Text(t) => match fill.primary_color() {
            Some(c) => {
                t.style.color = c;
                true
            }
            None => false,
        },
        _ => false,
    })
}

fn stroke(s: &mut Session, p: &Value) -> Result<Value> {
    let none = super::bool_or(p, "none", false);
    let color = color_param(p, "color", "style.stroke")?;
    let width = p.get("width").and_then(Value::as_f64).filter(|w| w.is_finite()).map(|w| w.clamp(0.0, 1000.0));
    let dash: Option<Vec<f64>> =
        p.get("dash").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_f64).filter(|v| v.is_finite() && *v >= 0.0).take(16).collect());
    let apply = |cur: &mut Option<StrokeStyle>| {
        if none {
            *cur = None;
            return;
        }
        let mut st = cur.clone().unwrap_or(StrokeStyle { color: genexpress_doc::Color::BLACK, width: 4.0, dash: vec![] });
        if let Some(c) = color {
            st.color = c;
        }
        if let Some(w) = width {
            st.width = w;
        }
        if let Some(d) = &dash {
            st.dash = d.clone();
        }
        *cur = Some(st);
    };
    each_kind(s, p, "style.stroke", "Border", |e| match &mut e.kind {
        ElementKind::Shape(sh) => {
            apply(&mut sh.stroke);
            true
        }
        ElementKind::Image(i) => {
            apply(&mut i.border);
            true
        }
        _ => false,
    })
}

fn shadow(s: &mut Session, p: &Value) -> Result<Value> {
    let none = super::bool_or(p, "none", false);
    let color = color_param(p, "color", "style.shadow")?;
    let (ox, oy, blur) =
        (p.get("offset_x").and_then(Value::as_f64), p.get("offset_y").and_then(Value::as_f64), p.get("blur").and_then(Value::as_f64));
    each_kind(s, p, "style.shadow", "Shadow", |e| match &mut e.kind {
        ElementKind::Shape(sh) => {
            if none {
                sh.shadow = None;
            } else {
                let mut sd = sh.shadow.clone().unwrap_or_default();
                if let Some(c) = color {
                    sd.color = c;
                }
                for (dst, src) in [(&mut sd.offset_x, ox), (&mut sd.offset_y, oy), (&mut sd.blur, blur)] {
                    if let Some(v) = src.filter(|v| v.is_finite()) {
                        *dst = v.clamp(-2000.0, 2000.0);
                    }
                }
                sd.blur = sd.blur.max(0.0);
                sh.shadow = Some(sd);
            }
            true
        }
        _ => false,
    })
}

fn corner_radius(s: &mut Session, p: &Value) -> Result<Value> {
    let r = super::f64_req(p, "radius", "style.cornerRadius")?.max(0.0);
    each_kind(s, p, "style.cornerRadius", "Corner Radius", |e| match &mut e.kind {
        ElementKind::Shape(sh) => match &mut sh.shape {
            ShapeKind::Rect { corner_radius } => {
                *corner_radius = r;
                true
            }
            _ => false,
        },
        ElementKind::Image(i) => {
            i.corner_radius = r;
            true
        }
        _ => false,
    })
}

fn set_shape(s: &mut Session, p: &Value) -> Result<Value> {
    let kind = super::create::shape_kind(p, "shape.setKind")?;
    each_kind(s, p, "shape.setKind", "Change Shape", |e| match &mut e.kind {
        ElementKind::Shape(sh) => {
            sh.shape = kind.clone();
            true
        }
        _ => false,
    })
}

fn set_text(s: &mut Session, p: &Value) -> Result<Value> {
    let text = str_param(p, "text").ok_or_else(|| bad("text.setText", "missing `text`"))?.to_string();
    if text.len() > 100_000 {
        return Err(bad("text.setText", "text longer than 100000 bytes"));
    }
    each_kind(s, p, "text.setText", "Edit Text", |e| match &mut e.kind {
        ElementKind::Text(t) => {
            t.text = text.clone();
            true
        }
        _ => false,
    })
}

fn set_text_style(s: &mut Session, p: &Value) -> Result<Value> {
    let mut patch = p.clone();
    if let Some(o) = patch.as_object_mut() {
        o.remove("ids");
        o.remove("id");
    }
    // Validate the patch once against a default style so errors name the command.
    let probe: TextStyle = merge(&TextStyle::default(), &patch, "text.setStyle")?;
    if !probe.font_size.is_finite() || !(1.0..=2000.0).contains(&probe.font_size) {
        return Err(bad("text.setStyle", "font_size must be 1–2000"));
    }
    each_kind(s, p, "text.setStyle", "Text Style", |e| match &mut e.kind {
        ElementKind::Text(t) => match merge(&t.style, &patch, "text.setStyle") {
            Ok(st) => {
                t.style = st;
                true
            }
            Err(_) => false,
        },
        _ => false,
    })
}

fn set_text_effect(s: &mut Session, p: &Value) -> Result<Value> {
    let effect: TextEffect = parse(p, "effect", "text.setEffect")?;
    each_kind(s, p, "text.setEffect", "Text Effect", |e| match &mut e.kind {
        ElementKind::Text(t) => {
            t.effect = effect.clone();
            true
        }
        _ => false,
    })
}

fn adjust(s: &mut Session, p: &Value) -> Result<Value> {
    let reset = super::bool_or(p, "reset", false);
    let mut patch = p.clone();
    if let Some(o) = patch.as_object_mut() {
        for k in ["ids", "id", "reset"] {
            o.remove(k);
        }
    }
    let _: Adjustments = merge(&Adjustments::default(), &patch, "image.adjust")?;
    each_kind(s, p, "image.adjust", "Adjust Picture", |e| match &mut e.kind {
        ElementKind::Image(i) => {
            let base = if reset { Adjustments::default() } else { i.adjustments.clone() };
            match merge(&base, &patch, "image.adjust") {
                Ok(a) => {
                    i.adjustments = a;
                    i.filter = None;
                    true
                }
                Err(_) => false,
            }
        }
        _ => false,
    })
}

fn crop(s: &mut Session, p: &Value) -> Result<Value> {
    let c = if super::bool_or(p, "reset", false) {
        CropRect::default()
    } else {
        CropRect { x: f64_or(p, "x", 0.0), y: f64_or(p, "y", 0.0), width: f64_or(p, "width", 1.0), height: f64_or(p, "height", 1.0) }.sanitized()
    };
    each_kind(s, p, "image.crop", "Crop Picture", |e| match &mut e.kind {
        ElementKind::Image(i) => {
            i.crop = c;
            true
        }
        _ => false,
    })
}

fn set_fit(s: &mut Session, p: &Value) -> Result<Value> {
    let fit: Fit = parse(p, "fit", "image.setFit")?;
    each_kind(s, p, "image.setFit", "Picture Fit", |e| match &mut e.kind {
        ElementKind::Image(i) => {
            i.fit = fit;
            true
        }
        _ => false,
    })
}

fn mask(s: &mut Session, p: &Value) -> Result<Value> {
    let m = if super::bool_or(p, "none", false) { None } else { Some(super::create::shape_kind(p, "image.mask")?) };
    each_kind(s, p, "image.mask", "Crop to Shape", |e| match &mut e.kind {
        ElementKind::Image(i) => {
            i.mask = m.clone();
            true
        }
        _ => false,
    })
}

fn copy_style(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = targets(s, p, "style.copy")?;
    let id = ids.first().copied().ok_or_else(|| bad("style.copy", "select an object"))?;
    let el = s.doc()?.doc.element(id)?.clone();
    let mut st = CopiedStyle { fill: None, stroke: None, shadow: None, text: None, adjustments: None, opacity: el.opacity };
    match &el.kind {
        ElementKind::Shape(sh) => {
            st.fill = Some(sh.fill.clone());
            st.stroke = Some(sh.stroke.clone());
            st.shadow = Some(sh.shadow.clone());
        }
        ElementKind::Text(t) => {
            st.text = Some((t.style.clone(), t.effect.clone()));
            st.fill = Some(Fill::solid(t.style.color));
        }
        ElementKind::Image(i) => {
            st.adjustments = Some((i.adjustments.clone(), i.filter.clone()));
            st.stroke = Some(i.border.clone());
        }
        ElementKind::Group(_) => return Err(bad("style.copy", "copy the style of a single object, not a group")),
    }
    s.style_clipboard = Some(st);
    Ok(ok(json!({"from": id})))
}

fn paste_style(s: &mut Session, p: &Value) -> Result<Value> {
    let st = s.style_clipboard.clone().ok_or_else(|| bad("style.paste", "copy a style first"))?;
    each_kind(s, p, "style.paste", "Paste Style", |e| {
        e.opacity = st.opacity;
        match &mut e.kind {
            ElementKind::Shape(sh) => {
                if let Some(f) = &st.fill {
                    sh.fill = f.clone();
                }
                if let Some(x) = &st.stroke {
                    sh.stroke = x.clone();
                }
                if let Some(x) = &st.shadow {
                    sh.shadow = x.clone();
                }
                true
            }
            ElementKind::Text(t) => {
                if let Some((style, effect)) = &st.text {
                    t.style = style.clone();
                    t.effect = effect.clone();
                } else if let Some(c) = st.fill.as_ref().and_then(Fill::primary_color) {
                    t.style.color = c;
                }
                true
            }
            ElementKind::Image(i) => {
                if let Some((a, f)) = &st.adjustments {
                    i.adjustments = a.clone();
                    i.filter = f.clone();
                }
                true
            }
            ElementKind::Group(_) => false,
        }
    })
}
