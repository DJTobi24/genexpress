//! Where the app keeps things between runs: Data Recovery copies (`recovery/…`), recent designs
//! (`recent/…`) and fonts the user added (`fonts/…`).
//!
//! The engine only sees the [`Store`] trait, so the same recovery, recent-designs and font logic
//! runs on the desktop (a folder: [`DirStore`]) and in the browser (IndexedDB behind an in-memory
//! copy, implemented in `genha-ui-egui`; [`MemStore`] is the plain in-memory version, for tests and
//! hosts that keep nothing). Keys are short relative paths: `dir/name`, ASCII letters, digits,
//! `.`, `-` and `_` only ([`valid_key`]), so a key can never leave the store.

/// Most entries [`Store::list`] returns for one folder.
pub const MAX_LIST: usize = 1000;
/// Longest key accepted.
pub const MAX_KEY: usize = 200;

/// Key-value storage for the app's own data. Implementations never panic; a failed write is an
/// error message, a failed read is `None`.
pub trait Store: Send + std::fmt::Debug {
    /// The bytes stored under `key`.
    fn read(&self, key: &str) -> Option<Vec<u8>>;
    /// Stores `bytes` under `key`, replacing what was there.
    fn write(&mut self, key: &str, bytes: &[u8]) -> Result<(), String>;
    /// Removes `key` (nothing happens when it is missing).
    fn remove(&mut self, key: &str);
    /// The keys directly in folder `dir` (`"recovery"` lists `recovery/…`), with when each was
    /// written (seconds since 1970, 0 when unknown); at most [`MAX_LIST`].
    fn list(&self, dir: &str) -> Vec<(String, u64)>;
    /// Where `key` is kept, for people and for older callers that pass paths (on the desktop: the
    /// file's path).
    fn locate(&self, key: &str) -> String {
        key.to_string()
    }
}

/// Whether `key` is a safe store key: `dir/name` segments of `[A-Za-z0-9._-]`, not starting with
/// a dot, at most [`MAX_KEY`] bytes.
pub fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= MAX_KEY
        && key
            .split('/')
            .all(|seg| !seg.is_empty() && !seg.starts_with('.') && seg.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_')))
}

/// `name` made safe for use in a key (other characters become `-`), at most 80 bytes.
pub fn key_part(name: &str) -> String {
    let s: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '-' }).take(80).collect();
    let s = s.trim_start_matches('.').to_string();
    if s.is_empty() { "item".into() } else { s }
}

/// A store kept in memory only (tests; hosts that keep nothing between runs). `clock` gives the
/// time recorded with each write.
#[derive(Debug, Default, Clone)]
pub struct MemStore {
    pub items: std::collections::BTreeMap<String, (Vec<u8>, u64)>,
    /// Seconds since 1970 recorded with the next write (the host sets it; wasm has no clock here).
    pub now: u64,
}

impl Store for MemStore {
    fn read(&self, key: &str) -> Option<Vec<u8>> {
        self.items.get(key).map(|(b, _)| b.clone())
    }
    fn write(&mut self, key: &str, bytes: &[u8]) -> Result<(), String> {
        if !valid_key(key) {
            return Err(format!("invalid storage key `{key}`"));
        }
        self.items.insert(key.to_string(), (bytes.to_vec(), self.now));
        Ok(())
    }
    fn remove(&mut self, key: &str) {
        self.items.remove(key);
    }
    fn list(&self, dir: &str) -> Vec<(String, u64)> {
        list_in(self.items.iter().map(|(k, (_, t))| (k.as_str(), *t)), dir)
    }
}

/// The keys of `items` directly in folder `dir` (shared by in-memory stores).
pub fn list_in<'a>(items: impl Iterator<Item = (&'a str, u64)>, dir: &str) -> Vec<(String, u64)> {
    let prefix = format!("{}/", dir.trim_end_matches('/'));
    items
        .filter(|(k, _)| k.strip_prefix(&prefix).is_some_and(|rest| !rest.is_empty() && !rest.contains('/')))
        .take(MAX_LIST)
        .map(|(k, t)| (k.to_string(), t))
        .collect()
}

