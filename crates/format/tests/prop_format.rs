//! Untrusted input: arbitrary bytes and mangled files never panic the reader.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use genexpress_format::{from_bytes, to_bytes};
use proptest::prelude::*;

proptest! {
    #[test]
    fn arbitrary_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..2048)) {
        let _ = from_bytes(&bytes);
    }

    #[test]
    fn mangled_files_never_panic(pos in 0usize..4096, val in any::<u8>(), compress in any::<bool>()) {
        let doc = genexpress_doc::Document::new("x", 100.0, 100.0);
        let mut bytes = to_bytes(&doc, compress).unwrap();
        let len = bytes.len();
        if let Some(b) = bytes.get_mut(pos % len) { *b = val; }
        let _ = from_bytes(&bytes);
        bytes.truncate(pos % len);
        let _ = from_bytes(&bytes);
    }
}
