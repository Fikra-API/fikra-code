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
  <a href="https://fikraapi.co.ke/code/models">Models</a> &middot;
  <a href="https://fikraapi.co.ke/dashboard/code">Dashboard</a>
</p>

<p align="center">
  <img alt="Apache-2.0" src="https://img.shields.io/badge/license-Apache--2.0-blue" />
  <img alt="Linux, macOS, Windows" src="https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey" />
  <img alt="Rust" src="https://img.shields.io/badge/built%20with-Rust-orange" />
</p>

---

Fikra Code is a small command-line coding agent. It reads a project, runs commands, edits files and reports back. It is built to stay usable on a low-end laptop and on a connection that drops, and it is designed around the [Fikra API](https://fikraapi.co.ke) as its default model provider.

## Install

Linux and macOS:

```sh
curl -fsSL https://fikraapi.co.ke/code/install.sh | sh
```

Windows PowerShell:

```powershell
irm https://fikraapi.co.ke/code/install.ps1 | iex
```

## Log in

Create a Fikra Code key at [fikraapi.co.ke/dashboard/code](https://fikraapi.co.ke/dashboard/code), then:

```sh
fikra login
```

Paste the key. `fikra logout` removes it. Fikra Code reads `FIKRA_API_KEY` from the environment when it is set, otherwise it uses the key stored by `fikra login`.

## Use

```sh
fikra "fix the failing tests in this repo"
```

Or pin a model:

```sh
fikra --model deepseek-v4-1-flash-economy-free "refactor this module"
```

## Main models

These are the first models on the [Fikra Code catalog](https://fikraapi.co.ke/code/models). Only models marked "In API" are currently accessible. More models are planned for later integration.

| Model | ID | Input / Output per 1M |
| --- | --- | --- |
| DeepSeek V4.1 Flash (free window) | `deepseek-v4-1-flash-economy-free` | free window |
| Qwen 3.8 Max | `qwen3.8-max` | 6.00 / 12.00 |
| Fikra Code 3B | `fikra-code-35b` | 0.045 / 0.33 |
| Fikra 7B V1 | `fikra-7b-v1` | 0.90 / 2.70 |
| Inkling Small | `inkling-small` | 0.56 / 1.50 |
| Inkling 2 Small | `inkling-2-small` | 0.38 / 1.25 |
| GLM 5.2 | `glm-5.2` | 1.75 / 5.50 |

## Why Fikra Code

- Small footprint and quick start, built for modest machines.
- Resumable installers and streaming responses for slow networks.
- No telemetry by default. Analytics, feedback and OpenTelemetry exporters are off in the default configuration.
- Code keys use the same balance and top-ups as normal Fikra API keys.
- A separate catalog for Fikra Code, with clear labels for what is available now and what is planned.

## Links

- Website: [fikraapi.co.ke](https://fikraapi.co.ke)
- Fikra Code: [fikraapi.co.ke/code](https://fikraapi.co.ke/code)
- Docs: [fikraapi.co.ke/code/docs](https://fikraapi.co.ke/code/docs)
- Models: [fikraapi.co.ke/code/models](https://fikraapi.co.ke/code/models)
- Dashboard: [fikraapi.co.ke/dashboard/code](https://fikraapi.co.ke/dashboard/code)
- Status: [fikraapi.co.ke/status](https://fikraapi.co.ke/status)

## Credits

Fikra Code is based on OpenAI Codex CLI, licensed under the Apache License 2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
