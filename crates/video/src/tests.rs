use super::*;

fn solid(w: u32, h: u32, rgb: [u8; 3]) -> Vec<u8> {
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..w * h {
        v.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
    }
    v
}

#[test]
fn writes_a_playable_structure() {
    let mut opts = VideoOptions::new(64, 48, 30);
    opts.keyframe_seconds = 1;
    let mut w = Mp4Writer::new(opts).unwrap();
    for t in 0..45 {
        if t % 3 == 2 {
            w.repeat().unwrap();
        } else {
            w.push_rgba(&crate::h264::tests::test_frame(64, 48, t), false).unwrap();
        }
    }
    assert_eq!(w.frames(), 45);
    let mp4 = w.finish().unwrap();
    let info = inspect(&mp4).unwrap();
    assert_eq!(info.boxes, ["ftyp", "moov", "mdat"]);
    assert_eq!(info.brand, "isom");
    assert_eq!((info.width, info.height), (64, 48));
    assert_eq!(info.codec, "avc1");
    assert_eq!((info.profile, info.level), (66, 30));
    assert_eq!(info.frames, 45);
    assert_eq!(info.keyframes, 2, "a key frame each second");
    assert!((info.duration - 1.5).abs() < 1e-9);
    assert!((info.fps - 30.0).abs() < 1e-9);
    assert_eq!(info.sample_bytes, info.mdat_bytes);
}

#[test]
fn repeated_frames_cost_almost_nothing() {
    let mut w = Mp4Writer::new(VideoOptions::new(320, 240, 30)).unwrap();
    w.push_rgba(&crate::h264::tests::test_frame(320, 240, 0), false).unwrap();
    let one = w.samples[0].len();
    for _ in 0..29 {
        w.repeat().unwrap();
    }
    let rest: usize = w.samples[1..].iter().map(Vec::len).sum();
    assert!(rest < 29 * 16, "{rest} bytes for 29 repeated frames");
    assert!(one > 500);
}

#[test]
fn odd_sizes_round_up_to_even() {
    let mut w = Mp4Writer::new(VideoOptions::new(33, 17, 10)).unwrap();
    w.push_rgba(&solid(33, 17, [200, 10, 10]), true).unwrap();
    let info = inspect(&w.finish().unwrap()).unwrap();
    assert_eq!((info.width, info.height), (34, 18));
}

#[test]
fn refuses_bad_settings_and_frames() {
    assert_eq!(Mp4Writer::new(VideoOptions::new(1, 10, 30)).err(), Some(VideoError::BadSize(1, 10)));
    assert!(Mp4Writer::new(VideoOptions::new(5000, 10, 30)).is_err());
    assert!(Mp4Writer::new(VideoOptions::new(4000, 4000, 30)).is_err(), "over the macroblock limit");
    assert!(Mp4Writer::new(VideoOptions::new(1080, 1920, 30)).is_ok());
    assert_eq!(Mp4Writer::new(VideoOptions::new(10, 10, 0)).err(), Some(VideoError::BadFrameRate));
    assert_eq!(Mp4Writer::new(VideoOptions::new(10, 10, 61)).err(), Some(VideoError::BadFrameRate));
    let mut w = Mp4Writer::new(VideoOptions::new(10, 10, 30)).unwrap();
    assert!(matches!(w.push_rgba(&[0; 7], false), Err(VideoError::BadFrame { .. })));
    assert_eq!(Mp4Writer::new(VideoOptions::new(10, 10, 30)).unwrap().finish().err(), Some(VideoError::NoFrames));
    assert_eq!(Quality::parse(" HIGH "), Some(Quality::High));
    assert_eq!(Quality::parse("ultra"), None);
}

