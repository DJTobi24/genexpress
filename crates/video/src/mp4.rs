//! MP4 (ISO/IEC 14496-12 base media file format, with the `avc1` sample entry of ISO/IEC 14496-15):
//! a writer for one H.264 video track, and a reader of the structure it writes, for checks.
//!
//! The file is `ftyp`, `moov`, `mdat`: the index comes first ("fast start"), so browsers start
//! playing before the whole file has arrived. All samples sit in one chunk.

use crate::VideoError;

/// What the writer needs to know about the stream.
pub(crate) struct Track<'a> {
    pub width: u32,
    pub height: u32,
    /// Samples per second; every sample lasts one tick.
    pub fps: u32,
    pub sps: &'a [u8],
    pub pps: &'a [u8],
    /// profile_idc, constraint flags, level_idc.
    pub profile: (u8, u8, u8),
}

/// Ticks per second of the track's media timescale per frame per second.
const TICKS_PER_FRAME: u32 = 1000;
/// Timescale of the movie header (milliseconds).
const MOVIE_TIMESCALE: u32 = 1000;

struct Boxes(Vec<u8>);

impl Boxes {
    fn new() -> Boxes {
        Boxes(Vec::new())
    }
    fn u8(&mut self, v: u8) -> &mut Self {
        self.0.push(v);
        self
    }
    fn u16(&mut self, v: u16) -> &mut Self {
        self.0.extend_from_slice(&v.to_be_bytes());
        self
    }
    fn u32(&mut self, v: u32) -> &mut Self {
        self.0.extend_from_slice(&v.to_be_bytes());
        self
    }
    fn bytes(&mut self, b: &[u8]) -> &mut Self {
        self.0.extend_from_slice(b);
        self
    }
    fn zeros(&mut self, n: usize) -> &mut Self {
        self.0.resize(self.0.len() + n, 0);
        self
    }
    /// A box of type `kind` holding what `body` writes.
    fn boxed(&mut self, kind: &[u8; 4], body: impl FnOnce(&mut Boxes)) -> &mut Self {
        let start = self.0.len();
        self.u32(0).bytes(kind);
        body(self);
        let size = (self.0.len() - start) as u32;
        if let Some(s) = self.0.get_mut(start..start + 4) {
            s.copy_from_slice(&size.to_be_bytes());
        }
        self
    }
    /// A full box: version and flags first.
    fn full(&mut self, kind: &[u8; 4], version: u8, flags: u32, body: impl FnOnce(&mut Boxes)) -> &mut Self {
        self.boxed(kind, |b| {
            b.u32((u32::from(version) << 24) | (flags & 0x00ff_ffff));
            body(b);
        })
    }
}

/// The unity transformation matrix of `mvhd` and `tkhd`.
fn matrix(b: &mut Boxes) {
    for v in [0x0001_0000u32, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x4000_0000] {
        b.u32(v);
    }
}

/// Writes an MP4 file: `samples` are the coded frames (length-prefixed NAL units), `sync` says
/// which are IDR pictures.
pub(crate) fn write(track: &Track, samples: &[Vec<u8>], sync: &[bool]) -> Result<Vec<u8>, VideoError> {
    let count = u32::try_from(samples.len()).map_err(|_| VideoError::TooLarge)?;
    if count == 0 {
        return Err(VideoError::NoFrames);
    }
    let fps = track.fps.max(1);
    let timescale = fps.checked_mul(TICKS_PER_FRAME).ok_or(VideoError::TooLarge)?;
    let media_duration = u64::from(count) * u64::from(TICKS_PER_FRAME);
    let movie_duration = u64::from(count) * u64::from(MOVIE_TIMESCALE) / u64::from(fps);
    let media_duration = u32::try_from(media_duration).map_err(|_| VideoError::TooLarge)?;
    let movie_duration = u32::try_from(movie_duration).map_err(|_| VideoError::TooLarge)?;
    let data_len: usize = samples.iter().map(Vec::len).sum();

    let mut ftyp = Boxes::new();
    ftyp.boxed(b"ftyp", |b| {
        b.bytes(b"isom").u32(0x200).bytes(b"isom").bytes(b"iso2").bytes(b"avc1").bytes(b"mp41");
    });

    // moov is built twice: the chunk offset depends on its size (which does not depend on the
    // offset's value).
    let build = |chunk_offset: u32| -> Boxes {
        let mut m = Boxes::new();
        m.boxed(b"moov", |b| {
            b.full(b"mvhd", 0, 0, |b| {
                b.u32(0).u32(0); // creation and modification time
                b.u32(MOVIE_TIMESCALE).u32(movie_duration);
                b.u32(0x0001_0000).u16(0x0100).zeros(10); // rate 1.0, volume 1.0, reserved
                matrix(b);
                b.zeros(24); // pre_defined
                b.u32(2); // next_track_ID
            });
            b.boxed(b"trak", |b| {
                b.full(b"tkhd", 0, 3, |b| {
                    b.u32(0).u32(0).u32(1).u32(0).u32(movie_duration); // times, track_ID 1, reserved, duration
                    b.zeros(8).u16(0).u16(0).u16(0).u16(0); // reserved, layer, alternate_group, volume, reserved
                    matrix(b);
                    b.u32(track.width << 16).u32(track.height << 16);
                });
                b.boxed(b"mdia", |b| {
                    b.full(b"mdhd", 0, 0, |b| {
                        b.u32(0).u32(0).u32(timescale).u32(media_duration);
                        b.u16(0x55c4).u16(0); // language "und", pre_defined
                    });
                    b.full(b"hdlr", 0, 0, |b| {
                        b.u32(0).bytes(b"vide").zeros(12).bytes(b"VideoHandler\0");
                    });
                    b.boxed(b"minf", |b| {
                        b.full(b"vmhd", 0, 1, |b| {
                            b.zeros(8);
                        });
                        b.boxed(b"dinf", |b| {
                            b.full(b"dref", 0, 0, |b| {
                                b.u32(1);
                                b.full(b"url ", 0, 1, |_| {});
                            });
                        });
                        b.boxed(b"stbl", |b| stbl(b, track, samples, sync, chunk_offset));
                    });
                });
            });
        });
        m
    };
    let moov_len = build(0).0.len();
    let offset = ftyp.0.len() + moov_len + 8;
    let offset = u32::try_from(offset).map_err(|_| VideoError::TooLarge)?;
    let moov = build(offset);
    let mdat_len = u32::try_from(data_len + 8).map_err(|_| VideoError::TooLarge)?;

    let mut out = Vec::with_capacity(offset as usize + data_len);
    out.extend_from_slice(&ftyp.0);
    out.extend_from_slice(&moov.0);
    out.extend_from_slice(&mdat_len.to_be_bytes());
    out.extend_from_slice(b"mdat");
    for s in samples {
        out.extend_from_slice(s);
    }
    Ok(out)
}

