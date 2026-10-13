//! Motion: element animations (entrance, loop, exit), page timelines and animated exports (GIF,
//! animated PNG, MP4 video). What an animation looks like at a moment is `genha_motion::page_at`;
//! exports render those moments with `genha_render`; videos are encoded by `genha_video`.

use serde_json::{Value, json};

use super::{always, bad, bool_or, cmd, color_param, f64_or, has_doc, ok, str_req, targets, usize_param};
use crate::{CommandSpec, EngineError, Result, Session};
use genha_doc::{Animation, AnimationKind, Document, LoopKind, NodeId, Page};
use genha_motion as motion;

/// Most frames an animated export renders (all pages together).
pub const MAX_FRAMES: usize = 600;
/// Largest frame of an animated export, in pixels (the scale is lowered to fit).
pub const MAX_FRAME_PIXELS: f64 = 2_000_000.0;
/// Most frames of a video export (2 minutes at 30 frames per second). Frames that repeat the
/// previous one are not rendered again, so only animated stretches cost rendering time.
pub const MAX_VIDEO_FRAMES: usize = 3600;
/// Largest frame of a video export, in pixels: 1920 × 1080 and 1080 × 1920 fit (the scale is
/// lowered to fit).
pub const MAX_VIDEO_PIXELS: f64 = 2_088_960.0;
/// Frame rate of a video export unless `fps` says otherwise.
pub const DEFAULT_VIDEO_FPS: u32 = 30;
/// Default delay between elements for `animation.applyToPage`, in seconds.
pub const DEFAULT_STAGGER: f64 = 0.25;

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "animation.set",
            "Animate…",
            ["Animate"],
            None,
            "{ids?, kind?: entrance (fade, rise, slide-left, slide-right, slide-up, slide-down, pop, zoom, bounce, spin, flip, typewriter, or none to remove; needed unless already animated), duration?: s = 0.6, delay?: s = 0, loop?: pulse|float|sway|blink|none, exit?: an entrance kind played in reverse at the end of the page, or none}",
            has_doc,
            set
        ),
        cmd!("animation.remove", "Remove Animation", ["Animate"], None, "{ids?}", has_doc, remove),
        cmd!(
            "animation.applyToPage",
            "Animate All Elements",
            ["Animate"],
            None,
            "{kind (or none to remove all), stagger?: s between elements = 0.25, duration?: s = 0.6, page?: index (default current), loop?, exit?, include_background?: bool = false (also animate objects covering the whole page)}",
            has_doc,
            apply_to_page
        ),
        cmd!(query "animation.list", "Animation Styles", [], None, "{} — entrance and loop kinds with labels", always, list),
        cmd!(
            query "animation.timeline",
            "Animation Timeline",
            [],
            None,
            "{page?: index} — the page duration and each animated object's start, entrance end and exit start, in seconds",
            has_doc,
            timeline
        ),
        cmd!(
            "export.frame",
            "Download Frame…",
            [],
            None,
            "{path, t: seconds since the page appeared, page?, scale?: 0.1–8 = 1} — one moment of the page's animation as PNG",
            has_doc,
            export_frame
        ),
        cmd!(
            "export.gif",
            "Download GIF (animated)…",
            ["File", "Download"],
            None,
            "{path, page?: index (default current) | all_pages?: bool | pages?: [indexes], fps?: 5–30 = 15, scale?: 0.1–4 = 1 (frames capped at 2 MP), loop?: bool = true, background?: colour around pages of another size = #000000} — at most 600 frames",
            has_doc,
            export_gif
        ),
        cmd!(
            "export.apng",
            "Download Animated PNG…",
            ["File", "Download"],
            None,
            "{path, page? | all_pages? | pages?, fps?: 5–30 = 15, scale?: 0.1–4 = 1, loop?: bool = true, background?: colour around pages of another size = #000000} — same frames as export.gif, full colour and transparency-free",
            has_doc,
            export_apng
        ),
        cmd!(
            "export.mp4",
            "Download MP4 Video…",
            ["File", "Download"],
            None,
            "{path, pages?: [indexes] | page?: index | all_pages?: bool (default: every page, in order), fps?: 1–60 = 30, quality?: low|medium|high = medium, scale?: 0.1–4 = 1 (frames capped at 1920 × 1080 pixels' worth), background?: colour around pages of another size = #000000} — H.264 video, each page for its duration, at most 3600 frames",
            has_doc,
            export_mp4
        ),
    ]
}

