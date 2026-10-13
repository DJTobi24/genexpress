//! The deblocking filter (Rec. ITU-T H.264 clause 8.7) for progressive frames with one QP: it
//! smooths the edges of 4x4 blocks where the coding left a visible step, after the whole picture
//! has been reconstructed. Decoders apply the same filter, and the filtered picture is the
//! reference for the next one, so this must match the specification exactly.

use super::MbState;
use super::tables::{ALPHA, BETA, TC0, chroma_qp};
use crate::yuv::Picture;

/// Boundary strength (clause 8.7.2.1) of the edge between luma 4x4 blocks `p` and `q` (raster
/// positions in their macroblocks `mp` and `mq`); `mb_edge` when they are in different macroblocks.
fn strength(mp: &MbState, p: usize, mq: &MbState, q: usize, mb_edge: bool) -> u8 {
    if mp.intra || mq.intra {
        return if mb_edge { 4 } else { 3 };
    }
    if mp.nz_y[p] > 0 || mq.nz_y[q] > 0 {
        return 2;
    }
    // One motion vector each, the same reference picture.
    if (mp.mv.0 - mq.mv.0).abs() >= 4 || (mp.mv.1 - mq.mv.1).abs() >= 4 {
        return 1;
    }
    0
}

/// A plane and how to step through it.
struct Edge<'a> {
    data: &'a mut [u8],
    stride: usize,
    alpha: i32,
    beta: i32,
    index_a: usize,
    chroma: bool,
}

impl Edge<'_> {
    /// Filters the set of samples across an edge (clause 8.7.2): `q0` at index `at`, `step` the
    /// distance between samples across the edge (1 for a vertical edge, the stride for a
    /// horizontal one).
    fn filter(&mut self, at: usize, step: usize, bs: u8) {
        if bs == 0 || at < 4 * step {
            return;
        }
        let get = |d: &[u8], i: usize| i32::from(d.get(i).copied().unwrap_or(0));
        let (p0, p1, p2, p3) =
            (get(self.data, at - step), get(self.data, at - 2 * step), get(self.data, at - 3 * step), get(self.data, at - 4 * step));
        let (q0, q1, q2, q3) = (get(self.data, at), get(self.data, at + step), get(self.data, at + 2 * step), get(self.data, at + 3 * step));
        let (alpha, beta) = (self.alpha, self.beta);
        if !((p0 - q0).abs() < alpha && (p1 - p0).abs() < beta && (q1 - q0).abs() < beta) {
            return;
        }
        let ap = (p2 - p0).abs();
        let aq = (q2 - q0).abs();
        let mut out = [(at - step, p0), (at - 2 * step, p1), (at - 3 * step, p2), (at, q0), (at + step, q1), (at + 2 * step, q2)];
        if bs < 4 {
            let tc0 = i32::from(TC0[usize::from(bs - 1)][self.index_a]);
            let tc = if self.chroma { tc0 + 1 } else { tc0 + i32::from(ap < beta) + i32::from(aq < beta) };
            let delta = ((((q0 - p0) << 2) + (p1 - q1) + 4) >> 3).clamp(-tc, tc);
            out[0].1 = (p0 + delta).clamp(0, 255);
            out[3].1 = (q0 - delta).clamp(0, 255);
            if !self.chroma && ap < beta {
                out[1].1 = p1 + ((p2 + ((p0 + q0 + 1) >> 1) - (p1 << 1)) >> 1).clamp(-tc0, tc0);
            }
            if !self.chroma && aq < beta {
                out[4].1 = q1 + ((q2 + ((p0 + q0 + 1) >> 1) - (q1 << 1)) >> 1).clamp(-tc0, tc0);
            }
        } else {
            let strong = (p0 - q0).abs() < (alpha >> 2) + 2;
            if !self.chroma && ap < beta && strong {
                out[0].1 = (p2 + 2 * p1 + 2 * p0 + 2 * q0 + q1 + 4) >> 3;
                out[1].1 = (p2 + p1 + p0 + q0 + 2) >> 2;
                out[2].1 = (2 * p3 + 3 * p2 + p1 + p0 + q0 + 4) >> 3;
            } else {
                out[0].1 = (2 * p1 + p0 + q1 + 2) >> 2;
            }
            if !self.chroma && aq < beta && strong {
                out[3].1 = (p1 + 2 * p0 + 2 * q0 + 2 * q1 + q2 + 4) >> 3;
                out[4].1 = (p0 + q0 + q1 + q2 + 2) >> 2;
                out[5].1 = (2 * q3 + 3 * q2 + q1 + q0 + p0 + 4) >> 3;
            } else {
                out[3].1 = (2 * q1 + q0 + p1 + 2) >> 2;
            }
        }
        for (i, v) in out {
            if let Some(s) = self.data.get_mut(i) {
                *s = v.clamp(0, 255) as u8;
            }
        }
    }
}

