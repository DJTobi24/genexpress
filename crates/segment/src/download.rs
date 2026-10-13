//! Downloading a model over HTTPS (desktop; the browser uses its own `fetch`).
//!
//! `ureq` with rustls, checking certificates with the operating system's verifier (so company
//! proxies with their own certificate authority work); the proxy comes from `HTTPS_PROXY` /
//! `ALL_PROXY` / `NO_PROXY`. The file is read into memory with a cap of the model's known size,
//! then checked against its pinned SHA-256 before anything is kept.

use std::io::Read;

use crate::{ModelSpec, Result, SegmentError, verify};

/// Bytes read between progress reports (and cancel checks).
const CHUNK: usize = 256 * 1024;

/// Downloads `spec`'s file from `url` (normally `spec.url`; a mirror must serve the same bytes)
/// and checks it. `progress(done, total)` is called as bytes arrive; returning `false` cancels.
pub fn fetch(spec: &ModelSpec, url: &str, progress: &mut dyn FnMut(u64, u64) -> bool) -> Result<Vec<u8>> {
    if !url.starts_with("https://") {
        return Err(SegmentError::Invalid(format!("models are only downloaded over https (got `{}`)", url.chars().take(100).collect::<String>())));
    }
    let tls = ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(tls)
        .timeout_connect(Some(std::time::Duration::from_secs(20)))
        // Between bytes: a stalled connection fails instead of hanging forever.
        .timeout_recv_body(Some(std::time::Duration::from_secs(60)))
        .user_agent(concat!("GenHa/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    let response = agent.get(url).call().map_err(|e| match e {
        ureq::Error::StatusCode(code) => SegmentError::Download(format!("the server answered {code}")),
        ureq::Error::Io(e) => SegmentError::Offline(e.to_string()),
        ureq::Error::HostNotFound | ureq::Error::ConnectionFailed => SegmentError::Offline(e.to_string()),
        ureq::Error::Timeout(_) => SegmentError::Offline(e.to_string()),
        other => SegmentError::Download(other.to_string()),
    })?;
    let total = spec.bytes;
    let mut reader = response.into_body().into_reader();
    let mut out: Vec<u8> = Vec::with_capacity(usize::try_from(total).unwrap_or(0).min(1 << 28));
    let mut buf = vec![0u8; CHUNK];
    if !progress(0, total) {
        return Err(SegmentError::Cancelled);
    }
    loop {
        let n = reader.read(&mut buf).map_err(|e| SegmentError::Offline(e.to_string()))?;
        if n == 0 {
            break;
        }
        out.extend_from_slice(buf.get(..n).unwrap_or_default());
        if out.len() as u64 > total {
            return Err(SegmentError::Size { name: spec.name.into(), got: out.len() as u64, expected: total });
        }
        if !progress(out.len() as u64, total) {
            return Err(SegmentError::Cancelled);
        }
    }
    verify(spec, &out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_plain_http_and_bad_hosts() {
        let spec = &crate::MODELS[0];
        let mut never = |_: u64, _: u64| true;
        assert!(matches!(fetch(spec, "http://example.com/m.onnx", &mut never), Err(SegmentError::Invalid(_))));
        assert!(matches!(fetch(spec, "file:///etc/passwd", &mut never), Err(SegmentError::Invalid(_))));
        // A host that cannot exist (the .invalid top-level domain is reserved): an offline-style error.
        let e = fetch(spec, "https://genha-model.invalid/u2netp.onnx", &mut never).unwrap_err();
        assert!(matches!(e, SegmentError::Offline(_) | SegmentError::Download(_)), "{e}");
    }
}
