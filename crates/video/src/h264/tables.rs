//! Tables from the public H.264 specification (Rec. ITU-T H.264 (08/2024)), written out as data:
//! the CAVLC code tables (clause 9.2, Tables 9-5, 9-7, 9-8, 9-9 (a) and 9-10), the
//! coded_block_pattern mapping (Table 9-4 (a)), the scans (Table 8-13), the scaling factors
//! (equation 8-315), the chroma QP mapping (Table 8-15) and the level limits (Table A-1).

/// A variable-length code: (length in bits, value).
pub(crate) type Vlc = (u8, u32);

/// A code written as its bits, as the specification prints it.
const fn vlc(bits: &str) -> Vlc {
    let b = bits.as_bytes();
    let mut i = 0;
    let mut v = 0u32;
    while i < b.len() {
        v = v * 2 + (b[i] == b'1') as u32;
        i += 1;
    }
    (b.len() as u8, v)
}

const N: Vlc = (0, 0);

/// coeff_token for 0 <= nC < 2, indexed `[TotalCoeff][TrailingOnes]` (Table 9-5).
pub(crate) const COEFF_TOKEN_NC0: [[Vlc; 4]; 17] = [
    [vlc("1"), N, N, N],
    [vlc("000101"), vlc("01"), N, N],
    [vlc("00000111"), vlc("000100"), vlc("001"), N],
    [vlc("000000111"), vlc("00000110"), vlc("0000101"), vlc("00011")],
    [vlc("0000000111"), vlc("000000110"), vlc("00000101"), vlc("000011")],
    [vlc("00000000111"), vlc("0000000110"), vlc("000000101"), vlc("0000100")],
    [vlc("0000000001111"), vlc("00000000110"), vlc("0000000101"), vlc("00000100")],
    [vlc("0000000001011"), vlc("0000000001110"), vlc("00000000101"), vlc("000000100")],
    [vlc("0000000001000"), vlc("0000000001010"), vlc("0000000001101"), vlc("0000000100")],
    [vlc("00000000001111"), vlc("00000000001110"), vlc("0000000001001"), vlc("00000000100")],
    [vlc("00000000001011"), vlc("00000000001010"), vlc("00000000001101"), vlc("0000000001100")],
    [vlc("000000000001111"), vlc("000000000001110"), vlc("00000000001001"), vlc("00000000001100")],
    [vlc("000000000001011"), vlc("000000000001010"), vlc("000000000001101"), vlc("00000000001000")],
    [vlc("0000000000001111"), vlc("000000000000001"), vlc("000000000001001"), vlc("000000000001100")],
    [vlc("0000000000001011"), vlc("0000000000001110"), vlc("0000000000001101"), vlc("000000000001000")],
    [vlc("0000000000000111"), vlc("0000000000001010"), vlc("0000000000001001"), vlc("0000000000001100")],
    [vlc("0000000000000100"), vlc("0000000000000110"), vlc("0000000000000101"), vlc("0000000000001000")],
];

/// coeff_token for 2 <= nC < 4 (Table 9-5).
pub(crate) const COEFF_TOKEN_NC2: [[Vlc; 4]; 17] = [
    [vlc("11"), N, N, N],
    [vlc("001011"), vlc("10"), N, N],
    [vlc("000111"), vlc("00111"), vlc("011"), N],
    [vlc("0000111"), vlc("001010"), vlc("001001"), vlc("0101")],
    [vlc("00000111"), vlc("000110"), vlc("000101"), vlc("0100")],
    [vlc("00000100"), vlc("0000110"), vlc("0000101"), vlc("00110")],
    [vlc("000000111"), vlc("00000110"), vlc("00000101"), vlc("001000")],
    [vlc("00000001111"), vlc("000000110"), vlc("000000101"), vlc("000100")],
    [vlc("00000001011"), vlc("00000001110"), vlc("00000001101"), vlc("0000100")],
    [vlc("000000001111"), vlc("00000001010"), vlc("00000001001"), vlc("000000100")],
    [vlc("000000001011"), vlc("000000001110"), vlc("000000001101"), vlc("00000001100")],
    [vlc("000000001000"), vlc("000000001010"), vlc("000000001001"), vlc("00000001000")],
    [vlc("0000000001111"), vlc("0000000001110"), vlc("0000000001101"), vlc("000000001100")],
    [vlc("0000000001011"), vlc("0000000001010"), vlc("0000000001001"), vlc("0000000001100")],
    [vlc("0000000000111"), vlc("00000000001011"), vlc("0000000000110"), vlc("0000000001000")],
    [vlc("00000000001001"), vlc("00000000001000"), vlc("00000000001010"), vlc("0000000000001")],
    [vlc("00000000000111"), vlc("00000000000110"), vlc("00000000000101"), vlc("00000000000100")],
];

