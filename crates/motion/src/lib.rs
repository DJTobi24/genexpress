//! Motion for GenHa (layer L2): element animations evaluated at a point in time.
//!
//! An element's [`Animation`] has an entrance (`kind`, `duration`, `delay`), an optional loop
//! played while it is on screen and an optional exit played at the end of the page. [`page_at`]
//! gives the page as it looks `t` seconds after it appears: every animated element moved, scaled,
//! turned, faded or (typewriter) cut short. Renderers draw that page like any other, so playback,
//! present mode and animated exports share one definition of what an animation looks like.
//!
//! Everything here is pure and total: non-finite or negative times and durations are clamped,
//! never trusted.
#![forbid(unsafe_code)]

use genha_doc::{Animation, AnimationKind, Element, ElementKind, LoopKind, Page, TextEffect};
use genha_geom::{Affine, Frame, Point};

/// Entrance length used when a duration is missing or not a number, in seconds.
pub const DEFAULT_DURATION: f64 = 0.6;
/// Longest entrance (or exit) and longest delay, in seconds.
pub const MAX_ANIMATION_SECONDS: f64 = 20.0;
/// How long a page stays still after its last entrance (derived durations), in seconds.
pub const HOLD: f64 = 1.5;
/// Shortest derived page duration, in seconds.
pub const MIN_PAGE_DURATION: f64 = 2.0;
/// Longest page duration, in seconds.
pub const MAX_PAGE_DURATION: f64 = 60.0;
/// Length of one loop cycle (pulse, float, sway, blink), in seconds.
pub const LOOP_PERIOD: f64 = 1.6;
/// How far slides start from the final position, as a fraction of the page size.
pub const SLIDE_DISTANCE: f64 = 0.2;

/// Groups nested deeper than this are drawn without animating their insides (untrusted files).
const MAX_DEPTH: usize = 64;

/// Entrances (and exits): kind, id (as in files and commands), label.
pub const ENTRANCES: &[(AnimationKind, &str, &str)] = &[
    (AnimationKind::Fade, "fade", "Fade"),
    (AnimationKind::Rise, "rise", "Rise"),
    (AnimationKind::SlideLeft, "slide-left", "Slide left"),
    (AnimationKind::SlideRight, "slide-right", "Slide right"),
    (AnimationKind::SlideUp, "slide-up", "Slide up"),
    (AnimationKind::SlideDown, "slide-down", "Slide down"),
    (AnimationKind::Pop, "pop", "Pop"),
    (AnimationKind::Zoom, "zoom", "Zoom"),
    (AnimationKind::Bounce, "bounce", "Bounce"),
    (AnimationKind::Spin, "spin", "Spin"),
    (AnimationKind::Flip, "flip", "Flip"),
    (AnimationKind::Typewriter, "typewriter", "Typewriter"),
];

/// Loops: kind, id, label.
pub const LOOPS: &[(LoopKind, &str, &str)] = &[
    (LoopKind::Pulse, "pulse", "Pulse"),
    (LoopKind::Float, "float", "Float"),
    (LoopKind::Sway, "sway", "Sway"),
    (LoopKind::Blink, "blink", "Blink"),
];

/// The id of an entrance kind (`"slide-left"`).
pub fn kind_id(kind: AnimationKind) -> &'static str {
    ENTRANCES.iter().find(|e| e.0 == kind).map(|e| e.1).unwrap_or("fade")
}

/// The label of an entrance kind (`"Slide left"`).
pub fn kind_label(kind: AnimationKind) -> &'static str {
    ENTRANCES.iter().find(|e| e.0 == kind).map(|e| e.2).unwrap_or("Fade")
}

/// The entrance kind with id `id` (case-insensitive; `_` and spaces read as `-`).
pub fn parse_kind(id: &str) -> Option<AnimationKind> {
    let id = normalize(id);
    ENTRANCES.iter().find(|e| e.1 == id).map(|e| e.0)
}

pub fn loop_id(kind: LoopKind) -> &'static str {
    LOOPS.iter().find(|e| e.0 == kind).map(|e| e.1).unwrap_or("pulse")
}

pub fn loop_label(kind: LoopKind) -> &'static str {
    LOOPS.iter().find(|e| e.0 == kind).map(|e| e.2).unwrap_or("Pulse")
}

pub fn parse_loop(id: &str) -> Option<LoopKind> {
    let id = normalize(id);
    LOOPS.iter().find(|e| e.1 == id).map(|e| e.0)
}

fn normalize(id: &str) -> String {
    id.trim().to_ascii_lowercase().replace(['_', ' '], "-")
}

