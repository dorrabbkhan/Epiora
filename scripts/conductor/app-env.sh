# Source this to act as the epiora-conductor GitHub App (bash).
# - gh: the wrapper in bin/ goes first on PATH (fresh token per call).
# - git: github.com SSH remotes are rewritten to HTTPS and authenticated
#   by bin/git-credential-epiora; commits are authored by the bot.
# Used by devin.sh, as-app.sh and start.sh. Your own shell is unaffected.
# BOT_USER_ID comes from ~/.config/epiora-conductor/app.env (see CONDUCTOR-SETUP.md).
_epiora_conf="${EPIORA_APP_CONF:-$HOME/.config/epiora-conductor/app.env}"
# shellcheck disable=SC1090
[ -r "$_epiora_conf" ] && source "$_epiora_conf"
_epiora_bin="$(cd "$(dirname "${BASH_SOURCE[0]}")/bin" && pwd)"
export PATH="$_epiora_bin:$PATH"
export GIT_CONFIG_COUNT=3
export GIT_CONFIG_KEY_0="url.https://github.com/.insteadOf" GIT_CONFIG_VALUE_0="git@github.com:"
export GIT_CONFIG_KEY_1="credential.https://github.com.helper" GIT_CONFIG_VALUE_1=""
export GIT_CONFIG_KEY_2="credential.https://github.com.helper" GIT_CONFIG_VALUE_2="$_epiora_bin/git-credential-epiora"
export GIT_AUTHOR_NAME="epiora-conductor[bot]" GIT_COMMITTER_NAME="epiora-conductor[bot]"
export GIT_AUTHOR_EMAIL="${BOT_USER_ID:?set BOT_USER_ID in app.env}+epiora-conductor[bot]@users.noreply.github.com"
export GIT_COMMITTER_EMAIL="$GIT_AUTHOR_EMAIL"
export EPIORA_ACTOR="epiora-conductor[bot]"
unset _epiora_bin _epiora_conf
