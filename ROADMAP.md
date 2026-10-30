# Roadmap

This roadmap describes intent, not a promise. Scope may change as Soroban semantics, user needs, and maintenance capacity become clearer.

## v0.1 Committed Scope

- Parse Soroban `contractspecv0` metadata from real WASM files.
- Normalize contract interfaces deterministically.
- Compare local old/new WASM files.
- Compare deployed Testnet contracts against candidate WASM through Stellar RPC.
- Provide a conservative compatibility engine with deterministic findings.
- Provide a reusable GitHub Action.
- Maintain repository CI and action-test workflows.
- Document limitations, contribution process, security reporting, and release readiness.

## Post-v0.1 work already on main

These changes are merged to `main` and listed under Unreleased in [CHANGELOG.md](CHANGELOG.md). No release containing them has been made.

- Configurable CI failure threshold: `compare --fail-on {breaking|unknown|warning}`, also available as the Action input `fail-on`. The default keeps the `v0.1.0` behavior.
- Bounded, configurable RPC request timeout (`--rpc-timeout`, default 30 seconds). Timeouts and connection failures are reported as distinct errors, and RPC URLs are no longer included in error messages.
- One `compare --format json` structure for local and deployed comparison, with `schema_version`. See [docs/JSON_OUTPUT.md](docs/JSON_OUTPUT.md).
- `contractenvmetav0` interface-version inspection and comparison. Differences are reported conservatively as `unknown`.
- CI verification on Ubuntu, Windows, and macOS: workspace check, tests, and a CLI run at the declared minimum Rust version, with formatting and clippy on Linux.

## v0.2 Planned Scope

- Richer human-readable reports, such as Markdown output and old/new values in text output, and a documented reference for JSON finding codes.
- RPC resilience beyond the request timeout, such as evaluating retry behavior for transient provider failures.
- Better event and union semantics where evidence exists.
- Evaluate SARIF output for code scanning integrations.
- Distribution improvements, such as release artifacts or crates.io publication.
- Cross-platform verification of the GitHub Action beyond its current Ubuntu-only test coverage.

## Later Research

- Declared storage schema support.
- Explicit migration manifests.
- Expanded CI integrations.
- Contract upgrade policy files.
- Deeper compatibility rules based on future Stellar/Soroban guidance.

These items require design work and should not be treated as committed release scope.
