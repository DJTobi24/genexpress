//! Brand kit: colours, fonts and logos, and applying them to a design.

use serde_json::{Value, json};

use super::{bad, cmd, color_req, has_doc, ok, str_param, str_req, usize_param};
use crate::{CommandSpec, Result, Session};
use genha_doc::{BrandColor, ElementKind, Fill};

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!("brand.rename", "Rename Brand", ["Brand"], None, "{name}", has_doc, rename),
        cmd!("brand.addColor", "Add Brand Colour", ["Brand"], None, "{color: #rrggbb, name?}", has_doc, add_color),
        cmd!("brand.setColor", "Change Brand Colour", [], None, "{index, color?, name?}", has_doc, set_color),
        cmd!("brand.removeColor", "Remove Brand Colour", [], None, "{index}", has_doc, remove_color),
        cmd!("brand.setFonts", "Brand Fonts", ["Brand"], None, "{heading?, body?}", has_doc, set_fonts),
        cmd!(
            "brand.apply",
            "Apply Brand to Page",
            ["Brand"],
            None,
            "{index?: page, all_pages?: bool} — headings (bold or ≥ 1.5× the smallest size) get the heading font, other text the body font",
            has_doc,
            apply
        ),
    ]
}

fn rename(s: &mut Session, p: &Value) -> Result<Value> {
    let name = str_req(p, "name", "brand.rename")?.to_string();
    s.edit("Rename Brand", |doc, _| {
        doc.brand.name = name;
        Ok(())
    })?;
    Ok(ok(json!({})))
}

fn add_color(s: &mut Session, p: &Value) -> Result<Value> {
    let color = color_req(p, "color", "brand.addColor")?;
    let name = str_param(p, "name").map(str::to_string).unwrap_or_else(|| color.to_hex());
    let i = s.edit("Add Brand Colour", |doc, _| {
        if doc.brand.colors.len() >= 64 {
            return Err(bad("brand.addColor", "a brand can have at most 64 colours"));
        }
        doc.brand.colors.push(BrandColor { name, color });
        Ok(doc.brand.colors.len() - 1)
    })?;
    Ok(ok(json!({"index": i})))
}

fn set_color(s: &mut Session, p: &Value) -> Result<Value> {
    let i = usize_param(p, "index").ok_or_else(|| bad("brand.setColor", "missing `index`"))?;
    let color = super::color_param(p, "color", "brand.setColor")?;
    let name = str_param(p, "name").map(str::to_string);
    s.edit("Change Brand Colour", |doc, _| {
        let c = doc.brand.colors.get_mut(i).ok_or_else(|| bad("brand.setColor", format!("no brand colour {i}")))?;
        if let Some(color) = color {
            c.color = color;
        }
        if let Some(n) = name {
            c.name = n;
        }
        Ok(())
    })?;
    Ok(ok(json!({})))
}

fn remove_color(s: &mut Session, p: &Value) -> Result<Value> {
    let i = usize_param(p, "index").ok_or_else(|| bad("brand.removeColor", "missing `index`"))?;
    s.edit("Remove Brand Colour", |doc, _| {
        if i >= doc.brand.colors.len() {
            return Err(bad("brand.removeColor", format!("no brand colour {i}")));
        }
        doc.brand.colors.remove(i);
        Ok(())
    })?;
    Ok(ok(json!({})))
}

fn set_fonts(s: &mut Session, p: &Value) -> Result<Value> {
    let heading = str_param(p, "heading").map(str::to_string);
    let body = str_param(p, "body").map(str::to_string);
    s.edit("Brand Fonts", |doc, _| {
        if let Some(h) = heading {
            doc.brand.heading_font = h;
        }
        if let Some(b) = body {
            doc.brand.body_font = b;
        }
        Ok(())
    })?;
    Ok(ok(json!({})))
}

fn apply(s: &mut Session, p: &Value) -> Result<Value> {
    let all = super::bool_or(p, "all_pages", false);
    let d = s.doc()?;
    let only = usize_param(p, "index").unwrap_or(d.page);
    let n = s.edit("Apply Brand", |doc, _| {
        let brand = doc.brand.clone();
        let mut n = 0;
        for (pi, page) in doc.pages.iter_mut().enumerate() {
            if !all && pi != only {
                continue;
            }
            let mut min_size = f64::MAX;
            page.for_each(&mut |e| {
                if let ElementKind::Text(t) = &e.kind {
                    min_size = min_size.min(t.style.font_size);
                }
            });
            page.for_each_mut(&mut |e| {
                if let ElementKind::Text(t) = &mut e.kind {
                    let heading = t.style.bold || t.style.font_size >= min_size * 1.5;
                    t.style.font_family = if heading { brand.heading_font.clone() } else { brand.body_font.clone() };
                    n += 1;
                }
            });
            // A plain white background takes the brand's light colour.
            if let (Fill::Solid { color }, Some(light)) = (&mut page.background, brand.colors.get(3))
                && *color == genha_doc::Color::WHITE
            {
                *color = light.color;
            }
        }
        Ok(n)
    })?;
    Ok(ok(json!({"texts": n})))
}
