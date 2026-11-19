<p align="center">
  <img src=".github/assets/fikra-logo.png" alt="Fikra Code" width="240" />
</p>

# Fikra Code

Fikra Code is a lightweight coding agent built to run well on modest laptops and slow connections. It is powered by the Fikra API and uses Fikra as its default model provider.

## Install

Linux and macOS:

```sh
curl -fsSL https://fikraapi.co.ke/code/install.sh | sh
```

Windows:

```powershell
irm https://fikraapi.co.ke/code/install.ps1 | iex
```

## Log in

Get an API key from https://fikraapi.co.ke/dashboard, then run:

```sh
fikra login
```

Paste the key when prompted. `fikra logout` removes it.

## Use

```sh
fikra "fix the failing tests in this repo"
```

Fikra Code reads `FIKRA_API_KEY` from the environment when it is set, otherwise it uses the key stored by `fikra login`.

## Credits

Fikra Code is based on OpenAI Codex CLI, licensed under the Apache License 2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