// ---------- editing ----------

/// `"none"` (or null) → `Ok(None)`; a known kind → `Ok(Some(kind))`.
fn kind_param(p: &Value, key: &str, cmd: &str) -> Result<Option<Option<AnimationKind>>> {
    match p.get(key) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) if s.eq_ignore_ascii_case("none") => Ok(Some(None)),
        Some(Value::String(s)) => {
            motion::parse_kind(s).map(|k| Some(Some(k))).ok_or_else(|| bad(cmd, format!("unknown animation `{s}` (see animation.list)")))
        }
        Some(_) => Err(bad(cmd, format!("`{key}` must be an animation name like \"fade\""))),
    }
}

fn loop_param(p: &Value, cmd: &str) -> Result<Option<Option<LoopKind>>> {
    match p.get("loop") {
        None => Ok(None),
        Some(Value::Null) | Some(Value::Bool(false)) => Ok(Some(None)),
        Some(Value::String(s)) if s.eq_ignore_ascii_case("none") => Ok(Some(None)),
        Some(Value::String(s)) => {
            motion::parse_loop(s).map(|k| Some(Some(k))).ok_or_else(|| bad(cmd, format!("unknown loop `{s}` (see animation.list)")))
        }
        Some(_) => Err(bad(cmd, "`loop` must be pulse, float, sway, blink or none")),
    }
}

/// A number of seconds in `0..=MAX_ANIMATION_SECONDS`, when given.
fn seconds(p: &Value, key: &str, cmd: &str) -> Result<Option<f64>> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_f64()
            .filter(|v| v.is_finite() && *v >= 0.0)
            .map(|v| Some(v.min(motion::MAX_ANIMATION_SECONDS)))
            .ok_or_else(|| bad(cmd, format!("`{key}` must be a number of seconds (0–{})", motion::MAX_ANIMATION_SECONDS))),
    }
}

/// The changes an `animation.set`-style call asks for.
struct Change {
    kind: Option<Option<AnimationKind>>,
    duration: Option<f64>,
    delay: Option<f64>,
    looping: Option<Option<LoopKind>>,
    exit: Option<Option<AnimationKind>>,
}

impl Change {
    fn read(p: &Value, cmd: &str) -> Result<Change> {
        Ok(Change {
            kind: kind_param(p, "kind", cmd)?,
            duration: seconds(p, "duration", cmd)?,
            delay: seconds(p, "delay", cmd)?,
            looping: loop_param(p, cmd)?,
            exit: kind_param(p, "exit", cmd)?,
        })
    }

    /// `current` with the change applied; `Ok(None)` removes the animation.
    fn apply(&self, current: Option<&Animation>, cmd: &str) -> Result<Option<Animation>> {
        let kind = match (self.kind, current) {
            (Some(None), _) => return Ok(None),
            (Some(Some(k)), _) => k,
            (None, Some(a)) => a.kind,
            (None, None) => return Err(bad(cmd, "missing `kind` (the object is not animated yet)")),
        };
        let mut a = current.cloned().unwrap_or_else(|| Animation::new(kind, motion::DEFAULT_DURATION, 0.0));
        a.kind = kind;
        if let Some(d) = self.duration {
            a.duration = d;
        }
        if let Some(d) = self.delay {
            a.delay = d;
        }
        if let Some(l) = self.looping {
            a.looping = l;
        }
        if let Some(e) = self.exit {
            a.exit = e;
        }
        Ok(Some(a))
    }
}