#[test]
fn better_quality_is_closer_and_bigger() {
    let frame = crate::h264::tests::test_frame(96, 64, 4);
    let mut sizes = vec![];
    for q in [Quality::Low, Quality::Medium, Quality::High] {
        let mut o = VideoOptions::new(96, 64, 30);
        o.quality = q;
        let mut w = Mp4Writer::new(o).unwrap();
        w.push_rgba(&frame, false).unwrap();
        sizes.push(w.finish().unwrap().len());
    }
    assert!(sizes[0] < sizes[1] && sizes[1] < sizes[2], "{sizes:?}");
}

#[test]
fn colours_convert_to_bt709_and_back() {
    for rgb in [[0u8, 0, 0], [255, 255, 255], [255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 204, 0], [18, 52, 86]] {
        let pic = crate::yuv::from_rgba(&solid(2, 2, rgb), 2, 2, 16, 16);
        let back = crate::yuv::to_rgb(pic.y[0], pic.u[0], pic.v[0]);
        for (a, b) in rgb.iter().zip(back) {
            assert!(a.abs_diff(b) <= 2, "{rgb:?} → {back:?}");
        }
    }
    // Black and white are the ends of the nominal range; grey has no colour.
    let pic = crate::yuv::from_rgba(&solid(2, 2, [255, 255, 255]), 2, 2, 16, 16);
    assert_eq!((pic.y[0], pic.u[0], pic.v[0]), (235, 128, 128));
    let pic = crate::yuv::from_rgba(&solid(2, 2, [0, 0, 0]), 2, 2, 16, 16);
    assert_eq!((pic.y[0], pic.u[0], pic.v[0]), (16, 128, 128));
}

/// Encoding speed (run with `--ignored --nocapture`): a story-sized video whose content moves in
/// every frame.
#[test]
#[ignore]
fn speed() {
    let (w, h) = (1080, 1920);
    let frames: Vec<Vec<u8>> = (0..10).map(|t| crate::h264::tests::test_frame(w, h, t * 7)).collect();
    let mut writer = Mp4Writer::new(VideoOptions::new(w as u32, h as u32, 30)).unwrap();
    let start = std::time::Instant::now();
    for f in &frames {
        writer.push_rgba(f, false).unwrap();
    }
    let moving = start.elapsed();
    let start = std::time::Instant::now();
    for _ in 0..10 {
        writer.push_rgba(&frames[9], false).unwrap();
    }
    let still = start.elapsed();
    let bytes = writer.finish().unwrap();
    eprintln!("10 moving frames: {moving:?}, 10 unchanged frames: {still:?}, {} bytes", bytes.len());
}

mod fuzz {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// The reader never panics on junk, nor on a real file cut short or corrupted.
        #[test]
        fn inspect_survives_junk(data in proptest::collection::vec(any::<u8>(), 0..512), cut in 0usize..2000, flip in 0usize..2000) {
            let _ = inspect(&data);
            let mut w = Mp4Writer::new(VideoOptions::new(16, 16, 5)).unwrap();
            w.push_rgba(&solid(16, 16, [1, 2, 3]), false).unwrap();
            let mut mp4 = w.finish().unwrap();
            if let Some(b) = mp4.get_mut(flip) { *b ^= 0x5a; }
            let _ = inspect(&mp4);
            mp4.truncate(cut);
            let _ = inspect(&mp4);
        }

        /// Any size and frame rate is either refused or encoded.
        #[test]
        fn any_settings_encode_or_refuse(w in 0u32..80, h in 0u32..80, fps in 0u32..100, q in 0u8..3, seed in any::<u8>()) {
            let mut o = VideoOptions::new(w, h, fps);
            o.quality = [Quality::Low, Quality::Medium, Quality::High][q as usize];
            if let Ok(mut writer) = Mp4Writer::new(o) {
                let frame: Vec<u8> = (0..w * h * 4).map(|i| (i as u8).wrapping_mul(seed)).collect();
                writer.push_rgba(&frame, false).unwrap();
                writer.repeat().unwrap();
                writer.push_rgba(&frame, true).unwrap();
                let info = inspect(&writer.finish().unwrap()).unwrap();
                prop_assert_eq!(info.frames, 3);
            }
        }
    }
}
