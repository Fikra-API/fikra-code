# Fikra Code changelog

Downloads and checksums: https://github.com/Fikra-API/fikra-code/releases

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

## Unreleased

- Fix missing command lifecycle events when unified exec cannot create a process, preserving approval rejection and sandbox retry behavior. Thanks @Marvinthebored ([#44557](https://github.com/openai/codex/issues/44557)).