/// coeff_token for 4 <= nC < 8 (Table 9-5).
pub(crate) const COEFF_TOKEN_NC4: [[Vlc; 4]; 17] = [
    [vlc("1111"), N, N, N],
    [vlc("001111"), vlc("1110"), N, N],
    [vlc("001011"), vlc("01111"), vlc("1101"), N],
    [vlc("001000"), vlc("01100"), vlc("01110"), vlc("1100")],
    [vlc("0001111"), vlc("01010"), vlc("01011"), vlc("1011")],
    [vlc("0001011"), vlc("01000"), vlc("01001"), vlc("1010")],
    [vlc("0001001"), vlc("001110"), vlc("001101"), vlc("1001")],
    [vlc("0001000"), vlc("001010"), vlc("001001"), vlc("1000")],
    [vlc("00001111"), vlc("0001110"), vlc("0001101"), vlc("01101")],
    [vlc("00001011"), vlc("00001110"), vlc("0001010"), vlc("001100")],
    [vlc("000001111"), vlc("00001010"), vlc("00001101"), vlc("0001100")],
    [vlc("000001011"), vlc("000001110"), vlc("00001001"), vlc("00001100")],
    [vlc("000001000"), vlc("000001010"), vlc("000001101"), vlc("00001000")],
    [vlc("0000001101"), vlc("000000111"), vlc("000001001"), vlc("000001100")],
    [vlc("0000001001"), vlc("0000001100"), vlc("0000001011"), vlc("0000001010")],
    [vlc("0000000101"), vlc("0000001000"), vlc("0000000111"), vlc("0000000110")],
    [vlc("0000000001"), vlc("0000000100"), vlc("0000000011"), vlc("0000000010")],
];

/// coeff_token for nC >= 8: a 6-bit fixed-length code (Table 9-5).
pub(crate) fn coeff_token_nc8(total: usize, trailing_ones: usize) -> Vlc {
    if total == 0 {
        return vlc("000011");
    }
    (6, (((total - 1) as u32) << 2) | trailing_ones as u32)
}

/// coeff_token for nC = −1 (chroma DC, 4:2:0), indexed `[TotalCoeff][TrailingOnes]` (Table 9-5).
pub(crate) const COEFF_TOKEN_CHROMA_DC: [[Vlc; 4]; 5] = [
    [vlc("01"), N, N, N],
    [vlc("000111"), vlc("1"), N, N],
    [vlc("000100"), vlc("000110"), vlc("001"), N],
    [vlc("000011"), vlc("0000011"), vlc("0000010"), vlc("000101")],
    [vlc("000010"), vlc("00000011"), vlc("00000010"), vlc("0000000")],
];

