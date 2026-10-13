//! A small H.264 encoder, written from the public specification (Rec. ITU-T H.264 (08/2024)).
//!
//! It writes the **Constrained Baseline** profile (CAVLC, progressive frames, one reference frame,
//! no B frames), which every H.264 decoder plays: browsers, QuickTime, Windows, Android and iOS.
//!
//! - **Pictures:** an IDR picture first and every `keyint` frames, P pictures in between. One
//!   slice per picture, constant QP, the in-loop deblocking filter on (clause 8.7).
//! - **Macroblocks:** each is coded the cheapest way found by rate–distortion choice
//!   (`distortion + λ · bits`, with the real bit count) among P_Skip, P_L0_16x16 with a full-sample
//!   motion vector (diamond search), Intra_16x16 (four modes) and Intra_4x4 (nine modes per block).
//!   Repeated frames are a P picture of skipped macroblocks: a few bytes.
//! - **Residuals:** the 4x4 integer transform, the Intra_16x16 and chroma DC transforms, deadzone
//!   quantization, CAVLC with the specification's code tables.

mod cavlc;
mod deblock;
mod intra;
pub(crate) mod tables;
mod transform;

use crate::bits::{BitWriter, nal, se_len};
use crate::yuv::Picture;
use intra::{Avail, Plane};
use tables::{CBP_INTER, CBP_INTRA, LEVELS, ZIGZAG, cbp_code, chroma_qp};

/// luma4x4BlkIdx → raster position (row * 4 + column) of the block in its macroblock (clause 6.4.3).
const BLK_RASTER: [usize; 16] = [0, 1, 4, 5, 2, 3, 6, 7, 8, 9, 12, 13, 10, 11, 14, 15];
/// Raster position → luma4x4BlkIdx (the same permutation: it is its own inverse).
const RASTER_BLK: [usize; 16] = BLK_RASTER;

/// log2 of MaxFrameNum: frame_num never wraps between IDR pictures.
const LOG2_MAX_FRAME_NUM: u32 = 16;
/// The farthest a motion vector reaches, in full samples.
const MAX_MV: i32 = 128;

/// What the encoder needs to know about the video.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Config {
    /// Frame size in samples (even).
    pub width: usize,
    pub height: usize,
    pub fps: u32,
    /// Quantization parameter, 0–51 (lower is better quality).
    pub qp: u8,
    /// Most frames from one IDR picture to the next.
    pub keyint: u32,
    /// Run the deblocking filter (off: the reconstruction is not smoothed).
    pub deblock: bool,
}

/// One coded picture.
pub(crate) struct Frame {
    /// The access unit as length-prefixed NAL units (4-byte big-endian lengths), as MP4 stores it.
    pub data: Vec<u8>,
    /// An IDR picture (a sync sample).
    pub keyframe: bool,
}

/// How a macroblock is coded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Skip,
    Inter,
    I16(u8),
    I4,
}

/// A macroblock coded one way: its syntax and its reconstruction.
#[derive(Clone)]
struct Coded {
    kind: Kind,
    chroma_mode: u8,
    cbp_luma: u8,
    cbp_chroma: u8,
    /// Intra_16x16 DC levels, scan order.
    dc: [i32; 16],
    /// Levels of each 4x4 luma block (raster position), scan order (index 0 unused for Intra_16x16).
    luma: [[i32; 16]; 16],
    chroma_dc: [[i32; 4]; 2],
    /// Chroma AC levels per component and block, scan order (index 0 unused).
    chroma_ac: [[[i32; 16]; 4]; 2],
    /// TotalCoeff of each coded 4x4 block (luma raster, chroma raster), for CAVLC contexts.
    nz_y: [u8; 16],
    nz_c: [[u8; 4]; 2],
    /// Intra_4x4 modes (raster) and their syntax (luma4x4BlkIdx order): predicted, or the
    /// remaining mode.
    modes4: [u8; 16],
    modes4_syntax: [Option<u8>; 16],
    /// Motion vector (quarter samples) and its difference from the prediction.
    mv: (i32, i32),
    mvd: (i32, i32),
    y: [u8; 256],
    u: [u8; 64],
    v: [u8; 64],
}

impl Coded {
    fn new(kind: Kind) -> Coded {
        Coded {
            kind,
            chroma_mode: 0,
            cbp_luma: 0,
            cbp_chroma: 0,
            dc: [0; 16],
            luma: [[0; 16]; 16],
            chroma_dc: [[0; 4]; 2],
            chroma_ac: [[[0; 16]; 4]; 2],
            nz_y: [0; 16],
            nz_c: [[0; 4]; 2],
            modes4: [2; 16],
            modes4_syntax: [None; 16],
            mv: (0, 0),
            mvd: (0, 0),
            y: [0; 256],
            u: [0; 64],
            v: [0; 64],
        }
    }

    fn intra(&self) -> bool {
        matches!(self.kind, Kind::I16(_) | Kind::I4)
    }
}

/// What later macroblocks of the picture see of a coded one.
#[derive(Clone, Copy, Default)]
struct MbState {
    intra: bool,
    i4: bool,
    mv: (i32, i32),
    nz_y: [u8; 16],
    nz_c: [[u8; 4]; 2],
    modes4: [u8; 16],
}

/// Motion data of a neighbouring partition: (refIdx, mv); refIdx −1 for intra.
type Motion = (i32, (i32, i32));

pub(crate) struct Encoder {
    cfg: Config,
    mb_w: usize,
    mb_h: usize,
    qpc: u8,
    lambda: f64,
    lambda_sad: f64,
    /// The picture being coded, and the previous one (the reference).
    recon: Picture,
    reference: Picture,
    /// The previous source picture (unchanged macroblocks are skipped without a search).
    previous: Option<Picture>,
    mbs: Vec<MbState>,
    /// Motion vectors of the previous picture (search candidates).
    previous_mvs: Vec<(i32, i32)>,
    frame_num: u32,
    idr_count: u32,
    since_idr: u32,
    started: bool,
    sps: Vec<u8>,
    pps: Vec<u8>,
    level: u8,
}

