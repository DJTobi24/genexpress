//! Version history: named or automatic snapshots of a design's pages, kept inside the file.

use serde::{Deserialize, Serialize};

use crate::{BrandKit, Page};

/// Most versions a document keeps (bookmarked ones first; the oldest automatic ones go).
pub const MAX_VERSIONS: usize = 100;
/// Automatic (unbookmarked) versions kept.
pub const MAX_AUTO_VERSIONS: usize = 20;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Version {
    pub id: u64,
    pub name: String,
    /// Seconds since 1970 (0 when unknown).
    pub created: u64,
    /// Bookmarked versions are kept until deleted; automatic ones are pruned.
    #[serde(default)]
    pub bookmarked: bool,
    pub title: String,
    pub pages: Vec<Page>,
    pub brand: BrandKit,
}