/// Filters every macroblock of `pic` in order (luma, then both chroma planes; vertical edges
/// before horizontal ones). `mbs` are the picture's macroblocks, `qp` their QP.
pub(crate) fn deblock(pic: &mut Picture, mbs: &[MbState], mb_w: usize, mb_h: usize, qp: u8) {
    let qp = usize::from(qp.min(51));
    let qpc = usize::from(chroma_qp(qp as u8));
    let none = MbState::default();
    let mb = |x: usize, y: usize| mbs.get(y * mb_w + x).unwrap_or(&none);
    let width = pic.width;
    for mby in 0..mb_h {
        for mbx in 0..mb_w {
            let cur = mb(mbx, mby);
            // bS of each luma edge segment [edge 0..4][segment 0..4], vertical and horizontal.
            let vertical: [[u8; 4]; 4] = core::array::from_fn(|e| {
                core::array::from_fn(|s| {
                    // Edge e: q is block (e, s), p the block to its left.
                    if e > 0 {
                        strength(cur, s * 4 + e - 1, cur, s * 4 + e, false)
                    } else if mbx > 0 {
                        strength(mb(mbx - 1, mby), s * 4 + 3, cur, s * 4, true)
                    } else {
                        0
                    }
                })
            });
            let horizontal: [[u8; 4]; 4] = core::array::from_fn(|e| {
                core::array::from_fn(|s| {
                    // Edge e: q is block (s, e), p the block above it.
                    if e > 0 {
                        strength(cur, (e - 1) * 4 + s, cur, e * 4 + s, false)
                    } else if mby > 0 {
                        strength(mb(mbx, mby - 1), 12 + s, cur, s, true)
                    } else {
                        0
                    }
                })
            });
            let mut luma =
                Edge { data: &mut pic.y, stride: width, alpha: i32::from(ALPHA[qp]), beta: i32::from(BETA[qp]), index_a: qp, chroma: false };
            let (x0, y0) = (mbx * 16, mby * 16);
            if vertical.iter().chain(&horizontal).flatten().all(|&b| b == 0) {
                continue;
            }
            for (e, segments) in vertical.iter().enumerate() {
                for (s, &b) in segments.iter().enumerate().filter(|(_, b)| **b > 0) {
                    for k in s * 4..s * 4 + 4 {
                        luma.filter((y0 + k) * width + x0 + e * 4, 1, b);
                    }
                }
            }
            for (e, segments) in horizontal.iter().enumerate() {
                for (s, &b) in segments.iter().enumerate().filter(|(_, b)| **b > 0) {
                    for k in s * 4..s * 4 + 4 {
                        luma.filter((y0 + e * 4) * width + x0 + k, width, b);
                    }
                }
            }
            let cw = width / 2;
            let (cx0, cy0) = (mbx * 8, mby * 8);
            for plane in [&mut pic.u, &mut pic.v] {
                let mut c = Edge { data: plane, stride: cw, alpha: i32::from(ALPHA[qpc]), beta: i32::from(BETA[qpc]), index_a: qpc, chroma: true };
                // Chroma edges 0 and 4 lie on luma edges 0 and 8; chroma row k on luma row 2k.
                for e in 0..2 {
                    for k in 0..8 {
                        c.filter((cy0 + k) * cw + cx0 + e * 4, 1, vertical[e * 2][k / 2]);
                    }
                }
                for e in 0..2 {
                    for k in 0..8 {
                        c.filter((cy0 + e * 4) * cw + cx0 + k, c.stride, horizontal[e * 2][k / 2]);
                    }
                }
            }
        }
    }
}