impl Encoder {
    /// An encoder for frames of `cfg.width` × `cfg.height` (checked by the caller: even, at least
    /// 2, within the level limits).
    pub(crate) fn new(cfg: Config) -> Encoder {
        let mb_w = cfg.width.div_ceil(16).max(1);
        let mb_h = cfg.height.div_ceil(16).max(1);
        let qp = cfg.qp.min(51);
        let cfg = Config { qp, ..cfg };
        let lambda = 0.85 * 2f64.powf((f64::from(qp) - 12.0) / 3.0);
        let level = level_idc(mb_w, mb_h, cfg.fps);
        let mut e = Encoder {
            cfg,
            mb_w,
            mb_h,
            qpc: chroma_qp(qp),
            lambda,
            lambda_sad: lambda.sqrt(),
            recon: Picture::new(mb_w * 16, mb_h * 16),
            reference: Picture::new(mb_w * 16, mb_h * 16),
            previous: None,
            mbs: vec![MbState::default(); mb_w * mb_h],
            previous_mvs: vec![(0, 0); mb_w * mb_h],
            frame_num: 0,
            idr_count: 0,
            since_idr: 0,
            started: false,
            sps: vec![],
            pps: vec![],
            level,
        };
        e.sps = e.write_sps();
        e.pps = e.write_pps();
        e
    }

    /// The sequence parameter set NAL unit.
    pub(crate) fn sps(&self) -> &[u8] {
        &self.sps
    }

    /// The picture parameter set NAL unit.
    pub(crate) fn pps(&self) -> &[u8] {
        &self.pps
    }

    /// The coded picture size in samples (whole macroblocks).
    pub(crate) fn coded_size(&self) -> (usize, usize) {
        (self.mb_w * 16, self.mb_h * 16)
    }

    /// The reconstruction of the last coded picture (what decoders show, before cropping).
    #[cfg(test)]
    pub(crate) fn last_picture(&self) -> &Picture {
        &self.reference
    }

    /// Encodes `src` (a picture of [`Encoder::coded_size`]); `keyframe` asks for an IDR picture.
    pub(crate) fn encode(&mut self, src: &Picture, keyframe: bool) -> Frame {
        let idr = keyframe || !self.started || self.since_idr >= self.cfg.keyint.max(1);
        let frame = self.encode_picture(src, idr);
        self.previous = Some(src.clone());
        frame
    }

    /// Shows the last picture once more: a P picture of skipped macroblocks (an IDR picture of the
    /// last source when one is due).
    pub(crate) fn repeat(&mut self) -> Frame {
        if !self.started || self.since_idr >= self.cfg.keyint.max(1) {
            let src = self.previous.clone().unwrap_or_else(|| Picture::new(self.mb_w * 16, self.mb_h * 16));
            return self.encode(&src, true);
        }
        let mut w = BitWriter::new();
        self.slice_header(&mut w, false);
        w.ue((self.mb_w * self.mb_h) as u32);
        w.trailing();
        self.previous_mvs.fill((0, 0));
        self.finish_picture(w, false)
    }

    fn finish_picture(&mut self, w: BitWriter, idr: bool) -> Frame {
        let unit = if idr { nal(3, 5, &w.into_bytes()) } else { nal(2, 1, &w.into_bytes()) };
        let mut data = Vec::with_capacity(unit.len() + 4);
        data.extend_from_slice(&(unit.len() as u32).to_be_bytes());
        data.extend_from_slice(&unit);
        self.frame_num = (self.frame_num + 1) % (1 << LOG2_MAX_FRAME_NUM);
        self.since_idr += 1;
        Frame { data, keyframe: idr }
    }

    // ---------- parameter sets and headers ----------

    /// The profile_idc, constraint flags byte and level_idc (for the MP4 `avcC` box).
    pub(crate) fn profile(&self) -> (u8, u8, u8) {
        (66, 0b1100_0000, self.level)
    }

    fn write_sps(&self) -> Vec<u8> {
        let mut w = BitWriter::new();
        let (profile, flags, level) = self.profile();
        w.put(u32::from(profile), 8);
        w.put(u32::from(flags), 8); // constraint_set0 and 1: Constrained Baseline
        w.put(u32::from(level), 8);
        w.ue(0); // seq_parameter_set_id
        w.ue(LOG2_MAX_FRAME_NUM - 4);
        w.ue(2); // pic_order_cnt_type: output order is decoding order
        w.ue(1); // max_num_ref_frames
        w.bit(false); // gaps_in_frame_num_value_allowed_flag
        w.ue(self.mb_w as u32 - 1);
        w.ue(self.mb_h as u32 - 1);
        w.bit(true); // frame_mbs_only_flag
        w.bit(true); // direct_8x8_inference_flag
        let crop_right = (self.mb_w * 16 - self.cfg.width) / 2;
        let crop_bottom = (self.mb_h * 16 - self.cfg.height) / 2;
        let crop = crop_right > 0 || crop_bottom > 0;
        w.bit(crop);
        if crop {
            w.ue(0);
            w.ue(crop_right as u32);
            w.ue(0);
            w.ue(crop_bottom as u32);
        }
        // VUI: square samples, BT.709 limited range, the frame rate, no reordering.
        w.bit(true);
        w.bit(true); // aspect_ratio_info_present_flag
        w.put(1, 8); // 1:1
        w.bit(false); // overscan_info_present_flag
        w.bit(true); // video_signal_type_present_flag
        w.put(5, 3); // video_format: unspecified
        w.bit(false); // video_full_range_flag
        w.bit(true); // colour_description_present_flag
        w.put(1, 8); // colour_primaries: BT.709
        w.put(1, 8); // transfer_characteristics: BT.709
        w.put(1, 8); // matrix_coefficients: BT.709
        w.bit(false); // chroma_loc_info_present_flag
        w.bit(true); // timing_info_present_flag
        w.put(1, 32); // num_units_in_tick
        w.put(self.cfg.fps.max(1) * 2, 32); // time_scale (two ticks per frame)
        w.bit(true); // fixed_frame_rate_flag
        w.bit(false); // nal_hrd_parameters_present_flag
        w.bit(false); // vcl_hrd_parameters_present_flag
        w.bit(false); // pic_struct_present_flag
        w.bit(true); // bitstream_restriction_flag
        w.bit(true); // motion_vectors_over_pic_boundaries_flag
        w.ue(0); // max_bytes_per_pic_denom
        w.ue(0); // max_bits_per_mb_denom
        w.ue(15); // log2_max_mv_length_horizontal
        w.ue(15); // log2_max_mv_length_vertical
        w.ue(0); // max_num_reorder_frames
        w.ue(1); // max_dec_frame_buffering
        w.trailing();
        nal(3, 7, &w.into_bytes())
    }