fn set(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "animation.set";
    let ids = targets(s, p, cmd)?;
    let change = Change::read(p, cmd)?;
    let d = s.doc()?;
    let mut planned: Vec<(NodeId, Option<Animation>)> = vec![];
    for id in &ids {
        let el = d.doc.element(*id)?;
        planned.push((*id, change.apply(el.animation.as_ref(), cmd)?));
    }
    let label = if planned.iter().all(|(_, a)| a.is_none()) { "Remove Animation" } else { "Animate" };
    s.edit(label, |doc, _| {
        for (id, a) in planned {
            doc.element_mut(id)?.animation = a;
        }
        Ok(())
    })?;
    Ok(ok(json!({"ids": ids.iter().map(|i| i.0).collect::<Vec<_>>()})))
}

fn remove(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = targets(s, p, "animation.remove")?;
    s.edit("Remove Animation", |doc, _| {
        for id in &ids {
            doc.element_mut(*id)?.animation = None;
        }
        Ok(())
    })?;
    Ok(ok(json!({"ids": ids.iter().map(|i| i.0).collect::<Vec<_>>()})))
}

fn page_index(s: &Session, p: &Value, cmd: &str) -> Result<usize> {
    let d = s.doc()?;
    let i = usize_param(p, "page").unwrap_or(d.page);
    if i >= d.doc.pages.len() {
        return Err(bad(cmd, format!("no page {i} (the design has {})", d.doc.pages.len())));
    }
    Ok(i)
}

/// Whether `el` covers (nearly) the whole page: a background, left still by "animate all".
fn is_background(page: &Page, frame: &genha_doc::Frame) -> bool {
    let b = frame.bounds();
    let area = page.width * page.height;
    let covered = (b.x1.min(page.width) - b.x0.max(0.0)).max(0.0) * (b.y1.min(page.height) - b.y0.max(0.0)).max(0.0);
    area > 0.0 && covered >= 0.95 * area
}

fn apply_to_page(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "animation.applyToPage";
    let index = page_index(s, p, cmd)?;
    let change = Change::read(p, cmd)?;
    if change.kind.is_none() {
        return Err(bad(cmd, "missing `kind` (an animation name, or none)"));
    }
    let stagger = seconds(p, "stagger", cmd)?.unwrap_or(DEFAULT_STAGGER);
    let start = change.delay.unwrap_or(0.0);
    let include_bg = bool_or(p, "include_background", false);
    let d = s.doc()?;
    let page = d.doc.page(index)?;
    let mut planned: Vec<(NodeId, Option<Animation>)> = vec![];
    let mut n = 0usize;
    for el in &page.elements {
        if el.hidden || (!include_bg && is_background(page, &el.frame)) {
            continue;
        }
        let mut a = change.apply(None, cmd)?;
        if let Some(a) = &mut a {
            a.delay = (start + stagger * n as f64).min(motion::MAX_ANIMATION_SECONDS);
        }
        planned.push((el.id, a));
        n += 1;
    }
    if planned.is_empty() {
        return Err(EngineError::Other("nothing to animate on this page".into()));
    }
    let count = planned.len();
    let label = if change.kind == Some(None) { "Remove Animations" } else { "Animate All" };
    s.edit(label, |doc, _| {
        let page = doc.page_mut(index)?;
        for (id, a) in planned {
            if let Some(el) = page.find_mut(id) {
                el.animation = a;
            }
        }
        Ok(())
    })?;
    let duration = motion::page_duration(s.doc()?.doc.page(index)?);
    Ok(ok(json!({"animated": count, "page_duration": duration})))
}

// ---------- queries ----------

fn list(_: &mut Session, _: &Value) -> Result<Value> {
    Ok(json!({
        "entrances": motion::ENTRANCES.iter().map(|(_, id, label)| json!({"id": id, "label": label})).collect::<Vec<_>>(),
        "loops": motion::LOOPS.iter().map(|(_, id, label)| json!({"id": id, "label": label})).collect::<Vec<_>>(),
        "defaults": {"duration": motion::DEFAULT_DURATION, "stagger": DEFAULT_STAGGER},
    }))
}

