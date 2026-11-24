# Fikra Code changelog

Downloads and checksums: https://github.com/Fikra-API/fikra-code/releases

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
