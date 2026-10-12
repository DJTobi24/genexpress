//! Embedded binary data (pictures, logos, fonts later), stored once per document.

use std::sync::Arc;

use base64::Engine as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetId(pub u64);

impl std::fmt::Display for AssetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "asset {}", self.0)
    }
}

/// Reference-counted bytes; serialized as base64 so the native format stays JSON.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Bytes(pub Arc<[u8]>);

impl Bytes {
    pub fn new(v: Vec<u8>) -> Self {
        Bytes(v.into())
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::ops::Deref for Bytes {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bytes({} bytes)", self.0.len())
    }
}

impl Serialize for Bytes {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&base64::engine::general_purpose::STANDARD.encode(&self.0))
    }
}

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        base64::engine::general_purpose::STANDARD.decode(s.as_bytes()).map(Bytes::new).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    /// The file name it came from (shown in the UI), e.g. `beach.jpg`.
    pub name: String,
    /// `image/png`, `image/jpeg`, `image/webp`, `image/gif`, `image/bmp`, `image/svg+xml`.
    pub mime: String,
    /// Pixel size of a picture (0 when unknown or not a picture).
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    pub data: Bytes,
}
