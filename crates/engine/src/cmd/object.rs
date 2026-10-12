//! Arrange and transform objects: frames, order, groups, lock, visibility, opacity.

use serde_json::{Value, json};

use super::{bad, cmd, f64_or, has_multi, has_selection, merge, ok, str_param, targets};
use crate::{CommandSpec, EngineError, Result, Session};
use genha_doc::{Element, ElementKind, Frame, GroupEl, NodeId};
use genha_geom::{Point, Rect};

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "object.setFrames",
            "Set Frames",
            [],
            None,
            "{frames: [{id, frame: {x, y, width, height, rotation?}}], label?} — one undo step (what the Select tool emits)",
            super::has_doc,
            set_frames
        ),
        cmd!(
            "object.setFrame",
            "Position & Size",
            ["Arrange"],
            None,
            "{ids?, x?, y?, width?, height?, rotation?} — partial",
            has_selection,
            set_frame
        ),
        cmd!("object.move", "Move", [], None, "{ids?, dx, dy}", has_selection, move_by),
        cmd!("object.rotate", "Rotate", ["Arrange"], None, "{ids?, degrees: added to the current rotation}", has_selection, rotate),
        cmd!("object.flipHorizontal", "Flip Horizontal", ["Arrange"], None, "{ids?}", has_selection, flip_h),
        cmd!("object.flipVertical", "Flip Vertical", ["Arrange"], None, "{ids?}", has_selection, flip_v),
        cmd!("object.setOpacity", "Opacity", [], None, "{ids?, opacity: 0..1}", has_selection, set_opacity),
        cmd!("object.lock", "Lock", ["Arrange"], Some("Cmd+L"), "{ids?}", has_selection, lock),
        cmd!("object.unlock", "Unlock", ["Arrange"], Some("Cmd+Shift+L"), "{ids?}", has_selection, unlock),
        cmd!("object.hide", "Hide", ["Arrange"], None, "{ids?}", has_selection, hide),
        cmd!("object.show", "Show", ["Arrange"], None, "{ids?}", has_selection, show),
        cmd!("object.rename", "Rename", [], None, "{id?, name}", has_selection, rename),
        cmd!("object.bringForward", "Bring Forward", ["Arrange"], Some("Cmd+]"), "{ids?}", has_selection, forward),
        cmd!("object.sendBackward", "Send Backward", ["Arrange"], Some("Cmd+["), "{ids?}", has_selection, backward),
        cmd!("object.bringToFront", "Bring to Front", ["Arrange"], Some("Cmd+Shift+]"), "{ids?}", has_selection, to_front),
        cmd!("object.sendToBack", "Send to Back", ["Arrange"], Some("Cmd+Shift+["), "{ids?}", has_selection, to_back),
        cmd!("object.group", "Group", ["Arrange"], Some("Cmd+G"), "{ids?: at least two objects on the current page}", has_multi, group),
        cmd!("object.ungroup", "Ungroup", ["Arrange"], Some("Cmd+Shift+G"), "{ids?}", has_selection, ungroup),
    ]
}

/// Sets `el`'s frame; a group's children follow (moved, scaled and rotated with it).
pub fn set_frame_moving_children(el: &mut Element, new: Frame) {
    let old = el.frame;
    el.frame = new;
    if let ElementKind::Group(g) = &mut el.kind {
        let sx = if old.width > 0.0 { new.width / old.width } else { 1.0 };
        let sy = if old.height > 0.0 { new.height / old.height } else { 1.0 };
        let dr = new.rotation - old.rotation;
        let (oc, nc) = (old.center(), new.center());
        let rot = genha_geom::Affine::rotate_about(dr.to_radians(), nc);
        for c in &mut g.children {
            let cc = c.frame.center();
            // Position relative to the old centre, scaled in the group's unrotated space.
            let un = genha_geom::Affine::rotate_about(-old.rotation.to_radians(), oc) * cc;
            let scaled = Point::new(nc.x + (un.x - oc.x) * sx, nc.y + (un.y - oc.y) * sy);
            let back = genha_geom::Affine::rotate_about(old.rotation.to_radians(), nc) * scaled;
            let p = rot * back;
            let (w, h) = (c.frame.width * sx, c.frame.height * sy);
            let f = Frame { x: p.x - w / 2.0, y: p.y - h / 2.0, width: w, height: h, rotation: c.frame.rotation + dr };
            set_frame_moving_children(c, f);
        }
    }
}

/// Recomputes a group's frame from its children (unrotated bounds).
pub fn fit_group_frame(el: &mut Element) {
    if let ElementKind::Group(g) = &el.kind {
        let mut it = g.children.iter().map(|c| c.frame.bounds());
        if let Some(first) = it.next() {
            let b = it.fold(first, |a, r| a.union(r));
            el.frame = Frame::new(b.x0, b.y0, b.width(), b.height());
        }
    }
}

