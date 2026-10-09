# Stellar Upgrade Guard

Stellar Upgrade Guard is early-stage open-source tooling for inspecting Soroban contract interfaces and detecting deterministic interface compatibility changes before a contract upgrade reaches users.

It can compare:

- two local compiled Soroban WASM files; or
- a deployed Stellar contract's current WASM against a local candidate WASM through Stellar RPC.

It does not prove storage migration safety, authorization behavior, runtime behavior, deployment safety, or complete upgrade safety.

## Current Status

Current release: `v0.1.0`. Current functionality is useful for CI guardrails, but compatibility semantics are intentionally conservative and pre-1.0 APIs may change.

## Why This Exists

Soroban contract upgrades can accidentally remove functions, change argument shapes, change return types, or alter user-defined types. Stellar Upgrade Guard provides a deterministic interface check that can fail CI before those changes are merged.

## Installation

Requires Rust 1.91.0 or newer.

From a source checkout:

```text
cargo install --path crates/guard-cli --locked --bin stellar-upgrade-guard
```

From the release tag, after `v0.1.0` is created:

```text
cargo install --git https://github.com/DayzLabs/stellar-upgrade-guard --tag v0.1.0 --locked --bin stellar-upgrade-guard
```

Before the release tag exists, install from a local source checkout or a reviewed commit SHA. The project is not yet published to crates.io and does not yet publish binary release artifacts.

## Inspect A WASM

```text
stellar-upgrade-guard inspect path/to/contract.wasm
```

For JSON:

```text
stellar-upgrade-guard inspect path/to/contract.wasm --format json
```

## Compare Two Local WASM Files

```text
stellar-upgrade-guard compare --old old.wasm --new new.wasm
```

For JSON:

```text
stellar-upgrade-guard compare --old old.wasm --new new.wasm --format json
```

Example text output:

```text
Stellar Upgrade Guard

Comparing:
old.wasm
-> new.wasm

Breaking changes:
- Function `value` return type changed: u32 -> u64

Unknown changes:
- none

Warnings:
- none

Compatible changes:
- none

Result: BREAKING
```

## Compare A Deployed Contract

```text
stellar-upgrade-guard compare --contract <CONTRACT_ID> --candidate candidate.wasm --network testnet
```

Use `--rpc-url` to supply an explicit RPC endpoint:

```text
stellar-upgrade-guard compare --contract <CONTRACT_ID> --candidate candidate.wasm --rpc-url https://example-rpc.invalid
```

`--contract`/`--candidate` mode is mutually exclusive with local `--old`/`--new` mode. The built-in `testnet` network resolves to Stellar's public testnet RPC endpoint. Mainnet comparison requires `--rpc-url` so callers choose their provider explicitly.

## GitHub Action

This repository provides a composite GitHub Action at `action/` for external Soroban projects. After the `v0.1.0` tag is created, reference:

```yaml
- uses: DayzLabs/stellar-upgrade-guard/action@v0.1.0
  with:
    old-wasm: ./artifacts/old.wasm
    new-wasm: ./artifacts/new.wasm
```

Deployed contract comparison:

```yaml
- uses: DayzLabs/stellar-upgrade-guard/action@v0.1.0
  with:
    contract-id: CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
    candidate-wasm: ./target/wasm32v1-none/release/contract.wasm
    network: testnet
```

Before the release tag exists, reference a reviewed commit SHA instead.

The action preserves CLI exit behavior and fails workflows on exit `1` or `2`. See [action/README.md](action/README.md) for inputs, outputs, and security notes.

## Exit Codes

- `0`: no breaking or unknown findings;
- `1`: breaking or unknown findings detected;
- `2`: input, parsing, RPC, or tool failure.

## Current Rule Coverage

Implemented rule categories:

- functions: added, removed, argument added/removed/renamed/reordered/type changed, return type changed;
- structs: added, removed, field added/removed/renamed/reordered/type changed;
- enums: added, removed, case added/removed, case value changed;
- error enums: added, removed, case added/removed, error code changed;
- unions: added, removed, case added/removed, case payload changed;
- events: added, removed, parameters changed, data format changed.

Conservative classifications are used where Soroban compatibility semantics need more evidence. Unknown findings currently cause `compare` to exit non-zero.

## Limitations

Stellar Upgrade Guard only compares interface metadata extracted from Soroban WASM or fetched through Stellar RPC. It does not analyze:

- storage migrations;
- authorization behavior;
- runtime behavior;
- deployment configuration;
- contract initialization state;
- whether an upgrade is safe for a particular protocol.

## Development

Run the local quality gate:

```text
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for prerequisites, fixture instructions, action development, and repository structure.

## Contributing

Contributions are welcome, especially fixtures, parser fixes, reporting improvements, docs, and evidence-backed compatibility rules. Start with [CONTRIBUTING.md](CONTRIBUTING.md). Larger semantic or architecture changes should begin with an issue or design discussion.

## Security

Do not open public issues for real vulnerabilities. See [SECURITY.md](SECURITY.md) for private reporting guidance and secret-handling expectations.

## Roadmap And Release Readiness

- [ROADMAP.md](ROADMAP.md)
- [CHANGELOG.md](CHANGELOG.md)
- [docs/V0_1_RELEASE_READINESS.md](docs/V0_1_RELEASE_READINESS.md)
- [docs/RELEASE_CHECKLIST.md](docs/RELEASE_CHECKLIST.md)

## License

Apache-2.0. See [LICENSE](LICENSE).
