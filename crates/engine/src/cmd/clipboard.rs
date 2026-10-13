//! Pictures pasted from the system clipboard. The engine has no access to the system clipboard:
//! the app reads the picture (Cmd+V of a copied picture, or this command from the menu) and
//! passes its bytes; agents pass `data` like `image.place`.

use base64::Engine as _;
use serde_json::Value;

use super::create::{MAX_PICTURE_BYTES, place_picture};
use super::{bad, cmd, has_doc, str_param, str_req};
use crate::{CommandSpec, Result, Session};

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![cmd!(
        "image.paste",
        "Paste Picture",
        ["Edit"],
        None,
        "{data: base64 picture (PNG, JPEG, WebP, GIF, BMP or SVG; the app fills it from the system clipboard), name?, x?, y?, width?, height?}",
        has_doc,
        paste
    )]
}

fn paste(s: &mut Session, p: &Value) -> Result<Value> {
    let data = str_req(p, "data", "image.paste").map_err(|_| bad("image.paste", "there is no picture on the clipboard"))?;
    if data.len() > MAX_PICTURE_BYTES * 4 / 3 + 16 {
        return Err(bad("image.paste", "the picture is too large"));
    }
    let bytes =
        base64::engine::general_purpose::STANDARD.decode(data.trim()).map_err(|e| bad("image.paste", format!("`data` is not base64: {e}")))?;
    place_picture(s, str_param(p, "name").unwrap_or("Pasted picture"), &bytes, p)
}
