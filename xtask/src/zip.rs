//! A small zip writer for the Windows and web downloads (deflate, no zip64, fixed timestamps so
//! the same files give the same archive).

use std::io::Write;
use std::path::Path;

use flate2::Compression;
use flate2::write::DeflateEncoder;

/// MS-DOS date of every entry: 2026-01-01 (the format cannot store dates before 1980).
const DOS_DATE: u16 = ((2026 - 1980) << 9) | (1 << 5) | 1;

struct Entry {
    name: String,
    crc: u32,
    compressed: u32,
    size: u32,
    offset: u32,
    mode: u32,
}

/// Writes a zip archive of the folder `dir` to `out`; entries are named `<prefix>/<path in dir>`.
pub fn zip_dir(dir: &Path, prefix: &str, out: &Path) -> Result<(), String> {
    let mut files = vec![];
    collect(dir, dir, &mut files)?;
    files.sort();
    let mut items = vec![];
    for rel in files {
        let path = dir.join(&rel);
        let data = std::fs::read(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        items.push((format!("{prefix}/{rel}"), data, executable(&path)));
    }
    let bytes = zip_bytes(&items)?;
    std::fs::write(out, bytes).map_err(|e| format!("cannot write {}: {e}", out.display()))
}

/// Relative paths (with `/`) of all files below `dir`.
fn collect(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(root, &p, out)?;
        } else if let Ok(rel) = p.strip_prefix(root) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p).map(|m| m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}

#[cfg(not(unix))]
fn executable(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"))
}

fn u32_of(n: usize, what: &str) -> Result<u32, String> {
    u32::try_from(n).map_err(|_| format!("{what} is too large for a zip archive (4 GB)"))
}

/// A zip archive of `(name, data, executable)` entries.
pub fn zip_bytes(items: &[(String, Vec<u8>, bool)]) -> Result<Vec<u8>, String> {
    let mut out: Vec<u8> = Vec::new();
    let mut entries = vec![];
    for (name, data, exec) in items {
        let mut crc = flate2::Crc::new();
        crc.update(data);
        let mut enc = DeflateEncoder::new(Vec::new(), Compression::best());
        enc.write_all(data).map_err(|e| format!("cannot compress {name}: {e}"))?;
        let packed = enc.finish().map_err(|e| format!("cannot compress {name}: {e}"))?;
        let entry = Entry {
            name: name.clone(),
            crc: crc.sum(),
            compressed: u32_of(packed.len(), name)?,
            size: u32_of(data.len(), name)?,
            offset: u32_of(out.len(), "the archive")?,
            mode: if *exec { 0o100755 } else { 0o100644 },
        };
        let name_len = u16::try_from(name.len()).map_err(|_| format!("name too long: {name}"))?;
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes()); // local file header
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed: 2.0 (deflate)
        out.extend_from_slice(&0x0800u16.to_le_bytes()); // flags: names are UTF-8
        out.extend_from_slice(&8u16.to_le_bytes()); // deflate
        out.extend_from_slice(&0u16.to_le_bytes()); // time
        out.extend_from_slice(&DOS_DATE.to_le_bytes());
        out.extend_from_slice(&entry.crc.to_le_bytes());
        out.extend_from_slice(&entry.compressed.to_le_bytes());
        out.extend_from_slice(&entry.size.to_le_bytes());
        out.extend_from_slice(&name_len.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra field length
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&packed);
        entries.push(entry);
    }
    let directory_at = u32_of(out.len(), "the archive")?;
    for e in &entries {
        out.extend_from_slice(&0x0201_4b50u32.to_le_bytes()); // central directory header
        out.extend_from_slice(&(3u16 << 8 | 20).to_le_bytes()); // made by: Unix, zip 2.0
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0x0800u16.to_le_bytes());
        out.extend_from_slice(&8u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&DOS_DATE.to_le_bytes());
        out.extend_from_slice(&e.crc.to_le_bytes());
        out.extend_from_slice(&e.compressed.to_le_bytes());
        out.extend_from_slice(&e.size.to_le_bytes());
        out.extend_from_slice(&(e.name.len() as u16).to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]); // extra, comment, disk, internal attributes
        out.extend_from_slice(&(e.mode << 16).to_le_bytes()); // external attributes: Unix mode
        out.extend_from_slice(&e.offset.to_le_bytes());
        out.extend_from_slice(e.name.as_bytes());
    }
    let directory_len = u32_of(out.len(), "the archive")? - directory_at;
    let count = u16::try_from(entries.len()).map_err(|_| "too many files for a zip archive".to_string())?;
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes()); // end of central directory
    out.extend_from_slice(&[0, 0, 0, 0]); // disk numbers
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&directory_len.to_le_bytes());
    out.extend_from_slice(&directory_at.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // comment length
    Ok(out)
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    fn u16_at(b: &[u8], i: usize) -> usize {
        u16::from_le_bytes([b[i], b[i + 1]]) as usize
    }
    fn u32_at(b: &[u8], i: usize) -> u32 {
        u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
    }

    #[test]
    fn entries_read_back() {
        let items = vec![
            ("GenHa/README.txt".to_string(), b"hello hello hello hello".to_vec(), false),
            ("GenHa/genha.exe".to_string(), (0..5000u32).map(|i| (i * 7 % 251) as u8).collect(), true),
            ("GenHa/empty".to_string(), vec![], false),
        ];
        let zip = super::zip_bytes(&items).unwrap();
        // End of central directory: entry count and where the directory starts.
        let eocd = zip.len() - 22;
        assert_eq!(u32_at(&zip, eocd), 0x0605_4b50);
        assert_eq!(u16_at(&zip, eocd + 10), 3);
        let mut at = u32_at(&zip, eocd + 16) as usize;
        for (name, data, exec) in &items {
            assert_eq!(u32_at(&zip, at), 0x0201_4b50);
            let name_len = u16_at(&zip, at + 28);
            assert_eq!(&zip[at + 46..at + 46 + name_len], name.as_bytes());
            let mode = u32_at(&zip, at + 38) >> 16;
            assert_eq!(mode & 0o111 != 0, *exec);
            let local = u32_at(&zip, at + 42) as usize;
            let compressed = u32_at(&zip, at + 20) as usize;
            let start = local + 30 + u16_at(&zip, local + 26);
            let mut back = vec![];
            flate2::read::DeflateDecoder::new(&zip[start..start + compressed]).read_to_end(&mut back).unwrap();
            assert_eq!(&back, data);
            let mut crc = flate2::Crc::new();
            crc.update(data);
            assert_eq!(u32_at(&zip, at + 16), crc.sum());
            at += 46 + name_len;
        }
    }
}
