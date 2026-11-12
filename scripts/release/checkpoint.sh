#!/usr/bin/env bash
# Pre-release checkpoint (docs/releasing.md, Phase 1). Read-only: it never
# creates a tag or a release, never pushes, never changes GitHub or
# Netlify settings.
#
#   scripts/release/checkpoint.sh <version>            e.g. 0.3.0
#   scripts/release/checkpoint.sh --versions-only <v>  version fields only
#                                                    (used by tests)
#
# Writes target/release/v<version>/checkpoint.md (the report),
# release-notes.md (a draft for the GitHub release body) and ci.env (the
# commit and run ids that scripts/release/collect-assets.sh reads).
# Exit 0 = no FAIL (MANUAL items still need you); exit 1 = at least one FAIL.
set -uo pipefail
cd "$(git rev-parse --show-toplevel)" || exit 2

MODE=full
if [ "${1:-}" = "--versions-only" ]; then MODE=versions; shift; fi
V="${1:-}"
if ! [[ "$V" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "usage: $0 [--versions-only] <version>   e.g. $0 0.3.0" >&2; exit 2
fi
TAG="v$V"; MM="${V%.*}"   # 0.3.0 -> 0.3
OUT="target/release/$TAG"; mkdir -p "$OUT"
REPORT="$OUT/checkpoint.md"
ROWS="$REPORT.rows"; : > "$ROWS"
for t in gh jq python3 git; do
  command -v "$t" >/dev/null || { echo "missing tool: $t" >&2; exit 2; }
done

fails=0; warns=0; manuals=0
row() { # status, check, detail
  printf '| %s | %s | %s |\n' "$1" "$2" "${3//|/\\|}" >> "$ROWS"
  printf '%-7s %s — %s\n' "$1" "$2" "$3"
}
pass()   { row PASS "$1" "$2"; }
fail()   { row FAIL "$1" "$2"; fails=$((fails + 1)); }
warn()   { row WARN "$1" "$2"; warns=$((warns + 1)); }
manual() { row MANUAL "$1" "$2"; manuals=$((manuals + 1)); }
info()   { row INFO "$1" "$2"; }

echo "== Epiora release checkpoint $TAG"

# --- B. Versions (this whole section is --versions-only mode) -------------
ws_version() { python3 - "$1" <<'EOF'
import re, sys
s = open(sys.argv[1]).read()
m = re.search(r'\[workspace\.package\][^\[]*?^version\s*=\s*"([^"]+)"', s, re.M | re.S)
print(m.group(1) if m else '?')
EOF
}
app_v="$(jq -r .version app/package.json)"
lock_v="$(jq -r '.version' app/package-lock.json)"
lock_root_v="$(jq -r '.packages[""].version' app/package-lock.json)"
tauri_v="$(jq -r .version app/src-tauri/tauri.conf.json)"
tauri_cargo_v="$(ws_version app/src-tauri/Cargo.toml)"
engine_v="$(ws_version Cargo.toml)"
bad=""
for pair in "app/package.json=$app_v" "app/package-lock.json=$lock_v" "app/package-lock.json-root=$lock_root_v" \
            "app/src-tauri/tauri.conf.json=$tauri_v" "app/src-tauri/Cargo.toml=$tauri_cargo_v"; do
  [ "${pair##*=}" = "$V" ] || bad="$bad ${pair}"
done
[ -z "$bad" ] && pass "app version" "all app version fields are $V" || fail "app version" "expected $V:$bad"
info "engine version" "Cargo.toml workspace = $engine_v (stamped into receipts; independent of the app version — do not bump for a release, it changes golden receipts)"

if [ "$MODE" = versions ]; then
  echo "Summary (versions only): $fails FAIL, $warns WARN, $manuals MANUAL"
  [ "$fails" -eq 0 ]; exit
fi

python3 -c 'import yaml' 2>/dev/null || { echo "missing python3 yaml module (pip install pyyaml)" >&2; exit 2; }
REPO="$(gh repo view --json nameWithOwner -q .nameWithOwner)" || exit 2

# --- A. Local repository --------------------------------------------------
branch="$(git branch --show-current)"
[ "$branch" = main ] && pass "on main" "branch main" || fail "on main" "current branch is '$branch'; run: git checkout main"
if git diff --quiet && git diff --cached --quiet; then pass "clean tree" "no tracked changes"
else fail "clean tree" "tracked changes present; commit or stash them"; fi
git fetch -q origin main || fail "fetch" "git fetch origin main failed"
HEAD_SHA="$(git rev-parse HEAD)"; ORIGIN_SHA="$(git rev-parse origin/main)"
if [ "$HEAD_SHA" = "$ORIGIN_SHA" ]; then pass "up to date" "HEAD = origin/main = ${HEAD_SHA:0:7}"
else fail "up to date" "HEAD ${HEAD_SHA:0:7} != origin/main ${ORIGIN_SHA:0:7}; run: git pull --ff-only"; fi

# --- C. Tag and release -----------------------------------------------------
if [ -n "$(git ls-remote --tags origin "refs/tags/$TAG")" ]; then fail "tag free" "tag $TAG already exists on origin"
else pass "tag free" "no $TAG tag on origin yet"; fi
if gh api "repos/$REPO/releases/tags/$TAG" >/dev/null 2>&1; then fail "release free" "a published GitHub release for $TAG already exists"
else pass "release free" "no published GitHub release for $TAG yet"; fi
draft="$(gh api "repos/$REPO/releases" -q "[.[] | select(.draft and .tag_name == \"$TAG\")] | length" 2>/dev/null || echo 0)"
[ "$draft" = 0 ] || warn "draft" "a draft release for $TAG already exists — reuse it or delete it"
rs="$(gh api "repos/$REPO/rulesets" -q '[.[] | select(.target == "tag" and .enforcement == "active")] | length' 2>/dev/null || echo "?")"
if [ "$rs" != "?" ] && [ "$rs" -ge 1 ]; then
  info "tag ruleset" "an active tag ruleset protects refs/tags/v* (creation/update/deletion)"
  manual "tag bypass" "Settings → Rules → Rulesets → 'release tags' → Bypass list must include you (e.g. Repository admin), or publishing $TAG is refused"
else
  warn "tag ruleset" "no active tag ruleset found (or not readable) — release tags are unprotected"
fi

# --- D. Merged work -----------------------------------------------------------
open_prs="$(gh pr list --state open --json number,title -q '.[] | "#\(.number) \(.title)"')"
gated="$(grep -E 'OWNER (GATE|REVIEW)' <<<"$open_prs" || true)"
if [ -n "$gated" ]; then fail "owner gates merged" "open gated PRs: $(tr '\n' ';' <<<"$gated")"
else pass "owner gates merged" "no open [OWNER GATE]/[OWNER REVIEW] PR"; fi
others="$(grep -vE 'OWNER (GATE|REVIEW)' <<<"$open_prs" | grep -v '^$' || true)"
[ -z "$others" ] || warn "open PRs" "still open (release ships without them): $(tr '\n' ';' <<<"$others")"
VAL="docs/validation/v$MM.md"
[ -f "$VAL" ] && pass "validation report" "$VAL present" || fail "validation report" "$VAL missing (closing task not merged?)"
if grep -qE "^### v$MM Task [0-9]+ — v$MM closed" docs/decisions.md; then pass "closing record" "decisions.md has the v$MM closing entry"
else fail "closing record" "no '### v$MM Task N — v$MM closed' entry in docs/decisions.md"; fi

# --- E. CI for the release commit -------------------------------------------
# For each workflow that runs on push to main: its latest completed main
# push run must be green, on an ancestor of HEAD, and nothing under the
# workflow's push path filter may have changed since that run's commit.
declare -A RUN_OF SHA_OF URL_OF
for wf in app desktop import engine spikes; do
  file=".github/workflows/$wf.yml"
  mapfile -t paths < <(python3 -c '
import sys, yaml
d = yaml.safe_load(open(sys.argv[1]))
on = d.get(True) or d.get("on") or {}
for p in (on.get("push") or {}).get("paths") or []: print(p)' "$file")
  inprog="$(gh run list --workflow "$wf.yml" --branch main --event push --commit "$HEAD_SHA" \
            --json status -q '[.[] | select(.status != "completed")] | length' 2>/dev/null || echo 0)"
  [ "$inprog" = 0 ] || warn "$wf CI" "a run for HEAD is still in progress — wait for it and re-run this script"
  found=""
  while IFS=$'\t' read -r id sha concl url; do
    [ -n "$id" ] || continue
    git merge-base --is-ancestor "$sha" "$HEAD_SHA" 2>/dev/null || continue
    found=1
    specs=(); for p in "${paths[@]}"; do specs+=(":(glob)$p"); done
    changed="$(git diff --name-only "$sha" "$HEAD_SHA" -- "${specs[@]}")"
    if [ "$concl" != success ]; then
      fail "$wf CI" "latest main run $url is '$concl' on ${sha:0:7} (perf red: rerun twice per the Task 17 policy before calling it real)"
    elif [ -n "$changed" ]; then
      fail "$wf CI" "run $url is on ${sha:0:7} but its paths changed since: $(tr '\n' ' ' <<<"$changed")— wait for the run on HEAD"
    else
      pass "$wf CI" "$url green on ${sha:0:7}; no $wf-path changes since"
      RUN_OF[$wf]="$id"; SHA_OF[$wf]="$sha"; URL_OF[$wf]="$url"
    fi
    break
  done < <(gh run list --workflow "$wf.yml" --branch main --event push --status completed --limit 30 \
             --json databaseId,headSha,conclusion,url -q '.[] | [.databaseId, .headSha, .conclusion, .url] | @tsv')
  [ -n "$found" ] || fail "$wf CI" "no completed main push run on an ancestor of HEAD"
done

PERF_TABLE=""; PERF_FAILED=""; PERF_SIZES=""; PERF_NOTES=""; PERF_JOB_URL=""
if [ -n "${RUN_OF[app]:-}" ]; then
  perf_job="$(gh run view "${RUN_OF[app]}" --json jobs -q '.jobs[] | select(.name == "perf") | "\(.databaseId) \(.conclusion)"')"
  if [ -z "$perf_job" ]; then fail "perf gate" "app run ${RUN_OF[app]} has no perf job"
  else
    pj="${perf_job%% *}"; pc="${perf_job##* }"
    PERF_JOB_URL="${URL_OF[app]}/job/$pj"
    log="$(gh api "repos/$REPO/actions/jobs/$pj/logs" 2>/dev/null | sed -E 's/^[0-9TZ:.+-]+ //')"
    PERF_TABLE="$(awk '/^\| scenario \|/{t=1} t&&/^\|/{print;next} t{exit}' <<<"$log" | awk '!seen[$0]++')"
    PERF_FAILED="$(grep -E '^- run [0-9]+ · ' <<<"$log" | sed -E 's/ · cpu=.*$//' | awk '!seen[$0]++')"
    PERF_SIZES="$(awk '/^\| file \|/{t=1} t&&/^\|/{print;next} t{exit}' <<<"$log" | awk '!seen[$0]++')"
    PERF_NOTES="$(grep -E '^(First-load budget|import-wasm gzip)' <<<"$log" | awk '!seen[$0]++')"
    if [ "$pc" = success ]; then pass "perf gate" "owner thresholds held (job $pj)"
    else fail "perf gate" "perf job $pj is '$pc'"; fi
  fi
fi

if [ -n "${RUN_OF[desktop]:-}" ]; then
  dr="${RUN_OF[desktop]}"; ds="${SHA_OF[desktop]}"
  arts="$(gh api "repos/$REPO/actions/runs/$dr/artifacts" -q '.artifacts[] | select(.expired | not) | .name')"
  missing=""
  for os in ubuntu-24.04 windows-latest macos-latest; do
    grep -qx "epiora-desktop-$os-$ds" <<<"$arts" || missing="$missing $os"
  done
  [ -z "$missing" ] && pass "desktop bundles" "Linux, Windows and macOS artifacts present in run $dr" \
                    || fail "desktop bundles" "missing or expired in run $dr:$missing"
  # The Linux smoke step exists on every matrix leg but runs only on
  # ubuntu; read it from the ubuntu job, not the first match overall.
  smoke="$(gh run view "$dr" --json jobs -q '[.jobs[] | select(.name | test("ubuntu")) | .steps[]? | select(.name | startswith("Linux smoke test")) | .conclusion] | first // empty')"
  [ "$smoke" = success ] && pass "desktop smoke" "Linux smoke test passed in run $dr" || fail "desktop smoke" "Linux smoke test step is '${smoke:-absent}' in run $dr"
fi

# --- F. Netlify ---------------------------------------------------------------
if names="$(gh secret list --json name -q '.[].name' 2>/dev/null)"; then
  for s in NETLIFY_AUTH_TOKEN NETLIFY_SITE_ID; do
    grep -qx "$s" <<<"$names" && pass "secret $s" "set" || fail "secret $s" "missing: production deploy will fail"
  done
else
  manual "Netlify secrets" "your token cannot list secrets: check Settings → Secrets and variables → Actions has NETLIFY_AUTH_TOKEN and NETLIFY_SITE_ID"
fi
manual "Netlify token" "NETLIFY_AUTH_TOKEN has not expired"
manual "browser check" "Phase 2 of docs/releasing.md on the latest preview"
manual "desktop check" "Phase 2 of docs/releasing.md: install the three bundles"

# --- Outputs: ci.env, checkpoint.md, release-notes.md -------------------------
{
  echo "APP_RUN=${RUN_OF[app]:-}"; echo "APP_SHA=${SHA_OF[app]:-}"
  echo "DESKTOP_RUN=${RUN_OF[desktop]:-}"; echo "DESKTOP_SHA=${SHA_OF[desktop]:-}"
  echo "HEAD_SHA=$HEAD_SHA"; echo "TAG=$TAG"; echo "REPO=$REPO"
  echo "FAILS=$fails"
} > "$OUT/ci.env"

{
  echo "# Release checkpoint $TAG"
  echo
  echo "- Repo: $REPO"
  echo "- Release candidate: \`$HEAD_SHA\` (HEAD of \`main\` when this ran)"
  echo "- Generated: $(date -u '+%Y-%m-%dT%H:%M:%SZ') by \`scripts/release/checkpoint.sh $V\`"
  if [ "$fails" -eq 0 ]; then
    echo "- Result: **0 FAIL · $warns WARN · $manuals MANUAL** — automated checks green; finish the MANUAL items before publishing"
  else
    echo "- Result: **$fails FAIL · $warns WARN · $manuals MANUAL** — NOT ready; fix every FAIL row and re-run"
  fi
  echo
  echo "| status | check | detail |"
  echo "|---|---|---|"
  cat "$ROWS"
  if [ -n "$PERF_TABLE" ]; then
    echo
    echo "## Performance — app run ${URL_OF[app]}, perf job $pj"
    echo
    echo "$PERF_TABLE"
    if [ -n "$PERF_FAILED" ]; then
      echo
      echo "Failed samples (truncated — see the job log for the full lines):"
      echo
      echo "$PERF_FAILED"
    fi
    if [ -n "$PERF_SIZES" ]; then
      echo
      echo "### Sizes (gzip -9, bytes)"
      echo
      echo "$PERF_SIZES"
      [ -z "$PERF_NOTES" ] || { echo; echo "$PERF_NOTES"; }
    fi
  fi
  echo
  echo "_MANUAL rows are confirmations only the owner can make; see docs/releasing.md. This script is read-only._"
} > "$REPORT"

# Known limitations: the closing entry's "Known follow-ups" bullet, read
# from docs/decisions.md so the notes never drift from the record.
KNOWN="$(python3 - "$MM" <<'EOF'
import re, sys
mm = re.escape(sys.argv[1])
lines = open("docs/decisions.md", encoding="utf-8").read().splitlines()
start = next((i for i, l in enumerate(lines)
              if re.match(rf"### v{mm} Task \d+ — v{mm} closed", l)), None)
if start is None:
    sys.exit(0)
end = next((i for i in range(start + 1, len(lines))
            if lines[i].startswith("### ")), len(lines))
sec = lines[start:end]
for i, l in enumerate(sec):
    if l.startswith("- Known follow-ups"):
        out = [l]
        for c in sec[i + 1:]:
            if c.startswith("- "):
                break
            out.append(c)
        print("\n".join(out).strip())
        break
EOF
)"
[ -n "$KNOWN" ] || KNOWN="- (the v$MM closing entry is not merged yet — the Known follow-ups bullet lands with it)"

BLOB="https://github.com/$REPO/blob/$HEAD_SHA"
export V TAG REPO HEAD_SHA BLOB KNOWN PERF_TABLE PERF_FAILED PERF_SIZES PERF_NOTES PERF_JOB_URL VAL
export APP_URL="${URL_OF[app]:-}" DESKTOP_URL="${URL_OF[desktop]:-}" IMPORT_URL="${URL_OF[import]:-}" ENGINE_URL="${URL_OF[engine]:-}" SPIKES_URL="${URL_OF[spikes]:-}"
python3 - <<'PYEOF' > "$OUT/release-notes.md"
import os

v = os.environ["V"]; tag = os.environ["TAG"]; repo = os.environ["REPO"]
sha = os.environ["HEAD_SHA"]; blob = os.environ["BLOB"]
val = os.environ["VAL"]

def run(name):
    u = os.environ.get(name.upper() + "_URL")
    return f"[{name}]({u})" if u else f"{name} (no verified run — checkpoint not green)"

print(f"""Epiora is a local-first, open-core successor to CDC's Epi Info. {tag}
is the **app-shell** release: the v0.2 validated statistics engine
under a project UI — record-level change log, preview-and-confirm
import (CSV/TSV/Excel/ODS), an analysis workspace with a persisted
output log and receipts, offline PWA with a cross-tab lock, an encrypted
desktop store, and owner-picked performance and size gates.

**Web app:** <https://epiora-dev.netlify.app> — deployed to production
automatically when this release is published.

> Pre-release of the 0.x internal foundation: desktop builds are
> **unsigned** and there are no accounts or telemetry.

## Desktop bundles (unsigned)

Verify every download against `SHA256SUMS.txt`
(`sha256sum -c SHA256SUMS.txt`; on Windows
`certutil -hashfile <file> SHA256`).

- **Linux** — `Epiora_{v}_amd64.AppImage` (`chmod +x`, then run),
  `Epiora_{v}_amd64.deb` (`sudo apt install ./Epiora_{v}_amd64.deb`),
  `Epiora-{v}-1.x86_64.rpm`.
- **Windows** — `Epiora_{v}_x64-setup.exe` (NSIS installer) or
  `Epiora_{v}_x64_en-US.msi`. SmartScreen shows "Windows protected your
  PC" for unsigned builds → **More info → Run anyway**.
- **macOS (Apple silicon)** — `Epiora_{v}_aarch64.dmg`. Gatekeeper
  blocks unsigned apps: right-click the app → **Open**, or
  `xattr -dr com.apple.quarantine /Applications/Epiora.app`.

## Evidence (release commit `{sha}`)

- Validation report: [{val}]({blob}/{val})
- CI at the release commit: {run('app')} · {run('desktop')} · {run('import')} · {run('engine')} · {run('spikes')}
- Decision log: [docs/decisions.md]({blob}/docs/decisions.md)
""")

pt = os.environ.get("PERF_TABLE", "")
if pt:
    print(f"Performance — `perf` job of the app run ({os.environ.get('PERF_JOB_URL', '')}):\n")
    print(pt)
    pf = os.environ.get("PERF_FAILED", "")
    if pf:
        print("\nFailed samples (truncated):\n")
        print(pf)
    ps = os.environ.get("PERF_SIZES", "")
    if ps:
        print("\n### Sizes (gzip -9, bytes)\n")
        print(ps)
    pn = os.environ.get("PERF_NOTES", "")
    if pn:
        print("\n" + pn)

print(f"""
## Known limitations

{os.environ['KNOWN']}
""")
PYEOF

echo
echo "wrote $REPORT and $OUT/release-notes.md (and $OUT/ci.env)"
echo "Summary: $fails FAIL, $warns WARN, $manuals MANUAL"
[ "$fails" -eq 0 ]
