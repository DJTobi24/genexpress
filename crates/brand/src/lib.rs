//! GenHa's identity (layer L0): the app's name and id, and the app icon, drawn in code.
//!
//! The icon is original artwork for GenHa (MIT OR Apache-2.0): a rounded tile in the UI's accent
//! violets with two overlapping primitives, a warm circle and a white rounded square in front of
//! it, cut apart by a thin gap. Everything is computed from signed distances, so every size is
//! drawn from the shapes themselves (no scaled bitmaps) and no image file is needed: the desktop
//! app sets its window icon from [`rgba`], `cargo xtask icon` / `cargo xtask bundle` write the
//! PNG, `.ico` and `.icns` files the installers need, and the Windows build scripts embed
//! [`ico`] into the executables ([`windows_resource`]).
#![forbid(unsafe_code)]

use std::fmt;
use std::path::{Path, PathBuf};

/// The product name shown to people.
pub const APP_NAME: &str = "GenHa";

/// The reverse-DNS application id: the macOS bundle identifier, the Wayland app id and the name of
/// the Linux `.desktop` file. Change it here (and only here) to publish under your own id.
pub const APP_ID: &str = "eu.tobicloud.genha";

/// File name extension of GenHa designs (the same as `genha_format::EXTENSION`).
pub const EXTENSION: &str = "genha";

/// Extension of designs saved before the rename (the same as `genha_format::LEGACY_EXTENSION`).
pub const LEGACY_EXTENSION: &str = "genexpress";

/// MIME type of GenHa designs (Linux shared-mime-info).
pub const MIME_TYPE: &str = "application/x-genha";

/// The macOS uniform type identifier of GenHa designs (exported by the app bundle).
pub fn document_uti() -> String {
    format!("{APP_ID}.document")
}

/// The macOS uniform type identifier of designs with the old `.genexpress` extension.
pub fn legacy_document_uti() -> String {
    format!("{APP_ID}.legacy-document")
}

/// Largest icon edge [`rgba`] draws, in pixels.
pub const MAX_SIZE: u32 = 2048;

/// Errors while encoding icon files.
#[derive(Debug)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// How the tile sits in the square canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// Nearly edge to edge (Windows, Linux, window icons there).
    Full,
    /// Apple's macOS app icon grid: an 824/1024 tile centred on the canvas with a soft shadow.
    MacOs,
}

impl Style {
    /// The style of this platform's window icon.
    pub fn native() -> Style {
        if cfg!(target_os = "macos") { Style::MacOs } else { Style::Full }
    }
}

/// A colour with straight (not premultiplied) components in 0..=1.
#[derive(Clone, Copy, Debug)]
struct Rgb(f32, f32, f32);

const fn hex(v: u32) -> Rgb {
    Rgb(((v >> 16) & 0xff) as f32 / 255.0, ((v >> 8) & 0xff) as f32 / 255.0, (v & 0xff) as f32 / 255.0)
}

/// Tile gradient, top left to bottom right: the dark theme's accent, the light theme's accent and
/// a deeper violet (`genha_ui_egui::theme`).
const TILE_STOPS: [Rgb; 3] = [hex(0x9a86ff), hex(0x5b3cf5), hex(0x3a1fc4)];
const CIRCLE: Rgb = hex(0xffc94d);
const CIRCLE_SHADE: Rgb = hex(0xff9f43);
const SQUARE: Rgb = hex(0xffffff);

/// The tile in canvas units (0..1): inset from each edge and corner radius.
fn tile(style: Style) -> (f32, f32) {
    match style {
        Style::Full => (0.035, 0.205),
        // 100 px inset and a 185 px radius on a 1024 px canvas.
        Style::MacOs => (100.0 / 1024.0, 185.0 / 1024.0),
    }
}

/// Signed distance from `p` to a rounded box (centre `c`, half extent `h`, corner radius `r`).
fn sd_round_box(p: (f32, f32), c: (f32, f32), h: f32, r: f32) -> f32 {
    let qx = (p.0 - c.0).abs() - h + r;
    let qy = (p.1 - c.1).abs() - h + r;
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    outside + qx.max(qy).min(0.0) - r
}

fn sd_circle(p: (f32, f32), c: (f32, f32), r: f32) -> f32 {
    ((p.0 - c.0).powi(2) + (p.1 - c.1).powi(2)).sqrt() - r
}

/// Area of a pixel (edge `px`) inside a shape whose edge is `d` away from the pixel's centre.
fn coverage(d: f32, px: f32) -> f32 {
    (0.5 - d / px).clamp(0.0, 1.0)
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    Rgb(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, a.2 + (b.2 - a.2) * t)
}

