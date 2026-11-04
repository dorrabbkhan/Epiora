#!/usr/bin/env bats
# crates/epiora-wasm/parity/diff.sh compares the native and wasm parity
# files bit for bit (spec §5.2): any difference fails and names the
# fixture id whose line disagrees.

REPO="$BATS_TEST_DIRNAME/../.."
DIFF="$REPO/crates/epiora-wasm/parity/diff.sh"

@test "identical parity files pass" {
  cat > "$BATS_TEST_TMPDIR/a.jsonl" <<'EOF'
{"actual":{"values":["3ff0000000000000"]},"id":"special/ln_gamma","receipt":null}
{"actual":{"x":["4000000000000000"]},"id":"2x2/oswego_vanilla","receipt":null}
EOF
  cp "$BATS_TEST_TMPDIR/a.jsonl" "$BATS_TEST_TMPDIR/b.jsonl"
  run "$DIFF" "$BATS_TEST_TMPDIR/a.jsonl" "$BATS_TEST_TMPDIR/b.jsonl"
  [ "$status" -eq 0 ]
}

@test "one flipped hex digit fails and prints the fixture id" {
  cat > "$BATS_TEST_TMPDIR/a.jsonl" <<'EOF'
{"actual":{"values":["3ff0000000000000"]},"id":"special/ln_gamma","receipt":null}
{"actual":{"x":["4000000000000000"]},"id":"2x2/oswego_vanilla","receipt":null}
EOF
  # One bit in one leaf's bit pattern.
  sed 's/4000000000000000/4000000000000001/' "$BATS_TEST_TMPDIR/a.jsonl" \
    > "$BATS_TEST_TMPDIR/b.jsonl"
  run "$DIFF" "$BATS_TEST_TMPDIR/a.jsonl" "$BATS_TEST_TMPDIR/b.jsonl"
  [ "$status" -eq 1 ]
  [[ "$output" == *"2x2/oswego_vanilla"* ]]
}
