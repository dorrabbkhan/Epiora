#![no_main]
use libfuzzer_sys::fuzz_target;

// Arbitrary bytes → decode → parse → plan (v1.1 spec §6.5, Task 12):
// never a panic; failures are a `pgm_syntax`-coded LangError, and
// every Unsupported action names its command.
fuzz_target!(|data: &[u8]| {
    epiora_lang::fuzz_harness::pgm_any(data);
});
