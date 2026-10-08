# Stellar Upgrade Guard

Early-stage open-source tooling for inspecting Soroban contract specifications and detecting deterministic interface compatibility changes.

The current Phase 3 capability is intentionally scoped: the CLI can inspect a compiled Soroban WASM file, extract its official `contractspecv0` interface data, compare two local WASM interfaces, compare a deployed contract against a candidate WASM through Stellar RPC, and report structured findings. It does not prove runtime behavior, authorization behavior, storage migration safety, or that an upgrade is safe.

## Build

```text
cargo check --workspace
```

## Inspect A WASM

```text
cargo run -p guard-cli -- inspect path/to/contract.wasm
```

For JSON:

```text
cargo run -p guard-cli -- inspect path/to/contract.wasm --format json
```

## Compare Two WASM Files

```text
cargo run -p guard-cli -- compare --old old.wasm --new new.wasm
```

For JSON:

```text
cargo run -p guard-cli -- compare --old old.wasm --new new.wasm --format json
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
cargo run -p guard-cli -- compare --contract <CONTRACT_ID> --candidate candidate.wasm --network testnet
```

Use `--rpc-url` to supply an explicit RPC endpoint:

```text
cargo run -p guard-cli -- compare --contract <CONTRACT_ID> --candidate candidate.wasm --rpc-url https://example-rpc.invalid
```

`--contract`/`--candidate` mode is mutually exclusive with local `--old`/`--new` mode. The built-in `testnet` network resolves to Stellar's public testnet RPC endpoint. Mainnet comparison requires `--rpc-url` so callers choose their provider explicitly.

## Current Rule Coverage

Implemented rule categories:

- functions: added, removed, argument added/removed/renamed/reordered/type changed, return type changed;
- structs: added, removed, field added/removed/renamed/reordered/type changed;
- enums: added, removed, case added/removed, case value changed;
- error enums: added, removed, case added/removed, error code changed;
- unions: added, removed, case added/removed, case payload changed;
- events: added, removed, parameters changed, data format changed.

Conservative classifications are used where Soroban compatibility semantics need more fixture validation. Unknown findings currently cause the compare command to exit non-zero.

## Exit Codes

- `0`: no breaking or unknown findings;
- `1`: breaking or unknown findings detected;
- `2`: input, parsing, or execution failure.

## Test

```text
cargo test --workspace
```

Full local checks:

```text
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for prerequisites, fixture instructions, and repository structure.
