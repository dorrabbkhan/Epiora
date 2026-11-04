#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    epiora_core::fuzz_harness::freq_any(data);
});
