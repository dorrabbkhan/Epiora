//! Fixes the engine's git SHA at build time (spec §4.1): the engine
//! reads no environment at run time — `EPIORA_GIT_SHA` comes from the
//! `GIT_SHA` build variable, falling back to `"unknown"`.

fn main() {
    let sha = std::env::var("GIT_SHA").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=EPIORA_GIT_SHA={sha}");
    println!("cargo:rerun-if-env-changed=GIT_SHA");
}