/// total_zeros for 4x4 blocks, indexed `[TotalCoeff − 1][total_zeros]` (Tables 9-7 and 9-8).
pub(crate) const TOTAL_ZEROS: [&[Vlc]; 15] = [
    &[
        vlc("1"),
        vlc("011"),
        vlc("010"),
        vlc("0011"),
        vlc("0010"),
        vlc("00011"),
        vlc("00010"),
        vlc("000011"),
        vlc("000010"),
        vlc("0000011"),
        vlc("0000010"),
        vlc("00000011"),
        vlc("00000010"),
        vlc("000000011"),
        vlc("000000010"),
        vlc("000000001"),
    ],
    &[
        vlc("111"),
        vlc("110"),
        vlc("101"),
        vlc("100"),
        vlc("011"),
        vlc("0101"),
        vlc("0100"),
        vlc("0011"),
        vlc("0010"),
        vlc("00011"),
        vlc("00010"),
        vlc("000011"),
        vlc("000010"),
        vlc("000001"),
        vlc("000000"),
    ],
    &[
        vlc("0101"),
        vlc("111"),
        vlc("110"),
        vlc("101"),
        vlc("0100"),
        vlc("0011"),
        vlc("100"),
        vlc("011"),
        vlc("0010"),
        vlc("00011"),
        vlc("00010"),
        vlc("000001"),
        vlc("00001"),
        vlc("000000"),
    ],
    &[
        vlc("00011"),
        vlc("111"),
        vlc("0101"),
        vlc("0100"),
        vlc("110"),
        vlc("101"),
        vlc("100"),
        vlc("0011"),
        vlc("011"),
        vlc("0010"),
        vlc("00010"),
        vlc("00001"),
        vlc("00000"),
    ],
    &[
        vlc("0101"),
        vlc("0100"),
        vlc("0011"),
        vlc("111"),
        vlc("110"),
        vlc("101"),
        vlc("100"),
        vlc("011"),
        vlc("0010"),
        vlc("00001"),
        vlc("0001"),
        vlc("00000"),
    ],
    &[vlc("000001"), vlc("00001"), vlc("111"), vlc("110"), vlc("101"), vlc("100"), vlc("011"), vlc("010"), vlc("0001"), vlc("001"), vlc("000000")],
    &[vlc("000001"), vlc("00001"), vlc("101"), vlc("100"), vlc("011"), vlc("11"), vlc("010"), vlc("0001"), vlc("001"), vlc("000000")],
    &[vlc("000001"), vlc("0001"), vlc("00001"), vlc("011"), vlc("11"), vlc("10"), vlc("010"), vlc("001"), vlc("000000")],
    &[vlc("000001"), vlc("000000"), vlc("0001"), vlc("11"), vlc("10"), vlc("001"), vlc("01"), vlc("00001")],
    &[vlc("00001"), vlc("00000"), vlc("001"), vlc("11"), vlc("10"), vlc("01"), vlc("0001")],
    &[vlc("0000"), vlc("0001"), vlc("001"), vlc("010"), vlc("1"), vlc("011")],
    &[vlc("0000"), vlc("0001"), vlc("01"), vlc("1"), vlc("001")],
    &[vlc("000"), vlc("001"), vlc("1"), vlc("01")],
    &[vlc("00"), vlc("01"), vlc("1")],
    &[vlc("0"), vlc("1")],
];

/// total_zeros for chroma DC 2x2 blocks, indexed `[TotalCoeff − 1][total_zeros]` (Table 9-9 (a)).
pub(crate) const TOTAL_ZEROS_CHROMA_DC: [&[Vlc]; 3] =
    [&[vlc("1"), vlc("01"), vlc("001"), vlc("000")], &[vlc("1"), vlc("01"), vlc("00")], &[vlc("1"), vlc("0")]];

/// run_before, indexed `[min(zerosLeft, 7) − 1][run_before]` (Table 9-10).
pub(crate) const RUN_BEFORE: [&[Vlc]; 7] = [
    &[vlc("1"), vlc("0")],
    &[vlc("1"), vlc("01"), vlc("00")],
    &[vlc("11"), vlc("10"), vlc("01"), vlc("00")],
    &[vlc("11"), vlc("10"), vlc("01"), vlc("001"), vlc("000")],
    &[vlc("11"), vlc("10"), vlc("011"), vlc("010"), vlc("001"), vlc("000")],
    &[vlc("11"), vlc("000"), vlc("001"), vlc("011"), vlc("010"), vlc("101"), vlc("100")],
    &[
        vlc("111"),
        vlc("110"),
        vlc("101"),
        vlc("100"),
        vlc("011"),
        vlc("010"),
        vlc("001"),
        vlc("0001"),
        vlc("00001"),
        vlc("000001"),
        vlc("0000001"),
        vlc("00000001"),
        vlc("000000001"),
        vlc("0000000001"),
        vlc("00000000001"),
    ],
];

/// coded_block_pattern of each codeNum for Intra_4x4 macroblocks (Table 9-4 (a)).
pub(crate) const CBP_INTRA: [u8; 48] = [
    47, 31, 15, 0, 23, 27, 29, 30, 7, 11, 13, 14, 39, 43, 45, 46, 16, 3, 5, 10, 12, 19, 21, 26, 28, 35, 37, 42, 44, 1, 2, 4, 8, 17, 18, 20, 24, 6, 9,
    22, 25, 32, 33, 34, 36, 40, 38, 41,
];

