//! The 4x4 integer transform, its DC transforms and quantization.
//!
//! The inverse side (scaling and inverse transform) follows Rec. ITU-T H.264 clauses 8.5.10–8.5.12
//! exactly, since the encoder must reconstruct what every decoder reconstructs. The forward side is
//! the encoder's own choice: the forward core transform whose inverse the decoder applies, and
//! quantization factors derived from the decoder's scaling factors (equation 8-315), so that
//! scaling a quantized level back gives the transform coefficient it stands for.
//!
//! Blocks are 16 values in raster order (row * 4 + column); for coefficients, row and column are
//! the vertical and horizontal frequency.

use super::tables::{NORM_ADJUST, position_class};

/// Largest level magnitude written: CAVLC in the Baseline profile (level_prefix at most 15) can code
/// every level up to this in any context.
pub(crate) const MAX_LEVEL: i32 = 2063;

/// LevelScale4x4(m, i, j) of equation 8-313 with the flat scaling matrix (weightScale = 16).
fn level_scale(m: usize, pos: usize) -> i32 {
    16 * NORM_ADJUST[m % 6][position_class(pos)]
}

/// Forward quantization factor for `qP % 6 == m` at `pos`: round(2^21 / (k · v)), where `v` is the
/// decoder's scaling factor and `k` (16, 25 or 20) the squared norm of the forward transform's
/// basis functions for that position, so that `level · v · 2^(qP/6) / 64 ≈ coefficient · norm`.
const fn quant_factor(m: usize, class: usize) -> i64 {
    let k: i64 = match class {
        0 => 16,
        1 => 25,
        _ => 20,
    };
    let v = NORM_ADJUST[m][class] as i64;
    ((1i64 << 21) + k * v / 2) / (k * v)
}

const fn quant_table() -> [[i64; 3]; 6] {
    let mut t = [[0i64; 3]; 6];
    let mut m = 0;
    while m < 6 {
        let mut c = 0;
        while c < 3 {
            t[m][c] = quant_factor(m, c);
            c += 1;
        }
        m += 1;
    }
    t
}

const QUANT: [[i64; 3]; 6] = quant_table();

/// Forward core transform of a 4x4 residual block (rows, then columns).
pub(crate) fn forward4(x: &[i32; 16]) -> [i32; 16] {
    let mut t = [0i32; 16];
    for r in 0..4 {
        let [a, b, c, d] = [x[r * 4], x[r * 4 + 1], x[r * 4 + 2], x[r * 4 + 3]];
        let (s0, s1, d0, d1) = (a + d, b + c, a - d, b - c);
        t[r * 4] = s0 + s1;
        t[r * 4 + 1] = 2 * d0 + d1;
        t[r * 4 + 2] = s0 - s1;
        t[r * 4 + 3] = d0 - 2 * d1;
    }
    let mut out = [0i32; 16];
    for c in 0..4 {
        let [a, b, cc, d] = [t[c], t[4 + c], t[8 + c], t[12 + c]];
        let (s0, s1, d0, d1) = (a + d, b + cc, a - d, b - cc);
        out[c] = s0 + s1;
        out[4 + c] = 2 * d0 + d1;
        out[8 + c] = s0 - s1;
        out[12 + c] = d0 - 2 * d1;
    }
    out
}

/// Inverse transform of scaled coefficients `d` to residual samples (clause 8.5.12.2).
pub(crate) fn inverse4(d: &[i32; 16]) -> [i32; 16] {
    let mut f = [0i32; 16];
    for i in 0..4 {
        let r = &d[i * 4..i * 4 + 4];
        let e0 = r[0] + r[2];
        let e1 = r[0] - r[2];
        let e2 = (r[1] >> 1) - r[3];
        let e3 = r[1] + (r[3] >> 1);
        f[i * 4] = e0 + e3;
        f[i * 4 + 1] = e1 + e2;
        f[i * 4 + 2] = e1 - e2;
        f[i * 4 + 3] = e0 - e3;
    }
    let mut out = [0i32; 16];
    for j in 0..4 {
        let g0 = f[j] + f[8 + j];
        let g1 = f[j] - f[8 + j];
        let g2 = (f[4 + j] >> 1) - f[12 + j];
        let g3 = f[4 + j] + (f[12 + j] >> 1);
        out[j] = (g0 + g3 + 32) >> 6;
        out[4 + j] = (g1 + g2 + 32) >> 6;
        out[8 + j] = (g1 - g2 + 32) >> 6;
        out[12 + j] = (g0 - g3 + 32) >> 6;
    }
    out
}

/// Rounds `|w| · factor` down by `shift` bits with `offset` added, keeping the sign, capped at
/// [`MAX_LEVEL`].
fn quantize(w: i32, factor: i64, offset: i64, shift: u32) -> i32 {
    let q = ((i64::from(w).abs() * factor + offset) >> shift).min(i64::from(MAX_LEVEL)) as i32;
    if w < 0 { -q } else { q }
}

/// The rounding offset: a third of a step for intra blocks, a sixth for inter blocks.
fn offset(shift: u32, intra: bool) -> i64 {
    (1i64 << shift) / if intra { 3 } else { 6 }
}

