# Fikra Code 0.3.7 plan

Written 2026-11-27. One item per branch/PR; CI
(`.github/workflows/fikra-ci.yml`) must pass before merge. Release = bump the
workspace version in `codex-rs/Cargo.toml` plus the workspace entries in
`codex-rs/Cargo.lock`, then tag `v0.3.7` and let
`.github/workflows/fikra-release.yml` build.

Ground rules for whoever picks this up (agent or human):

- CI status is read once with `gh run list`. Do not poll GitHub in a loop,
  and do not re-run a workflow that already reported `completed`. If a run is
  `queued` or `in_progress`, say so once and move to other work.
- Keep each change minimal and covered by a test that fails before the fix.

## Theme

0.3.6 fixed defaults and rebranding. 0.3.7 is about **money you can see** and
**work you can undo**: a user should know what a turn costs before it runs,
never be silently stuck on a paid model, and be able to take back a bad edit.

## Fixes (do these first)

1. **Resume restores the thread's approval mode.** _Done: PR #10._
   `threads.approval_mode` and `threads.sandbox_policy` are written by
   `codex-rs/thread-store/src/thread_metadata_sync.rs:299`, but the resume path
   (`codex-rs/core/src/session/mod.rs:1866`, `InitialHistory::Resumed`) only
   re-persists the current settings and never applies the persisted ones. A
   reopened thread therefore falls back to the config default
   (`on-request`), so the first "continue" prompts even though the user granted
   full access before closing.
   Done when: a thread saved with `never` + `danger-full-access` resumes with
   those settings and runs an escalated command with no prompt; a test
   (`resume_restores_persisted_approval_mode`) asserts it, and a thread saved
   with `on-request` still resumes as `on-request`.

2. **Gate tools on catalog capabilities.** _Done: PR #11._
   `models.json` carries `capabilities` (for example `["tools","vision"]`), but
   the client ignores it, so `view_image` is offered on text-only models and
   errors ("not allowed because you do not support image inputs").
   Do: parse `capabilities` in `codex-rs/models-manager`, expose them on
   `ModelInfo`, and hide `view_image` unless `vision` is present.
   Done when: a text-only model lists no `view_image` tool and a vision model
   does; unit test over two catalog entries.

3. **Release signing and SBOM.** _Done: PR #12._
   `.github/workflows/fikra-release.yml` publishes `.sha256` only.
   Do: sign every asset (minisign or cosign), publish `*.sig`, and emit a
   CycloneDX SBOM per release.
   Done when: a release publishes a detached signature that verifies against
   the published key, and `sbom.cdx.json` is attached.

## Features

4. **`/budget`: a spend cap the user sets.** _Done: PR #14._
   Nothing today (`max_cost`, `budget_usd`, `cost_limit` have no hits). Add
   `budget.session_usd` and `budget.daily_usd` to config, track spend from the
   existing `thread_cost_micros` plumbing
   (`codex-rs/tui/src/status/thread_usage.rs:36`), warn at 80%, and at 100%
   either stop the turn with a clear message or downgrade to the configured
   `budget.fallback_model` (default `qwen3.8-27b-free`).
   Done when: with `session_usd = 0.01`, a session prints the 80% warning, then
   stops (or downgrades) before exceeding the cap; a test drives the accounting
   without a network call.

5. **Per-session cost for everyone.**
   The statusline cost item is gated to `PlanType::Ent26`
   (`codex-rs/tui/src/status/thread_usage.rs:36`). Compute cost for ordinary
   Fikra accounts from per-model catalogue prices and show `EstimatedThreadCost`
   by default; keep the old behaviour when a price is unknown.
   Done when: a `qwen3.8-max` session shows a non-zero USD cost in the
   statusline and `/status`, and an unpriced model shows nothing rather than a
   wrong number.

6. **`/undo` for file edits.** _Done: PR #13._
   No workspace rewind exists (`Undo` only appears in composer text history).
   Snapshot the touched files before each `apply_patch`, keep the last N
   snapshots per thread, and let `/undo` revert the most recent edit.
   Done when: editing a file then `/undo` restores its exact previous bytes, and
   `/undo` twice reverts two edits; test uses a temp dir, no network.

7. **Model fallback on 429/402.**
   `codex-rs/codex-api/src/api_bridge.rs` already parses the rate-limit type,
   but a 429 ends the turn. On a rate or quota limit, retry the same turn once
   on the next catalogue model, and say which model it moved to.
   Done when: a mock server returning 429 for the first model makes the CLI
   finish the turn on the fallback and print the switch; a 401 still fails
   without falling back.

8. **KES-native money display.**
   `/balance` prints USD only (`codex-rs/login/src/fikra_usage.rs:63`) and the
   plans are priced in KES. Add a KES line using the account's currency from
   the balance payload, and a `currency = "kes" | "usd"` config that reorders
   which one leads.
   Done when: with a KES account, `/balance` shows the KES balance first and the
   USD equivalent second; with `currency = "usd"` the order flips.

9. **Free-tier training notice.**
   Roadmap item 8 from 0.1.8 was never built: nothing tells a user that prompts
   on a free model are stored and used for training. Print a one-time notice on
   the first turn of a session that uses `qwen3.8-27b-free`, with a pointer to a
   paid model plus ZDR.
   Done when: the first free-model turn prints it once, a second turn does not,
   and a paid-model session never prints it.

10. **`--zdr` flag and `zdr = true`.**
    Only the `FIKRA_ZDR` environment variable works today
    (`codex-rs/model-provider-info/src/lib.rs:783`). Add `fikra --zdr` and
    `zdr = true` in `~/.fikra/config.toml`, both of which send
    `X-Fikra-ZDR: true` on the built-in Fikra provider.
    Done when: a local mock sees the header from both the flag and the config
    key, and neither affects a custom provider.

11. **Low-memory mode.**
    No `low_memory` setting exists despite the 4 GB minimum in
    `docs/install.md`. Add `low_memory = true` which caps retained history in
    memory, trims UI scrollback, and reduces diff/transcript buffers.
    Done when: a scripted 200-turn session with `low_memory = true` keeps peak
    RSS below the same run without it, and the transcript still resumes.

## How to work an item

```
git checkout -b feat/<item-number>-<short-name> origin/main
fikra "Implement item N from docs/ROADMAP-0.3.7.md. Keep the change minimal, add a test for the 'Done when' condition, and run cargo test for the touched crates."
```

Push the branch, open a PR, then read CI once with `gh run list --limit 5`.