    fn write_pps(&self) -> Vec<u8> {
        let mut w = BitWriter::new();
        w.ue(0); // pic_parameter_set_id
        w.ue(0); // seq_parameter_set_id
        w.bit(false); // entropy_coding_mode_flag: CAVLC
        w.bit(false); // bottom_field_pic_order_in_frame_present_flag
        w.ue(0); // num_slice_groups_minus1
        w.ue(0); // num_ref_idx_l0_default_active_minus1
        w.ue(0); // num_ref_idx_l1_default_active_minus1
        w.bit(false); // weighted_pred_flag
        w.put(0, 2); // weighted_bipred_idc
        w.se(i32::from(self.cfg.qp) - 26); // pic_init_qp_minus26
        w.se(0); // pic_init_qs_minus26
        w.se(0); // chroma_qp_index_offset
        w.bit(true); // deblocking_filter_control_present_flag
        w.bit(false); // constrained_intra_pred_flag
        w.bit(false); // redundant_pic_cnt_present_flag
        w.trailing();
        nal(3, 8, &w.into_bytes())
    }

    fn slice_header(&mut self, w: &mut BitWriter, idr: bool) {
        if idr {
            self.frame_num = 0;
            self.since_idr = 0;
        }
        w.ue(0); // first_mb_in_slice
        w.ue(if idr { 7 } else { 5 }); // slice_type: I or P (all slices of the picture)
        w.ue(0); // pic_parameter_set_id
        w.put(self.frame_num, LOG2_MAX_FRAME_NUM);
        if idr {
            w.ue(self.idr_count % 65_536); // idr_pic_id: differs between consecutive IDR pictures
            self.idr_count = self.idr_count.wrapping_add(1);
        } else {
            w.bit(false); // num_ref_idx_active_override_flag
            w.bit(false); // ref_pic_list_modification_flag_l0
        }
        // dec_ref_pic_marking()
        if idr {
            w.bit(false); // no_output_of_prior_pics_flag
            w.bit(false); // long_term_reference_flag
        } else {
            w.bit(false); // adaptive_ref_pic_marking_mode_flag: sliding window
        }
        w.se(0); // slice_qp_delta
        if self.cfg.deblock {
            w.ue(0); // disable_deblocking_filter_idc: filter every edge
            w.se(0); // slice_alpha_c0_offset_div2
            w.se(0); // slice_beta_offset_div2
        } else {
            w.ue(1);
        }
        self.started = true;
    }

    // ---------- pictures ----------

    fn encode_picture(&mut self, src: &Picture, idr: bool) -> Frame {
        let mut w = BitWriter::new();
        self.slice_header(&mut w, idr);
        self.mbs.fill(MbState::default());
        let mut skip_run = 0u32;
        let mut mvs = vec![(0, 0); self.mb_w * self.mb_h];
        for mby in 0..self.mb_h {
            for mbx in 0..self.mb_w {
                let coded = self.decide(src, mbx, mby, !idr);
                if coded.kind == Kind::Skip {
                    skip_run += 1;
                } else {
                    if !idr {
                        w.ue(skip_run);
                    }
                    skip_run = 0;
                    self.write_mb(&mut w, &coded, mbx, mby, !idr);
                }
                if let Some(m) = mvs.get_mut(mby * self.mb_w + mbx) {
                    *m = coded.mv;
                }
                self.commit(&coded, mbx, mby);
            }
        }
        if skip_run > 0 {
            w.ue(skip_run);
        }
        w.trailing();
        if self.cfg.deblock {
            deblock::deblock(&mut self.recon, &self.mbs, self.mb_w, self.mb_h, self.cfg.qp);
        }
        std::mem::swap(&mut self.recon, &mut self.reference);
        self.previous_mvs = mvs;
        self.finish_picture(w, idr)
    }

    /// Stores the chosen coding of a macroblock: its reconstruction and what neighbours see.
    fn commit(&mut self, c: &Coded, mbx: usize, mby: usize) {
        let pw = self.recon.width;
        copy_block(&mut self.recon.y, pw, mbx * 16, mby * 16, &c.y, 16);
        copy_block(&mut self.recon.u, pw / 2, mbx * 8, mby * 8, &c.u, 8);
        copy_block(&mut self.recon.v, pw / 2, mbx * 8, mby * 8, &c.v, 8);
        if let Some(s) = self.mbs.get_mut(mby * self.mb_w + mbx) {
            *s = MbState { intra: c.intra(), i4: c.kind == Kind::I4, mv: c.mv, nz_y: c.nz_y, nz_c: c.nz_c, modes4: c.modes4 };
        }
    }

    /// The cheapest coding of macroblock (`mbx`, `mby`).
    fn decide(&mut self, src: &Picture, mbx: usize, mby: usize, p_slice: bool) -> Coded {
        let s = source_mb(src, mbx, mby);
        let mut best: Option<(f64, Coded)> = None;
        let consider = |cost: f64, c: Coded, best: &mut Option<(f64, Coded)>| {
            if best.as_ref().is_none_or(|(b, _)| cost < *b) {
                *best = Some((cost, c));
            }
        };
        if p_slice {
            let skip_mv = self.skip_mv(mbx, mby);
            // Unchanged macroblocks are skipped without trying anything else.
            if skip_mv == (0, 0) && self.previous.as_ref().is_some_and(|p| p.same_mb(src, mbx, mby)) {
                return self.code_skip(mbx, mby, skip_mv);
            }
            let skip = self.code_skip(mbx, mby, skip_mv);
            let d = ssd(&s, &skip);
            // So are nearly unchanged ones.
            if d <= 384 {
                return skip;
            }
            consider(d as f64 + self.lambda, skip, &mut best);
        }
        let i16_mode = self.best_i16_mode(&s, mbx, mby);
        if p_slice {
            let (inter, inter_satd) = self.code_inter(&s, mbx, mby);
            let cost = self.rd_cost(&s, &inter, mbx, mby, p_slice);
            consider(cost, inter, &mut best);
            // Intra coding is tried only where motion predicts worse than the best 16x16 intra
            // prediction (new content); elsewhere it would not win and costs most of the time.
            if i16_mode.as_ref().is_none_or(|m| inter_satd <= m.2) {
                return best.map(|(_, c)| c).unwrap_or_else(|| Coded::new(Kind::Skip));
            }
        }
        let chroma_mode = self.best_chroma_mode(&s, mbx, mby);
        if let Some((mode, pred, _)) = i16_mode {
            let i16 = self.code_i16(&s, mbx, mby, mode, &pred, chroma_mode);
            let cost = self.rd_cost(&s, &i16, mbx, mby, p_slice);
            consider(cost, i16, &mut best);
        }
        let i4 = self.code_i4(&s, mbx, mby, chroma_mode);
        let cost = self.rd_cost(&s, &i4, mbx, mby, p_slice);
        consider(cost, i4, &mut best);
        best.map(|(_, c)| c).unwrap_or_else(|| Coded::new(Kind::I4))
    }

