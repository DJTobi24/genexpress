//! Intra prediction (Rec. ITU-T H.264 clauses 8.3.1.2, 8.3.3 and 8.3.4): the prediction a decoder
//! forms from already reconstructed samples, which the encoder must reproduce exactly.

/// A plane of samples.
#[derive(Clone, Copy)]
pub(crate) struct Plane<'a> {
    pub data: &'a [u8],
    pub stride: usize,
}

impl Plane<'_> {
    /// The sample at (`x`, `y`); 128 outside the plane (never read for available neighbours).
    pub(crate) fn at(&self, x: usize, y: usize) -> i32 {
        let i = y.checked_mul(self.stride).and_then(|v| v.checked_add(x));
        i32::from(i.and_then(|i| self.data.get(i)).copied().unwrap_or(128))
    }
}

fn clip(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

/// Which neighbours of a block have been reconstructed (and are in the picture).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Avail {
    pub left: bool,
    pub top: bool,
    pub top_left: bool,
    pub top_right: bool,
}

/// Intra_16x16 prediction of the macroblock at sample (`x0`, `y0`) in `mode` (0 vertical,
/// 1 horizontal, 2 DC, 3 plane), or None when the mode needs neighbours that are missing.
pub(crate) fn pred16(p: Plane, x0: usize, y0: usize, mode: u8, a: Avail) -> Option<[u8; 256]> {
    let top = |x: usize| p.at(x0 + x, y0.wrapping_sub(1));
    let left = |y: usize| p.at(x0.wrapping_sub(1), y0 + y);
    let mut out = [0u8; 256];
    match mode {
        0 if a.top => {
            for y in 0..16 {
                for x in 0..16 {
                    out[y * 16 + x] = top(x) as u8;
                }
            }
        }
        1 if a.left => {
            for y in 0..16 {
                let v = left(y) as u8;
                out[y * 16..y * 16 + 16].fill(v);
            }
        }
        2 => {
            let st: i32 = (0..16).map(top).sum();
            let sl: i32 = (0..16).map(left).sum();
            let v = match (a.top, a.left) {
                (true, true) => (st + sl + 16) >> 5,
                (false, true) => (sl + 8) >> 4,
                (true, false) => (st + 8) >> 4,
                (false, false) => 128,
            };
            out.fill(v as u8);
        }
        3 if a.top && a.left && a.top_left => {
            let corner = p.at(x0.wrapping_sub(1), y0.wrapping_sub(1));
            // p[x, −1] for x = −1..15 and p[−1, y] for y = −1..15, with index 0 the corner.
            let t = |x: i32| if x < 0 { corner } else { top(x as usize) };
            let l = |y: i32| if y < 0 { corner } else { left(y as usize) };
            let h: i32 = (0..8).map(|i| (i + 1) * (t(8 + i) - t(6 - i))).sum();
            let v: i32 = (0..8).map(|i| (i + 1) * (l(8 + i) - l(6 - i))).sum();
            let aa = 16 * (l(15) + t(15));
            let b = (5 * h + 32) >> 6;
            let c = (5 * v + 32) >> 6;
            for y in 0..16 {
                for x in 0..16 {
                    out[y * 16 + x] = clip((aa + b * (x as i32 - 7) + c * (y as i32 - 7) + 16) >> 5);
                }
            }
        }
        _ => return None,
    }
    Some(out)
}

/// Intra chroma prediction of an 8x8 chroma block at (`x0`, `y0`) in `mode` (0 DC, 1 horizontal,
/// 2 vertical, 3 plane), or None when the mode needs missing neighbours.
pub(crate) fn pred_chroma(p: Plane, x0: usize, y0: usize, mode: u8, a: Avail) -> Option<[u8; 64]> {
    let top = |x: usize| p.at(x0 + x, y0.wrapping_sub(1));
    let left = |y: usize| p.at(x0.wrapping_sub(1), y0 + y);
    let mut out = [0u8; 64];
    match mode {
        0 => {
            for (bx, by) in [(0usize, 0usize), (1, 0), (0, 1), (1, 1)] {
                let (xo, yo) = (bx * 4, by * 4);
                let st: i32 = (0..4).map(|x| top(xo + x)).sum();
                let sl: i32 = (0..4).map(|y| left(yo + y)).sum();
                let both = (st + sl + 4) >> 3;
                let only_top = (st + 2) >> 2;
                let only_left = (sl + 2) >> 2;
                let v = if (xo == 0 && yo == 0) || (xo > 0 && yo > 0) {
                    match (a.top, a.left) {
                        (true, true) => both,
                        (false, true) => only_left,
                        (true, false) => only_top,
                        (false, false) => 128,
                    }
                } else if xo > 0 {
                    // Top-right block: the samples above first.
                    if a.top {
                        only_top
                    } else if a.left {
                        only_left
                    } else {
                        128
                    }
                } else if a.left {
                    // Bottom-left block: the samples to the left first.
                    only_left
                } else if a.top {
                    only_top
                } else {
                    128
                };
                for y in 0..4 {
                    for x in 0..4 {
                        out[(yo + y) * 8 + xo + x] = v as u8;
                    }
                }
            }
        }
        1 if a.left => {
            for y in 0..8 {
                let v = left(y) as u8;
                out[y * 8..y * 8 + 8].fill(v);
            }
        }
        2 if a.top => {
            for y in 0..8 {
                for x in 0..8 {
                    out[y * 8 + x] = top(x) as u8;
                }
            }
        }
        3 if a.top && a.left && a.top_left => {
            let corner = p.at(x0.wrapping_sub(1), y0.wrapping_sub(1));
            let t = |x: i32| if x < 0 { corner } else { top(x as usize) };
            let l = |y: i32| if y < 0 { corner } else { left(y as usize) };
            let h: i32 = (0..4).map(|i| (i + 1) * (t(4 + i) - t(2 - i))).sum();
            let v: i32 = (0..4).map(|i| (i + 1) * (l(4 + i) - l(2 - i))).sum();
            let aa = 16 * (l(7) + t(7));
            let b = (34 * h + 32) >> 6;
            let c = (34 * v + 32) >> 6;
            for y in 0..8 {
                for x in 0..8 {
                    out[y * 8 + x] = clip((aa + b * (x as i32 - 3) + c * (y as i32 - 3) + 16) >> 5);
                }
            }
        }
        _ => return None,
    }
    Some(out)
}

