# Development

## Prerequisites

- Rust 1.91.0 or newer with Cargo.
- For fixture WASM builds, Rust 1.91.0 or newer with the `wasm32v1-none` target.
- Stellar CLI v25.2.0 or newer for Soroban SDK v28 contract builds.

Install the contract build target:

```text
rustup target add wasm32v1-none
```

Stellar CLI is not required to build or test the parser crates. It is required to rebuild the real Soroban fixture WASM.

## Dependency Versions

The Phase 1 workspace uses aligned Stellar v28 crates:

- `stellar-xdr = 28.0.0`
- `soroban-spec = 28.0.0`
- `stellar-strkey = 0.0.18`
- fixture-only `soroban-sdk = 28.0.0`

The v28 line is selected because the current Soroban SDK v28 release uses `stellar-xdr` v28 and requires Stellar CLI v25.2.0 or newer for WASM contract builds.

## Repository Structure

```text
crates/
  guard-core/    parser, errors, normalization, public library API
  guard-cli/     inspect and compare command-line interface
  guard-rpc/     Stellar RPC ledger-entry client and deployed WASM fetcher
fixtures/
  basic-contract/  tiny Soroban contract source for rebuilding a real fixture
  phase2/          old/new Soroban fixture pairs plus committed small WASM files
  phase3/          live-validation fixture source
docs/
```

## Repository CI

The repository CI workflow runs the local quality gate on pull requests and pushes to `main`:

```text
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

CI uses stable Rust, Cargo caching, and committed offline fixtures. It does not require live Stellar RPC or Testnet access.

## Build

```text
cargo check --workspace
```

## Inspect A Contract

```text
cargo run -p guard-cli -- inspect path/to/contract.wasm
```

JSON output is also available:

```text
cargo run -p guard-cli -- inspect path/to/contract.wasm --format json
```

## Compare A Deployed Contract

Deployed comparison fetches the contract instance and contract code through Stellar RPC, then passes the deployed WASM and local candidate WASM to the same compatibility engine used by local comparison.

```text
cargo run -p guard-cli -- compare --contract <CONTRACT_ID> --candidate candidate.wasm --network testnet
```

For any endpoint not covered by a built-in network default, provide `--rpc-url`:

```text
cargo run -p guard-cli -- compare --contract <CONTRACT_ID> --candidate candidate.wasm --rpc-url https://example-rpc.invalid
```

Normal workspace tests are offline. `guard-rpc` unit tests use mocked ledger-entry responses and generated XDR, so they do not require network access or account keys. Manual live RPC validation should use an externally supplied contract id and RPC URL when available.

## Tests

```text
cargo test --workspace
```

Full local quality gate:

```text
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## GitHub Action Development

The reusable action lives in `action/` and is implemented as a composite action. It builds `stellar-upgrade-guard` from the action repository, runs the CLI in JSON mode, exposes stable count outputs, and returns the CLI exit code.

For local command simulation, run the same fixture comparisons the action-test workflow uses:

```text
cargo run -p guard-cli -- compare --old fixtures/phase2/compatible-function-added/old.wasm --new fixtures/phase2/compatible-function-added/new.wasm
cargo run -p guard-cli -- compare --old fixtures/phase2/breaking-return-type-changed/old.wasm --new fixtures/phase2/breaking-return-type-changed/new.wasm
```

Expected results:

- compatible fixture: exit `0`;
- breaking fixture: exit `1`.

`.github/workflows/action-test.yml` validates the composite action against those same committed Phase 2 fixtures. The intentionally breaking action step uses `continue-on-error` and then verifies that the step outcome is `failure`.

The action is targeted at GitHub-hosted Ubuntu runners first. It relies on Rust and Python being available on the runner; the action installs stable Rust before building the CLI.

For release examples, reference the action by the release tag after `v0.1.0` is created:

```yaml
uses: DayzLabs/stellar-upgrade-guard/action@v0.1.0
```

Before the release tag exists, use a reviewed commit SHA.

## Fixture Build

The source fixture lives at `fixtures/basic-contract`.

To rebuild it as real Soroban WASM:

```text
cd fixtures/basic-contract
stellar contract build
```

Expected output:

```text
target/wasm32v1-none/release/basic_contract.wasm
```

Build artifacts are intentionally not committed in Phase 1. Parser tests construct minimal WASM modules containing official `SCSpecEntry` XDR streams so they can run without requiring Stellar CLI on every developer machine.

Phase 2 includes small committed WASM files under `fixtures/phase2` for real-WASM compatibility tests. To rebuild a pair, run Stellar CLI against the pair's manifest and copy the optimized output to `old.wasm` or `new.wasm`.

Example:

```text
stellar contract build --manifest-path fixtures/phase2/breaking-return-type-changed/old/Cargo.toml
```

Phase 3 includes `fixtures/phase3/live-breaking-candidate`, a source-only fixture used for live Testnet validation. It intentionally preserves `hello(name: Symbol)` from `fixtures/basic-contract` while changing the return type to `String`.

To rebuild the candidate:

```text
cd fixtures/phase3/live-breaking-candidate
stellar contract build
```

Expected output:

```text
target/wasm32v1-none/release/live_breaking_candidate.wasm
```

The generated `target/` directory is build output and must not be committed.