    fn rd_cost(&self, s: &SourceMb, c: &Coded, mbx: usize, mby: usize, p_slice: bool) -> f64 {
        let mut w = BitWriter::new();
        self.write_mb(&mut w, c, mbx, mby, p_slice);
        ssd(s, c) as f64 + self.lambda * w.len() as f64
    }

    // ---------- availability and neighbours ----------

    fn state(&self, mbx: isize, mby: isize) -> Option<&MbState> {
        if mbx < 0 || mby < 0 || mbx as usize >= self.mb_w || mby as usize >= self.mb_h {
            return None;
        }
        self.mbs.get(mby as usize * self.mb_w + mbx as usize)
    }

    fn motion(&self, mbx: isize, mby: isize) -> Option<Motion> {
        self.state(mbx, mby).map(|s| if s.intra { (-1, (0, 0)) } else { (0, s.mv) })
    }

    /// The motion vector prediction for a 16x16 partition (clauses 8.4.1.3 and 8.4.1.3.1).
    fn mvp(&self, mbx: usize, mby: usize) -> (i32, i32) {
        let (x, y) = (mbx as isize, mby as isize);
        let a = self.motion(x - 1, y);
        let b = self.motion(x, y - 1);
        let c = self.motion(x + 1, y - 1).or_else(|| self.motion(x - 1, y - 1));
        let none: Motion = (-1, (0, 0));
        let ma = a.unwrap_or(none);
        let (mb, mc) = if b.is_none() && c.is_none() && a.is_some() { (ma, ma) } else { (b.unwrap_or(none), c.unwrap_or(none)) };
        let refs = [ma, mb, mc];
        let mut same = refs.iter().filter(|m| m.0 == 0);
        if let (Some(only), None) = (same.next(), same.next()) {
            return only.1;
        }
        (median(ma.1.0, mb.1.0, mc.1.0), median(ma.1.1, mb.1.1, mc.1.1))
    }

    /// The motion vector of a P_Skip macroblock (clause 8.4.1.1).
    fn skip_mv(&self, mbx: usize, mby: usize) -> (i32, i32) {
        let (x, y) = (mbx as isize, mby as isize);
        let (Some(a), Some(b)) = (self.motion(x - 1, y), self.motion(x, y - 1)) else { return (0, 0) };
        if a == (0, (0, 0)) || b == (0, (0, 0)) {
            return (0, 0);
        }
        self.mvp(mbx, mby)
    }

    fn avail16(&self, mbx: usize, mby: usize) -> Avail {
        Avail { left: mbx > 0, top: mby > 0, top_left: mbx > 0 && mby > 0, top_right: mby > 0 && mbx + 1 < self.mb_w }
    }

    /// nC for a luma block at raster position `pos` of the macroblock, whose earlier blocks have
    /// TotalCoeff `own` (clause 9.2.1).
    fn nc_luma(&self, mbx: usize, mby: usize, pos: usize, own: &[u8; 16]) -> i32 {
        let (bx, by) = (pos % 4, pos / 4);
        let left = if bx > 0 { Some(own[pos - 1]) } else { self.state(mbx as isize - 1, mby as isize).map(|s| s.nz_y[by * 4 + 3]) };
        let top = if by > 0 { Some(own[pos - 4]) } else { self.state(mbx as isize, mby as isize - 1).map(|s| s.nz_y[12 + bx]) };
        nc_of(left, top)
    }

    fn nc_chroma(&self, mbx: usize, mby: usize, comp: usize, pos: usize, own: &[u8; 4]) -> i32 {
        let (bx, by) = (pos % 2, pos / 2);
        let left = if bx > 0 { Some(own[pos - 1]) } else { self.state(mbx as isize - 1, mby as isize).map(|s| s.nz_c[comp][by * 2 + 1]) };
        let top = if by > 0 { Some(own[pos - 2]) } else { self.state(mbx as isize, mby as isize - 1).map(|s| s.nz_c[comp][2 + bx]) };
        nc_of(left, top)
    }

    /// predIntra4x4PredMode for the block at raster position `pos` (clause 8.3.1.1).
    fn predicted_mode4(&self, mbx: usize, mby: usize, pos: usize, own: &[u8; 16]) -> u8 {
        let (bx, by) = (pos % 4, pos / 4);
        let mode_of = |s: &MbState, p: usize| if s.i4 { s.modes4[p] } else { 2 };
        let left = if bx > 0 { Some(own[pos - 1]) } else { self.state(mbx as isize - 1, mby as isize).map(|s| mode_of(s, by * 4 + 3)) };
        let top = if by > 0 { Some(own[pos - 4]) } else { self.state(mbx as isize, mby as isize - 1).map(|s| mode_of(s, 12 + bx)) };
        match (left, top) {
            (Some(a), Some(b)) => a.min(b),
            _ => 2,
        }
    }

    // ---------- the ways to code a macroblock ----------

    fn code_skip(&self, mbx: usize, mby: usize, mv: (i32, i32)) -> Coded {
        let mut c = Coded::new(Kind::Skip);
        c.mv = mv;
        (c.y, c.u, c.v) = self.motion_pred(mbx, mby, mv);
        c
    }