fn editable(s: &Session, ids: &[NodeId]) -> Result<Vec<NodeId>> {
    let d = s.doc()?;
    let v: Vec<NodeId> = ids.iter().filter(|id| d.doc.find(**id).is_some_and(|(_, e)| !e.locked)).copied().collect();
    if v.is_empty() { Err(EngineError::Other("the selected objects are locked".into())) } else { Ok(v) }
}

fn each(s: &mut Session, p: &Value, cmd: &str, label: &str, f: impl Fn(&mut Element)) -> Result<Value> {
    let ids = editable(s, &targets(s, p, cmd)?)?;
    s.edit(label, |doc, _| {
        for id in &ids {
            f(doc.element_mut(*id)?);
        }
        Ok(())
    })?;
    Ok(ok(json!({"changed": ids.len()})))
}

fn set_frames(s: &mut Session, p: &Value) -> Result<Value> {
    let list = p.get("frames").and_then(Value::as_array).ok_or_else(|| bad("object.setFrames", "missing `frames` array"))?;
    let mut frames = vec![];
    for item in list {
        let id = item.get("id").and_then(Value::as_u64).map(NodeId).ok_or_else(|| bad("object.setFrames", "each entry needs an `id`"))?;
        let f: Frame = super::parse(item, "frame", "object.setFrames")?;
        if !f.is_valid() {
            return Err(bad("object.setFrames", format!("invalid frame for {id}")));
        }
        frames.push((id, f));
    }
    let label = str_param(p, "label").unwrap_or("Transform").to_string();
    s.edit(&label, |doc, _| {
        for (id, f) in &frames {
            let el = doc.element_mut(*id)?;
            if !el.locked {
                set_frame_moving_children(el, *f);
            }
        }
        Ok(())
    })?;
    Ok(ok(json!({"changed": frames.len()})))
}

fn set_frame(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = editable(s, &targets(s, p, "object.setFrame")?)?;
    let mut patch = serde_json::Map::new();
    for k in ["x", "y", "width", "height", "rotation"] {
        if let Some(v) = p.get(k) {
            patch.insert(k.to_string(), v.clone());
        }
    }
    let patch = Value::Object(patch);
    s.edit("Position & Size", |doc, _| {
        for id in &ids {
            let el = doc.element_mut(*id)?;
            let f: Frame = merge(&el.frame, &patch, "object.setFrame")?;
            if !f.is_valid() {
                return Err(bad("object.setFrame", "frame values must be finite and sizes not negative"));
            }
            set_frame_moving_children(el, f);
        }
        Ok(())
    })?;
    Ok(ok(json!({})))
}

fn move_by(s: &mut Session, p: &Value) -> Result<Value> {
    let (dx, dy) = (f64_or(p, "dx", 0.0), f64_or(p, "dy", 0.0));
    each(s, p, "object.move", "Move", |e| super::edit::shift(e, dx, dy))
}

fn rotate(s: &mut Session, p: &Value) -> Result<Value> {
    let deg = super::f64_req(p, "degrees", "object.rotate")?;
    each(s, p, "object.rotate", "Rotate", |e| {
        let f = Frame { rotation: (e.frame.rotation + deg) % 360.0, ..e.frame };
        set_frame_moving_children(e, f);
    })
}

fn flip_h(s: &mut Session, p: &Value) -> Result<Value> {
    each(s, p, "object.flipHorizontal", "Flip Horizontal", |e| e.flip_h = !e.flip_h)
}

fn flip_v(s: &mut Session, p: &Value) -> Result<Value> {
    each(s, p, "object.flipVertical", "Flip Vertical", |e| e.flip_v = !e.flip_v)
}

fn set_opacity(s: &mut Session, p: &Value) -> Result<Value> {
    let o = super::f64_req(p, "opacity", "object.setOpacity")?.clamp(0.0, 1.0) as f32;
    each(s, p, "object.setOpacity", "Opacity", |e| e.opacity = o)
}

fn lock(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = targets(s, p, "object.lock")?;
    s.edit("Lock", |doc, _| {
        for id in &ids {
            doc.element_mut(*id)?.locked = true;
        }
        Ok(())
    })?;
    Ok(ok(json!({})))
}

fn unlock(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = targets(s, p, "object.unlock")?;
    s.edit("Unlock", |doc, _| {
        for id in &ids {
            doc.element_mut(*id)?.locked = false;
        }
        Ok(())
    })?;
    Ok(ok(json!({})))
}

fn hide(s: &mut Session, p: &Value) -> Result<Value> {
    each(s, p, "object.hide", "Hide", |e| e.hidden = true)
}

fn show(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = targets(s, p, "object.show")?;
    s.edit("Show", |doc, _| {
        for id in &ids {
            doc.element_mut(*id)?.hidden = false;
        }
        Ok(())
    })?;
    Ok(ok(json!({})))
}

