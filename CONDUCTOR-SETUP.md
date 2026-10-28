# Setting up the Epiora conductor (Claude Code + local Devin)

Same model as NeoPhylo: Claude Code is the conductor, each task and review is a fresh local Devin session,
and both act as a GitHub App so only your comments count as commands and only you can merge.

## 1. Repository (once)
1. Put this repo at `~/Epiora` (the wrapper defaults to that path; set `REPO=` to use another).
2. Create `github.com/dorrabbkhan/Epiora` and push `main` from your own shell.
   - Public or private: the open core is AGPL, so public is consistent and gives free standard GitHub-hosted
     runners. Private keeps the spec's business model private, but v0.1's Windows and macOS spike jobs
     consume Actions minutes at a higher rate on private repos. Decide before pushing.
3. Rulesets: `bash docs/conductor/rulesets/apply.sh` (your login). `main` then requires a PR with your Code
   Owner approval (`.github/CODEOWNERS`), and only you can create `v*` tags.

## 2. Tools (once)
- Claude Code installed and signed in (https://code.claude.com/docs).
- Local Devin CLI works non-interactively: `devin --help`; adjust `DEVIN_CMD` in
  `scripts/conductor/devin.sh` if its flags differ from NeoPhylo's.
- `gh auth status` shows your account (your own shell).
- For the spikes: Rust stable, Node 22, R 4.4+, `bats`, Emscripten (Task 4) and Playwright (Tasks 6–8).
  CI covers Windows, macOS and the Android emulator.
- Devin write access: `~/Epiora/.devin/config.local.json` (git-ignored):
  `{ "permissions": { "allow": ["Write(/home/dorrabb/Epiora/**)"] } }`

## 3. The conductor's GitHub identity (once)
1. github.com/settings/apps/new → name `epiora-conductor`, homepage the repo URL, webhook off, installable only
   on your account. Repository permissions: Contents, Pull requests, Issues, Actions, **Workflows** = Read &
   write; Checks, Commit statuses, Metadata = Read; everything else (notably Administration, Secrets) = No
   access. Workflows is needed here (unlike NeoPhylo) because v0.1 tasks add workflow files; `main` stays
   protected by your Code Owner approval.
2. Generate a private key → `~/.config/epiora-conductor/app.pem` (`chmod 600`). Install the app on the Epiora
   repository only.
3. Find the bot's user id: `gh api users/epiora-conductor%5Bbot%5D -q .id`.
4. Write `~/.config/epiora-conductor/app.env`:
   `APP_ID=…`, `APP_CLIENT_ID=…`, `INSTALLATION_ID=…`, `APP_REPO=Epiora`,
   `APP_KEY=$HOME/.config/epiora-conductor/app.pem`, `BOT_USER_ID=…`.
5. Check: `scripts/conductor/app-token.sh --permissions` prints exactly the permissions above.

Your own shell keeps using `gh` and git as you. Every Claude Code session opened in this repo acts as the app
(SessionStart hook `scripts/conductor/session-env.sh`), as do processes started through `scripts/conductor/`.

## 4. Start it
```bash
cd ~/Epiora
tmux new -s epiora              # keeps it running if you close the terminal
claude                          # acts as the app; then type:  /conduct v0.1 1
```
For a range: `/run-plan v0.1 1 11` (pauses at gates and human steps; waits for your merges).
Tasks 3–10 are independent once Tasks 1, 2 and 5 are merged and can run in parallel worktrees.

## 5. Talking to it
Comment on the PR:
- `conductor: fix <instructions>` · `conductor: rerun review` · `conductor: continue` (after you answered an
  "Owner decision needed" comment) · `conductor: hold` / `conductor: next`
In the tmux window, run `/inbox` (or let /run-plan poll it). Watch the sticky "Conductor status" comment on
each PR for the verdict, CI state and what needs your eyes.

## 6. What stays yours
Merging, Code Owner approvals, owner gates (Tasks 2, 5, 8, 10, 11), the human steps in Tasks 8 and 10 (the
reference phone), spec changes, sign-off lines in docs/decisions.md, and release tags. The permission file
denies these to the conductor.