fn timeline(s: &mut Session, p: &Value) -> Result<Value> {
    let index = page_index(s, p, "animation.timeline")?;
    let page = s.doc()?.doc.page(index)?;
    let total = motion::page_duration(page);
    let mut items = vec![];
    page.for_each(&mut |e| {
        if let Some(a) = &e.animation {
            let (delay, duration) = motion::timing(a);
            items.push(json!({
                "id": e.id.0,
                "name": e.display_name(),
                "kind": motion::kind_id(a.kind),
                "start": delay,
                "end": delay + duration,
                "loop": a.looping.map(motion::loop_id),
                "exit": a.exit.map(motion::kind_id),
                "exit_start": a.exit.map(|_| (delay + duration).max(total - duration)),
            }));
        }
    });
    Ok(json!({"page": index, "duration": total, "explicit_duration": page.duration, "animations": items}))
}

// ---------- exports ----------

/// The design with only `page` in it, for rendering one moment.
fn single_page_doc(doc: &Document, page: Page) -> Document {
    let mut d = doc.clone();
    d.pages = vec![page];
    d
}

fn export_frame(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "export.frame";
    let path = str_req(p, "path", cmd)?.to_string();
    let t = p.get("t").and_then(Value::as_f64).filter(|v| v.is_finite()).ok_or_else(|| bad(cmd, "missing number `t` (seconds)"))?;
    let index = page_index(s, p, cmd)?;
    let scale = f64_or(p, "scale", 1.0).clamp(0.1, 8.0);
    let d = s.doc()?;
    let posed = motion::page_at(d.doc.page(index)?, t);
    let doc = single_page_doc(&d.doc, posed);
    let opts = genha_render::RenderOptions { scale, transparent_background: false };
    let r = genha_render::render_page(&doc, 0, &s.fonts, &opts).map_err(|e| EngineError::Other(e.to_string()))?;
    let png = genha_render::encode_png(&r).map_err(|e| EngineError::Other(e.to_string()))?;
    super::export::write(&path, &png)?;
    Ok(ok(json!({"files": [{"path": path, "bytes": png.len()}], "width": r.width, "height": r.height})))
}

/// An animated file format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Animated {
    Gif,
    Apng,
}

/// One frame of an animated export: a moment of a page, shown from `start` for `length` seconds.
#[derive(Clone, Debug)]
struct Shot {
    page: usize,
    t: f64,
    start: f64,
    length: f64,
    /// Video frames it lasts (constant frame rate; set by the video export).
    ticks: usize,
}

/// The frames to render: each page's timeline at `fps`, at most `budget` moments in all, frames
/// that would look the same as the previous one merged into it (judged on the posed model, before
/// rendering). Returns the shots and whether the budget cut the export short.
fn plan(doc: &Document, pages: &[usize], fps: f64, budget: usize) -> Result<(Vec<Shot>, bool)> {
    let mut shots: Vec<Shot> = vec![];
    let mut offset = 0.0;
    let mut budget = budget;
    let mut truncated = false;
    let mut previous: Option<(usize, Page)> = None;
    for &index in pages {
        if budget == 0 {
            truncated = true;
            break;
        }
        let page = doc.page(index)?;
        let duration = motion::page_duration(page);
        let times = motion::frame_times(page, fps, budget);
        if (times.len() as f64) < (duration * fps).ceil() {
            truncated = true;
        }
        budget = budget.saturating_sub(times.len());
        for t in times {
            let posed = motion::page_at(page, t);
            if previous.as_ref().is_some_and(|(i, p)| *i == index && *p == posed) {
                continue;
            }
            shots.push(Shot { page: index, t, start: offset + t, length: 0.0, ticks: 0 });
            previous = Some((index, posed));
        }
        offset += duration;
    }
    // Each frame lasts until the next one starts; the last until the end.
    let ends: Vec<f64> = shots.iter().skip(1).map(|s| s.start).chain(std::iter::once(offset)).collect();
    for (s, end) in shots.iter_mut().zip(ends) {
        s.length = (end - s.start).max(0.0);
    }
    Ok((shots, truncated))
}

