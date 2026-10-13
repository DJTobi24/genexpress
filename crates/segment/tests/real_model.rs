#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Runs the real models when they are on this machine; skipped otherwise (no model ships with
//! GenHa, and CI does not download one).
//!
//! Models are looked for in `GENHA_MODEL_DIR`, else in the app's data folder (`models/`, where
//! `model.download` puts them). With `GENHA_ML_OUT=<folder>` the cut-outs are also written there
//! as PNGs on a checkerboard, colour-based next to ML, for people to look at; with
//! `GENHA_ML_PHOTOS=<folder>` every JPEG/PNG in that folder is cut out too (local test photos;
//! never commit them).

use std::path::PathBuf;
use std::time::Instant;

use genha_segment::{MODELS, ModelSpec, Segmenter};
use image::{GrayImage, Rgba, RgbaImage};

fn model_dir() -> Option<PathBuf> {
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    if let Some(d) = env("GENHA_MODEL_DIR") {
        return Some(d);
    }
    let data = if cfg!(target_os = "windows") {
        env("APPDATA").map(|d| d.join("GenHa"))
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library/Application Support/GenHa"))
    } else {
        env("XDG_DATA_HOME").or_else(|| env("HOME").map(|h| h.join(".local/share"))).map(|d| d.join("genha"))
    };
    data.map(|d| d.join("models"))
}

