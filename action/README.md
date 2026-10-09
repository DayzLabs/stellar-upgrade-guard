# Stellar Upgrade Guard Action

Composite GitHub Action for comparing Soroban contract interfaces with `stellar-upgrade-guard`.

The action builds the CLI from this repository, runs `stellar-upgrade-guard compare --format json`, exposes stable summary outputs, and preserves the CLI exit semantics.

## Inputs

Local WASM mode:

- `old-wasm`: previous compiled Soroban WASM.
- `new-wasm`: candidate compiled Soroban WASM.

Deployed contract mode:

- `contract-id`: deployed Stellar contract id.
- `candidate-wasm`: candidate compiled Soroban WASM.
- `network`: Stellar network, such as `testnet`.
- `rpc-url`: optional explicit Stellar RPC URL.

Local WASM inputs are mutually exclusive with deployed contract inputs. Deployed mode requires `contract-id` and `candidate-wasm`, plus either `network` or `rpc-url`.

## Examples

Use the release tag after `v0.1.0` is created. Before the tag exists, use a reviewed commit SHA.

```yaml
- uses: DayzLabs/stellar-upgrade-guard/action@v0.1.0
  with:
    old-wasm: ./artifacts/old.wasm
    new-wasm: ./artifacts/new.wasm
```

```yaml
- uses: DayzLabs/stellar-upgrade-guard/action@v0.1.0
  with:
    contract-id: CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
    candidate-wasm: ./target/wasm32v1-none/release/contract.wasm
    network: testnet
```

For mainnet or private RPC providers, pass `rpc-url`. Do not include credentials in logs, and prefer GitHub secrets for sensitive endpoint values.

## Outputs

- `result`: compatibility result from the CLI JSON report.
- `breaking-count`: number of breaking findings.
- `unknown-count`: number of unknown findings.
- `warning-count`: number of warning findings.

## Failure Behavior

The action preserves CLI exit semantics:

- `0`: no breaking or unknown findings;
- `1`: breaking or unknown findings detected;
- `2`: input, parsing, RPC, or tool failure.

GitHub Actions marks the step failed for exit codes `1` and `2`. Breaking findings are not downgraded to warnings.

## Step Summary

When `$GITHUB_STEP_SUMMARY` is available, the action writes a compact summary with result and finding counts.

## Limitations

This action compares contract interface metadata. It does not prove storage migration safety, runtime behavior, authorization behavior, deployment safety, or upgrade safety.

Normal repository CI should avoid live Testnet dependencies. Deployed mode can depend on RPC provider availability, so use it deliberately in consumer workflows.

## Security Notes

The action does not print RPC authorization headers, API keys, Stellar private keys, or environment secrets. If stderr contains URLs with query strings or embedded credentials, the action redacts those values before printing.