/// Draws `src` centred in a `w` × `h` frame filled with `bars` (pages of another size in a
/// multi-page export).
fn letterbox(src: &genha_render::Raster, w: u32, h: u32, bars: [u8; 3]) -> Vec<u8> {
    let (w, h) = (w as usize, h as usize);
    let mut out = vec![0u8; w * h * 4];
    for px in out.as_chunks_mut::<4>().0 {
        *px = [bars[0], bars[1], bars[2], 255];
    }
    let (sw, sh) = (src.width as usize, src.height as usize);
    let (ox, oy) = (w.saturating_sub(sw) / 2, h.saturating_sub(sh) / 2);
    for y in 0..sh.min(h) {
        let row = src.rgba.get(y * sw * 4..(y + 1) * sw * 4);
        let dst = out.get_mut(((y + oy) * w + ox) * 4..((y + oy) * w + ox + sw.min(w)) * 4);
        if let (Some(row), Some(dst)) = (row, dst) {
            let n = dst.len().min(row.len());
            dst[..n].copy_from_slice(&row[..n]);
        }
    }
    out
}

/// Opaque RGBA (animated exports have no transparency): composited on white.
fn opaque(mut rgba: Vec<u8>) -> Vec<u8> {
    for px in rgba.as_chunks_mut::<4>().0 {
        let a = u16::from(px[3]);
        if a < 255 {
            for c in &mut px[..3] {
                *c = ((u16::from(*c) * a + 255 * (255 - a) + 127) / 255) as u8;
            }
            px[3] = 255;
        }
    }
    rgba
}

/// How animated exports draw their frames.
struct Frames<'a> {
    doc: &'a Document,
    fonts: &'a genha_text::FontBook,
    scale: f64,
    size: (u32, u32),
    /// Colour around pages of another size.
    bars: [u8; 3],
}

impl Frames<'_> {
    /// Renders the frames `shots` describe, handing each (RGBA, `size`) to `sink`.
    fn render(&self, shots: &[Shot], sink: &mut dyn FnMut(&[u8], &Shot) -> Result<()>) -> Result<()> {
        let mut renderer = genha_render::Renderer::new();
        let mut doc = single_page_doc(self.doc, Page::new(NodeId(0), "", 1.0, 1.0));
        let (w, h) = self.size;
        for shot in shots {
            let page = self.doc.page(shot.page)?;
            // Pages of another size are fitted into the first page's frame.
            let fit = (f64::from(w) / (page.width * self.scale)).min(f64::from(h) / (page.height * self.scale)).min(1.0);
            let fit = if fit.is_finite() && fit > 0.0 { fit } else { 1.0 };
            doc.pages = vec![motion::page_at(page, shot.t)];
            let opts = genha_render::RenderOptions { scale: self.scale * fit, transparent_background: false };
            let r = renderer.render_page(&doc, 0, self.fonts, &opts).map_err(|e| EngineError::Other(e.to_string()))?;
            let rgba = if r.width == w && r.height == h { opaque(r.rgba) } else { opaque(letterbox(&r, w, h, self.bars)) };
            sink(&rgba, shot)?;
        }
        Ok(())
    }
}

/// The pages an export covers: `pages`, `all_pages` or `page`; without any of them the current
/// page, or every page when `all_by_default`.
fn export_pages(doc: &Document, current: usize, p: &Value, cmd: &str, all_by_default: bool) -> Result<Vec<usize>> {
    let n = doc.pages.len();
    let all = match p.get("all_pages").and_then(Value::as_bool) {
        Some(all) => all,
        None => all_by_default && p.get("page").is_none(),
    };
    let pages: Vec<usize> = if let Some(Value::Array(a)) = p.get("pages") {
        a.iter()
            .map(|v| v.as_u64().and_then(|v| usize::try_from(v).ok()).filter(|i| *i < n).ok_or_else(|| bad(cmd, "`pages` must be page indexes")))
            .collect::<Result<_>>()?
    } else if all {
        (0..n).collect()
    } else {
        let i = usize_param(p, "page").unwrap_or(current);
        if i >= n {
            return Err(bad(cmd, format!("no page {i} (the design has {n})")));
        }
        vec![i]
    };
    if pages.is_empty() {
        return Err(bad(cmd, "no pages to export"));
    }
    Ok(pages)
}