    /// The prediction of a 16x16 block moved by `mv` (quarter samples; full samples only) from
    /// the reference picture.
    fn motion_pred(&self, mbx: usize, mby: usize, mv: (i32, i32)) -> ([u8; 256], [u8; 64], [u8; 64]) {
        let r = &self.reference;
        let (pw, ph) = (r.width as i32, r.height as i32);
        let (x0, y0) = ((mbx * 16) as i32 + (mv.0 >> 2), (mby * 16) as i32 + (mv.1 >> 2));
        let mut y = [0u8; 256];
        if x0 >= 0 && y0 >= 0 && x0 + 16 <= pw && y0 + 16 <= ph {
            read_block(&r.y, r.width, x0 as usize, y0 as usize, &mut y, 16);
        } else {
            // Outside the picture the edge samples repeat.
            for j in 0..16 {
                let yy = (y0 + j).clamp(0, ph - 1) as usize;
                for i in 0..16 {
                    let xx = (x0 + i).clamp(0, pw - 1) as usize;
                    y[(j * 16 + i) as usize] = r.y.get(yy * r.width + xx).copied().unwrap_or(128);
                }
            }
        }
        // Chroma: eighth samples (clause 8.4.2.2.2).
        let (cw, ch) = (pw / 2, ph / 2);
        let (fx, fy) = (mv.0 & 7, mv.1 & 7);
        let (cx0, cy0) = ((mbx * 8) as i32 + (mv.0 >> 3), (mby * 8) as i32 + (mv.1 >> 3));
        let chroma = |plane: &[u8]| {
            let mut out = [0u8; 64];
            if fx == 0 && fy == 0 && cx0 >= 0 && cy0 >= 0 && cx0 + 8 <= cw && cy0 + 8 <= ch {
                read_block(plane, cw as usize, cx0 as usize, cy0 as usize, &mut out, 8);
                return out;
            }
            let at = |x: i32, y: i32| {
                i32::from(plane.get(y.clamp(0, ch - 1) as usize * cw as usize + x.clamp(0, cw - 1) as usize).copied().unwrap_or(128))
            };
            for j in 0..8 {
                for i in 0..8 {
                    let (x, y) = (cx0 + i, cy0 + j);
                    let v = ((8 - fx) * (8 - fy) * at(x, y)
                        + fx * (8 - fy) * at(x + 1, y)
                        + (8 - fx) * fy * at(x, y + 1)
                        + fx * fy * at(x + 1, y + 1)
                        + 32)
                        >> 6;
                    out[(j * 8 + i) as usize] = v as u8;
                }
            }
            out
        };
        (y, chroma(&r.u), chroma(&r.v))
    }

    /// P_L0_16x16 with the best full-sample motion vector found, and the SATD of its luma
    /// prediction.
    fn code_inter(&self, s: &SourceMb, mbx: usize, mby: usize) -> (Coded, u32) {
        let mvp = self.mvp(mbx, mby);
        let mv = self.motion_search(s, mbx, mby, mvp);
        let mut c = Coded::new(Kind::Inter);
        c.mv = mv;
        c.mvd = (mv.0 - mvp.0, mv.1 - mvp.1);
        let (py, pu, pv) = self.motion_pred(mbx, mby, mv);
        let satd = satd16(&s.y, &py);
        self.code_luma_4x4_residual(s, &py, &mut c, false);
        self.code_chroma(s, &pu, &pv, &mut c, false);
        (c, satd)
    }

    fn motion_search(&self, s: &SourceMb, mbx: usize, mby: usize, mvp: (i32, i32)) -> (i32, i32) {
        let (x, y) = (mbx as isize, mby as isize);
        let full = |m: (i32, i32)| ((m.0 >> 2).clamp(-MAX_MV, MAX_MV), (m.1 >> 2).clamp(-MAX_MV, MAX_MV));
        let mut candidates = vec![(0, 0), full(mvp)];
        for (dx, dy) in [(-1, 0), (0, -1), (1, -1)] {
            if let Some((0, m)) = self.motion(x + dx, y + dy) {
                candidates.push(full(m));
            }
        }
        if let Some(&m) = self.previous_mvs.get(mby * self.mb_w + mbx) {
            candidates.push(full(m));
        }
        let cost = |m: (i32, i32)| -> f64 {
            let sad = self.sad(s, mbx, mby, m);
            let bits = se_len(m.0 * 4 - mvp.0) + se_len(m.1 * 4 - mvp.1);
            sad as f64 + self.lambda_sad * f64::from(bits)
        };
        let mut best = (0, 0);
        let mut best_cost = f64::INFINITY;
        for m in candidates {
            let c = cost(m);
            if c < best_cost {
                (best, best_cost) = (m, c);
            }
        }
        for step in [8, 4, 2, 1] {
            for _ in 0..8 {
                if best_cost <= self.lambda_sad * 2.0 {
                    break; // a perfect match at the predicted vector
                }
                let mut moved = false;
                for (dx, dy) in [(-step, 0), (step, 0), (0, -step), (0, step), (-step, -step), (step, -step), (-step, step), (step, step)] {
                    let m = ((best.0 + dx).clamp(-MAX_MV, MAX_MV), (best.1 + dy).clamp(-MAX_MV, MAX_MV));
                    let c = cost(m);
                    if c < best_cost {
                        (best, best_cost, moved) = (m, c, true);
                    }
                }
                if !moved {
                    break;
                }
            }
        }
        (best.0 * 4, best.1 * 4)
    }

    /// Sum of absolute luma differences between the source and the reference moved by `m` (full
    /// samples).
    fn sad(&self, s: &SourceMb, mbx: usize, mby: usize, m: (i32, i32)) -> u32 {
        let r = &self.reference;
        let (pw, ph) = (r.width as i32, r.height as i32);
        let (x0, y0) = ((mbx * 16) as i32 + m.0, (mby * 16) as i32 + m.1);
        let mut sum = 0u32;
        if x0 >= 0 && y0 >= 0 && x0 + 16 <= pw && y0 + 16 <= ph {
            for j in 0..16 {
                let row = (y0 as usize + j) * r.width + x0 as usize;
                if let Some(rr) = r.y.get(row..row + 16) {
                    for (a, b) in s.y[j * 16..j * 16 + 16].iter().zip(rr) {
                        sum += u32::from(a.abs_diff(*b));
                    }
                }
            }
        } else {
            for j in 0..16 {
                let yy = (y0 + j as i32).clamp(0, ph - 1) as usize;
                for i in 0..16 {
                    let xx = (x0 + i as i32).clamp(0, pw - 1) as usize;
                    let b = r.y.get(yy * r.width + xx).copied().unwrap_or(128);
                    sum += u32::from(s.y[j * 16 + i].abs_diff(b));
                }
            }
        }
        sum
    }

    /// Codes the luma residual against `pred` as sixteen 4x4 blocks (inter and, block by block,
    /// Intra_4x4 use the same quantization).
    fn code_luma_4x4_residual(&self, s: &SourceMb, pred: &[u8; 256], c: &mut Coded, intra: bool) {
        for pos in 0..16 {
            let (bx, by) = (pos % 4, pos / 4);
            let mut p = [0u8; 16];
            for j in 0..4 {
                for i in 0..4 {
                    p[j * 4 + i] = pred[(by * 4 + j) * 16 + bx * 4 + i];
                }
            }
            let (levels, recon) = self.code_4x4(s, bx, by, &p, intra);
            c.luma[pos] = levels;
            put4(&mut c.y, bx, by, &recon);
        }
        c.finish_luma();
    }

