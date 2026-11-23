#![no_main]
use libfuzzer_sys::fuzz_target;

// Arbitrary bytes → open + table list + first batch (v1.1 spec §6.5):
// must return, never panic.
fuzz_target!(|data: &[u8]| {
    epiora_mdb::mdb_any(data);
});