/// Intra_4x4 prediction of the 4x4 block at (`x0`, `y0`) in `mode` (0–8, Table 8-2), or None when
/// the mode needs missing neighbours.
pub(crate) fn pred4(p: Plane, x0: usize, y0: usize, mode: u8, a: Avail) -> Option<[u8; 16]> {
    // The 13 neighbours: top[0..8] = p[0..7, −1], left[0..4] = p[−1, 0..3], corner = p[−1, −1].
    let mut top = [128i32; 8];
    if a.top {
        for (x, t) in top.iter_mut().enumerate().take(4) {
            *t = p.at(x0 + x, y0.wrapping_sub(1));
        }
        for x in 4..8 {
            top[x] = if a.top_right { p.at(x0 + x, y0.wrapping_sub(1)) } else { top[3] };
        }
    }
    let mut left = [128i32; 4];
    if a.left {
        for (y, l) in left.iter_mut().enumerate() {
            *l = p.at(x0.wrapping_sub(1), y0 + y);
        }
    }
    let corner = if a.top_left { p.at(x0.wrapping_sub(1), y0.wrapping_sub(1)) } else { 128 };
    // p[x, −1] for x ≥ −1 and p[−1, y] for y ≥ −1.
    let t = |x: i32| if x < 0 { corner } else { top.get(x as usize).copied().unwrap_or(128) };
    let l = |y: i32| if y < 0 { corner } else { left.get(y as usize).copied().unwrap_or(128) };
    let mut out = [0u8; 16];
    let mut set = |f: &dyn Fn(i32, i32) -> i32| {
        for y in 0..4 {
            for x in 0..4 {
                out[(y * 4 + x) as usize] = f(x, y) as u8;
            }
        }
    };
    match mode {
        0 if a.top => set(&|x, _| t(x)),
        1 if a.left => set(&|_, y| l(y)),
        2 => {
            let st: i32 = (0..4).map(t).sum();
            let sl: i32 = (0..4).map(l).sum();
            let v = match (a.top, a.left) {
                (true, true) => (st + sl + 4) >> 3,
                (false, true) => (sl + 2) >> 2,
                (true, false) => (st + 2) >> 2,
                (false, false) => 128,
            };
            set(&|_, _| v);
        }
        3 if a.top => set(&|x, y| {
            if x == 3 && y == 3 { (t(6) + 3 * t(7) + 2) >> 2 } else { (t(x + y) + 2 * t(x + y + 1) + t(x + y + 2) + 2) >> 2 }
        }),
        4 if a.top && a.left && a.top_left => set(&|x, y| {
            if x > y {
                (t(x - y - 2) + 2 * t(x - y - 1) + t(x - y) + 2) >> 2
            } else if x < y {
                (l(y - x - 2) + 2 * l(y - x - 1) + l(y - x) + 2) >> 2
            } else {
                (t(0) + 2 * corner + l(0) + 2) >> 2
            }
        }),
        5 if a.top && a.left && a.top_left => set(&|x, y| {
            let z = 2 * x - y;
            match z {
                0 | 2 | 4 | 6 => (t(x - (y >> 1) - 1) + t(x - (y >> 1)) + 1) >> 1,
                1 | 3 | 5 => (t(x - (y >> 1) - 2) + 2 * t(x - (y >> 1) - 1) + t(x - (y >> 1)) + 2) >> 2,
                -1 => (l(0) + 2 * corner + t(0) + 2) >> 2,
                _ => (l(y - 1) + 2 * l(y - 2) + l(y - 3) + 2) >> 2,
            }
        }),
        6 if a.top && a.left && a.top_left => set(&|x, y| {
            let z = 2 * y - x;
            match z {
                0 | 2 | 4 | 6 => (l(y - (x >> 1) - 1) + l(y - (x >> 1)) + 1) >> 1,
                1 | 3 | 5 => (l(y - (x >> 1) - 2) + 2 * l(y - (x >> 1) - 1) + l(y - (x >> 1)) + 2) >> 2,
                -1 => (l(0) + 2 * corner + t(0) + 2) >> 2,
                _ => (t(x - 1) + 2 * t(x - 2) + t(x - 3) + 2) >> 2,
            }
        }),
        7 if a.top => set(&|x, y| {
            if y % 2 == 0 {
                (t(x + (y >> 1)) + t(x + (y >> 1) + 1) + 1) >> 1
            } else {
                (t(x + (y >> 1)) + 2 * t(x + (y >> 1) + 1) + t(x + (y >> 1) + 2) + 2) >> 2
            }
        }),
        8 if a.left => set(&|x, y| {
            let z = x + 2 * y;
            match z {
                0 | 2 | 4 => (l(y + (x >> 1)) + l(y + (x >> 1) + 1) + 1) >> 1,
                1 | 3 => (l(y + (x >> 1)) + 2 * l(y + (x >> 1) + 1) + l(y + (x >> 1) + 2) + 2) >> 2,
                5 => (l(2) + 3 * l(3) + 2) >> 2,
                _ => l(3),
            }
        }),
        _ => return None,
    }
    Some(out)
}