    /// Codes one 4x4 luma block (`bx`, `by`) against prediction `p`: its levels (scan order) and
    /// reconstruction.
    fn code_4x4(&self, s: &SourceMb, bx: usize, by: usize, p: &[u8; 16], intra: bool) -> ([i32; 16], [u8; 16]) {
        let mut res = [0i32; 16];
        for j in 0..4 {
            for i in 0..4 {
                res[j * 4 + i] = i32::from(s.y[(by * 4 + j) * 16 + bx * 4 + i]) - i32::from(p[j * 4 + i]);
            }
        }
        let q = transform::quant4(&transform::forward4(&res), self.cfg.qp, intra, false);
        let mut levels = [0i32; 16];
        for (k, l) in levels.iter_mut().enumerate() {
            *l = q[ZIGZAG[k]];
        }
        let r = transform::inverse4(&transform::dequant4(&q, self.cfg.qp, None));
        let mut recon = [0u8; 16];
        for k in 0..16 {
            recon[k] = (i32::from(p[k]) + r[k]).clamp(0, 255) as u8;
        }
        (levels, recon)
    }

    fn best_chroma_mode(&self, s: &SourceMb, mbx: usize, mby: usize) -> u8 {
        let a = self.avail16(mbx, mby);
        let cw = self.recon.width / 2;
        let mut best = (0u8, u32::MAX);
        for mode in 0..4u8 {
            let pu = intra::pred_chroma(Plane { data: &self.recon.u, stride: cw }, mbx * 8, mby * 8, mode, a);
            let pv = intra::pred_chroma(Plane { data: &self.recon.v, stride: cw }, mbx * 8, mby * 8, mode, a);
            if let (Some(pu), Some(pv)) = (pu, pv) {
                let cost = satd8(&s.u, &pu) + satd8(&s.v, &pv);
                if cost < best.1 {
                    best = (mode, cost);
                }
            }
        }
        best.0
    }

    fn intra_chroma(&self, c: &mut Coded, s: &SourceMb, mbx: usize, mby: usize, mode: u8) {
        let a = self.avail16(mbx, mby);
        let cw = self.recon.width / 2;
        let pu = intra::pred_chroma(Plane { data: &self.recon.u, stride: cw }, mbx * 8, mby * 8, mode, a);
        let pv = intra::pred_chroma(Plane { data: &self.recon.v, stride: cw }, mbx * 8, mby * 8, mode, a);
        let (pu, pv) = match (pu, pv) {
            (Some(u), Some(v)) => (u, v),
            _ => {
                // DC is always possible.
                c.chroma_mode = 0;
                let pu = intra::pred_chroma(Plane { data: &self.recon.u, stride: cw }, mbx * 8, mby * 8, 0, a).unwrap_or([128; 64]);
                let pv = intra::pred_chroma(Plane { data: &self.recon.v, stride: cw }, mbx * 8, mby * 8, 0, a).unwrap_or([128; 64]);
                (pu, pv)
            }
        };
        self.code_chroma(s, &pu, &pv, c, true);
    }

    /// Codes both chroma components against their predictions.
    fn code_chroma(&self, s: &SourceMb, pu: &[u8; 64], pv: &[u8; 64], c: &mut Coded, intra: bool) {
        let qp = self.qpc;
        let mut any_ac = false;
        let mut any_dc = false;
        for (comp, (src, pred)) in [(&s.u, pu), (&s.v, pv)].into_iter().enumerate() {
            let mut w = [[0i32; 16]; 4];
            let mut dc = [0i32; 4];
            for (b, wb) in w.iter_mut().enumerate() {
                let (bx, by) = (b % 2, b / 2);
                let mut res = [0i32; 16];
                for j in 0..4 {
                    for i in 0..4 {
                        let k = (by * 4 + j) * 8 + bx * 4 + i;
                        res[j * 4 + i] = i32::from(src[k]) - i32::from(pred[k]);
                    }
                }
                *wb = transform::forward4(&res);
                dc[b] = wb[0];
            }
            let dcl = transform::quant_chroma_dc(&dc, qp, intra);
            any_dc |= dcl.iter().any(|&l| l != 0);
            c.chroma_dc[comp] = dcl;
            let dcv = transform::dequant_chroma_dc(&dcl, qp);
            let mut recon = [0u8; 64];
            for (b, wb) in w.iter().enumerate() {
                let (bx, by) = (b % 2, b / 2);
                let q = transform::quant4(wb, qp, intra, true);
                let mut levels = [0i32; 16];
                for (k, l) in levels.iter_mut().enumerate().skip(1) {
                    *l = q[ZIGZAG[k]];
                }
                any_ac |= levels.iter().any(|&l| l != 0);
                c.chroma_ac[comp][b] = levels;
                let r = transform::inverse4(&transform::dequant4(&q, qp, Some(dcv[b])));
                for j in 0..4 {
                    for i in 0..4 {
                        let k = (by * 4 + j) * 8 + bx * 4 + i;
                        recon[k] = (i32::from(pred[k]) + r[j * 4 + i]).clamp(0, 255) as u8;
                    }
                }
            }
            if comp == 0 {
                c.u = recon;
            } else {
                c.v = recon;
            }
        }
        c.cbp_chroma = if any_ac {
            2
        } else if any_dc {
            1
        } else {
            0
        };
        for comp in 0..2 {
            for b in 0..4 {
                c.nz_c[comp][b] = if c.cbp_chroma == 2 { c.chroma_ac[comp][b][1..].iter().filter(|&&l| l != 0).count() as u8 } else { 0 };
            }
        }
    }

    /// The Intra_16x16 mode with the lowest SATD: (mode, prediction, SATD).
    fn best_i16_mode(&self, s: &SourceMb, mbx: usize, mby: usize) -> Option<(u8, [u8; 256], u32)> {
        let a = self.avail16(mbx, mby);
        let plane = Plane { data: &self.recon.y, stride: self.recon.width };
        let mut best: Option<(u8, [u8; 256], u32)> = None;
        for mode in 0..4u8 {
            if let Some(p) = intra::pred16(plane, mbx * 16, mby * 16, mode, a) {
                let cost = satd16(&s.y, &p);
                if best.as_ref().is_none_or(|b| cost < b.2) {
                    best = Some((mode, p, cost));
                }
            }
        }
        best
    }