fn load(spec: &'static ModelSpec) -> Option<Segmenter> {
    let path = model_dir()?.join(spec.file);
    let bytes = std::fs::read(&path).ok()?;
    let t = Instant::now();
    let s = Segmenter::load(spec, &bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    eprintln!("{}: loaded in {:.2?}", spec.name, t.elapsed());
    Some(s)
}

/// A person-like figure (head and shoulders) on a busy, colourful backdrop, and its true mask.
fn busy_scene() -> (RgbaImage, GrayImage) {
    let (w, h) = (640u32, 480u32);
    let inside = |x: f64, y: f64| {
        let head = (x - 320.0).hypot(y - 170.0) < 70.0;
        let neck = (295.0..345.0).contains(&x) && (220.0..270.0).contains(&y);
        // Shoulders: a wide rounded block to the bottom edge.
        let body = y > 260.0 && (x - 320.0).abs() < 170.0 - ((y - 260.0) * -0.3).max(-40.0) && {
            let cx = (x - 320.0).abs() - 120.0;
            let cy = 310.0 - y;
            !(cx > 0.0 && cy > 0.0 && cx.hypot(cy) > 50.0)
        };
        head || neck || body
    };
    let mut seed = 0x2545_f491_u32;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    // Backdrop: stripes, then many random coloured blocks and discs (a market stall).
    let mut img = RgbaImage::from_fn(w, h, |x, y| {
        let s = ((x / 23 + y / 41) % 3) as u8;
        Rgba([90 + s * 50, 60 + s * 30, 140 - s * 30, 255])
    });
    for _ in 0..140 {
        let (cx, cy, r) = (rnd() % w, rnd() % h, 8 + rnd() % 40);
        let c = Rgba([(rnd() % 256) as u8, (rnd() % 256) as u8, (rnd() % 256) as u8, 255]);
        let disc = rnd() % 2 == 0;
        for y in cy.saturating_sub(r)..(cy + r).min(h) {
            for x in cx.saturating_sub(r)..(cx + r).min(w) {
                let d = (f64::from(x) - f64::from(cx)).hypot(f64::from(y) - f64::from(cy));
                if !disc || d < f64::from(r) {
                    img.put_pixel(x, y, c);
                }
            }
        }
    }
    let truth = GrayImage::from_fn(w, h, |x, y| image::Luma([if inside(f64::from(x) + 0.5, f64::from(y) + 0.5) { 255 } else { 0 }]));
    for (x, y, p) in img.enumerate_pixels_mut() {
        if inside(f64::from(x) + 0.5, f64::from(y) + 0.5) {
            // Skin for the head and neck, a shaded shirt below; lit from the top left.
            let shade = 1.0 - (f64::from(x) - 250.0).hypot(f64::from(y) - 120.0) / 900.0;
            let base = if y < 255 { [222.0, 180.0, 150.0] } else { [40.0, 70.0, 160.0] };
            let [r, g, b] = base.map(|c: f64| (c * shade).clamp(0.0, 255.0) as u8);
            *p = Rgba([r, g, b, 255]);
        }
    }
    (img, truth)
}

/// Intersection over union of two masks (pixels over 50 %).
fn iou(a: &GrayImage, b: &GrayImage) -> f64 {
    let (mut inter, mut union) = (0u64, 0u64);
    for (p, q) in a.pixels().zip(b.pixels()) {
        let (x, y) = (p[0] > 127, q[0] > 127);
        inter += u64::from(x && y);
        union += u64::from(x || y);
    }
    inter as f64 / union.max(1) as f64
}

fn alpha(img: &RgbaImage) -> GrayImage {
    GrayImage::from_fn(img.width(), img.height(), |x, y| image::Luma([img.get_pixel(x, y)[3]]))
}

/// `cut` over a grey checkerboard (so transparency shows in a PNG viewer).
fn on_checkerboard(cut: &RgbaImage) -> RgbaImage {
    RgbaImage::from_fn(cut.width(), cut.height(), |x, y| {
        let bg = if (x / 16 + y / 16) % 2 == 0 { 200.0 } else { 150.0 };
        let p = cut.get_pixel(x, y);
        let a = f64::from(p[3]) / 255.0;
        let mix = |c: u8| (f64::from(c) * a + bg * (1.0 - a)).round() as u8;
        Rgba([mix(p[0]), mix(p[1]), mix(p[2]), 255])
    })
}

/// Side by side: original, colour-based, ML (each on a checkerboard).
fn side_by_side(parts: &[&RgbaImage]) -> RgbaImage {
    let (w, h) = (parts[0].width(), parts[0].height());
    let mut out = RgbaImage::from_pixel(w * parts.len() as u32 + 8 * (parts.len() as u32 - 1), h, Rgba([255, 255, 255, 255]));
    for (i, p) in parts.iter().enumerate() {
        image::imageops::overlay(&mut out, *p, i64::from(i as u32 * (w + 8)), 0);
    }
    out
}

fn out_dir() -> Option<PathBuf> {
    let d = std::env::var_os("GENHA_ML_OUT").filter(|v| !v.is_empty()).map(PathBuf::from)?;
    std::fs::create_dir_all(&d).ok()?;
    Some(d)
}

fn compare(name: &str, img: &RgbaImage, seg: &Segmenter) -> (GrayImage, GrayImage) {
    let colour = genha_effects::remove_background(img, 30.0);
    let t = Instant::now();
    let raw = seg.raw(img).unwrap();
    let model = t.elapsed();
    let mask = genha_segment::post::refine(img, &raw, seg.spec.input);
    eprintln!("{name} ({}×{}) with {}: {:.2?} (model {model:.2?})", img.width(), img.height(), seg.spec.name, t.elapsed());
    let ml = genha_effects::cut_out(img, &mask);
    if let Some(d) = out_dir() {
        let flat = on_checkerboard(img);
        side_by_side(&[&flat, &on_checkerboard(&colour), &on_checkerboard(&ml)]).save(d.join(format!("{name}-{}.png", seg.spec.name))).unwrap();
    }
    (alpha(&colour), mask)
}

#[test]
fn real_models_beat_the_colour_method_on_a_busy_scene() {
    let mut ran = 0;
    for spec in MODELS {
        let Some(seg) = load(spec) else {
            eprintln!("skipped: {} model not found (set GENHA_MODEL_DIR or run `genha-cli exec model.download`)", spec.name);
            continue;
        };
        ran += 1;
        let (img, truth) = busy_scene();
        let (colour, ml) = compare("busy-scene", &img, &seg);
        let (c, m) = (iou(&colour, &truth), iou(&ml, &truth));
        eprintln!("{}: IoU colour {c:.3}, ML {m:.3}", spec.name);
        // The figure's head is found and the backdrop is gone either way…
        assert!(ml.get_pixel(320, 170)[0] > 200, "{}: the head is kept", spec.name);
        assert!(ml.get_pixel(20, 20)[0] < 20 && ml.get_pixel(620, 100)[0] < 20, "{}: the backdrop goes", spec.name);
        assert!(m > c, "{}: ML {m:.3} vs colour {c:.3}", spec.name);
        // …and the default model takes the whole figure. (The large model, measured: only the
        // head; on this drawn scene that is also a fair answer to "what stands out".)
        if spec.name == genha_segment::DEFAULT_MODEL {
            assert!(m > 0.8, "{}: ML IoU {m:.3}", spec.name);
        }
        // The mask is soft only along the outline.
        let soft = ml.pixels().filter(|p| p[0] > 10 && p[0] < 245).count() as f64 / (ml.width() * ml.height()) as f64;
        assert!(soft < 0.08, "{soft}");

        // Local photos, when given: only written out for people to compare.
        if let Some(dir) = std::env::var_os("GENHA_ML_PHOTOS").filter(|v| !v.is_empty()) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let path = e.path();
                let ext = path.extension().and_then(|x| x.to_str()).unwrap_or("").to_ascii_lowercase();
                if !matches!(ext.as_str(), "jpg" | "jpeg" | "png") {
                    continue;
                }
                let Ok(img) = genha_effects::decode(&std::fs::read(&path).unwrap()) else {
                    eprintln!("skipped {}: not a picture", path.display());
                    continue;
                };
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("photo").to_string();
                compare(&stem, &img, &seg);
            }
        }
    }
    if ran == 0 {
        eprintln!("skipped: no segmentation model on this machine");
    }
}