fn rename(s: &mut Session, p: &Value) -> Result<Value> {
    let name = str_param(p, "name").ok_or_else(|| bad("object.rename", "missing `name`"))?.to_string();
    let ids = targets(s, p, "object.rename")?;
    s.edit("Rename", |doc, _| {
        for id in &ids {
            doc.element_mut(*id)?.name = name.clone();
        }
        Ok(())
    })?;
    Ok(ok(json!({})))
}

/// Reorders top-level elements of the current page. `step`: +1 forward, -1 backward, `i64::MAX`
/// front, `i64::MIN` back.
fn arrange(s: &mut Session, p: &Value, cmd: &str, label: &str, step: i64) -> Result<Value> {
    let ids = targets(s, p, cmd)?;
    s.edit(label, |doc, cx| {
        let page = doc.page_mut(cx.page)?;
        let n = page.elements.len();
        let mut order: Vec<usize> = page.elements.iter().enumerate().filter(|(_, e)| ids.contains(&e.id)).map(|(i, _)| i).collect();
        if order.is_empty() {
            return Err(bad(cmd, "the objects are not on the current page (or are inside a group)"));
        }
        match step {
            i64::MAX | i64::MIN => {
                let (mut picked, rest): (Vec<Element>, Vec<Element>) = page.elements.drain(..).partition(|e| ids.contains(&e.id));
                page.elements = if step == i64::MAX {
                    let mut v = rest;
                    v.append(&mut picked);
                    v
                } else {
                    picked.extend(rest);
                    picked
                };
            }
            1 => {
                order.reverse();
                for i in order {
                    if i + 1 < n && !ids.contains(&page.elements[i + 1].id) {
                        page.elements.swap(i, i + 1);
                    }
                }
            }
            _ => {
                for i in order {
                    if i > 0 && !ids.contains(&page.elements[i - 1].id) {
                        page.elements.swap(i, i - 1);
                    }
                }
            }
        }
        Ok(())
    })?;
    Ok(ok(json!({})))
}

fn forward(s: &mut Session, p: &Value) -> Result<Value> {
    arrange(s, p, "object.bringForward", "Bring Forward", 1)
}
fn backward(s: &mut Session, p: &Value) -> Result<Value> {
    arrange(s, p, "object.sendBackward", "Send Backward", -1)
}
fn to_front(s: &mut Session, p: &Value) -> Result<Value> {
    arrange(s, p, "object.bringToFront", "Bring to Front", i64::MAX)
}
fn to_back(s: &mut Session, p: &Value) -> Result<Value> {
    arrange(s, p, "object.sendToBack", "Send to Back", i64::MIN)
}

fn group(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = targets(s, p, "object.group")?;
    if ids.len() < 2 {
        return Err(bad("object.group", "select at least two objects"));
    }
    let gid = s.edit("Group", |doc, cx| {
        let gid = doc.alloc_id();
        let page = doc.page_mut(cx.page)?;
        let positions: Vec<usize> = page.elements.iter().enumerate().filter(|(_, e)| ids.contains(&e.id)).map(|(i, _)| i).collect();
        let (Some(&top), true) = (positions.last(), positions.len() == ids.len()) else {
            return Err(bad("object.group", "all objects must be on the current page, outside groups"));
        };
        let insert_at = top + 1 - positions.len();
        let (children, rest): (Vec<Element>, Vec<Element>) = page.elements.drain(..).partition(|e| ids.contains(&e.id));
        page.elements = rest;
        let mut g = Element::new(gid, Frame::default(), ElementKind::Group(GroupEl { children }));
        fit_group_frame(&mut g);
        page.elements.insert(insert_at.min(page.elements.len()), g);
        cx.selection = vec![gid];
        Ok(gid)
    })?;
    Ok(ok(json!({"id": gid})))
}

fn ungroup(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = targets(s, p, "object.ungroup")?;
    let released = s.edit("Ungroup", |doc, cx| {
        let page = doc.page_mut(cx.page)?;
        let mut out = vec![];
        let mut released = vec![];
        for e in page.elements.drain(..) {
            match e.kind {
                ElementKind::Group(g) if ids.contains(&e.id) => {
                    for mut c in g.children {
                        c.opacity *= e.opacity;
                        c.hidden |= e.hidden;
                        released.push(c.id);
                        out.push(c);
                    }
                }
                kind => out.push(Element { kind, ..e }),
            }
        }
        page.elements = out;
        if released.is_empty() {
            return Err(bad("object.ungroup", "select a group"));
        }
        cx.selection = released.clone();
        Ok(released)
    })?;
    Ok(ok(json!({"ids": released})))
}

/// The bounds of several frames.
pub fn bounds_of(frames: impl Iterator<Item = Frame>) -> Option<Rect> {
    frames.map(|f| f.bounds()).reduce(|a, b| a.union(b))
}