/// `(delay, duration)` of `a` in seconds, made safe: non-finite or negative values become
/// defaults or zero, and both are capped at [`MAX_ANIMATION_SECONDS`].
pub fn timing(a: &Animation) -> (f64, f64) {
    let delay = if a.delay.is_finite() { a.delay.clamp(0.0, MAX_ANIMATION_SECONDS) } else { 0.0 };
    let duration = if a.duration.is_finite() { a.duration.clamp(0.0, MAX_ANIMATION_SECONDS) } else { DEFAULT_DURATION };
    (delay, duration)
}

/// Whether anything on the page (groups included) is animated.
pub fn is_animated(page: &Page) -> bool {
    let mut any = false;
    page.for_each(&mut |e| any |= e.animation.is_some());
    any
}

/// How long the page plays, in seconds: its own `duration` when set, else until the last
/// entrance (and exit) ends plus [`HOLD`] (longer when something loops), at least
/// [`MIN_PAGE_DURATION`]. Always finite, in `0.1..=MAX_PAGE_DURATION`.
pub fn page_duration(page: &Page) -> f64 {
    if let Some(d) = page.duration
        && d.is_finite()
        && d > 0.0
    {
        return d.clamp(0.1, MAX_PAGE_DURATION);
    }
    let (mut end, mut loops) = (0.0f64, false);
    page.for_each(&mut |e| {
        if let Some(a) = &e.animation {
            let (delay, duration) = timing(a);
            let exit = if a.exit.is_some() { duration } else { 0.0 };
            end = end.max(delay + duration + exit);
            loops |= a.looping.is_some();
        }
    });
    let hold = if loops { HOLD.max(2.0 * LOOP_PERIOD) } else { HOLD };
    (end + hold).clamp(MIN_PAGE_DURATION, MAX_PAGE_DURATION)
}

/// Frame times (seconds) for playing the page at `fps` frames per second: `0, 1/fps, …` up to
/// (not including) the page duration, at most `max_frames` of them (at least one).
pub fn frame_times(page: &Page, fps: f64, max_frames: usize) -> Vec<f64> {
    let fps = if fps.is_finite() { fps.clamp(1.0, 120.0) } else { 15.0 };
    let n = (page_duration(page) * fps).ceil().max(1.0);
    let n = if n.is_finite() { (n as usize).clamp(1, max_frames.max(1)) } else { 1 };
    (0..n).map(|i| i as f64 / fps).collect()
}

/// The page as it looks `t` seconds after it appears. At `t = 0` entrances have not started (their
/// elements are hidden or out of place); once every entrance and loop has finished (from
/// [`page_duration`] on, when nothing exits) the result equals `page`. Elements in a group move
/// with the group (and play their own animations too).
pub fn page_at(page: &Page, t: f64) -> Page {
    let mut out = page.clone();
    if !is_animated(page) {
        return out;
    }
    let t = if t.is_nan() { 0.0 } else { t.max(0.0) };
    let cx = Cx { width: sane_side(page.width), height: sane_side(page.height), duration: page_duration(page) };
    for el in &mut out.elements {
        animate(el, t, &cx, 0);
    }
    out
}

fn sane_side(v: f64) -> f64 {
    if v.is_finite() { v.clamp(0.0, 1.0e6) } else { 0.0 }
}

struct Cx {
    width: f64,
    height: f64,
    duration: f64,
}

/// What an animation does to an element at one moment.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pose {
    dx: f64,
    dy: f64,
    sx: f64,
    sy: f64,
    /// Degrees, clockwise.
    rotation: f64,
    opacity: f64,
    /// Fraction of a text's characters shown (typewriter).
    reveal: Option<f64>,
}

impl Pose {
    const STILL: Pose = Pose { dx: 0.0, dy: 0.0, sx: 1.0, sy: 1.0, rotation: 0.0, opacity: 1.0, reveal: None };

    fn moves(&self) -> bool {
        self.dx != 0.0 || self.dy != 0.0 || self.sx != 1.0 || self.sy != 1.0 || self.rotation != 0.0
    }
}

fn animate(el: &mut Element, t: f64, cx: &Cx, depth: usize) {
    if depth > MAX_DEPTH {
        return;
    }
    // Children play their own animations first; the group's then carries them along.
    if let ElementKind::Group(g) = &mut el.kind {
        for c in &mut g.children {
            animate(c, t, cx, depth + 1);
        }
    }
    let Some(a) = el.animation.clone() else { return };
    if let Some(pose) = pose_at(&a, t, cx) {
        apply(el, &pose);
    }
}

/// `(t - start) / duration` in `0..=1`; a zero duration jumps at `start`.
fn progress(t: f64, start: f64, duration: f64) -> f64 {
    if duration <= 1e-9 {
        return if t >= start { 1.0 } else { 0.0 };
    }
    ((t - start) / duration).clamp(0.0, 1.0)
}