fn stbl(b: &mut Boxes, track: &Track, samples: &[Vec<u8>], sync: &[bool], chunk_offset: u32) {
    let count = samples.len() as u32;
    b.full(b"stsd", 0, 0, |b| {
        b.u32(1);
        b.boxed(b"avc1", |b| {
            b.zeros(6).u16(1); // reserved, data_reference_index
            b.zeros(16); // pre_defined, reserved
            b.u16(track.width as u16).u16(track.height as u16);
            b.u32(0x0048_0000).u32(0x0048_0000); // 72 dpi
            b.u32(0).u16(1); // reserved, frame_count
            b.zeros(32); // compressorname
            b.u16(0x0018).u16(0xffff); // depth, pre_defined = −1
            b.boxed(b"avcC", |b| {
                let (profile, flags, level) = track.profile;
                b.u8(1).u8(profile).u8(flags).u8(level);
                b.u8(0xff); // 4-byte NAL unit lengths
                b.u8(0xe1).u16(track.sps.len() as u16).bytes(track.sps);
                b.u8(1).u16(track.pps.len() as u16).bytes(track.pps);
            });
            // Colour: BT.709 primaries, transfer and matrix, limited range.
            b.boxed(b"colr", |b| {
                b.bytes(b"nclx").u16(1).u16(1).u16(1).u8(0);
            });
            b.boxed(b"pasp", |b| {
                b.u32(1).u32(1);
            });
        });
    });
    b.full(b"stts", 0, 0, |b| {
        b.u32(1).u32(count).u32(TICKS_PER_FRAME);
    });
    let keys: Vec<u32> = sync.iter().enumerate().filter(|(_, k)| **k).map(|(i, _)| i as u32 + 1).collect();
    if keys.len() < samples.len() {
        b.full(b"stss", 0, 0, |b| {
            b.u32(keys.len() as u32);
            for k in &keys {
                b.u32(*k);
            }
        });
    }
    b.full(b"stsc", 0, 0, |b| {
        b.u32(1).u32(1).u32(count).u32(1);
    });
    b.full(b"stsz", 0, 0, |b| {
        b.u32(0).u32(count);
        for s in samples {
            b.u32(s.len() as u32);
        }
    });
    b.full(b"stco", 0, 0, |b| {
        b.u32(1).u32(chunk_offset);
    });
}

/// What [`inspect`] finds in an MP4 file.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mp4Info {
    /// Top-level boxes in order (`ftyp`, `moov`, `mdat`).
    pub boxes: Vec<String>,
    /// The brand in `ftyp`.
    pub brand: String,
    /// Track width and height (`tkhd`), and the sample entry's (`avc1`).
    pub width: u32,
    pub height: u32,
    /// The sample entry's coding (`avc1`).
    pub codec: String,
    /// AVC profile and level from `avcC`.
    pub profile: u8,
    pub level: u8,
    /// Number of samples (`stsz`) and of sync samples (`stss`; all when absent).
    pub frames: u32,
    pub keyframes: u32,
    /// Media duration in seconds (`mdhd`).
    pub duration: f64,
    /// Samples per second (from `stts`).
    pub fps: f64,
    /// Total sample bytes, and the `mdat` payload size.
    pub sample_bytes: u64,
    pub mdat_bytes: u64,
}

