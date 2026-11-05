#![no_main]
use libfuzzer_sys::fuzz_target;

// Arbitrary bytes → `decode_snapshot` (v0.3 spec §5.1). Decoding is
// strict about the canonical form, so any `Ok` must re-encode to the
// same bytes.
fuzz_target!(|data: &[u8]| {
    if let Ok(ds) = epiora_core::snapshot::decode_snapshot("fuzz", 1, data) {
        assert_eq!(epiora_core::snapshot::encode_snapshot(&ds), data);
    }
});