/// Premultiplied accumulator: paints `c` with opacity `a` over it.
#[derive(Clone, Copy, Default)]
struct Px(f32, f32, f32, f32);

impl Px {
    fn over(self, c: Rgb, a: f32) -> Px {
        let k = 1.0 - a;
        Px(c.0 * a + self.0 * k, c.1 * a + self.1 * k, c.2 * a + self.2 * k, a + self.3 * k)
    }
}

/// The icon at `size` × `size` pixels as straight-alpha RGBA bytes, row by row (`size` is clamped
/// to 1..=[`MAX_SIZE`]).
pub fn rgba(size: u32, style: Style) -> Vec<u8> {
    let size = size.clamp(1, MAX_SIZE);
    let n = size as f32;
    let px = 1.0 / n;
    let (inset, radius) = tile(style);
    let tile_half = 0.5 - inset;
    // The mark, in tile units (0..1 across the tile), mapped to the canvas.
    let span = 1.0 - 2.0 * inset;
    let at = |u: f32, v: f32| (inset + u * span, inset + v * span);
    let circle_c = at(0.385, 0.385);
    let circle_r = 0.228 * span;
    let square_c = at(0.645, 0.645);
    let square_h = 0.195 * span;
    let square_r = 0.055 * span;
    // The gap that cuts the circle where the square passes in front of it. Never thinner than
    // two thirds of a pixel, so the two shapes stay apart at 16 px.
    let gap = (0.042 * span).max(0.66 * px);
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let p = ((x as f32 + 0.5) * px, (y as f32 + 0.5) * px);
            let mut acc = Px::default();
            if style == Style::MacOs {
                // Soft shadow below the tile.
                let d = sd_round_box((p.0, p.1 - 10.0 / 1024.0), (0.5, 0.5), tile_half, radius);
                let blur = 22.0 / 1024.0;
                acc = acc.over(Rgb(0.0, 0.0, 0.0), 0.32 * (1.0 - smoothstep(-blur, blur, d)));
            }
            let tile_cov = coverage(sd_round_box(p, (0.5, 0.5), tile_half, radius), px);
            if tile_cov > 0.0 {
                // Diagonal gradient in tile units.
                let t = (((p.0 - inset) + (p.1 - inset)) / (2.0 * span)).clamp(0.0, 1.0);
                let base = if t < 0.5 { lerp(TILE_STOPS[0], TILE_STOPS[1], t * 2.0) } else { lerp(TILE_STOPS[1], TILE_STOPS[2], t * 2.0 - 1.0) };
                acc = acc.over(base, tile_cov);
                let ds = sd_round_box(p, square_c, square_h, square_r);
                let circle_cov = coverage(sd_circle(p, circle_c, circle_r), px) * (1.0 - coverage(ds - gap, px));
                if circle_cov > 0.0 {
                    // A little depth: warmer towards the bottom right of the circle.
                    let k = (((p.0 - circle_c.0) + (p.1 - circle_c.1)) / (2.0 * circle_r) + 0.5).clamp(0.0, 1.0);
                    acc = acc.over(lerp(CIRCLE, CIRCLE_SHADE, k * 0.8), circle_cov * tile_cov);
                }
                acc = acc.over(SQUARE, coverage(ds, px) * tile_cov);
            }
            let a = acc.3.clamp(0.0, 1.0);
            let un = |c: f32| if a > 0.0 { ((c / a).clamp(0.0, 1.0) * 255.0).round() as u8 } else { 0 };
            out.extend_from_slice(&[un(acc.0), un(acc.1), un(acc.2), (a * 255.0).round() as u8]);
        }
    }
    out
}

/// The icon as a PNG file.
pub fn png(size: u32, style: Style) -> Result<Vec<u8>, Error> {
    let size = size.clamp(1, MAX_SIZE);
    encode_png(&rgba(size, style), size)
}

fn encode_png(rgba: &[u8], size: u32) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, size, size);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
    let err = |e: png::EncodingError| Error(format!("cannot encode the icon as PNG: {e}"));
    let mut writer = enc.write_header().map_err(err)?;
    writer.write_image_data(rgba).map_err(err)?;
    writer.finish().map_err(err)?;
    Ok(out)
}

/// Sizes in a Windows `.ico` file.
pub const ICO_SIZES: &[u32] = &[16, 20, 24, 32, 40, 48, 64, 96, 128, 256];

