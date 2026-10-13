//! Bit writing for H.264 syntax (ITU-T H.264 clause 7.2: u(n), ue(v), se(v)) and NAL units
//! (clause 7.3.1, with emulation prevention).

/// Writes bits most significant first.
#[derive(Clone, Debug, Default)]
pub(crate) struct BitWriter {
    bytes: Vec<u8>,
    /// Bits not yet in `bytes` (the low `pending` bits).
    acc: u64,
    pending: u32,
}

impl BitWriter {
    pub(crate) fn new() -> BitWriter {
        BitWriter::default()
    }

    /// Bits written so far.
    pub(crate) fn len(&self) -> usize {
        self.bytes.len() * 8 + self.pending as usize
    }

    /// `len` (at most 32) low bits of `value`.
    pub(crate) fn put(&mut self, value: u32, len: u32) {
        let len = len.min(32);
        if len == 0 {
            return;
        }
        let mask = if len == 32 { u64::from(u32::MAX) } else { (1u64 << len) - 1 };
        self.acc = (self.acc << len) | (u64::from(value) & mask);
        self.pending += len;
        while self.pending >= 8 {
            self.pending -= 8;
            self.bytes.push((self.acc >> self.pending) as u8);
        }
        // Keep only the pending bits so the accumulator never overflows.
        self.acc &= (1u64 << self.pending) - 1;
    }

    pub(crate) fn bit(&mut self, b: bool) {
        self.put(u32::from(b), 1);
    }

    /// Unsigned Exp-Golomb code (ue(v)).
    pub(crate) fn ue(&mut self, v: u32) {
        let x = u64::from(v) + 1;
        let bits = 64 - x.leading_zeros(); // significant bits of x
        let zeros = bits - 1;
        // `zeros` zero bits, then x in `bits` bits (x has its top bit set).
        self.put(0, zeros.min(32));
        if zeros > 32 {
            self.put(0, zeros - 32);
        }
        if bits > 32 {
            self.put((x >> 32) as u32, bits - 32);
            self.put(x as u32, 32);
        } else {
            self.put(x as u32, bits);
        }
    }

    /// Signed Exp-Golomb code (se(v)): 1 → 1, −1 → 2, 2 → 3, …
    pub(crate) fn se(&mut self, v: i32) {
        let v = i64::from(v);
        let code = if v > 0 { 2 * v - 1 } else { -2 * v };
        self.ue(u32::try_from(code).unwrap_or(u32::MAX - 1));
    }

    /// rbsp_trailing_bits(): a one bit, then zero bits up to the byte boundary.
    pub(crate) fn trailing(&mut self) {
        self.bit(true);
        if self.pending > 0 {
            let pad = 8 - self.pending;
            self.put(0, pad);
        }
    }

    /// The bytes (call after [`BitWriter::trailing`]; a partial last byte is padded with zeros).
    pub(crate) fn into_bytes(mut self) -> Vec<u8> {
        if self.pending > 0 {
            let pad = 8 - self.pending;
            self.put(0, pad);
        }
        self.bytes
    }
}

/// Bits of `ue(v)`.
pub(crate) fn ue_len(v: u32) -> u32 {
    let x = u64::from(v) + 1;
    2 * (64 - x.leading_zeros()) - 1
}

/// Bits of `se(v)`.
pub(crate) fn se_len(v: i32) -> u32 {
    let v = i64::from(v);
    let code = if v > 0 { 2 * v - 1 } else { -2 * v };
    ue_len(u32::try_from(code).unwrap_or(u32::MAX - 1))
}

/// A NAL unit (without start code or length): the header byte, then the RBSP with emulation
/// prevention bytes inserted (a 0x03 after two zero bytes when the next byte is 0..=3).
pub(crate) fn nal(ref_idc: u8, unit_type: u8, rbsp: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rbsp.len() + rbsp.len() / 64 + 2);
    out.push(((ref_idc & 3) << 5) | (unit_type & 31));
    let mut zeros = 0;
    for &b in rbsp {
        if zeros >= 2 && b <= 3 {
            out.push(3);
            zeros = 0;
        }
        out.push(b);
        zeros = if b == 0 { zeros + 1 } else { 0 };
    }
    out
}
