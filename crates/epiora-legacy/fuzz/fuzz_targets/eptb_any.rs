#![no_main]
use libfuzzer_sys::fuzz_target;

// Arbitrary bytes → EPTB `decode_batch`, the boundary wasm
// `legacyPush` exposes (v1.1 spec §6.5, plan Task 13): must return a
// typed `LegacyError` or decode — never panic.
fuzz_target!(|data: &[u8]| {
    epiora_legacy::eptb_any(data);
});
