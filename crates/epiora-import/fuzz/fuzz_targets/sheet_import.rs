#![no_main]
use libfuzzer_sys::fuzz_target;

// Arbitrary bytes behind each container magic (zip, OLE) + an
// arbitrary `ImportSettings` → `import` (v0.3 spec §6.5): must return,
// never panic.
fuzz_target!(|data: &[u8]| {
    epiora_import::sheet_import(data);
});