/// The pose of an element with animation `a` at `t`, or None when it is at rest.
fn pose_at(a: &Animation, t: f64, cx: &Cx) -> Option<Pose> {
    let (delay, duration) = timing(a);
    let in_end = delay + duration;
    let exit = a.exit.map(|kind| (kind, in_end.max(cx.duration - duration)));
    if let Some((kind, exit_start)) = exit
        && t >= exit_start
    {
        let mut pose = entrance(kind, 1.0 - progress(t, exit_start, duration), cx);
        // Leaving continues the movement rather than reversing it (slide left exits to the left).
        pose.dx = -pose.dx;
        pose.dy = -pose.dy;
        pose.rotation = -pose.rotation;
        return Some(pose);
    }
    if t < in_end {
        return Some(entrance(a.kind, progress(t, delay, duration), cx));
    }
    let kind = a.looping?;
    // Whole cycles only, so the element is at rest when the loop ends.
    let loop_end = exit.map_or(cx.duration, |(_, start)| start.min(cx.duration));
    let cycles = ((loop_end - in_end) / LOOP_PERIOD).floor();
    if !cycles.is_finite() || cycles < 1.0 || t >= in_end + cycles * LOOP_PERIOD {
        return None;
    }
    Some(looping(kind, t - in_end, cx))
}

/// Eases out: fast start, gentle stop.
pub fn ease_out_cubic(p: f64) -> f64 {
    let q = 1.0 - p.clamp(0.0, 1.0);
    1.0 - q * q * q
}

/// Eases out past the target and settles back (about 10 % overshoot).
pub fn ease_out_back(p: f64) -> f64 {
    let q = p.clamp(0.0, 1.0) - 1.0;
    let s = 1.70158;
    1.0 + (s + 1.0) * q * q * q + s * q * q
}

/// A damped bounce: overshoots the target, swings back and settles; 0 at 0, exactly 1 at 1.
pub fn ease_bounce(p: f64) -> f64 {
    let p = p.clamp(0.0, 1.0);
    1.0 - (1.0 - p) * (-4.0 * p).exp() * (3.0 * std::f64::consts::PI * p).cos()
}

/// The entrance `kind` at progress `p` (0 = start, 1 = in place).
fn entrance(kind: AnimationKind, p: f64, cx: &Cx) -> Pose {
    let p = p.clamp(0.0, 1.0);
    let rest = 1.0 - ease_out_cubic(p);
    let quick = |share: f64| (p / share).min(1.0);
    let (w, h) = (cx.width, cx.height);
    let mut pose = Pose::STILL;
    match kind {
        AnimationKind::Fade => pose.opacity = 1.0 - rest,
        AnimationKind::SlideLeft => (pose.dx, pose.opacity) = (SLIDE_DISTANCE * w * rest, quick(0.35)),
        AnimationKind::SlideRight => (pose.dx, pose.opacity) = (-SLIDE_DISTANCE * w * rest, quick(0.35)),
        AnimationKind::SlideUp => (pose.dy, pose.opacity) = (SLIDE_DISTANCE * h * rest, quick(0.35)),
        AnimationKind::SlideDown => (pose.dy, pose.opacity) = (-SLIDE_DISTANCE * h * rest, quick(0.35)),
        AnimationKind::Rise => (pose.dy, pose.opacity) = (0.08 * h * rest, 1.0 - rest),
        AnimationKind::Pop => {
            let s = 1.0 - 0.4 * (1.0 - ease_out_back(p));
            (pose.sx, pose.sy, pose.opacity) = (s, s, quick(0.25));
        }
        AnimationKind::Zoom => {
            let s = 1.0 + 0.3 * rest;
            (pose.sx, pose.sy, pose.opacity) = (s, s, 1.0 - rest);
        }
        AnimationKind::Bounce => (pose.dy, pose.opacity) = (-0.2 * h * (1.0 - ease_bounce(p)), quick(0.15)),
        AnimationKind::Spin => {
            let s = 1.0 - 0.5 * rest;
            (pose.rotation, pose.sx, pose.sy, pose.opacity) = (-120.0 * rest, s, s, 1.0 - rest);
        }
        AnimationKind::Flip => (pose.sx, pose.opacity) = ((1.0 - rest).max(0.001), quick(0.2)),
        AnimationKind::Typewriter => pose.reveal = Some(p),
    }
    pose
}

