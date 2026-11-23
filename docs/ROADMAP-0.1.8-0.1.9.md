# Fikra Code 0.1.8 and 0.1.9 plan

Written 2026-11-24 so any agent (including Fikra Code itself) can pick up an
item, implement it, and prove it is done. Work one item per branch/PR; CI
(`.github/workflows/fikra-ci.yml`) must pass before merge. Release = bump the
workspace version in `codex-rs/Cargo.toml` + workspace entries in
`codex-rs/Cargo.lock` (see commit "Release 0.1.7: set the version"), then tag.

Server-side context: the "stops out of nowhere" bug was mostly the Fikra API
server being OOM-killed every 10-20 min (fixed 2026-11-23, see fikraapi
commits 75dfd60, 9db607a, a980955, 16962bd). The CLI items below make the
client resilient to the next outage of any kind.

## 0.1.8: stability (the CLI must not "just stop")

1. **Resume a turn after a dropped stream.**
   Today a mid-stream disconnect ends the turn after retries
   (`codex-rs/core/src/responses_retry*`, `codex-api` SSE handling).
   Do: on disconnect after partial output, retry the same turn with backoff
   (1s, 2s, 4s, 8s, cap 30s, up to ~2 min), keeping already-applied tool results.
   Done when: killing the server mid-answer (or `tc qdisc` packet loss) makes the
   CLI print "connection lost, retrying in Ns" and finish the turn when the
   server returns.

2. **Say why it stopped.** Every terminal failure prints one line with the
   cause and next step: network down, server restarting (502/503/504), balance
   used up (402), rate/plan limit (429, with reset time), auth (401/403),
   firewall block (HTML 403, already handled in 0.1.7 api_bridge.rs).
   Done when: each status above, simulated with a mock server, shows its own
   message and no raw HTML/JSON dumps.

3. **`fikra doctor`.** New subcommand in `codex-rs/cli`: checks connectivity to
   https://fikraapi.co.ke/v1/models, key validity (GET /v1/models with auth),
   plan/balance (existing dashboard API), sandbox availability (Linux bwrap /
   Landlock), installed version vs latest GitHub release. Prints pass/fail per line.
   Done when: `fikra doctor` passes on a healthy machine and names the failing
   check with a fix hint on a broken one (no key, offline, old version).

4. **Session history corruption.** Logs show "attempted to call begin_with at
   non-zero transaction depth" (`codex_thread_store::local::live_writer`) and
   "skipping malformed rollout line ... control character"
   (`thread_history_materialization`). Do: serialize rollout writes so no nested
   transaction happens; escape/strip control characters before writing JSONL.
   Done when: a 200-turn scripted session resumes cleanly and the two warnings
   never appear.

5. **Tools offered only when the model supports them.** `view_image` errors
   on text-only models ("not allowed because you do not support image inputs").
   Hide it unless the model metadata says images are supported.

6. **Remove leftover upstream pieces.** `gpt-5.6-luna` / `gpt-5.6-terra`
   defaults (guardian samplers in `ext/guardian-v2`, `ext/guardian-v3`,
   `tui/src/debug_config.rs`), ChatGPT URLs in core/cli/tui/login.
   Done when: `grep -r "gpt-5.6\|chatgpt.com" codex-rs --include=*.rs` returns
   only tests, and a fresh session logs no "Unknown model" warnings.

7. **ZDR switch.** `fikra --zdr` and `zdr = true` in `~/.fikra/config.toml` add
   `X-Fikra-ZDR: true` to every request on the built-in Fikra provider.
   Done when: requests carry the header (check with a local mock) and the
   custom-provider workaround is no longer needed.

8. **Free-tier notice.** First time a session uses
   `deepseek-v4-1-flash-economy-free`, print once: free-model prompts are stored
   and used for training; use a paid model + ZDR for private code.
   (The free models end 2026-11-25 23:59 UTC; keep the notice generic.)

9. **`/usage` in the TUI.** Show plan allowance left (5-hour and weekly windows)
   and balance from the Fikra API; warn at 80%.

## 0.1.9: comfort and reach

10. Model picker with the Fikra catalog (name, context, price, free/included).
11. Switch model mid-session without losing context.
12. `fikra update`: self-update with checksum verification.
13. Homebrew tap (`brew install fikra`) now that macOS builds exist.
14. Low-memory mode for 4 GB machines (cap history kept in memory, UI buffers).
15. Faster cold start: skip plugin/skills/hooks scans that only apply to ChatGPT sign-in.
16. Release signing (minisign or cosign) + SBOM alongside the per-package `.sha256`.

## How to work an item with Fikra Code

```
git checkout -b fix/<item-number>-<short-name> origin/main
fikra "Implement item N from docs/ROADMAP-0.1.8-0.1.9.md. Keep the change minimal, add a test for the 'Done when' condition, and run cargo test for the touched crates."
```
Then push the branch and open a PR; CI decides.
