//! CAVLC residual coding (Rec. ITU-T H.264 clauses 7.3.5.3.2 and 9.2), the encoder side: a block
//! of levels in scan order becomes coeff_token, trailing-one signs, levels, total_zeros and
//! run_before.

use super::tables::{
    COEFF_TOKEN_CHROMA_DC, COEFF_TOKEN_NC0, COEFF_TOKEN_NC2, COEFF_TOKEN_NC4, RUN_BEFORE, TOTAL_ZEROS, TOTAL_ZEROS_CHROMA_DC, Vlc, coeff_token_nc8,
};
use super::transform::MAX_LEVEL;
use crate::bits::BitWriter;

/// The table context of a block: `nc` as derived in clause 9.2.1, or [`CHROMA_DC`].
pub(crate) const CHROMA_DC: i32 = -1;

fn put(w: &mut BitWriter, code: Vlc) {
    w.put(code.1, u32::from(code.0));
}

/// Writes `levels` (in scan order; 4, 15 or 16 of them) as residual_block_cavlc() with table
/// context `nc`, and returns TotalCoeff (the number of non-zero levels).
pub(crate) fn write_block(w: &mut BitWriter, levels: &[i32], nc: i32) -> u8 {
    let max = levels.len();
    // Non-zero levels from the highest frequency down, with the zeros before each one.
    let mut nonzero: [(i32, usize); 16] = [(0, 0); 16];
    let mut total = 0usize;
    let mut last = None;
    for (i, &l) in levels.iter().enumerate() {
        if l != 0 {
            last = Some(i);
        }
    }
    let total_zeros = match last {
        Some(last_pos) => {
            let mut run = 0;
            for &l in levels.get(..=last_pos).unwrap_or_default().iter().rev() {
                if l == 0 {
                    run += 1;
                    continue;
                }
                if let Some(slot) = nonzero.get_mut(total) {
                    *slot = (l.clamp(-MAX_LEVEL, MAX_LEVEL), 0);
                    total += 1;
                }
                // The zeros counted so far lie above this level: they are the run of the previous one.
                if total >= 2
                    && let Some(prev) = nonzero.get_mut(total - 2)
                {
                    prev.1 = run;
                }
                run = 0;
            }
            // Zeros below the lowest-frequency level are not coded (implied by total_zeros).
            last_pos + 1 - total
        }
        None => 0,
    };
    let nz = &nonzero[..total];
    let mut trailing_ones = 0usize;
    for &(l, _) in nz {
        if trailing_ones < 3 && l.abs() == 1 {
            trailing_ones += 1;
        } else {
            break;
        }
    }

    // coeff_token
    let token = if nc == CHROMA_DC {
        COEFF_TOKEN_CHROMA_DC.get(total).and_then(|r| r.get(trailing_ones)).copied().unwrap_or((0, 0))
    } else if nc >= 8 {
        coeff_token_nc8(total, trailing_ones)
    } else {
        let table = match nc {
            ..2 => &COEFF_TOKEN_NC0,
            2..4 => &COEFF_TOKEN_NC2,
            _ => &COEFF_TOKEN_NC4,
        };
        table.get(total).and_then(|r| r.get(trailing_ones)).copied().unwrap_or((0, 0))
    };
    put(w, token);
    if total == 0 {
        return 0;
    }

    // Trailing ones: their signs.
    for &(l, _) in &nz[..trailing_ones] {
        w.bit(l < 0);
    }

    // The other levels (clause 9.2.2, inverted).
    let mut suffix_len: u32 = if total > 10 && trailing_ones < 3 { 1 } else { 0 };
    for (k, &(l, _)) in nz.iter().enumerate().skip(trailing_ones) {
        let mut code: i32 = if l > 0 { 2 * l - 2 } else { -2 * l - 1 };
        if k == trailing_ones && trailing_ones < 3 {
            code -= 2;
        }
        let code = code.max(0) as u32;
        if suffix_len == 0 {
            if code < 14 {
                put_prefix(w, code);
            } else if code < 30 {
                put_prefix(w, 14);
                w.put(code - 14, 4);
            } else {
                put_prefix(w, 15);
                w.put((code - 30).min(4095), 12);
            }
        } else if code < (15 << suffix_len) {
            put_prefix(w, code >> suffix_len);
            w.put(code & ((1 << suffix_len) - 1), suffix_len);
        } else {
            put_prefix(w, 15);
            w.put((code - (15 << suffix_len)).min(4095), 12);
        }
        if suffix_len == 0 {
            suffix_len = 1;
        }
        if l.unsigned_abs() > (3 << (suffix_len - 1)) && suffix_len < 6 {
            suffix_len += 1;
        }
    }

    // total_zeros
    if total < max {
        let table: &[&[Vlc]] = if nc == CHROMA_DC { &TOTAL_ZEROS_CHROMA_DC } else { &TOTAL_ZEROS };
        if let Some(code) = table.get(total - 1).and_then(|t| t.get(total_zeros)) {
            put(w, *code);
        }
    }

    // run_before for every level but the last (lowest frequency), while zeros are left.
    let mut zeros_left = total_zeros;
    for &(_, run) in nz.iter().take(total - 1) {
        if zeros_left == 0 {
            break;
        }
        if let Some(code) = RUN_BEFORE.get(zeros_left.min(7) - 1).and_then(|t| t.get(run)) {
            put(w, *code);
        }
        zeros_left = zeros_left.saturating_sub(run);
    }
    total as u8
}

/// level_prefix: `prefix` zero bits and a one.
fn put_prefix(w: &mut BitWriter, prefix: u32) {
    w.put(0, prefix);
    w.bit(true);
}