/// The loop `kind`, `tau` seconds after it started (at rest on whole cycles).
fn looping(kind: LoopKind, tau: f64, cx: &Cx) -> Pose {
    let phase = std::f64::consts::TAU * tau / LOOP_PERIOD;
    let swing = (1.0 - phase.cos()) / 2.0;
    let mut pose = Pose::STILL;
    match kind {
        LoopKind::Pulse => (pose.sx, pose.sy) = (1.0 + 0.06 * swing, 1.0 + 0.06 * swing),
        LoopKind::Float => pose.dy = -0.02 * cx.width.min(cx.height) * swing,
        LoopKind::Sway => pose.rotation = 4.0 * phase.sin(),
        LoopKind::Blink => pose.opacity = 1.0 - 0.55 * swing,
    }
    pose
}

fn apply(el: &mut Element, pose: &Pose) {
    if pose.moves() {
        let c = el.frame.sanitized().center();
        let sx = if pose.sx.is_finite() { pose.sx.max(0.0) } else { 1.0 };
        let sy = if pose.sy.is_finite() { pose.sy.max(0.0) } else { 1.0 };
        let m = Affine::translate((pose.dx, pose.dy))
            * Affine::translate((c.x, c.y))
            * Affine::rotate(pose.rotation.to_radians())
            * Affine::scale_non_uniform(sx, sy)
            * Affine::translate((-c.x, -c.y));
        transform(el, m, sx, sy, pose.rotation, 0);
    }
    let base = if el.opacity.is_finite() { el.opacity } else { 1.0 };
    el.opacity = (base * pose.opacity.clamp(0.0, 1.0) as f32).clamp(0.0, 1.0);
    if let Some(r) = pose.reveal {
        reveal(el, r, 0);
    }
}

/// Moves `el` (and, for a group, everything in it) by `m`, scaling sizes by `sx` × `sy` and
/// turning by `rotation` degrees. Text keeps its proportions (it scales by the smaller factor) so
/// its lines break where they do at rest.
fn transform(el: &mut Element, m: Affine, sx: f64, sy: f64, rotation: f64, depth: usize) {
    let f = el.frame.sanitized();
    let u = sx.min(sy);
    let (w, h) = if matches!(el.kind, ElementKind::Text(_)) { (f.width * u, f.height * u) } else { (f.width * sx, f.height * sy) };
    let c: Point = m * f.center();
    el.frame = Frame { x: c.x - w / 2.0, y: c.y - h / 2.0, width: w, height: h, rotation: f.rotation + rotation };
    if u != 1.0 {
        scale_style(el, u);
    }
    if depth < MAX_DEPTH
        && let ElementKind::Group(g) = &mut el.kind
    {
        for child in &mut g.children {
            transform(child, m, sx, sy, rotation, depth + 1);
        }
    }
}

/// Scales what is measured in page pixels inside an element (font size, borders, shadows).
fn scale_style(el: &mut Element, u: f64) {
    if let ElementKind::Text(t) = &mut el.kind {
        t.style.font_size *= u;
        match &mut t.effect {
            TextEffect::Shadow { offset_x, offset_y, blur, .. } => {
                *offset_x *= u;
                *offset_y *= u;
                *blur *= u;
            }
            TextEffect::Outline { width, .. } => *width *= u,
            TextEffect::Highlight { padding, corner_radius, .. } => {
                *padding *= u;
                *corner_radius *= u;
            }
            TextEffect::None => {}
        }
    } else if let ElementKind::Shape(s) = &mut el.kind {
        if let Some(st) = &mut s.stroke {
            st.width *= u;
            st.dash.iter_mut().for_each(|d| *d *= u);
        }
        if let Some(sh) = &mut s.shadow {
            sh.offset_x *= u;
            sh.offset_y *= u;
            sh.blur *= u;
        }
        if let genha_doc::ShapeKind::Rect { corner_radius } = &mut s.shape {
            *corner_radius *= u;
        }
    } else if let ElementKind::Image(i) = &mut el.kind {
        i.corner_radius *= u;
        if let Some(b) = &mut i.border {
            b.width *= u;
        }
    }
}

/// Shows the first `fraction` of every text's characters (a group's texts included).
fn reveal(el: &mut Element, fraction: f64, depth: usize) {
    if let ElementKind::Text(t) = &mut el.kind {
        let n = t.text.chars().count();
        let shown = (n as f64 * fraction.clamp(0.0, 1.0)).floor();
        let shown = if shown.is_finite() { (shown as usize).min(n) } else { n };
        if shown < n {
            t.text = t.text.chars().take(shown).collect();
        }
    } else if let ElementKind::Group(g) = &mut el.kind
        && depth < MAX_DEPTH
    {
        for c in &mut g.children {
            reveal(c, fraction, depth + 1);
        }
    } else if fraction < 1.0 {
        // Nothing to type: fade in instead.
        el.opacity *= fraction.clamp(0.0, 1.0) as f32;
    }
}

#[cfg(test)]
mod tests;
