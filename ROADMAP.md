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

## v0.2 Planned Scope

- Richer human-readable and machine-readable reports.
- RPC provider configuration improvements.
- Better event and union semantics where evidence exists.
- Evaluate SARIF output for code scanning integrations.
- Distribution improvements, such as release artifacts or crates.io publication.
- More cross-platform verification for CLI and action workflows.

## Later Research

- Declared storage schema support.
- Explicit migration manifests.
- Expanded CI integrations.
- Contract upgrade policy files.
- Deeper compatibility rules based on future Stellar/Soroban guidance.

These items require design work and should not be treated as committed release scope.