/// The per-user data folder the desktop app and the CLI share (Data Recovery copies, recent
/// designs, added fonts, downloaded models): `GENHA_DATA_DIR` when set, else `%APPDATA%\GenHa`,
/// `~/Library/Application Support/GenHa` or `$XDG_DATA_HOME/genha` (`~/.local/share/genha`).
#[cfg(not(target_arch = "wasm32"))]
pub fn user_data_dir() -> Option<std::path::PathBuf> {
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(std::path::PathBuf::from);
    if let Some(d) = env("GENHA_DATA_DIR") {
        return Some(d);
    }
    if cfg!(target_os = "windows") {
        return env("APPDATA").map(|d| d.join("GenHa"));
    }
    if cfg!(target_os = "macos") {
        return env("HOME").map(|h| h.join("Library/Application Support/GenHa"));
    }
    env("XDG_DATA_HOME").or_else(|| env("HOME").map(|h| h.join(".local/share"))).map(|d| d.join("genha"))
}

/// A store in a folder on disk: key `recovery/a.genha` is the file `<root>/recovery/a.genha`.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct DirStore {
    pub root: std::path::PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl DirStore {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        DirStore { root: root.into() }
    }

    fn path(&self, key: &str) -> Option<std::path::PathBuf> {
        valid_key(key).then(|| key.split('/').fold(self.root.clone(), |p, seg| p.join(seg)))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Store for DirStore {
    fn read(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.path(key)?;
        let len = std::fs::metadata(&path).ok()?.len();
        if len > genha_format::MAX_FILE_BYTES {
            return None;
        }
        std::fs::read(path).ok()
    }

    fn write(&mut self, key: &str, bytes: &[u8]) -> Result<(), String> {
        let path = self.path(key).ok_or_else(|| format!("invalid storage key `{key}`"))?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        }
        genha_format::write_atomic(&path, bytes).map_err(|e| e.to_string())
    }

    fn remove(&mut self, key: &str) {
        if let Some(path) = self.path(key) {
            // A missing file is what removing wants.
            let _ = std::fs::remove_file(path);
        }
    }

    fn list(&self, dir: &str) -> Vec<(String, u64)> {
        let dir = dir.trim_end_matches('/');
        let Some(folder) = self.path(dir) else { return vec![] };
        let Ok(entries) = std::fs::read_dir(folder) else { return vec![] };
        let mut out = vec![];
        for e in entries.flatten() {
            if out.len() >= MAX_LIST {
                break;
            }
            let name = e.file_name().to_string_lossy().into_owned();
            let key = format!("{dir}/{name}");
            if !valid_key(&key) || !e.file_type().is_ok_and(|t| t.is_file()) {
                continue;
            }
            let modified = e
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            out.push((key, modified));
        }
        out.sort();
        out
    }

    fn locate(&self, key: &str) -> String {
        self.path(key).map(|p| p.display().to_string()).unwrap_or_else(|| key.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_stay_inside() {
        assert!(valid_key("recovery/123-4.genha"));
        assert!(valid_key("fonts/My-Font_Bold.ttf"));
        for bad in ["", "/etc/passwd", "../x", "a/../b", "a//b", "a/.hidden", "a\\b", "a b", "ä"] {
            assert!(!valid_key(bad), "{bad}");
        }
        assert!(valid_key(&format!("fonts/{}", key_part("Ünïcode Name (Bold).ttf"))));
        assert_eq!(key_part(".."), "item");
    }

    #[test]
    fn mem_store_lists_one_folder() {
        let mut s = MemStore { now: 7, ..Default::default() };
        s.write("recovery/a.genha", b"1").unwrap();
        s.write("recovery/b.genha", b"2").unwrap();
        s.write("recent/list.json", b"[]").unwrap();
        assert!(s.write("../x", b"").is_err());
        assert_eq!(s.list("recovery").len(), 2);
        assert_eq!(s.list("recovery/")[0], ("recovery/a.genha".to_string(), 7));
        s.remove("recovery/a.genha");
        assert_eq!(s.read("recovery/b.genha").as_deref(), Some(&b"2"[..]));
        assert!(s.read("recovery/a.genha").is_none());
    }

    #[test]
    fn dir_store_round_trip() {
        let root = std::env::temp_dir().join(format!("genha-store-{}", std::process::id()));
        let mut s = DirStore::new(&root);
        s.write("fonts/a.ttf", b"abc").unwrap();
        assert_eq!(s.read("fonts/a.ttf").as_deref(), Some(&b"abc"[..]));
        assert_eq!(s.list("fonts").len(), 1);
        assert!(s.read("../../etc/passwd").is_none());
        assert_eq!(s.locate("fonts/a.ttf"), root.join("fonts").join("a.ttf").display().to_string());
        s.remove("fonts/a.ttf");
        assert!(s.list("fonts").is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