/// Reads the structure of an MP4 file (untrusted input: never panics, errors on anything it does
/// not understand).
pub fn inspect(data: &[u8]) -> Result<Mp4Info, VideoError> {
    let mut info = Mp4Info::default();
    let top = children(data).ok_or(VideoError::Malformed("top-level boxes"))?;
    for (kind, body) in &top {
        info.boxes.push(String::from_utf8_lossy(kind).into_owned());
        match kind {
            b"ftyp" => info.brand = String::from_utf8_lossy(body.get(..4).unwrap_or_default()).into_owned(),
            b"mdat" => info.mdat_bytes = body.len() as u64,
            b"moov" => read_moov(body, &mut info)?,
            _ => {}
        }
    }
    Ok(info)
}

fn read_moov(moov: &[u8], info: &mut Mp4Info) -> Result<(), VideoError> {
    let bad = VideoError::Malformed;
    let trak = find(moov, b"trak").ok_or(bad("no trak"))?;
    let tkhd = find(trak, b"tkhd").ok_or(bad("no tkhd"))?;
    // version 0: 4 (version/flags) + 20 + 52 bytes to width.
    info.width = be32(tkhd, 76).ok_or(bad("tkhd"))? >> 16;
    info.height = be32(tkhd, 80).ok_or(bad("tkhd"))? >> 16;
    let mdia = find(trak, b"mdia").ok_or(bad("no mdia"))?;
    let mdhd = find(mdia, b"mdhd").ok_or(bad("no mdhd"))?;
    let timescale = be32(mdhd, 12).ok_or(bad("mdhd"))?;
    let duration = be32(mdhd, 16).ok_or(bad("mdhd"))?;
    if timescale > 0 {
        info.duration = f64::from(duration) / f64::from(timescale);
    }
    let stbl = find(mdia, b"minf").and_then(|m| find(m, b"stbl")).ok_or(bad("no stbl"))?;
    let stsd = find(stbl, b"stsd").ok_or(bad("no stsd"))?;
    let entries = children(stsd.get(8..).unwrap_or_default()).ok_or(bad("stsd"))?;
    let (kind, entry) = entries.first().ok_or(bad("no sample entry"))?;
    info.codec = String::from_utf8_lossy(kind).into_owned();
    let w = be16(entry, 24).ok_or(bad("avc1"))?;
    let h = be16(entry, 26).ok_or(bad("avc1"))?;
    if (u32::from(w), u32::from(h)) != (info.width, info.height) {
        return Err(bad("track and sample entry sizes differ"));
    }
    let avcc = children(entry.get(78..).unwrap_or_default()).and_then(|c| c.into_iter().find(|(k, _)| k == b"avcC")).ok_or(bad("no avcC"))?;
    info.profile = *avcc.1.get(1).ok_or(bad("avcC"))?;
    info.level = *avcc.1.get(3).ok_or(bad("avcC"))?;
    let stsz = find(stbl, b"stsz").ok_or(bad("no stsz"))?;
    info.frames = be32(stsz, 8).ok_or(bad("stsz"))?;
    let fixed = be32(stsz, 4).ok_or(bad("stsz"))?;
    info.sample_bytes = if fixed != 0 {
        u64::from(fixed) * u64::from(info.frames)
    } else {
        (0..info.frames as usize).map(|i| be32(stsz, 12 + i * 4).map(u64::from)).sum::<Option<u64>>().ok_or(bad("stsz"))?
    };
    info.keyframes = match find(stbl, b"stss") {
        Some(stss) => be32(stss, 4).ok_or(bad("stss"))?,
        None => info.frames,
    };
    let stts = find(stbl, b"stts").ok_or(bad("no stts"))?;
    let entries = be32(stts, 4).ok_or(bad("stts"))? as usize;
    let mut samples = 0u64;
    let mut ticks = 0u64;
    for i in 0..entries.min(1 << 20) {
        let n = be32(stts, 8 + i * 8).ok_or(bad("stts"))?;
        let d = be32(stts, 12 + i * 8).ok_or(bad("stts"))?;
        samples += u64::from(n);
        ticks += u64::from(n) * u64::from(d);
    }
    if samples != u64::from(info.frames) {
        return Err(bad("stts and stsz count different samples"));
    }
    if ticks > 0 {
        info.fps = samples as f64 * f64::from(timescale) / ticks as f64;
    }
    Ok(())
}

fn be16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// The boxes in `data`, in order (None when a size is wrong).
fn children(data: &[u8]) -> Option<Vec<([u8; 4], &[u8])>> {
    let mut out = vec![];
    let mut at = 0usize;
    while at < data.len() {
        let size = be32(data, at)? as usize;
        let kind: [u8; 4] = data.get(at + 4..at + 8)?.try_into().ok()?;
        if size < 8 {
            return None;
        }
        let body = data.get(at + 8..at.checked_add(size)?)?;
        out.push((kind, body));
        at += size;
    }
    Some(out)
}

/// The body of the first child box of type `kind` (full boxes keep their version and flags).
fn find<'a>(data: &'a [u8], kind: &[u8; 4]) -> Option<&'a [u8]> {
    children(data)?.into_iter().find(|(k, _)| k == kind).map(|(_, b)| b)
}
