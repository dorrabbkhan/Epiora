#!/usr/bin/env bash
# Usage: build-prompt.sh <task|review|fix> <version> <task> [PR|-] [extra instructions file]
# Prints a Devin prompt to stdout; pipe it into devin.sh:
#   scripts/conductor/build-prompt.sh task v1.0 3 | scripts/conductor/devin.sh task v1.0 3
#   scripts/conductor/build-prompt.sh fix v1.0 3 63 required.md | scripts/conductor/devin.sh fix v1.0 3
#
# Order matters. The skill is named by its file path, never as a leading "/epiora-…" line:
# with a slash command first, Devin expands the skill and may ignore what follows it. So:
# what to do (skill by path) → owner decisions (per-task notes) → extra instructions →
# standing notes → output contract.
set -euo pipefail
if [ "$#" -lt 3 ]; then
  echo "usage: $0 <task|review|fix> <version> <task> [PR|-] [extra file]" >&2
  exit 2
fi
KIND="$1"; VER="$2"; N="$3"; PR="${4:-}"; EXTRA="${5:-}"
ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
NOTES="$ROOT/docs/conductor/notes/$VER/task$N.md"

# The task skill maps a version to one plan file; a version can have more
# than one plan (v1.0 has two), and a plan outside the mapping must be named
# by path in the prompt so Devin finds it.
case "$VER" in
  v0.1) MAPPED="2026-10-28-epiora-v0.1-spikes.md" ;;
  v0.2) MAPPED="2026-10-31-epiora-v0.2-engine-core.md" ;;
  v0.3) MAPPED="2026-11-05-epiora-v0.3-app-shell.md" ;;
  v1.0) MAPPED="2026-11-12-epiora-v1.0-calculators-and-means.md" ;;
  *)    MAPPED="" ;;
esac

PLAN=""
shopt -s nullglob
for f in "$ROOT"/docs/superpowers/plans/*epiora-"$VER"-*.md; do
  if grep -qE "^### Task $N:" "$f"; then PLAN="$f"; fi
done
shopt -u nullglob
if [ -z "$PLAN" ]; then
  echo "build-prompt.sh: no plan under docs/superpowers/plans/*epiora-$VER-*.md contains \"### Task $N:\"." >&2
  echo "Available: $(ls "$ROOT"/docs/superpowers/plans/ 2>/dev/null | tr '\n' ' ')" >&2
  exit 2
fi
[ "$PR" = "-" ] && PR=""
need_pr() { [ -n "$PR" ] || { echo "build-prompt.sh: $KIND needs a PR number" >&2; exit 2; }; }

# First line: name the skill by path and the plan by its version alias when
# it is the skill's mapped file, else by path (unmapped plans can't be found
# by the version argument alone).
case "$KIND" in
  task)
    if [ "${PLAN##*/}" = "$MAPPED" ]; then
      echo "Implement plan \`$VER\`, Task $N by following \`.devin/skills/epiora-task/SKILL.md\` exactly (arguments: \`$VER $N\`)."
    else
      echo "Implement Task $N of the plan \`docs/superpowers/plans/${PLAN##*/}\` by following \`.devin/skills/epiora-task/SKILL.md\`."
      echo "This plan is not in the skill's version mapping; the plan file is the path given above."
    fi
    ;;
  review)
    need_pr
    if [ "${PLAN##*/}" = "$MAPPED" ]; then
      echo "Review PR #$PR (plan \`$VER\`, Task $N) by following \`.devin/skills/epiora-review/SKILL.md\` exactly (arguments: \`$VER $PR\`)."
    else
      echo "Review PR #$PR for Task $N of the plan \`docs/superpowers/plans/${PLAN##*/}\` by following \`.devin/skills/epiora-review/SKILL.md\`."
      echo "This plan is not in the skill's version mapping; the plan file is the path given above."
    fi
    echo "This is a review session: do not modify files or push. Check the PR out in a separate \`git worktree\`, not in the main checkout."
    ;;
  fix)
    need_pr
    echo "Fix round for PR #$PR (plan \`$VER\`, Task $N). Check out the PR's branch, work only on it and push to it (no new PR, no force-push)."
    echo "Do not re-implement the task; change only what the REQUIRED items below need. Show \`git diff --stat HEAD~1\` matching your claims."
    ;;
  *) echo "usage: $0 <task|review|fix> <version> <task> [PR|-] [extra file]" >&2; exit 2 ;;
esac

if [ -f "$NOTES" ]; then
  echo
  echo "## Owner decisions for this task — authoritative; read before anything else"
  echo
  echo "From \`docs/conductor/notes/$VER/task$N.md\`. Where they approve something a rule or skill would otherwise stop on,"
  echo "they are the \"task says so\" exception, exactly as scoped there and no wider. Do not stop on a question they already answer."
  echo
  cat "$NOTES"
fi

if [ -n "$EXTRA" ]; then
  echo
  echo "## Instructions"
  echo
  cat "$EXTRA"
fi

echo
echo "## Standing notes"
echo
cat "$ROOT/docs/conductor/standing-notes.md"
echo

if [ "$KIND" = "review" ]; then
  echo "End with a line \`VERDICT: <APPROVE|CHANGES REQUIRED>\` listing REQUIRED items explicitly."
else
  echo "## Output contract"
  echo
  cat "$ROOT/scripts/conductor/footer.md"
fi