/// A Windows `.ico` file with every size in [`ICO_SIZES`], each a PNG image (Windows Vista and
/// later read PNG entries of any size).
pub fn ico() -> Result<Vec<u8>, Error> {
    let images = ICO_SIZES.iter().map(|&s| png(s, Style::Full).map(|p| (s, p))).collect::<Result<Vec<_>, _>>()?;
    let count = u16::try_from(images.len()).map_err(|_| Error("too many icon sizes".into()))?;
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes()); // reserved
    out.extend_from_slice(&1u16.to_le_bytes()); // 1 = icon
    out.extend_from_slice(&count.to_le_bytes());
    let mut offset = 6 + 16 * images.len();
    for (size, data) in &images {
        let edge = if *size >= 256 { 0 } else { *size as u8 }; // 0 means 256
        out.extend_from_slice(&[edge, edge, 0, 0]); // width, height, palette size, reserved
        out.extend_from_slice(&1u16.to_le_bytes()); // colour planes
        out.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        out.extend_from_slice(&u32::try_from(data.len()).map_err(|_| Error("icon image too large".into()))?.to_le_bytes());
        out.extend_from_slice(&u32::try_from(offset).map_err(|_| Error("icon file too large".into()))?.to_le_bytes());
        offset += data.len();
    }
    for (_, data) in &images {
        out.extend_from_slice(data);
    }
    Ok(out)
}

/// The images of a macOS `.iconset` folder (`iconutil -c icns` turns it into an `.icns` file):
/// file name and edge in pixels.
pub const ICONSET: &[(&str, u32)] = &[
    ("icon_16x16.png", 16),
    ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128),
    ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256),
    ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512),
    ("icon_512x512@2x.png", 1024),
];

/// Entries of the `.icns` file [`icns`] writes: OSType and edge in pixels (all PNG data).
const ICNS_TYPES: &[(&[u8; 4], u32)] = &[
    (b"icp4", 16),
    (b"ic11", 32),
    (b"icp5", 32),
    (b"ic12", 64),
    (b"ic07", 128),
    (b"ic13", 256),
    (b"ic08", 256),
    (b"ic14", 512),
    (b"ic09", 512),
    (b"ic10", 1024),
];