/// The colour of the bars around pages of another size (`background`, default black).
fn bars_param(p: &Value, cmd: &str) -> Result<[u8; 3]> {
    Ok(color_param(p, "background", cmd)?.map_or([0, 0, 0], |c| [c.r, c.g, c.b]))
}

/// The export scale: `scale` lowered until a frame of the first page fits `max_pixels` (and, for
/// video, the encoder's limits). Returns the scale and the frame size.
fn frame_size(first: &Page, p: &Value, max_scale: f64, max_pixels: f64, video: bool) -> (f64, (u32, u32)) {
    let mut scale = f64_or(p, "scale", 1.0).clamp(0.1, max_scale);
    let (pw, ph) =
        (if first.width.is_finite() { first.width.max(1.0) } else { 1.0 }, if first.height.is_finite() { first.height.max(1.0) } else { 1.0 });
    let pixels = pw * ph * scale * scale;
    if pixels > max_pixels {
        scale *= (max_pixels / pixels).sqrt();
    }
    let longest = pw.max(ph) * scale;
    if video && longest > f64::from(genha_video::MAX_SIDE) {
        scale *= f64::from(genha_video::MAX_SIDE) / longest;
    }
    let side = |v: f64, scale: f64| {
        let v = (v * scale).round();
        if v.is_finite() { (v as u32).clamp(if video { 2 } else { 1 }, u32::from(u16::MAX)) } else { 2 }
    };
    let mut size = (side(pw, scale), side(ph, scale));
    if video {
        // Whole macroblocks of the (even) frame must fit the encoder's limit.
        let mbs = |(w, h): (u32, u32)| u64::from((w + (w & 1)).div_ceil(16)) * u64::from((h + (h & 1)).div_ceil(16));
        let mut tries = 0;
        while mbs(size) > u64::from(genha_video::MAX_MACROBLOCKS) && tries < 200 {
            scale *= 0.99;
            size = (side(pw, scale), side(ph, scale));
            tries += 1;
        }
    }
    (scale, size)
}

/// An animated export as file bytes; returns the bytes and a summary.
pub fn animated_bytes(s: &Session, p: &Value, format: Animated) -> Result<(Vec<u8>, Value)> {
    let cmd = match format {
        Animated::Gif => "export.gif",
        Animated::Apng => "export.apng",
    };
    let d = s.doc()?;
    let pages = export_pages(&d.doc, d.page, p, cmd, false)?;
    let fps = f64_or(p, "fps", 15.0).clamp(5.0, 30.0);
    let looping = bool_or(p, "loop", true);
    let bars = bars_param(p, cmd)?;
    let first = d.doc.page(pages.first().copied().unwrap_or(0))?;
    let (scale, size) = frame_size(first, p, 4.0, MAX_FRAME_PIXELS, false);
    let (shots, truncated) = plan(&d.doc, &pages, fps, MAX_FRAMES)?;
    let frames = Frames { doc: &d.doc, fonts: &s.fonts, scale, size, bars };
    let bytes = match format {
        Animated::Gif => encode_gif(&frames, &shots, looping)?,
        Animated::Apng => encode_apng(&frames, &shots, looping)?,
    };
    let duration: f64 = shots.iter().map(|s| s.length).sum();
    let summary = json!({"frames": shots.len(), "width": size.0, "height": size.1, "duration": duration, "fps": fps, "truncated": truncated});
    Ok((bytes, summary))
}

