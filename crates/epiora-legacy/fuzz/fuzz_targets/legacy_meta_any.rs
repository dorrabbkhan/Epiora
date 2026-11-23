#![no_main]
use libfuzzer_sys::fuzz_target;

// Arbitrary bytes → generated meta/data tables → detect + map_ei7
// (v1.1 spec §6.5, plan Task 7): must return, never panic.
fuzz_target!(|data: &[u8]| {
    epiora_legacy::legacy_meta_any(data);
});
