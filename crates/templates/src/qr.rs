//! QR codes as a vector shape.

use std::fmt::Write as _;

use genha_color::{Color, contrast_ratio};
use genha_doc::ShapeKind;

use crate::TemplateError;

/// Most bytes encoded in one QR code.
pub const MAX_QR_BYTES: usize = 1024;
/// Light modules around the code that scanners need, per side.
const QUIET_ZONE: usize = 4;
/// Lowest contrast against white that still scans reliably.
const MIN_CONTRAST: f64 = 2.5;

/// A QR code for `data` (error correction level M) as a [`ShapeKind::Path`] of the dark modules,
/// one rectangle per horizontal run. The view box is the module count including a 4-module quiet
/// zone on each side, so the shape is square; place it on a light background.
///
/// `color` is the colour the code will be drawn in: it must contrast enough with white to scan
/// (fill the element with it).
pub fn qr_code(data: &str, color: Color) -> Result<ShapeKind, TemplateError> {
    if data.is_empty() {
        return Err(TemplateError::Empty);
    }
    if data.len() > MAX_QR_BYTES {
        return Err(TemplateError::TooLong(data.len()));
    }
    if color.a < 128 || contrast_ratio(color, Color::WHITE) < MIN_CONTRAST {
        return Err(TemplateError::LowContrast);
    }
    let code = qrcode::QrCode::new(data.as_bytes()).map_err(|e| TemplateError::Qr(e.to_string()))?;
    let n = code.width();
    let modules = code.to_colors();
    let mut d = String::new();
    for (y, row) in modules.chunks(n.max(1)).enumerate() {
        let mut x = 0;
        while x < row.len() {
            if row.get(x) == Some(&qrcode::Color::Dark) {
                let start = x;
                while row.get(x) == Some(&qrcode::Color::Dark) {
                    x += 1;
                }
                let _ = write!(d, "M{} {}h{}v1h-{}z", start + QUIET_ZONE, y + QUIET_ZONE, x - start, x - start);
            } else {
                x += 1;
            }
        }
    }
    let side = (n + 2 * QUIET_ZONE) as f64;
    Ok(ShapeKind::Path { d, view_width: side, view_height: side })
}