    /// Intra_16x16 in `mode` with prediction `pred`.
    fn code_i16(&self, s: &SourceMb, mbx: usize, mby: usize, mode: u8, pred: &[u8; 256], chroma_mode: u8) -> Coded {
        let qp = self.cfg.qp;
        let mut c = Coded::new(Kind::I16(mode));
        c.chroma_mode = chroma_mode;
        let mut w = [[0i32; 16]; 16];
        let mut dc = [0i32; 16];
        for (pos, wb) in w.iter_mut().enumerate() {
            let (bx, by) = (pos % 4, pos / 4);
            let mut res = [0i32; 16];
            for j in 0..4 {
                for i in 0..4 {
                    let k = (by * 4 + j) * 16 + bx * 4 + i;
                    res[j * 4 + i] = i32::from(s.y[k]) - i32::from(pred[k]);
                }
            }
            *wb = transform::forward4(&res);
            dc[pos] = wb[0];
        }
        let dcl = transform::quant_luma_dc(&dc, qp);
        for (k, l) in c.dc.iter_mut().enumerate() {
            *l = dcl[ZIGZAG[k]];
        }
        let dcv = transform::dequant_luma_dc(&dcl, qp);
        let mut any_ac = false;
        for (pos, wb) in w.iter().enumerate() {
            let (bx, by) = (pos % 4, pos / 4);
            let q = transform::quant4(wb, qp, true, true);
            let mut levels = [0i32; 16];
            for (k, l) in levels.iter_mut().enumerate().skip(1) {
                *l = q[ZIGZAG[k]];
            }
            any_ac |= levels.iter().any(|&l| l != 0);
            c.luma[pos] = levels;
            let r = transform::inverse4(&transform::dequant4(&q, qp, Some(dcv[pos])));
            for j in 0..4 {
                for i in 0..4 {
                    let k = (by * 4 + j) * 16 + bx * 4 + i;
                    c.y[k] = (i32::from(pred[k]) + r[j * 4 + i]).clamp(0, 255) as u8;
                }
            }
        }
        c.cbp_luma = if any_ac { 15 } else { 0 };
        for pos in 0..16 {
            c.nz_y[pos] = if any_ac { c.luma[pos][1..].iter().filter(|&&l| l != 0).count() as u8 } else { 0 };
        }
        self.intra_chroma(&mut c, s, mbx, mby, chroma_mode);
        c
    }

    /// Intra_4x4: each block in its best mode (SATD plus the mode's cost), coded in decoding order.
    /// The blocks are reconstructed into the picture as they go (the macroblock is rewritten when
    /// it is committed).
    fn code_i4(&mut self, s: &SourceMb, mbx: usize, mby: usize, chroma_mode: u8) -> Coded {
        let mut c = Coded::new(Kind::I4);
        c.chroma_mode = chroma_mode;
        let stride = self.recon.width;
        for (blk, &pos) in BLK_RASTER.iter().enumerate() {
            let (bx, by) = (pos % 4, pos / 4);
            let (x0, y0) = (mbx * 16 + bx * 4, mby * 16 + by * 4);
            let top_right = if by == 0 {
                if bx < 3 { mby > 0 } else { mby > 0 && mbx + 1 < self.mb_w }
            } else {
                bx < 3 && RASTER_BLK[(by - 1) * 4 + bx + 1] < blk
            };
            let a = Avail { left: x0 > 0, top: y0 > 0, top_left: x0 > 0 && y0 > 0, top_right };
            let predicted = self.predicted_mode4(mbx, mby, pos, &c.modes4);
            let mut src = [0u8; 16];
            for j in 0..4 {
                for i in 0..4 {
                    src[j * 4 + i] = s.y[(by * 4 + j) * 16 + bx * 4 + i];
                }
            }
            let plane = Plane { data: &self.recon.y, stride };
            let mut best: Option<(u8, [u8; 16], f64)> = None;
            for mode in 0..9u8 {
                if let Some(p) = intra::pred4(plane, x0, y0, mode, a) {
                    let bits = if mode == predicted { 1.0 } else { 4.0 };
                    let cost = f64::from(satd4(&src, &p)) + self.lambda_sad * bits;
                    if best.as_ref().is_none_or(|b| cost < b.2) {
                        best = Some((mode, p, cost));
                    }
                }
            }
            let (mode, pred, _) = best.unwrap_or((2, [128; 16], 0.0));
            let (levels, recon) = self.code_4x4(s, bx, by, &pred, true);
            c.luma[pos] = levels;
            c.modes4[pos] = mode;
            c.modes4_syntax[blk] = if mode == predicted { None } else { Some(if mode < predicted { mode } else { mode - 1 }) };
            put4(&mut c.y, bx, by, &recon);
            copy_block(&mut self.recon.y, stride, x0, y0, &recon, 4);
        }
        c.finish_luma();
        self.intra_chroma(&mut c, s, mbx, mby, chroma_mode);
        c
    }

    // ---------- syntax ----------

    /// macroblock_layer() of `c` (clause 7.3.5).
    fn write_mb(&self, w: &mut BitWriter, c: &Coded, mbx: usize, mby: usize, p_slice: bool) {
        let intra_offset = if p_slice { 5 } else { 0 };
        match c.kind {
            Kind::Skip => return,
            Kind::Inter => {
                w.ue(0); // P_L0_16x16
                w.se(c.mvd.0);
                w.se(c.mvd.1);
                w.ue(cbp_code(&CBP_INTER, c.cbp_luma | (c.cbp_chroma << 4)));
            }
            Kind::I16(mode) => {
                let t = 1 + u32::from(mode) + 4 * u32::from(c.cbp_chroma) + if c.cbp_luma != 0 { 12 } else { 0 };
                w.ue(intra_offset + t);
                w.ue(u32::from(c.chroma_mode));
            }
            Kind::I4 => {
                w.ue(intra_offset);
                for syntax in &c.modes4_syntax {
                    match syntax {
                        None => w.bit(true),
                        Some(rem) => {
                            w.bit(false);
                            w.put(u32::from(*rem), 3);
                        }
                    }
                }
                w.ue(u32::from(c.chroma_mode));
                w.ue(cbp_code(&CBP_INTRA, c.cbp_luma | (c.cbp_chroma << 4)));
            }
        }
        let i16 = matches!(c.kind, Kind::I16(_));
        if c.cbp_luma == 0 && c.cbp_chroma == 0 && !i16 {
            return;
        }
        w.se(0); // mb_qp_delta
        if i16 {
            let nc = self.nc_luma(mbx, mby, 0, &c.nz_y);
            cavlc::write_block(w, &c.dc, nc);
        }
        for (blk, &pos) in BLK_RASTER.iter().enumerate() {
            if c.cbp_luma & (1 << (blk / 4)) == 0 {
                continue;
            }
            let nc = self.nc_luma(mbx, mby, pos, &c.nz_y);
            let levels = if i16 { &c.luma[pos][1..] } else { &c.luma[pos][..] };
            cavlc::write_block(w, levels, nc);
        }
        if c.cbp_chroma != 0 {
            for comp in 0..2 {
                cavlc::write_block(w, &c.chroma_dc[comp], cavlc::CHROMA_DC);
            }
        }
        if c.cbp_chroma == 2 {
            for comp in 0..2 {
                for b in 0..4 {
                    let nc = self.nc_chroma(mbx, mby, comp, b, &c.nz_c[comp]);
                    cavlc::write_block(w, &c.chroma_ac[comp][b][1..], nc);
                }
            }
        }
    }
}

