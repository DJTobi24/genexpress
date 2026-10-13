//! RGBA frames to Y′CbCr 4:2:0 planes (BT.709 matrix, limited range), padded to whole macroblocks.

/// Three sample planes; the chroma planes are half the size of the luma plane each way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Picture {
    /// Luma width and height (multiples of 16).
    pub width: usize,
    pub height: usize,
    pub y: Vec<u8>,
    pub u: Vec<u8>,
    pub v: Vec<u8>,
}

impl Picture {
    /// A mid-grey picture.
    pub(crate) fn new(width: usize, height: usize) -> Picture {
        Picture { width, height, y: vec![128; width * height], u: vec![128; width * height / 4], v: vec![128; width * height / 4] }
    }

    /// Whether macroblock (`mbx`, `mby`) has the same samples here and in `other`.
    pub(crate) fn same_mb(&self, other: &Picture, mbx: usize, mby: usize) -> bool {
        if self.width != other.width || self.height != other.height {
            return false;
        }
        let rows = |a: &[u8], b: &[u8], stride: usize, size: usize| {
            (0..size).all(|r| {
                let i = (mby * size + r) * stride + mbx * size;
                a.get(i..i + size) == b.get(i..i + size)
            })
        };
        let cw = self.width / 2;
        rows(&self.y, &other.y, self.width, 16) && rows(&self.u, &other.u, cw, 8) && rows(&self.v, &other.v, cw, 8)
    }
}

// BT.709 (Kr = 0.2126, Kb = 0.0722), 8-bit limited range, in units of 1/65536.
const YR: i32 = 11966;
const YG: i32 = 40254;
const YB: i32 = 4064;
const UR: i32 = -6596;
const UG: i32 = -22188;
const UB: i32 = 28784;
const VR: i32 = 28784;
const VG: i32 = -26145;
const VB: i32 = -2639;

/// Converts `rgba` (`width` × `height`, 4 bytes per pixel, alpha ignored) into a picture of
/// `padded_w` × `padded_h` (even, at least the frame size); the padding repeats the last column
/// and row. Missing pixels (a short buffer) read as black.
pub(crate) fn from_rgba(rgba: &[u8], width: usize, height: usize, padded_w: usize, padded_h: usize) -> Picture {
    let mut pic = Picture::new(padded_w, padded_h);
    let (w, h) = (width.max(1), height.max(1));
    // Row `y` of the frame (rows below the frame repeat the last one; a short buffer gives none).
    let row = |y: usize| -> &[u8] {
        let start = y.min(h - 1) * w * 4;
        rgba.get(start..start + w * 4).unwrap_or_default()
    };
    // The pixel at `x` of a row (columns right of the frame repeat the last one; black when missing).
    let px = |r: &[u8], x: usize| -> [i32; 3] {
        let i = x.min(w - 1) * 4;
        r.get(i..i + 3).map_or([0, 0, 0], |p| [i32::from(p[0]), i32::from(p[1]), i32::from(p[2])])
    };
    let luma = |r: i32, g: i32, b: i32| ((YR * r + YG * g + YB * b + (16 << 16) + (1 << 15)) >> 16).clamp(0, 255) as u8;
    for y in 0..padded_h {
        let src = row(y);
        let Some(out) = pic.y.get_mut(y * padded_w..(y + 1) * padded_w) else { continue };
        let inside = src.len() / 4;
        for (o, p) in out.iter_mut().zip(src.as_chunks::<4>().0) {
            *o = luma(i32::from(p[0]), i32::from(p[1]), i32::from(p[2]));
        }
        let [r, g, b] = px(src, w - 1);
        let edge = luma(r, g, b);
        out.iter_mut().skip(inside).for_each(|o| *o = edge);
    }
    let cw = padded_w / 2;
    let chroma = |sum: [i32; 3]| {
        // Four pixels: the sums are 4× the mean.
        let [r, g, b] = sum;
        let u = ((UR * r + UG * g + UB * b + (512 << 16) + (1 << 17)) >> 18).clamp(0, 255) as u8;
        let v = ((VR * r + VG * g + VB * b + (512 << 16) + (1 << 17)) >> 18).clamp(0, 255) as u8;
        (u, v)
    };
    for cy in 0..padded_h / 2 {
        let (top, bottom) = (row(cy * 2), row(cy * 2 + 1));
        let at = cy * cw;
        let (Some(us), Some(vs)) = (pic.u.get_mut(at..at + cw), pic.v.get_mut(at..at + cw)) else { continue };
        // Whole 2x2 blocks inside the frame, then the edge (an odd last column, the padding).
        let pairs = (top.len().min(bottom.len()) / 8).min(cw);
        for ((u, v), (t, b)) in us.iter_mut().zip(vs.iter_mut()).zip(top.as_chunks::<8>().0.iter().zip(bottom.as_chunks::<8>().0)) {
            let sum = [0, 1, 2].map(|c| i32::from(t[c]) + i32::from(t[4 + c]) + i32::from(b[c]) + i32::from(b[4 + c]));
            (*u, *v) = chroma(sum);
        }
        for cx in pairs..cw {
            let mut sum = [0i32; 3];
            for p in [px(top, cx * 2), px(top, cx * 2 + 1), px(bottom, cx * 2), px(bottom, cx * 2 + 1)] {
                for (s, v) in sum.iter_mut().zip(p) {
                    *s += v;
                }
            }
            if let (Some(u), Some(v)) = (us.get_mut(cx), vs.get_mut(cx)) {
                (*u, *v) = chroma(sum);
            }
        }
    }
    pic
}

/// Converts limited-range BT.709 Y′CbCr back to RGB (for tests and checks).
#[cfg(test)]
pub(crate) fn to_rgb(y: u8, u: u8, v: u8) -> [u8; 3] {
    let (y, u, v) = (f64::from(y) - 16.0, f64::from(u) - 128.0, f64::from(v) - 128.0);
    let y = y * 255.0 / 219.0;
    let (u, v) = (u * 255.0 / 224.0, v * 255.0 / 224.0);
    let r = y + 1.5748 * v;
    let g = y - 0.1873 * u - 0.4681 * v;
    let b = y + 1.8556 * u;
    [r, g, b].map(|c| c.round().clamp(0.0, 255.0) as u8)
}
