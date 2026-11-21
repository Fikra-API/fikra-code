<p align="center">
  <img src=".github/assets/fikra-logo.png" alt="Fikra Code" width="220" />
</p>

<h1 align="center">Fikra Code</h1>

<p align="center">
  <strong>A lightweight coding agent that runs well on modest laptops and slow connections.</strong><br>
  Powered by the <a href="https://fikraapi.co.ke">Fikra API</a>.
</p>

<p align="center">
  <a href="https://fikraapi.co.ke/code">Website</a> &middot;
  <a href="https://fikraapi.co.ke/code/docs">Docs</a> &middot;
  <a href="https://fikraapi.co.ke/code/pricing">Pricing</a> &middot;
  <a href="https://fikraapi.co.ke/code/models">Models</a> &middot;
  <a href="https://fikraapi.co.ke/code/changelog">Changelog</a>
</p>

<p align="center">
  <img alt="Apache-2.0" src="https://img.shields.io/badge/license-Apache--2.0-blue" />
  <img alt="Linux, Windows" src="https://img.shields.io/badge/platform-Linux%20%7C%20Windows-lightgrey" />
  <img alt="Rust" src="https://img.shields.io/badge/built%20with-Rust-orange" />
</p>

---

Fikra Code is a small command-line coding agent. It reads a project, runs commands, edits files and reports back. It is built to stay usable on a low-end laptop and on a connection that drops, and it is designed around the [Fikra API](https://fikraapi.co.ke) as its default model provider.

## Install

Linux:

```sh
curl -fsSL https://fikraapi.co.ke/code/install.sh | sh
```

Windows PowerShell:

```powershell
irm https://fikraapi.co.ke/code/install.ps1 | iex
```

## Sign in

```sh
fikra
```

Your browser opens [fikraapi.co.ke/code/plans](https://fikraapi.co.ke/code/plans). Pick a plan or continue free, check that the code on the page matches your terminal, and Fikra Code signs in on its own. Paying creates your account from the checkout email.

- Over SSH or on a server: `fikra login --device-auth` prints the link instead of opening a browser.
- In CI and scripts: set `FIKRA_API_KEY`, or run `printenv FIKRA_API_KEY | fikra login --with-api-key`.
- `fikra login status` shows the sign-in, `fikra logout` removes it.

## Plans

| Plan | Price | Allowance |
| --- | --- | --- |
| Free | $0 | DeepSeek V4.1 Flash: 150 requests every 5 hours, 500 a week |
| Lite | $10 / month | $0.50 every 5 hours, $3 a week, every model |
| Pro | $25 / month | $1.50 every 5 hours, $8 a week, every model |
| Max | $60 / month | $5 every 5 hours, $18 a week, every model |
| Team | $100 / seat / month | Max allowances per seat, one bill |
| Kenya Weekly | KES 200 / week | M-Pesa or card, every model |

Details and limits per model: [fikraapi.co.ke/code/pricing](https://fikraapi.co.ke/code/pricing).

## Use

```sh
fikra "fix the failing tests in this repo"
```

Pick a model with `-m` or `/model` inside a session:

```sh
fikra -m fikra-auto "refactor this module"
```

Update with `fikra update`.

## Models

Every paid plan includes every model in the catalog. Prices are USD per 1M input / output tokens for pay-as-you-go, checked 2026-11-21. Full list: [fikraapi.co.ke/code/models](https://fikraapi.co.ke/code/models).

| Model | ID | Input / Output |
| --- | --- | --- |
| DeepSeek V4.1 Flash (free, default) | `deepseek-v4-1-flash-economy-free` | free |
| Fikra Auto | `fikra-auto` | 0.045 / 0.35 |
| Fikra Code 35B | `fikra-code-35b` | 0.045 / 0.33 |
| DeepSeek V4.1 Flash | `deepseek-v4.1-flash` | 0.1875 / 0.75 |
| Qwen 4 Max | `qwen4-max` | 1.875 / 5.625 |
| Gemini 4 Argon | `gemini-4-argon` | 2.50 / 12.00 |
| Kimi K3 | `kimi-k3` | 2.4375 / 12.1875 |
| GLM 5.3 | `glm-5.3` | 1.225 / 3.85 |

## Why Fikra Code

- Small footprint and quick start, built for modest machines.
- Resumable installers and streaming responses for slow networks.
- No telemetry by default. Analytics, feedback and OpenTelemetry exporters are off in the default configuration.
- One plan for every model, from $10 a month, or KES 200 a week with M-Pesa.
- Uses the same account and balance as the Fikra API.

## Links

- Website: [fikraapi.co.ke](https://fikraapi.co.ke)
- Fikra Code: [fikraapi.co.ke/code](https://fikraapi.co.ke/code)
- Docs: [fikraapi.co.ke/code/docs](https://fikraapi.co.ke/code/docs)
- Pricing: [fikraapi.co.ke/code/pricing](https://fikraapi.co.ke/code/pricing)
- Models: [fikraapi.co.ke/code/models](https://fikraapi.co.ke/code/models)
- Changelog: [fikraapi.co.ke/code/changelog](https://fikraapi.co.ke/code/changelog)
- Dashboard: [fikraapi.co.ke/dashboard/code](https://fikraapi.co.ke/dashboard/code)
- Status: [fikraapi.co.ke/status](https://fikraapi.co.ke/status)

## Credits

Fikra Code is based on OpenAI Codex CLI, licensed under the Apache License 2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