fn encode_gif(frames: &Frames, shots: &[Shot], looping: bool) -> Result<Vec<u8>> {
    use image::codecs::gif::{GifEncoder, Repeat};
    let err = |e: image::ImageError| EngineError::Other(format!("could not encode the GIF: {e}"));
    let (w, h) = frames.size;
    let mut out = vec![];
    {
        // Speed 10 (of 1–30): good colours at a fraction of the slowest setting's time.
        let mut enc = GifEncoder::new_with_speed(&mut out, 10);
        enc.set_repeat(if looping { Repeat::Infinite } else { Repeat::Finite(0) }).map_err(err)?;
        frames.render(shots, &mut |rgba, shot| {
            let img =
                image::RgbaImage::from_raw(w, h, rgba.to_vec()).ok_or_else(|| EngineError::Other("frame data does not match its size".into()))?;
            // GIF delays are whole centiseconds; rounding the start and end keeps the total right.
            let cs = ((shot.start + shot.length) * 100.0).round() - (shot.start * 100.0).round();
            let ms = if cs.is_finite() { (cs.clamp(1.0, 65_535.0) as u32) * 10 } else { 100 };
            let frame = image::Frame::from_parts(img, 0, 0, image::Delay::from_numer_denom_ms(ms, 1));
            enc.encode_frame(frame).map_err(err)
        })?;
    }
    Ok(out)
}

fn encode_apng(frames: &Frames, shots: &[Shot], looping: bool) -> Result<Vec<u8>> {
    let err = |e: png::EncodingError| EngineError::Other(format!("could not encode the animated PNG: {e}"));
    let count = u32::try_from(shots.len()).map_err(|_| EngineError::Other("too many frames".into()))?;
    if count == 0 {
        return Err(EngineError::Other("nothing to export".into()));
    }
    let mut out = vec![];
    {
        let mut enc = png::Encoder::new(&mut out, frames.size.0, frames.size.1);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_animated(count, if looping { 0 } else { 1 }).map_err(err)?;
        let mut writer = enc.write_header().map_err(err)?;
        frames.render(shots, &mut |rgba, shot| {
            let ms = (shot.length * 1000.0).round();
            let ms = if ms.is_finite() { ms.clamp(1.0, 65_535.0) as u16 } else { 100 };
            writer.set_frame_delay(ms, 1000).map_err(err)?;
            writer.write_image_data(rgba).map_err(err)
        })?;
        writer.finish().map_err(err)?;
    }
    Ok(out)
}

fn export_animated(s: &mut Session, p: &Value, format: Animated) -> Result<Value> {
    let cmd = if format == Animated::Gif { "export.gif" } else { "export.apng" };
    let path = str_req(p, "path", cmd)?.to_string();
    let (bytes, mut summary) = animated_bytes(s, p, format)?;
    super::export::write(&path, &bytes)?;
    if let Some(o) = summary.as_object_mut() {
        o.insert("files".into(), json!([{"path": path, "bytes": bytes.len()}]));
    }
    Ok(ok(summary))
}

fn export_gif(s: &mut Session, p: &Value) -> Result<Value> {
    export_animated(s, p, Animated::Gif)
}

fn export_apng(s: &mut Session, p: &Value) -> Result<Value> {
    export_animated(s, p, Animated::Apng)
}

// ---------- video ----------

/// How far a video export has come: frames rendered and to render. Returning `false` cancels it.
pub type Progress<'a> = &'a mut dyn FnMut(usize, usize) -> bool;

