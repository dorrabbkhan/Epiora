#!/usr/bin/env bats

ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
CK="$ROOT/scripts/release/checkpoint.sh"
CA="$ROOT/scripts/release/collect-assets.sh"
VD="$ROOT/scripts/release/verify-deploy.py"

setup() {
  DIR="$(mktemp -d)"
}

teardown() {
  rm -rf "$DIR"
}

# A temp git repo with the five version files checkpoint.sh reads.
fake_repo() { # version_to_write
  mkdir -p "$DIR/repo/app/src-tauri"
  git -C "$DIR/repo" init -q
  printf '{"version":"%s"}' "$1" > "$DIR/repo/app/package.json"
  printf '{"version":"%s","packages":{"":{"version":"%s"}}}' "$1" "$1" > "$DIR/repo/app/package-lock.json"
  printf '{"version":"%s"}' "$1" > "$DIR/repo/app/src-tauri/tauri.conf.json"
  printf '[workspace.package]\nversion = "%s"\n' "$1" > "$DIR/repo/app/src-tauri/Cargo.toml"
  printf '[workspace.package]\nversion = "0.2.0"\n' > "$DIR/repo/Cargo.toml"
}

@test "checkpoint.sh: usage error on no args and on a bad version" {
  run "$CK"
  [ "$status" -eq 2 ]
  [[ "$output" == *usage:* ]]
  run "$CK" 0.3
  [ "$status" -eq 2 ]
  [[ "$output" == *usage:* ]]
}

@test "checkpoint.sh --versions-only fails on a version mismatch" {
  fake_repo 9.9.9
  run bash -c "cd '$DIR/repo' && '$CK' --versions-only 0.3.0"
  [ "$status" -eq 1 ]
  [[ "$output" == *"expected 0.3.0"* ]]
}

@test "checkpoint.sh --versions-only passes when every field matches" {
  fake_repo 0.3.0
  run bash -c "cd '$DIR/repo' && '$CK' --versions-only 0.3.0"
  [ "$status" -eq 0 ]
  [[ "$output" == *"all app version fields are 0.3.0"* ]]
}

@test "collect-assets.sh: usage error on no args" {
  run "$CA"
  [ "$status" -eq 2 ]
  [[ "$output" == *usage:* ]]
}

@test "collect-assets.sh refuses when the checkpoint has not run" {
  run "$CA" 9.9.9
  [ "$status" -eq 1 ]
  [[ "$output" == *"no target/release/v9.9.9/ci.env"* ]]
}

@test "collect-assets.sh refuses when the checkpoint reported FAILs" {
  mkdir -p "$DIR/repo/target/release/v9.9.9"
  git -C "$DIR/repo" init -q
  cat > "$DIR/repo/target/release/v9.9.9/ci.env" <<'EOF'
DESKTOP_RUN=1
DESKTOP_SHA=abc
HEAD_SHA=abc
TAG=v9.9.9
REPO=x/y
FAILS=2
EOF
  run bash -c "cd '$DIR/repo' && '$CA' 9.9.9"
  [ "$status" -eq 1 ]
  [[ "$output" == *"2 FAIL(s)"* ]]
}

@test "collect-assets.sh --check-name enforces the filename version" {
  run "$CA" --check-name 0.3.0 "Epiora_0.3.0_x64-setup.exe"
  [ "$status" -eq 0 ]
  run "$CA" --check-name 0.3.0 "Epiora_9.9.9_x64-setup.exe"
  [ "$status" -eq 1 ]
  run "$CA" --check-name 0.3.0 "Epiora-0.3.0-1.x86_64.rpm"
  [ "$status" -eq 0 ]
  run "$CA" --check-name 0.3.0 "README.txt"
  [ "$status" -eq 1 ]
  run "$CA" --check-name 0.3.0
  [ "$status" -eq 2 ]
}

@test "verify-deploy.py: usage error on no args" {
  run python3 "$VD"
  [ "$status" -eq 2 ]
}

@test "verify-deploy.py --self-test passes compliant and catches broken" {
  run python3 "$VD" --self-test
  [ "$status" -eq 0 ]
  [[ "$output" == *"self-test OK"* ]]
}
