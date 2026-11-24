#![no_main]
use libfuzzer_sys::fuzz_target;

// Arbitrary bytes → dataset + command sequence → replay twice (v1.1
// spec §6.5): equal view hashes or a typed LangError, never a panic.
fuzz_target!(|data: &[u8]| {
    epiora_lang::fuzz_harness::recipe_any(data);
});