/// An MP4 video of the design as file bytes, and a summary. Takes the design itself (not the
/// session) so a host can run it on another thread; `current` is the page used when `p` names
/// none. Every page plays for its duration at a constant frame rate; frames that look like the
/// previous one are not rendered again (they cost the video a few bytes).
pub fn video_bytes(doc: &Document, fonts: &genha_text::FontBook, current: usize, p: &Value, progress: Progress) -> Result<(Vec<u8>, Value)> {
    let cmd = "export.mp4";
    let pages = export_pages(doc, current, p, cmd, true)?;
    let fps = match p.get("fps") {
        None | Some(Value::Null) => DEFAULT_VIDEO_FPS,
        Some(v) => v
            .as_f64()
            .filter(|f| f.is_finite())
            .map(|f| f.round().clamp(1.0, f64::from(genha_video::MAX_FPS)) as u32)
            .ok_or_else(|| bad(cmd, "`fps` must be a number of frames per second (1–60)"))?,
    };
    let quality = match p.get("quality") {
        None | Some(Value::Null) => genha_video::Quality::Medium,
        Some(Value::String(q)) => genha_video::Quality::parse(q).ok_or_else(|| bad(cmd, format!("unknown quality `{q}` (low, medium or high)")))?,
        Some(_) => return Err(bad(cmd, "`quality` must be low, medium or high")),
    };
    let bars = bars_param(p, cmd)?;
    let first = doc.page(pages.first().copied().unwrap_or(0))?;
    let (scale, size) = frame_size(first, p, 4.0, MAX_VIDEO_PIXELS, true);
    let rate = f64::from(fps);
    let (shots, truncated) = plan(doc, &pages, rate, MAX_VIDEO_FRAMES)?;
    let err = |e: genha_video::VideoError| EngineError::Other(format!("could not encode the video: {e}"));
    let mut opts = genha_video::VideoOptions::new(size.0, size.1, fps);
    opts.quality = quality;
    let mut video = genha_video::Mp4Writer::new(opts).map_err(err)?;
    let frames = Frames { doc, fonts, scale, size, bars };
    // Constant frame rate: each shot covers the frame ticks from its start to its end.
    let tick = |seconds: f64| {
        let t = (seconds * rate).round();
        if t.is_finite() { t.max(0.0) as usize } else { 0 }
    };
    let mut shots = shots;
    for s in &mut shots {
        s.ticks = tick(s.start + s.length).saturating_sub(tick(s.start));
    }
    // A video shorter than a frame still has one.
    if shots.iter().all(|s| s.ticks == 0)
        && let Some(first) = shots.first_mut()
    {
        first.ticks = 1;
    }
    let visible: Vec<Shot> = shots.into_iter().filter(|s| s.ticks > 0).collect();
    let total = visible.len();
    let mut done = 0usize;
    let mut last_page = None;
    if !progress(0, total) {
        return Err(EngineError::Other("the video export was cancelled".into()));
    }
    frames.render(&visible, &mut |rgba, shot| {
        // A key frame where a new page starts (players can seek there).
        let new_page = last_page != Some(shot.page);
        last_page = Some(shot.page);
        video.push_rgba(rgba, new_page).map_err(err)?;
        for _ in 1..shot.ticks {
            video.repeat().map_err(err)?;
        }
        done += 1;
        if progress(done, total) { Ok(()) } else { Err(EngineError::Other("the video export was cancelled".into())) }
    })?;
    let count = video.frames();
    let bytes = video.finish().map_err(err)?;
    let (w, h) = opts.output_size();
    let summary = json!({
        "frames": count,
        "rendered": total,
        "width": w,
        "height": h,
        "duration": count as f64 / rate,
        "fps": fps,
        "quality": quality.id(),
        "truncated": truncated,
    });
    Ok((bytes, summary))
}

/// The work of `export.mp4`: writes the video `p` describes to `p.path` and returns the command's
/// result. It needs the design, not the session, so the desktop app runs it on a worker thread
/// (with `progress` reporting and cancelling).
pub fn video_file(doc: &Document, fonts: &genha_text::FontBook, current: usize, p: &Value, progress: Progress) -> Result<Value> {
    let path = str_req(p, "path", "export.mp4")?.to_string();
    let (bytes, mut summary) = video_bytes(doc, fonts, current, p, progress)?;
    super::export::write(&path, &bytes)?;
    if let Some(o) = summary.as_object_mut() {
        o.insert("files".into(), json!([{"path": path, "bytes": bytes.len()}]));
    }
    Ok(ok(summary))
}

fn export_mp4(s: &mut Session, p: &Value) -> Result<Value> {
    let d = s.doc()?;
    video_file(&d.doc, &s.fonts, d.page, p, &mut |_, _| true)
}

#[cfg(test)]
mod tests;
