## Contributing

We welcome contributions through the [Fikra-API/fikra-code issue tracker](https://github.com/Fikra-API/fikra-code/issues). Bug reports, root-cause analyses and feature requests help us understand what matters most.

### Reporting bugs

Before opening a new issue, search the issue tracker to see whether the problem has already been reported. If it has, add any new information to the existing issue.

When reporting a bug, include as much relevant detail as possible:

- Clear, detailed steps to reproduce the problem.
- Expected and actual behavior.
- Your Fikra Code version, operating system and other relevant environment details.
- Logs, error messages or other diagnostic information, with sensitive information removed.
- Root-cause analysis, technical observations or potential approaches to a fix, if available.

### Requesting features

Open a feature request in the issue tracker, or upvote an existing request that describes the same need. Explain your use case, the behavior you would like, and why it would improve your workflow.

### Code contributions

Fikra Code is open source under the Apache License 2.0. Pull requests are welcome against the `main` branch. Keep the change focused, include tests where practical, and update the docs when behavior changes.

Build fully in GitHub Actions. A push to `main` or a pull request that touches `codex-rs/**` or
`scripts/fikra/**` runs `.github/workflows/fikra-ci.yml`, and tagged releases run
`.github/workflows/fikra-release.yml`. Do not run full `cargo build`/`cargo check` on the shared
development machine: a single build of this workspace can consume 17 GB and fill the disk.
Local `cargo` use should stay limited to cheap, single-crate checks, and created `target/`
directories should be removed afterwards.

### Community values

- **Be kind and inclusive.** Treat others with respect; we follow the [Contributor Covenant](https://www.contributor-covenant.org/).
- **Assume good intent.** Written communication is hard, so err on the side of generosity.
- **Share what you learn.** Reproduction details, logs and analysis help the entire community.

### Security

If you discover a security vulnerability, follow the [security policy](../SECURITY.md) instead of reporting it in a public issue.
