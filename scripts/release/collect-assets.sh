#!/usr/bin/env bash
# Release asset collector (docs/releasing.md, Phase 3). Downloads the
# three desktop artifacts of the desktop run the checkpoint verified,
# copies the installable bundles into target/release/v<version>/upload/
# with SHA256SUMS.txt, and prints the gh release commands for the owner.
#
# Read-only towards GitHub and Netlify: it downloads artifacts only — it
# never creates a tag or release, never pushes, never uploads, never
# changes settings.
#
#   scripts/release/collect-assets.sh <version>
#   scripts/release/collect-assets.sh --check-name <version> <filename>
#       bundle filename version check only (used by tests)
#   scripts/release/collect-assets.sh --title <version>
#       release title only (used by tests)
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

V="${2:-${1:-}}"

bundle_version_ok() { # every x.y.z token in a basename must equal $V
  local tokens t
  tokens="$(grep -oE '[0-9]+\.[0-9]+\.[0-9]+' <<<"$1" | sort -u)"
  [ -n "$tokens" ] || return 1
  while IFS= read -r t; do [ "$t" = "$V" ] || return 1; done <<<"$tokens"
}

# The release title lives in exactly one place: the tables in
# docs/releasing.md ("Release title" in Phase 3, "Title" in the values
# table). The scripts read it from the doc so the printed gh command can
# never drift from the checklist the owner follows.
release_title() {
  local t r
  t="$(grep -oP '^\|\s*Title\s*\|\s*`\K[^`]+' docs/releasing.md | head -n1 || true)"
  r="$(grep -oP '^\|\s*Release title\s*\|\s*`\K[^`]+' docs/releasing.md | head -n1 || true)"
  if [ -n "$t" ] && [ -n "$r" ] && [ "$t" != "$r" ]; then
    echo "docs/releasing.md title rows disagree: '$t' vs '$r'" >&2; return 1
  fi
  [ -n "$t$r" ] || return 1
  printf '%s\n' "${t:-$r}"
}

release_title_for() { # <tag> — doc title, validated against the version
  local t
  t="$(release_title)" || return 1
  [[ "$t" == "Epiora $1 "* ]] \
    || { echo "docs/releasing.md title '$t' does not start with 'Epiora $1 '" >&2; return 1; }
  printf '%s\n' "$t"
}

if [ "${1:-}" = "--check-name" ]; then
  { [ $# -eq 3 ] && [[ "$2" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; } \
    || { echo "usage: $0 --check-name <version> <filename>" >&2; exit 2; }
  if bundle_version_ok "$(basename "$3")"; then exit 0; else exit 1; fi
fi

if [ "${1:-}" = "--title" ]; then
  { [ $# -eq 2 ] && [[ "$2" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; } \
    || { echo "usage: $0 --title <version>" >&2; exit 2; }
  release_title_for "v$V" && exit 0
  echo "refusing: no valid release title for v$V in docs/releasing.md" >&2; exit 1
fi

if ! [[ "$V" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "usage: $0 <version>   e.g. $0 0.3.0" >&2; exit 2
fi
TAG="v$V"; OUT="target/release/$TAG"; ENV="$OUT/ci.env"
for t in gh git; do
  command -v "$t" >/dev/null || { echo "missing tool: $t" >&2; exit 2; }
done
[ -f "$ENV" ] || { echo "refusing: no $ENV — run scripts/release/checkpoint.sh $V first" >&2; exit 1; }
# shellcheck disable=SC1090
. "$ENV"
if [ "${FAILS:-0}" != 0 ]; then
  echo "refusing: the checkpoint reported ${FAILS} FAIL(s) — fix them and re-run checkpoint.sh $V" >&2; exit 1
fi
if [ -z "${DESKTOP_RUN:-}" ] || [ -z "${DESKTOP_SHA:-}" ]; then
  echo "refusing: ci.env has no verified desktop run (checkpoint did not pass 'desktop CI')" >&2; exit 1
fi
TITLE="$(release_title_for "$TAG")" \
  || { echo "refusing: no valid release title for $TAG in docs/releasing.md" >&2; exit 1; }

ARTS="$OUT/artifacts"; UP="$OUT/upload"
rm -rf "$ARTS" "$UP"; mkdir -p "$ARTS" "$UP"

oses="ubuntu-24.04 windows-latest macos-latest"
for os in $oses; do
  name="epiora-desktop-$os-$DESKTOP_SHA"
  echo "downloading $name (run $DESKTOP_RUN)"
  gh run download "$DESKTOP_RUN" -n "$name" -D "$ARTS/$os" >/dev/null \
    || { echo "refusing: artifact $name not downloadable (expired? check the run)" >&2; exit 1; }
done

mapfile -t bundles < <(find "$ARTS" -type f \
  \( -iname '*.appimage' -o -iname '*.deb' -o -iname '*.rpm' \
     -o -iname '*.msi' -o -iname '*.exe' -o -iname '*.dmg' \) | sort)
if [ "${#bundles[@]}" -eq 0 ]; then
  echo "refusing: no installable bundles (.AppImage/.deb/.rpm/.msi/.exe/.dmg) in the artifacts" >&2; exit 1
fi

bad=""
for f in "${bundles[@]}"; do
  bundle_version_ok "$(basename "$f")" || bad="$bad\n  $(basename "$f") (in ${f#"$ARTS"/})"
done
if [ -n "$bad" ]; then
  printf 'refusing: bundle version in filename != %s:%b\n' "$V" "$bad" >&2; exit 1
fi

for os in $oses; do
  n="$(find "$ARTS/$os" -type f \
    \( -iname '*.appimage' -o -iname '*.deb' -o -iname '*.rpm' \
       -o -iname '*.msi' -o -iname '*.exe' -o -iname '*.dmg' \) | wc -l)"
  [ "$n" -ge 1 ] || { echo "refusing: no installable bundle in the $os artifact" >&2; exit 1; }
done

for f in "${bundles[@]}"; do cp "$f" "$UP/"; done
(cd "$UP" && sha256sum -- * > SHA256SUMS.txt)

echo
echo "== $TAG upload bundle ($UP)"
ls -l "$UP"
echo
echo "Owner commands (run them yourself — this script is read-only):"
echo
echo "  # create the release as a DRAFT pre-release with the assets attached"
echo "  gh release create $TAG --draft --prerelease --target $HEAD_SHA \\"
echo "    --title \"$TITLE\" --notes-file $OUT/release-notes.md \\"
echo "    $UP/*"
echo
echo "  # or attach to an existing draft release"
echo "  gh release upload $TAG $UP/*"
echo
echo "  # publish the draft (fires the app.yml release deploy)"
echo "  gh release edit $TAG --draft=false"
echo
echo "(title \"$TITLE\" is read from docs/releasing.md — the doc is the source of truth)"
