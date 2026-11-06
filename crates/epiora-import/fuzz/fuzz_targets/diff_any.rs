#![no_main]
use libfuzzer_sys::fuzz_target;

// Two small arbitrary datasets + a key choice → `diff` (plan v0.3 Task
// 5): must return, never panic, and on `Ok` applying the diff to the
// previous dataset's records reproduces the next dataset's records.
fuzz_target!(|data: &[u8]| {
    epiora_import::diff_any(data);
});
