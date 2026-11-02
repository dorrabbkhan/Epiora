#!/usr/bin/env bats
# The std f64 transcendental methods are banned in epiora-core by
# crates/epiora-core/clippy.toml (spec §3.4, E1): platform maths would
# diverge between native and wasm. libm is the only allowed source.

REPO="$BATS_TEST_DIRNAME/../.."

setup() {
  cp "$REPO/Cargo.toml" "$REPO/rust-toolchain.toml" "$REPO/deny.toml" "$BATS_TEST_TMPDIR/"
  cp -r "$REPO/crates" "$BATS_TEST_TMPDIR/"
}

@test "std ln is rejected by clippy in epiora-core" {
  printf '\npub fn probe(x: f64) -> f64 { x.ln() }\n' \
    >> "$BATS_TEST_TMPDIR/crates/epiora-core/src/lib.rs"
  run bash -c "cd '$BATS_TEST_TMPDIR' && cargo clippy -p epiora-core -- -D warnings"
  [ "$status" -ne 0 ]
  [[ "$output" == *disallowed* ]]
}

@test "libm ln is accepted" {
  printf '\npub fn probe(x: f64) -> f64 { libm::log(x) }\n' \
    >> "$BATS_TEST_TMPDIR/crates/epiora-core/src/lib.rs"
  run bash -c "cd '$BATS_TEST_TMPDIR' && cargo clippy -p epiora-core -- -D warnings"
  [ "$status" -eq 0 ]
}
