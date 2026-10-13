//! Video export for GenHa (layer L3): an H.264 encoder and an MP4 writer, in pure Rust.
//!
//! ```text
//! RGBA frames ─► Y′CbCr 4:2:0 (BT.709) ─► H.264 Constrained Baseline ─► MP4 (ftyp, moov, mdat)
//! ```
//!
//! [`Mp4Writer`] takes frames of one size at a constant frame rate and returns the bytes of an
//! `.mp4` file that browsers, QuickTime, Windows and phones play. The encoder
//! ([`h264`](crate::h264)) was written from the public H.264 specification (Rec. ITU-T H.264); it
//! has no C code and no `unsafe`, so it builds everywhere GenHa does, the browser included.
//!
//! Patents: H.264 is covered by patents licensed through a pool. This crate's licence (MIT OR
//! Apache-2.0) covers its code only; whoever distributes a product that encodes H.264 should check
//! what applies to them (see `docs/architecture.md` › Video).
#![forbid(unsafe_code)]

mod bits;
mod h264;
mod mp4;
mod yuv;

pub use mp4::{Mp4Info, inspect};

/// Largest frame side, in pixels.
pub const MAX_SIDE: u32 = 4096;
/// Largest frame, in macroblocks (16 × 16 pixels): H.264 level 4.1's limit, which 1920 × 1080
/// and 1080 × 1920 fit.
pub const MAX_MACROBLOCKS: u32 = 8192;
/// Highest frame rate.
pub const MAX_FPS: u32 = 60;
/// Most frames in one video.
pub const MAX_FRAMES: usize = 36_000;

/// Something the video writer could not do.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VideoError {
    #[error("the video frame size {0} × {1} is not supported (2–{MAX_SIDE} pixels a side, at most {MAX_MACROBLOCKS} macroblocks)")]
    BadSize(u32, u32),
    #[error("the frame rate must be 1–{MAX_FPS} frames per second")]
    BadFrameRate,
    #[error("a frame has {got} bytes, {want} expected")]
    BadFrame { got: usize, want: usize },
    #[error("the video has no frames")]
    NoFrames,
    #[error("the video is too long")]
    TooLarge,
    #[error("not an MP4 file this writer understands: {0}")]
    Malformed(&'static str),
}

/// How much the encoder may lose for a smaller file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Quality {
    Low,
    #[default]
    Medium,
    High,
}

impl Quality {
    /// `low`, `medium` or `high` (any case).
    pub fn parse(s: &str) -> Option<Quality> {
        match s.trim().to_ascii_lowercase().as_str() {
            "low" => Some(Quality::Low),
            "medium" | "normal" | "default" => Some(Quality::Medium),
            "high" | "best" => Some(Quality::High),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Quality::Low => "low",
            Quality::Medium => "medium",
            Quality::High => "high",
        }
    }

    /// The quantization parameter (constant quality; lower is better).
    fn qp(self) -> u8 {
        match self {
            Quality::Low => 30,
            Quality::Medium => 25,
            Quality::High => 20,
        }
    }
}

/// Settings of a video.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VideoOptions {
    /// Frame size in pixels. Odd sizes are rounded up to even (the last column or row repeats).
    pub width: u32,
    pub height: u32,
    /// Frames per second (1–60).
    pub fps: u32,
    pub quality: Quality,
    /// Seconds between key frames (where players can seek to).
    pub keyframe_seconds: u32,
}

impl VideoOptions {
    pub fn new(width: u32, height: u32, fps: u32) -> VideoOptions {
        VideoOptions { width, height, fps, quality: Quality::Medium, keyframe_seconds: 2 }
    }

    /// The size of the video (even width and height).
    pub fn output_size(&self) -> (u32, u32) {
        (self.width + (self.width & 1), self.height + (self.height & 1))
    }
}

/// Builds an MP4 video frame by frame.
pub struct Mp4Writer {
    opts: VideoOptions,
    encoder: h264::Encoder,
    samples: Vec<Vec<u8>>,
    sync: Vec<bool>,
}

impl Mp4Writer {
    /// A writer for frames of `opts.width` × `opts.height`.
    pub fn new(opts: VideoOptions) -> Result<Mp4Writer, VideoError> {
        let (w, h) = opts.output_size();
        let mbs = u64::from(w.div_ceil(16)) * u64::from(h.div_ceil(16));
        if opts.width < 2 || opts.height < 2 || w > MAX_SIDE || h > MAX_SIDE || mbs > u64::from(MAX_MACROBLOCKS) {
            return Err(VideoError::BadSize(opts.width, opts.height));
        }
        if opts.fps == 0 || opts.fps > MAX_FPS {
            return Err(VideoError::BadFrameRate);
        }
        let keyint = opts.fps.saturating_mul(opts.keyframe_seconds.clamp(1, 60));
        let config = h264::Config { width: w as usize, height: h as usize, fps: opts.fps, qp: opts.quality.qp(), keyint, deblock: true };
        let encoder = h264::Encoder::new(config);
        Ok(Mp4Writer { opts, encoder, samples: vec![], sync: vec![] })
    }

    /// Frames written so far.
    pub fn frames(&self) -> usize {
        self.samples.len()
    }

    /// Adds a frame: `rgba` holds `width` × `height` pixels, 4 bytes each (alpha is ignored).
    /// `keyframe` asks for a key frame (say, at the start of a new scene).
    pub fn push_rgba(&mut self, rgba: &[u8], keyframe: bool) -> Result<(), VideoError> {
        let want = self.opts.width as usize * self.opts.height as usize * 4;
        if rgba.len() != want {
            return Err(VideoError::BadFrame { got: rgba.len(), want });
        }
        self.check_room()?;
        let (pw, ph) = self.encoder.coded_size();
        let pic = yuv::from_rgba(rgba, self.opts.width as usize, self.opts.height as usize, pw, ph);
        let frame = self.encoder.encode(&pic, keyframe);
        self.samples.push(frame.data);
        self.sync.push(frame.keyframe);
        Ok(())
    }

    /// Shows the previous frame for one more frame (a few bytes: nothing changed). Before any
    /// frame, a grey frame.
    pub fn repeat(&mut self) -> Result<(), VideoError> {
        self.check_room()?;
        let frame = self.encoder.repeat();
        self.samples.push(frame.data);
        self.sync.push(frame.keyframe);
        Ok(())
    }

    fn check_room(&self) -> Result<(), VideoError> {
        if self.samples.len() >= MAX_FRAMES { Err(VideoError::TooLarge) } else { Ok(()) }
    }

    /// The MP4 file.
    pub fn finish(self) -> Result<Vec<u8>, VideoError> {
        let (width, height) = self.opts.output_size();
        let track =
            mp4::Track { width, height, fps: self.opts.fps, sps: self.encoder.sps(), pps: self.encoder.pps(), profile: self.encoder.profile() };
        mp4::write(&track, &self.samples, &self.sync)
    }
}

#[cfg(test)]
mod tests;
