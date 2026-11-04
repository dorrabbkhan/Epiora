#!/usr/bin/env bats
# scripts/wasm-size.sh enforces the spec §5.4 size budget (E3): the
# optimised engine .wasm must be ≤ 409,600 bytes after `gzip -9`.

REPO="$BATS_TEST_DIRNAME/../.."

@test "a 1 KB wasm file passes and prints its size" {
  head -c 1024 /dev/zero > "$BATS_TEST_TMPDIR/small.wasm"
  run "$REPO/scripts/wasm-size.sh" "$BATS_TEST_TMPDIR/small.wasm"
  [ "$status" -eq 0 ]
  # The printed size is the gzip -9 byte count — a bare integer.
  [[ "$output" =~ ^[0-9]+$ ]]
  [ "$output" -gt 0 ]
  [ "$output" -le 409600 ]
}

@test "a 500 KB random wasm file fails" {
  head -c 512000 /dev/urandom > "$BATS_TEST_TMPDIR/big.wasm"
  run "$REPO/scripts/wasm-size.sh" "$BATS_TEST_TMPDIR/big.wasm"
  [ "$status" -eq 1 ]
}