impl Coded {
    /// Sets the luma coded_block_pattern and TotalCoeff counts from 4x4 block levels.
    fn finish_luma(&mut self) {
        self.cbp_luma = 0;
        for (blk, &pos) in BLK_RASTER.iter().enumerate() {
            let n = self.luma[pos].iter().filter(|&&l| l != 0).count() as u8;
            self.nz_y[pos] = n;
            if n > 0 {
                self.cbp_luma |= 1 << (blk / 4);
            }
        }
    }
}

/// The source samples of one macroblock.
struct SourceMb {
    y: [u8; 256],
    u: [u8; 64],
    v: [u8; 64],
}

fn source_mb(src: &Picture, mbx: usize, mby: usize) -> SourceMb {
    let mut s = SourceMb { y: [0; 256], u: [0; 64], v: [0; 64] };
    read_block(&src.y, src.width, mbx * 16, mby * 16, &mut s.y, 16);
    read_block(&src.u, src.width / 2, mbx * 8, mby * 8, &mut s.u, 8);
    read_block(&src.v, src.width / 2, mbx * 8, mby * 8, &mut s.v, 8);
    s
}

fn read_block(plane: &[u8], stride: usize, x0: usize, y0: usize, out: &mut [u8], size: usize) {
    for j in 0..size {
        let i = (y0 + j) * stride + x0;
        if let (Some(src), Some(dst)) = (plane.get(i..i + size), out.get_mut(j * size..j * size + size)) {
            dst.copy_from_slice(src);
        }
    }
}

fn copy_block(plane: &mut [u8], stride: usize, x0: usize, y0: usize, block: &[u8], size: usize) {
    for j in 0..size {
        let i = (y0 + j) * stride + x0;
        if let (Some(dst), Some(src)) = (plane.get_mut(i..i + size), block.get(j * size..j * size + size)) {
            dst.copy_from_slice(src);
        }
    }
}

/// Writes 4x4 block `b` into a 16x16 block at block position (`bx`, `by`).
fn put4(mb: &mut [u8; 256], bx: usize, by: usize, b: &[u8; 16]) {
    for j in 0..4 {
        for i in 0..4 {
            mb[(by * 4 + j) * 16 + bx * 4 + i] = b[j * 4 + i];
        }
    }
}

fn ssd(s: &SourceMb, c: &Coded) -> u64 {
    let d = |a: &[u8], b: &[u8]| a.iter().zip(b).map(|(x, y)| u64::from(x.abs_diff(*y)).pow(2)).sum::<u64>();
    d(&s.y, &c.y) + d(&s.u, &c.u) + d(&s.v, &c.v)
}

/// Sum of absolute Hadamard-transformed differences of a 4x4 block.
fn satd4(a: &[u8; 16], b: &[u8; 16]) -> u32 {
    let mut d = [0i32; 16];
    for k in 0..16 {
        d[k] = i32::from(a[k]) - i32::from(b[k]);
    }
    transform::hadamard4(&d).iter().map(|v| v.unsigned_abs()).sum::<u32>() / 2
}

fn satd_blocks(a: &[u8], b: &[u8], size: usize) -> u32 {
    let mut sum = 0;
    for by in 0..size / 4 {
        for bx in 0..size / 4 {
            let mut d = [0i32; 16];
            for j in 0..4 {
                let k = (by * 4 + j) * size + bx * 4;
                if let (Some(ra), Some(rb)) = (a.get(k..k + 4), b.get(k..k + 4)) {
                    for i in 0..4 {
                        d[j * 4 + i] = i32::from(ra[i]) - i32::from(rb[i]);
                    }
                }
            }
            sum += transform::hadamard4(&d).iter().map(|v| v.unsigned_abs()).sum::<u32>() / 2;
        }
    }
    sum
}

fn satd16(a: &[u8; 256], b: &[u8; 256]) -> u32 {
    satd_blocks(a, b, 16)
}

fn satd8(a: &[u8; 64], b: &[u8; 64]) -> u32 {
    satd_blocks(a, b, 8)
}

fn median(a: i32, b: i32, c: i32) -> i32 {
    a.max(b).min(a.min(b).max(c))
}

/// nC from the neighbouring blocks' TotalCoeff (None: not available).
fn nc_of(left: Option<u8>, top: Option<u8>) -> i32 {
    match (left, top) {
        (Some(a), Some(b)) => (i32::from(a) + i32::from(b) + 1) >> 1,
        (Some(a), None) => i32::from(a),
        (None, Some(b)) => i32::from(b),
        (None, None) => 0,
    }
}

/// The lowest level (Table A-1) whose frame size, macroblock rate and picture buffer fit, at
/// least 3.0 (lower levels allow too little bit rate for constant-quality video), and 4.1 rather
/// than 4 (more bit rate, the same decoders).
fn level_idc(mb_w: usize, mb_h: usize, fps: u32) -> u8 {
    let mbs = (mb_w * mb_h) as u64;
    let rate = mbs * u64::from(fps.max(1));
    let side = mb_w.max(mb_h) as u64;
    for &(level, max_mbps, max_fs, max_dpb) in &LEVELS {
        if level < 30 || level == 40 {
            continue;
        }
        if mbs <= max_fs && rate <= max_mbps && mbs <= max_dpb && side * side <= 8 * max_fs {
            return level;
        }
    }
    51
}

#[cfg(test)]
pub(crate) mod tests;