/// A macOS `.icns` file (PNG entries, macOS 10.7 and later). On a Mac, `iconutil` on the
/// [`ICONSET`] images makes the same file; this writer lets other systems build it too.
pub fn icns() -> Result<Vec<u8>, Error> {
    let mut cache: Vec<(u32, Vec<u8>)> = Vec::new();
    let mut body = Vec::new();
    for (kind, size) in ICNS_TYPES {
        let data = match cache.iter().find(|(s, _)| s == size) {
            Some((_, d)) => d.clone(),
            None => {
                let d = png(*size, Style::MacOs)?;
                cache.push((*size, d.clone()));
                d
            }
        };
        body.extend_from_slice(*kind);
        body.extend_from_slice(&u32::try_from(data.len() + 8).map_err(|_| Error("icon image too large".into()))?.to_be_bytes());
        body.extend_from_slice(&data);
    }
    let mut out = Vec::with_capacity(body.len() + 8);
    out.extend_from_slice(b"icns");
    out.extend_from_slice(&u32::try_from(body.len() + 8).map_err(|_| Error("icon file too large".into()))?.to_be_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

/// Writes `genha.ico` and a resource script `genha.rc` (the icon and the version information
/// Explorer shows) into `dir`, for a Windows build script to compile and link into an executable.
/// Returns the path of the `.rc` file.
pub fn windows_resource(dir: &Path, description: &str, version: &str) -> Result<PathBuf, Error> {
    std::fs::create_dir_all(dir).map_err(|e| Error(format!("cannot create {}: {e}", dir.display())))?;
    let ico_path = dir.join("genha.ico");
    std::fs::write(&ico_path, ico()?).map_err(|e| Error(format!("cannot write {}: {e}", ico_path.display())))?;
    let rc_path = dir.join("genha.rc");
    std::fs::write(&rc_path, resource_script(&ico_path, description, version))
        .map_err(|e| Error(format!("cannot write {}: {e}", rc_path.display())))?;
    Ok(rc_path)
}

/// The text of a Windows resource script with the icon at `ico` and version information.
pub fn resource_script(ico: &Path, description: &str, version: &str) -> String {
    let mut nums = version.split(['.', '-', '+']).map(|p| p.parse::<u16>().unwrap_or(0));
    let v: [u16; 4] = [nums.next().unwrap_or(0), nums.next().unwrap_or(0), nums.next().unwrap_or(0), 0];
    let comma = format!("{},{},{},{}", v[0], v[1], v[2], v[3]);
    // Resource scripts are C-like: backslashes and quotes in strings are escaped.
    let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    let ico = esc(&ico.display().to_string());
    let description = esc(description);
    let version = esc(version);
    format!(
        r#"1 ICON "{ico}"

1 VERSIONINFO
FILEVERSION {comma}
PRODUCTVERSION {comma}
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "GenHa contributors"
      VALUE "FileDescription", "{description}"
      VALUE "FileVersion", "{version}"
      VALUE "LegalCopyright", "MIT OR Apache-2.0"
      VALUE "ProductName", "{APP_NAME}"
      VALUE "ProductVersion", "{version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(img: &[u8], size: u32, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * size + x) * 4) as usize;
        [img[i], img[i + 1], img[i + 2], img[i + 3]]
    }

    #[test]
    fn corners_are_transparent_and_the_tile_is_opaque() {
        for style in [Style::Full, Style::MacOs] {
            for size in [16, 32, 256] {
                let img = rgba(size, style);
                assert_eq!(img.len(), (size * size * 4) as usize);
                assert_eq!(pixel(&img, size, 0, 0)[3], 0, "{style:?} {size}");
                // The middle of the tile, left of the mark: opaque violet.
                let p = pixel(&img, size, size / 2, size * 9 / 10 - 1);
                assert_eq!(p[3], 255, "{style:?} {size}");
                assert!(p[2] > p[1] && p[2] > p[0] / 2, "violet, got {p:?}");
            }
        }
    }

    #[test]
    fn the_mark_is_there_at_every_size() {
        for size in [16, 32, 64, 1024] {
            let img = rgba(size, Style::Full);
            // Centre of the square: white. Centre of the circle: warm (red > blue).
            let sq = pixel(&img, size, (size as f32 * 0.66) as u32, (size as f32 * 0.66) as u32);
            assert!(sq[0] > 230 && sq[1] > 230 && sq[2] > 230, "{size}: {sq:?}");
            let c = pixel(&img, size, (size as f32 * 0.36) as u32, (size as f32 * 0.36) as u32);
            assert!(c[0] > 200 && c[2] < 140, "{size}: {c:?}");
        }
    }

    #[test]
    fn sizes_are_clamped() {
        assert_eq!(rgba(0, Style::Full).len(), 4);
        assert_eq!(rgba(u32::MAX, Style::Full).len(), (MAX_SIZE * MAX_SIZE * 4) as usize);
    }

    #[test]
    fn png_has_a_signature() {
        let p = png(48, Style::Full).unwrap();
        assert_eq!(&p[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn ico_directory_points_at_png_images() {
        let ico = ico().unwrap();
        assert_eq!(&ico[..6], &[0, 0, 1, 0, ICO_SIZES.len() as u8, 0]);
        for (i, size) in ICO_SIZES.iter().enumerate() {
            let e = &ico[6 + 16 * i..6 + 16 * (i + 1)];
            assert_eq!(e[0] as u32, if *size == 256 { 0 } else { *size });
            let len = u32::from_le_bytes([e[8], e[9], e[10], e[11]]) as usize;
            let off = u32::from_le_bytes([e[12], e[13], e[14], e[15]]) as usize;
            assert_eq!(&ico[off..off + 8], b"\x89PNG\r\n\x1a\n");
            assert!(off + len <= ico.len());
        }
    }

    #[test]
    fn icns_entries_add_up() {
        let icns = icns().unwrap();
        assert_eq!(&icns[..4], b"icns");
        let total = u32::from_be_bytes([icns[4], icns[5], icns[6], icns[7]]) as usize;
        assert_eq!(total, icns.len());
        let mut at = 8;
        let mut kinds = vec![];
        while at < icns.len() {
            let len = u32::from_be_bytes([icns[at + 4], icns[at + 5], icns[at + 6], icns[at + 7]]) as usize;
            kinds.push(String::from_utf8_lossy(&icns[at..at + 4]).into_owned());
            assert_eq!(&icns[at + 8..at + 16], b"\x89PNG\r\n\x1a\n");
            at += len;
        }
        assert_eq!(at, icns.len());
        assert!(kinds.contains(&"ic10".to_string()));
    }

    #[test]
    fn resource_script_names_icon_and_version() {
        let rc = resource_script(Path::new(r"C:\out\genha.ico"), "GenHa", "0.1.0");
        assert!(rc.contains(r#"1 ICON "C:\\out\\genha.ico""#));
        assert!(rc.contains("FILEVERSION 0,1,0,0"));
        assert!(rc.contains(r#"VALUE "ProductVersion", "0.1.0""#));
    }

    #[test]
    fn ids_are_reverse_dns() {
        assert!(APP_ID.split('.').count() >= 3);
        assert!(APP_ID.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-'));
        assert!(document_uti().starts_with(APP_ID));
    }
}
