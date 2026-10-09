# Stellar Upgrade Guard v0.1.0 Release Notes

Release date: 2026-10-09

`v0.1.0` is the first GitHub source release of Stellar Upgrade Guard. Crates.io publication and binary release artifacts are deferred.

## What Is Included

- Soroban `contractspecv0` parsing from compiled WASM files.
- Deterministic normalization of contract interface metadata.
- Compatibility checks for functions, structs, enums, error enums, unions, and events.
- Local WASM comparison with text and JSON output.
- Deployed contract comparison through Stellar RPC.
- A composite GitHub Action for local and deployed comparison modes.
- Offline Phase 2 fixture coverage and Phase 3 live Testnet validation.

## Installation

From a source checkout:

```text
cargo install --path crates/guard-cli --locked --bin stellar-upgrade-guard
```

From the release tag, after `v0.1.0` is created:

```text
cargo install --git https://github.com/DayzLabs/stellar-upgrade-guard --tag v0.1.0 --locked --bin stellar-upgrade-guard
```

## GitHub Action

After the `v0.1.0` tag is created:

```yaml
- uses: DayzLabs/stellar-upgrade-guard/action@v0.1.0
  with:
    old-wasm: ./artifacts/old.wasm
    new-wasm: ./artifacts/new.wasm
```

Before the tag exists, use a reviewed commit SHA.

## Compatibility Scope

The guard detects deterministic interface changes exposed through Soroban contract metadata. Unknown findings are conservative and cause `compare` to exit non-zero.

Exit codes:

- `0`: no breaking or unknown findings.
- `1`: breaking or unknown findings detected.
- `2`: input, parsing, RPC, or tool failure.

## Requirements

- Rust 1.91.0 or newer.
- Stellar CLI v25.2.0 or newer only when rebuilding fixture WASM with `stellar contract build`.

## Known Limitations

This release does not prove storage migration safety, authorization behavior, runtime behavior, deployment configuration safety, initialization-state safety, or full protocol upgrade safety.

## Security

GitHub private vulnerability reporting is enabled for the repository. Do not report real vulnerabilities in public issues; follow `SECURITY.md`.
