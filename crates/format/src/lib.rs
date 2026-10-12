//! The native `.genexpress` file format (layer L3).
//!
//! A file is gzip-compressed JSON:
//!
//! ```json
//! {"format": "genexpress", "version": 1, "generator": "GenExpress 0.1.0", "document": { … }}
//! ```
//!
//! The document is [`genexpress_doc::Document`] as serde writes it; pictures are embedded as base64.
//! Plain (uncompressed) JSON opens too, which is handy for hand-written test files and agents.
//! Files from a newer format version are refused with a clear message instead of being misread;
//! everything read is passed through [`Document::repair`] because files are untrusted input.
#![forbid(unsafe_code)]

use std::io::{Read, Write};

use genexpress_doc::Document;
use serde::{Deserialize, Serialize};

pub const EXTENSION: &str = "genexpress";
pub const FORMAT_NAME: &str = "genexpress";
/// Current format version. Bump it (and keep reading older ones) when the model changes shape.
pub const VERSION: u32 = 1;
/// Largest file, compressed or not, that is read.
pub const MAX_FILE_BYTES: u64 = 1 << 30;

#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("not a GenExpress file: {0}")]
    NotOurs(String),
    #[error("this file was made by a newer GenExpress (format version {0}; this version reads up to {VERSION}). Please update GenExpress.")]
    TooNew(u32),
    #[error("the file is damaged: {0}")]
    Damaged(String),
    #[error("the file is larger than {} MB", MAX_FILE_BYTES >> 20)]
    TooLarge,
    #[error("could not write the file: {0}")]
    Write(String),
}

pub type Result<T> = std::result::Result<T, FormatError>;

#[derive(Serialize)]
struct HeaderOut<'a> {
    format: &'static str,
    version: u32,
    generator: String,
    document: &'a Document,
}

#[derive(Deserialize)]
struct HeaderIn {
    format: String,
    version: u32,
    document: serde_json::Value,
}

/// The document as file bytes, gzip-compressed unless `compress` is false.
pub fn to_bytes(doc: &Document, compress: bool) -> Result<Vec<u8>> {
    let header = HeaderOut { format: FORMAT_NAME, version: VERSION, generator: format!("GenExpress {}", env!("CARGO_PKG_VERSION")), document: doc };
    let json = serde_json::to_vec(&header).map_err(|e| FormatError::Write(e.to_string()))?;
    if !compress {
        return Ok(json);
    }
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(&json).map_err(|e| FormatError::Write(e.to_string()))?;
    enc.finish().map_err(|e| FormatError::Write(e.to_string()))
}

/// Reads file bytes (compressed or plain). Returns the document and notes on anything repaired.
pub fn from_bytes(bytes: &[u8]) -> Result<(Document, Vec<String>)> {
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(FormatError::TooLarge);
    }
    let json: std::borrow::Cow<[u8]> = if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(bytes)
            .take(MAX_FILE_BYTES + 1)
            .read_to_end(&mut out)
            .map_err(|e| FormatError::Damaged(format!("could not decompress: {e}")))?;
        if out.len() as u64 > MAX_FILE_BYTES {
            return Err(FormatError::TooLarge);
        }
        out.into()
    } else {
        bytes.into()
    };
    let trimmed = json.iter().position(|b| !b.is_ascii_whitespace()).and_then(|i| json.get(i));
    if trimmed != Some(&b'{') {
        return Err(FormatError::NotOurs("expected a GenExpress (.genexpress) file".into()));
    }
    let header: HeaderIn = serde_json::from_slice(&json).map_err(|e| FormatError::NotOurs(e.to_string()))?;
    if header.format != FORMAT_NAME {
        return Err(FormatError::NotOurs(format!("the file says it is `{}`", header.format)));
    }
    if header.version > VERSION {
        return Err(FormatError::TooNew(header.version));
    }
    let mut doc: Document = serde_json::from_value(header.document).map_err(|e| FormatError::Damaged(e.to_string()))?;
    let notes = doc.repair();
    Ok((doc, notes))
}

/// Writes `bytes` to `path` atomically: a temporary file next to it, then a rename, so a crash or a
/// full disk never leaves a half-written document behind.
#[cfg(not(target_arch = "wasm32"))]
pub fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(std::path::Path::new("."));
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "document".into());
    let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    let write = || -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)
    };
    write().map_err(|e| {
        // Best effort: the temporary file is useless once writing failed.
        let _ = std::fs::remove_file(&tmp);
        FormatError::Write(e.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_compressed_and_plain() {
        let doc = Document::new("Hello", 1080.0, 1350.0);
        for compress in [true, false] {
            let bytes = to_bytes(&doc, compress).unwrap();
            assert_eq!(bytes.starts_with(&[0x1f, 0x8b]), compress);
            let (back, notes) = from_bytes(&bytes).unwrap();
            assert_eq!(back, doc);
            assert!(notes.is_empty(), "{notes:?}");
        }
    }

    #[test]
    fn refuses_other_and_newer_files() {
        assert!(matches!(from_bytes(b"%PDF-1.4"), Err(FormatError::NotOurs(_))));
        assert!(matches!(from_bytes(br#"{"format":"other","version":1,"document":{}}"#), Err(FormatError::NotOurs(_))));
        assert!(matches!(from_bytes(br#"{"format":"genexpress","version":99,"document":{}}"#), Err(FormatError::TooNew(99))));
        assert!(matches!(from_bytes(br#"{"format":"genexpress","version":1,"document":{"title":1}}"#), Err(FormatError::Damaged(_))));
        assert!(from_bytes(&[0x1f, 0x8b, 1, 2, 3]).is_err());
    }

    #[test]
    fn atomic_write() {
        let dir = std::env::temp_dir().join(format!("genexpress-format-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.genexpress");
        write_atomic(&path, b"abc").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"abc");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