/// coded_block_pattern of each codeNum for Inter macroblocks (Table 9-4 (a)).
pub(crate) const CBP_INTER: [u8; 48] = [
    0, 16, 1, 2, 4, 8, 32, 3, 5, 10, 12, 15, 47, 7, 11, 13, 14, 6, 9, 31, 35, 37, 42, 44, 33, 34, 36, 40, 39, 43, 45, 46, 17, 18, 20, 24, 19, 21, 26,
    28, 23, 27, 29, 30, 22, 25, 38, 41,
];

/// The codeNum of `cbp` in `table`.
pub(crate) fn cbp_code(table: &[u8; 48], cbp: u8) -> u32 {
    table.iter().position(|&c| c == cbp).unwrap_or(0) as u32
}

/// Zig-zag scan: scan index → raster position (row * 4 + column) in a 4x4 block (Table 8-13).
pub(crate) const ZIGZAG: [usize; 16] = [0, 1, 4, 8, 5, 2, 3, 6, 9, 12, 13, 10, 7, 11, 14, 15];

/// `v` of equation 8-315: `[qP % 6][0: (even, even) positions, 1: (odd, odd), 2: the others]`.
pub(crate) const NORM_ADJUST: [[i32; 3]; 6] = [[10, 16, 13], [11, 18, 14], [13, 20, 16], [14, 23, 18], [16, 25, 20], [18, 29, 23]];

/// Which column of [`NORM_ADJUST`] a raster position of a 4x4 block uses.
pub(crate) const fn position_class(pos: usize) -> usize {
    let (i, j) = (pos / 4, pos % 4);
    match (i % 2, j % 2) {
        (0, 0) => 0,
        (1, 1) => 1,
        _ => 2,
    }
}

/// QPC for qPI ≥ 30 (Table 8-15; below 30 QPC = qPI).
const QPC_FROM_30: [u8; 22] = [29, 30, 31, 32, 32, 33, 34, 34, 35, 35, 36, 36, 37, 37, 37, 38, 38, 38, 39, 39, 39, 39];

/// The chroma quantization parameter for luma QP `qp` (chroma_qp_index_offset 0).
pub(crate) fn chroma_qp(qp: u8) -> u8 {
    let qp = qp.min(51);
    if qp < 30 { qp } else { QPC_FROM_30.get(usize::from(qp - 30)).copied().unwrap_or(39) }
}

/// α′ of the deblocking filter, indexed by indexA (Table 8-16).
pub(crate) const ALPHA: [u8; 52] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 4, 5, 6, 7, 8, 9, 10, 12, 13, 15, 17, 20, 22, 25, 28, 32, 36, 40, 45, 50, 56, 63, 71, 80, 90,
    101, 113, 127, 144, 162, 182, 203, 226, 255, 255,
];

/// β′ of the deblocking filter, indexed by indexB (Table 8-16).
pub(crate) const BETA: [u8; 52] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 14, 14, 15,
    15, 16, 16, 17, 17, 18, 18,
];

/// t′C0 of the deblocking filter, indexed `[bS − 1][indexA]` (Table 8-17).
pub(crate) const TC0: [[u8; 52]; 3] = [
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 6, 6, 7,
        8, 9, 10, 11, 13,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 4, 4, 5, 5, 6, 7, 8, 8, 10,
        11, 12, 13, 15, 17,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 6, 6, 7, 8, 9, 10, 11, 13,
        14, 16, 18, 20, 23, 25,
    ],
];

/// One row of Table A-1: (level_idc, MaxMBPS, MaxFS, MaxDpbMbs).
pub(crate) const LEVELS: [(u8, u64, u64, u64); 15] = [
    (10, 1_485, 99, 396),
    (11, 3_000, 396, 900),
    (12, 6_000, 396, 2_376),
    (13, 11_880, 396, 2_376),
    (20, 11_880, 396, 2_376),
    (21, 19_800, 792, 4_752),
    (22, 20_250, 1_620, 8_100),
    (30, 40_500, 1_620, 8_100),
    (31, 108_000, 3_600, 18_000),
    (32, 216_000, 5_120, 20_480),
    (40, 245_760, 8_192, 32_768),
    (41, 245_760, 8_192, 32_768),
    (42, 522_240, 8_704, 34_816),
    (50, 589_824, 22_080, 110_400),
    (51, 983_040, 36_864, 184_320),
];
