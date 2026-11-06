#![no_main]
use libfuzzer_sys::fuzz_target;

// Arbitrary bytes + an arbitrary `ImportSettings` → `import` (v0.3 spec
// §6.5): must return, never panic.
fuzz_target!(|data: &[u8]| {
    epiora_import::text_import(data);
});
