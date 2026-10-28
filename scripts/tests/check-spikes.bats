#!/usr/bin/env bats

CHECK="$BATS_TEST_DIRNAME/../check-spikes.sh"

setup() {
  DIR="$(mktemp -d)"
}

teardown() {
  rm -rf "$DIR"
}

write_complete_report() {
  cat >"$DIR/report.md" <<'EOF'
# Example spike

**Status:** go

## Question

What is being asked?

## Method

How it was probed.

## Environment

Where it ran.

## Measurements

Numbers from runs.

## Decision

go

## Consequences

What follows.

## Licences

None.
EOF
}

@test "passes on a complete report" {
  write_complete_report
  run "$CHECK" "$DIR"
  [ "$status" -eq 0 ]
}

@test "fails naming the missing heading" {
  write_complete_report
  sed -i '/^## Licences$/,$d' "$DIR/report.md"
  run "$CHECK" "$DIR"
  [ "$status" -eq 1 ]
  [[ "$output" == *Licences* ]]
}

@test "fails on missing status" {
  write_complete_report
  sed -i '/^\*\*Status:\*\*/d' "$DIR/report.md"
  run "$CHECK" "$DIR"
  [ "$status" -eq 1 ]
}

@test "ignores TEMPLATE.md" {
  echo "# Template without headings or status" >"$DIR/TEMPLATE.md"
  run "$CHECK" "$DIR"
  [ "$status" -eq 0 ]
}