/// Quantizes transform coefficients `w` at `qp`. With `skip_dc` the DC position is left 0 (it is
/// coded by a DC transform).
pub(crate) fn quant4(w: &[i32; 16], qp: u8, intra: bool, skip_dc: bool) -> [i32; 16] {
    let m = usize::from(qp % 6);
    let shift = 15 + u32::from(qp / 6);
    let off = offset(shift, intra);
    let mut out = [0i32; 16];
    for (pos, (o, &v)) in out.iter_mut().zip(w).enumerate() {
        if skip_dc && pos == 0 {
            continue;
        }
        *o = quantize(v, QUANT[m][position_class(pos)], off, shift);
    }
    out
}

/// Scales levels `c` back to coefficients (clause 8.5.12.1). With `dc` the DC position takes
/// that value as it is (Intra_16x16 and chroma blocks: it comes from the DC transform).
pub(crate) fn dequant4(c: &[i32; 16], qp: u8, dc: Option<i32>) -> [i32; 16] {
    let m = usize::from(qp % 6);
    let q6 = u32::from(qp / 6);
    let mut d = [0i32; 16];
    for (pos, (o, &v)) in d.iter_mut().zip(c).enumerate() {
        if pos == 0
            && let Some(dc) = dc
        {
            *o = dc;
            continue;
        }
        let s = v * level_scale(m, pos);
        *o = if q6 >= 4 { s << (q6 - 4) } else { (s + (1 << (3 - q6))) >> (4 - q6) };
    }
    d
}

/// The 4x4 Hadamard transform `H · c · H` (equation 8-320; also the forward luma DC transform).
pub(crate) fn hadamard4(c: &[i32; 16]) -> [i32; 16] {
    let mut t = [0i32; 16];
    for r in 0..4 {
        let [a, b, cc, d] = [c[r * 4], c[r * 4 + 1], c[r * 4 + 2], c[r * 4 + 3]];
        t[r * 4] = a + b + cc + d;
        t[r * 4 + 1] = a + b - cc - d;
        t[r * 4 + 2] = a - b - cc + d;
        t[r * 4 + 3] = a - b + cc - d;
    }
    let mut out = [0i32; 16];
    for col in 0..4 {
        let [a, b, cc, d] = [t[col], t[4 + col], t[8 + col], t[12 + col]];
        out[col] = a + b + cc + d;
        out[4 + col] = a + b - cc - d;
        out[8 + col] = a - b - cc + d;
        out[12 + col] = a - b + cc - d;
    }
    out
}

/// Quantizes the Intra_16x16 DC coefficients: `dc` holds the DC coefficient of each 4x4 block
/// (raster order of the blocks); the result is the levels as a 4x4 frequency array.
pub(crate) fn quant_luma_dc(dc: &[i32; 16], qp: u8) -> [i32; 16] {
    let m = usize::from(qp % 6);
    let shift = 17 + u32::from(qp / 6);
    let off = offset(shift, true);
    let f = hadamard4(dc);
    let mut out = [0i32; 16];
    for (o, &v) in out.iter_mut().zip(&f) {
        *o = quantize(v, QUANT[m][0], off, shift);
    }
    out
}

/// Scales Intra_16x16 DC levels back (clause 8.5.10): the DC value of each 4x4 block, raster order.
pub(crate) fn dequant_luma_dc(c: &[i32; 16], qp: u8) -> [i32; 16] {
    let f = hadamard4(c);
    let ls = level_scale(usize::from(qp % 6), 0);
    let q6 = u32::from(qp / 6);
    let mut out = [0i32; 16];
    for (o, &v) in out.iter_mut().zip(&f) {
        *o = if q6 >= 6 { (v * ls) << (q6 - 6) } else { (v * ls + (1 << (5 - q6))) >> (6 - q6) };
    }
    out
}

/// The 2x2 transform of chroma DC values (equation 8-324; also the forward transform).
pub(crate) fn hadamard2(c: &[i32; 4]) -> [i32; 4] {
    let [a, b, cc, d] = *c;
    [a + b + cc + d, a - b + cc - d, a + b - cc - d, a - b - cc + d]
}

/// Quantizes the chroma DC coefficients of the four 4x4 blocks of one component.
pub(crate) fn quant_chroma_dc(dc: &[i32; 4], qp: u8, intra: bool) -> [i32; 4] {
    let m = usize::from(qp % 6);
    let shift = 16 + u32::from(qp / 6);
    let off = offset(shift, intra);
    hadamard2(dc).map(|v| quantize(v, QUANT[m][0], off, shift))
}

/// Scales chroma DC levels back (clause 8.5.11.2, 4:2:0): the DC value of each 4x4 block.
pub(crate) fn dequant_chroma_dc(c: &[i32; 4], qp: u8) -> [i32; 4] {
    let ls = level_scale(usize::from(qp % 6), 0);
    let q6 = u32::from(qp / 6);
    hadamard2(c).map(|f| ((f * ls) << q6) >> 5)
}
