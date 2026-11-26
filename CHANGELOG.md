# Fikra Code changelog

Downloads and checksums: https://github.com/Fikra-API/fikra-code/releases

## 0.3.4

Changed
- The model list is merged with the live catalog at `fikraapi.co.ke/code/models.json`, so names, descriptions, context windows and ordering follow the server instead of only the copy built into the binary. A model the catalog does not list, or marks unavailable, is dropped. Tool and prompt settings always stay in the binary, and if the catalog cannot be fetched the built-in list is used exactly as before.
- Long tasks finish instead of stopping mid-thought. When a model ends a turn on a sentence that only announces the next step, Fikra Code continues the turn. It now takes that decision from the message phase the model reports (commentary vs final answer) and only falls back to reading the wording when the model reports nothing.

Fixed
- `/upgrade` opened the API plans page instead of the Fikra Code plans page, so anyone who hit a limit was sent somewhere that does not sell Fikra Code.
- The README listed `qwen3.8-27b` as the free default. The free default is `qwen3.8-27b-free`; `qwen3.8-27b` is the same model on pay-per-token with no daily cap.

## 0.3.3

Changed
- The free model is now `qwen3.8-27b-free` (200 requests and 2M tokens a day, no card). `qwen3.8-27b` is the same model, paid per token with no daily cap. New sessions start on `qwen3.8-27b-free`; the model list is refreshed from the live catalog.

## 0.3.2

Fixed
- Still stopped after "All 32 pass. Now an end-to-end check through the MCP handler...": a last sentence starting with "Now" or "Next" now continues unless it reports a result ("Now all tests pass" ends normally).
- Long tasks ran out of continues. The limit is now 6 in a row without progress (resets whenever the agent runs a tool), with a ceiling of 40 per turn.

## 0.3.1

Fixed
- Fikra Code still stopped mid-task on narrations the 0.2.1 list did not know ("Rendering both emails...", "Lowering it.", "Probing the real contract.", "Now doing a real synthesis..."). Two general rules replace the word list: any sentence that opens with a verb in -ing (also after now/next/then/also), and, once a turn has run tools, any short final reply that is neither a question nor a reported result. Checked on 83 real turns: all 30 mid-task stops continue, and questions and summaries end normally.

## 0.3.0

New look
- Fikra orange is the accent colour across the app (it was the inherited ChatGPT blue): highlights, selections and prompts all change.
- The session header is a card: the Fikra Code mark and version, the model with a free/paid tag, the folder, and live GitHub, Kaggle and Colab status.
- Every session starts with a quick-actions grid: /pr, /issue, /review, /connect, /kaggle, /colab, /model, /balance.
- The footer shows the model, git branch, open PR number, context left and folder.

Changed
- Model list prices match the live catalogue: DeepSeek V4.1 Flash is $0.075 in / $0.30 out per 1M tokens, and Qwen 3.8 Max is a paid model.

## 0.2.1

Fixed
- Fikra Code stopped mid-task when the model ended a turn on an announcement instead of an action. 0.2.0 caught "Let me...", but not "Fixing my CN row to match.", "Posting the correction.", "Now confirming...", "..., let me check..." or a trailing "...". All of these now continue the turn. Checked against 61 real session endings: the 21 mid-task stops are all caught, and none of the 40 real answers are affected.
- A long run can now recover from up to 6 such stalls per turn (was 2).

## 0.2.0

New
- GitHub, Kaggle and Colab from the composer:
  - `/pr [title]` commits, pushes and opens a pull request with a real description.
  - `/issue <number>` reads a GitHub issue, fixes it on a branch, adds tests and opens a PR that closes it (no number: lists your open issues).
  - `/kaggle <owner/dataset | competition>` downloads the data with the Kaggle CLI and builds a starter notebook with EDA and a baseline.
  - `/colab [notebook]` makes a notebook Colab-ready (install cell, relative paths, badge) and gives you the Open in Colab link.
  - `/connect` shows whether GitHub (gh), Kaggle (API token) and Colab (GitHub remote) are ready, and how to fix what isn't.
- `/balance` shows your Fikra balance, free allowance and the last 7 days of usage without leaving the session.
- `/upgrade` opens the Fikra plans (KES 200 a week with M-Pesa, or from $10 a month); `/docs` opens the docs.
- New tips for all of the above.

Changed
- Default model is `qwen3.8-27b`, free on the Fikra API (200 requests and 2M tokens a day, no card). Other models are marked Paid in the picker.

Fixed
- A long run could end on a reply that only announced the next step ("Let me run the whole suite...") with no tool call, so you got no summary. Fikra Code now continues the turn (at most twice) and asks for a final summary.
- `fikra update` keeps only the newly installed version and removes older versions and downloads.

## 0.1.8

New
- `fikra usage [--days N]`: your balance, account tier and a per-day table of requests, tokens and cost, straight from the terminal.
- Zero data retention switch: `export FIKRA_ZDR=true` and every request is sent with `X-Fikra-ZDR` (no caching or storage on Fikra's side). No custom provider config needed.
- The model picker shows each model's context window and price per 1M tokens.

Changed
- Default model is now `deepseek-v4.1-flash`. The free DeepSeek offer ends on 25 November 2026 (23:59 EAT); paying users who still ask for the free model id are served by `deepseek-v4.1-flash` automatically.
- Fikra Code rides out short outages: up to 12 stream and 10 request retries with waits capped at 15 s (about 100 s in total), instead of giving up after about 6 s.

Fixed
- `fikra doctor` and the update notice checked OpenAI's Codex releases and reported "0.172.0 available". They now check Fikra Code releases.
- Text-only models are no longer offered the image viewer tool, which could only fail on them.
- Background helpers (guardian and housekeeping) asked for `gpt-5.6-luna`, which the Fikra API does not serve; they now use `fikra-auto`.

## 0.1.7

- Requests to any provider on fikraapi.co.ke are compressed, so custom providers (for example a ZDR provider) are not blocked by the edge firewall.
- A firewall block shows a short message instead of an HTML page.
- No ChatGPT plugin sync on startup when signed in with a Fikra API key.
- First release with macOS (Apple Silicon and Intel) and Linux ARM builds.

## Upstream (OpenAI Codex CLI)

Fikra Code is a fork of the OpenAI Codex CLI (Apache 2.0). Upstream history: https://github.com/openai/codex/releases

## Upstream Codex notes (not part of the Fikra Code release line)

Not shipped as Fikra Code changes; kept for attribution of the inherited base.

- Fix missing command lifecycle events when unified exec cannot create a process, preserving approval rejection and sandbox retry behavior. Thanks @Marvinthebored ([#44557](https://github.com/openai/codex/issues/44557)).
