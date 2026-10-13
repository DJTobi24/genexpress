use super::tables::*;
use super::transform::*;
use super::*;
use crate::bits::{BitWriter, ue_len};

/// Every code of a table is distinct and none is a prefix of another.
fn assert_prefix_free(codes: &[Vlc], what: &str) {
    let codes: Vec<&Vlc> = codes.iter().filter(|c| c.0 > 0).collect();
    for (i, a) in codes.iter().enumerate() {
        for (j, b) in codes.iter().enumerate() {
            if i == j {
                continue;
            }
            let (short, long) = if a.0 <= b.0 { (a, b) } else { (b, a) };
            let prefix = long.1 >> (long.0 - short.0);
            assert!(!(prefix == short.1), "{what}: code {i} and {j} clash ({a:?}, {b:?})");
        }
    }
}

#[test]
fn code_tables_are_prefix_free() {
    for (name, t) in [("nC 0", &COEFF_TOKEN_NC0), ("nC 2", &COEFF_TOKEN_NC2), ("nC 4", &COEFF_TOKEN_NC4)] {
        let all: Vec<Vlc> = t.iter().flatten().copied().collect();
        assert_eq!(all.iter().filter(|c| c.0 > 0).count(), 62, "{name}");
        assert_prefix_free(&all, name);
    }
    let all: Vec<Vlc> = COEFF_TOKEN_CHROMA_DC.iter().flatten().copied().collect();
    assert_eq!(all.iter().filter(|c| c.0 > 0).count(), 14);
    assert_prefix_free(&all, "chroma DC");
    let all: Vec<Vlc> = (0..17).flat_map(|t| (0..4).filter(move |o| *o <= t).map(move |o| coeff_token_nc8(t, o))).collect();
    assert_prefix_free(&all, "nC 8");
    for (i, t) in TOTAL_ZEROS.iter().enumerate() {
        assert_eq!(t.len(), 16 - i, "total_zeros for {} coefficients", i + 1);
        assert_prefix_free(t, "total_zeros");
    }
    for (i, t) in TOTAL_ZEROS_CHROMA_DC.iter().enumerate() {
        assert_eq!(t.len(), 4 - i);
        assert_prefix_free(t, "chroma total_zeros");
    }
    for (i, t) in RUN_BEFORE.iter().enumerate() {
        assert_eq!(t.len(), if i < 6 { i + 2 } else { 15 });
        assert_prefix_free(t, "run_before");
    }
    let mut a = CBP_INTRA.to_vec();
    let mut b = CBP_INTER.to_vec();
    a.sort();
    b.sort();
    assert_eq!(a, (0..48).collect::<Vec<u8>>());
    assert_eq!(b, (0..48).collect::<Vec<u8>>());
}

#[test]
fn exp_golomb_codes() {
    let mut w = BitWriter::new();
    for v in [0u32, 1, 2, 3, 7, 255, u32::MAX - 1, u32::MAX] {
        let before = w.len();
        w.ue(v);
        assert_eq!((w.len() - before) as u32, ue_len(v), "ue({v})");
    }
    let mut w = BitWriter::new();
    w.ue(0); // 1
    w.ue(1); // 010
    w.ue(2); // 011
    w.se(-1); // 011 (codeNum 2)
    w.trailing();
    assert_eq!(w.into_bytes(), vec![0b1010_0110, 0b1110_0000]);
}

#[test]
fn emulation_prevention() {
    assert_eq!(crate::bits::nal(3, 7, &[0, 0, 1, 0, 0, 0, 5]), vec![0x67, 0, 0, 3, 1, 0, 0, 3, 0, 5]);
}

#[test]
fn transform_round_trip() {
    // At a fine QP the residual comes back nearly exactly.
    let x: [i32; 16] = [12, -7, 30, 4, 0, 5, -20, 9, 3, 3, 3, 3, -60, 40, 2, 1];
    let q = quant4(&forward4(&x), 4, true, false);
    let r = inverse4(&dequant4(&q, 4, None));
    for (a, b) in x.iter().zip(&r) {
        assert!((a - b).abs() <= 1, "{x:?} → {r:?}");
    }
    // DC transforms.
    let dc: [i32; 16] = core::array::from_fn(|i| forward4(&[i as i32 * 3; 16])[0]);
    let back = dequant_luma_dc(&quant_luma_dc(&dc, 10), 10);
    for (i, b) in back.iter().enumerate() {
        // dcY is the scaled DC that the block's inverse transform turns into a flat residual.
        let mut d = [0i32; 16];
        d[0] = *b;
        let flat = inverse4(&d)[0];
        assert!((flat - i as i32 * 3).abs() <= 1, "block {i}: {flat}");
    }
    let cdc = [forward4(&[50; 16])[0], forward4(&[-10; 16])[0], forward4(&[0; 16])[0], forward4(&[7; 16])[0]];
    let back = dequant_chroma_dc(&quant_chroma_dc(&cdc, 10, true), 10);
    for (b, want) in back.iter().zip([50, -10, 0, 7]) {
        let mut d = [0i32; 16];
        d[0] = *b;
        assert!((inverse4(&d)[0] - want).abs() <= 1);
    }
}

#[test]
fn cavlc_block_lengths() {
    // An empty block with nC 0 is the single bit "1".
    let mut w = BitWriter::new();
    assert_eq!(cavlc::write_block(&mut w, &[0; 16], 0), 0);
    assert_eq!(w.len(), 1);
    // Large levels stay codable.
    let mut w = BitWriter::new();
    let levels: Vec<i32> = (0..16).map(|i| if i % 2 == 0 { 5000 } else { -3000 }).collect();
    assert_eq!(cavlc::write_block(&mut w, &levels, 9), 16);
}

