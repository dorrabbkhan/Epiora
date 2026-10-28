#!/usr/bin/env bash
# OWNER ONLY. Run from your own shell (your gh login, not the app's):
#   bash docs/conductor/rulesets/apply.sh
# 1. main: PR + 1 Code Owner approval (not by the last pusher), stale
#    approvals dismissed, no force-push or deletion; repo admins (you)
#    may bypass. The epiora-conductor app is not a bypass actor.
# 2. v* tags: only repo admins (you) may create, move or delete them.
# Creates each ruleset on first run and updates it by name afterwards.
set -euo pipefail
cd "$(dirname "$0")"
R=repos/dorrabbkhan/Epiora
case "$(gh api user -q .login)" in
  dorrabbkhan) ;;
  *) echo "run this as dorrabbkhan, not through scripts/conductor" >&2; exit 1 ;;
esac
upsert() {
  local name="$1" file="$2" id
  id=$(gh api "$R/rulesets" -q ".[] | select(.name == \"$name\") | .id")
  if [ -n "$id" ]; then gh api -X PUT "$R/rulesets/$id" --input "$file" -q '"updated: \(.name)"'
  else gh api -X POST "$R/rulesets" --input "$file" -q '"created: \(.name)"'; fi
}
upsert main main.json
upsert "release tags" tags.json
