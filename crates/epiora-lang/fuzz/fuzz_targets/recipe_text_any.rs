#![no_main]
use libfuzzer_sys::fuzz_target;

// Arbitrary bytes → decoded text → `Recipe::parse`, the boundary wasm
// `replay`'s recipe text crosses (v1.1 spec §6.5, plan Task 13):
// typed `LangError` or a round-tripping recipe — never a panic.
fuzz_target!(|data: &[u8]| {
    epiora_lang::fuzz_harness::recipe_text_any(data);
});