#[test]
fn deblocking_tables_have_every_index() {
    assert_eq!((ALPHA.len(), BETA.len()), (52, 52));
    for row in &TC0 {
        assert_eq!(row.len(), 52);
        assert!(row.windows(2).all(|w| w[0] <= w[1]));
    }
    assert!(ALPHA.windows(2).all(|w| w[0] <= w[1]) && BETA.windows(2).all(|w| w[0] <= w[1]));
}

#[test]
fn levels_fit_the_frame_size() {
    assert_eq!(level_idc(25, 19, 30), 30);
    assert_eq!(level_idc(120, 68, 30), 41);
    assert_eq!(level_idc(68, 120, 60), 42);
    assert_eq!(level_idc(80, 45, 30), 31);
}

/// Test pictures: gradients, hard edges, small detail, and a block that moves and fades.
pub(crate) fn test_frame(w: usize, h: usize, t: usize) -> Vec<u8> {
    let mut rgba = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            let mut p = [(x * 255 / w) as u8, (y * 255 / h) as u8, 160, 255];
            // Stripes and a checkerboard ("text-like" detail).
            if y > h / 2 && (x / 3) % 2 == 0 {
                p = [20, 20, 30, 255];
            }
            if x < w / 3 && y < h / 3 && ((x / 2) + (y / 2)) % 2 == 0 {
                p = [250, 250, 250, 255];
            }
            // A moving, fading square.
            let (sx, sy) = (10 + t * 5, 20 + t * 2);
            if x >= sx && x < sx + 40 && y >= sy && y < sy + 30 {
                let a = (t * 30).min(255) as u8;
                p = [255 - a / 2, 40, a, 255];
            }
            rgba[i..i + 4].copy_from_slice(&p);
        }
    }
    rgba
}

/// Encodes frames and returns (file, the reconstructed pictures cropped to the frame size as
/// planar Y′CbCr 4:2:0).
fn encode_with_recon(w: usize, h: usize, frames: usize, qp: u8, deblock: bool) -> (Vec<u8>, Vec<u8>) {
    let (ew, eh) = (w + (w & 1), h + (h & 1));
    let mut enc = Encoder::new(Config { width: ew, height: eh, fps: 25, qp, keyint: 8, deblock });
    let (pw, ph) = enc.coded_size();
    let mut samples = vec![];
    let mut sync = vec![];
    let mut recon = vec![];
    for t in 0..frames {
        let frame = if t == 3 {
            enc.repeat()
        } else {
            let pic = crate::yuv::from_rgba(&test_frame(w, h, t), w, h, pw, ph);
            enc.encode(&pic, false)
        };
        samples.push(frame.data);
        sync.push(frame.keyframe);
        let p = enc.last_picture();
        for y in 0..eh {
            recon.extend_from_slice(&p.y[y * pw..y * pw + ew]);
        }
        for plane in [&p.u, &p.v] {
            for y in 0..eh / 2 {
                recon.extend_from_slice(&plane[y * pw / 2..y * pw / 2 + ew / 2]);
            }
        }
    }
    let track = crate::mp4::Track { width: ew as u32, height: eh as u32, fps: 25, sps: enc.sps(), pps: enc.pps(), profile: enc.profile() };
    (crate::mp4::write(&track, &samples, &sync).unwrap(), recon)
}

fn ffmpeg() -> Option<std::path::PathBuf> {
    let ok = std::process::Command::new("ffmpeg").arg("-version").output().map(|o| o.status.success()).unwrap_or(false);
    ok.then(|| std::path::PathBuf::from("ffmpeg"))
}

/// The decisive check: an independent decoder (FFmpeg's) decodes the stream to exactly the
/// pictures the encoder reconstructed. Skipped where FFmpeg is not installed.
#[test]
fn ffmpeg_decodes_exactly_what_the_encoder_reconstructed() {
    let Some(ffmpeg) = ffmpeg() else {
        eprintln!("ffmpeg not found: skipped");
        return;
    };
    let dir = std::env::temp_dir().join(format!("genha-video-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (w, h, qp, deblock) in [(176, 144, 26, true), (176, 144, 26, false), (99, 61, 12, true), (200, 120, 40, true), (64, 48, 51, true)] {
        let (mp4, recon) = encode_with_recon(w, h, 12, qp, deblock);
        let file = dir.join(format!("t{w}x{h}-{qp}-{deblock}.mp4"));
        std::fs::write(&file, &mp4).unwrap();
        let out = std::process::Command::new(&ffmpeg)
            .args(["-v", "error", "-i"])
            .arg(&file)
            .args(["-f", "rawvideo", "-pix_fmt", "yuv420p", "-"])
            .output()
            .unwrap();
        assert!(out.status.success() && out.stderr.is_empty(), "ffmpeg: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(out.stdout.len(), recon.len(), "{w}x{h}: frame data size");
        let frame = recon.len() / 12;
        for (i, (a, b)) in out.stdout.chunks(frame).zip(recon.chunks(frame)).enumerate() {
            let diff = a.iter().zip(b).filter(|(x, y)| x != y).count();
            assert_eq!(diff, 0, "{w}x{h} qp {qp}: frame {i} differs in {diff} samples");
        }
    }
}
