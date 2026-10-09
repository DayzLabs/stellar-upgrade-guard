# RPC Architecture

Phase 3 adds deployed-contract comparison without changing the compatibility engine. The RPC layer lives in `crates/guard-rpc` and returns raw deployed WASM bytes. `guard-cli` then compares those bytes against the local candidate WASM with `guard-core::compare_wasm`.

## Flow

1. Parse the `--contract` value as a Stellar contract strkey.
2. Build a `LedgerKey::ContractData` key for the contract instance:
   - `contract`: `SCAddress::Contract`
   - `key`: `SCVal::LedgerKeyContractInstance`
   - `durability`: `Persistent`
3. Call JSON-RPC `getLedgerEntries` with the base64 XDR ledger key.
4. Decode the returned `LedgerEntryData::ContractData`.
5. Read the instance executable:
   - `ContractExecutable::Wasm(hash)` continues to code lookup.
   - `ContractExecutable::StellarAsset` returns a structured unsupported-contract error.
   - `ContractExecutable::ExternalRef` returns a structured unsupported-reference error.
6. Build a `LedgerKey::ContractCode` key from the WASM hash.
7. Call `getLedgerEntries` again and decode `LedgerEntryData::ContractCode`.
8. Return the contract code bytes, hash, and ledger metadata to the CLI.

This mirrors Stellar RPC's documented two-step lookup for contract WASM: first fetch the contract instance, then use its executable WASM hash to fetch the contract code entry.

## Network Resolution

The CLI resolves the RPC endpoint in `compare_mode` before fetching:

```text
stellar-upgrade-guard compare --contract <CONTRACT_ID> --candidate candidate.wasm --network testnet
stellar-upgrade-guard compare --contract <CONTRACT_ID> --candidate candidate.wasm --rpc-url https://my-rpc.example.com
stellar-upgrade-guard compare --contract <CONTRACT_ID> --candidate candidate.wasm --network mainnet --rpc-url https://my-mainnet-rpc.example.com
```

Resolution order:

1. When `--rpc-url` is supplied, it is used verbatim and overrides any network default.
2. Otherwise `--network` is required and its built-in default endpoint is used.
3. `testnet` resolves to `https://soroban-testnet.stellar.org`.
4. `mainnet` has no built-in default, so omitting `--rpc-url` fails with `mainnet deployed compare requires --rpc-url`.

Because `--rpc-url` takes precedence over `--network`, supplying both is allowed; the network value is retained only to label the output.

## Credential Handling

RPC URLs can embed provider API keys or tokens. The RPC layer treats the endpoint as opaque and does not redact credentials, so supply credential-bearing URLs through environment variables or CI secrets rather than committing them, and avoid logging them.

## Testing

Normal tests are offline. `guard-rpc` exposes a small `LedgerEntriesClient` trait so tests can provide mocked `getLedgerEntries` responses. The tests construct real Stellar XDR values and encode them to base64, which keeps RPC parsing covered without depending on public network availability.

## Live Testnet Validation

Phase 3 was validated against Testnet contract `CBK6WZDW7UOOKZQ4RK7X25YCNWWBAGIYAMNNAGK7VBVJQ52PMSAPACMJ`.

The Stellar CLI deployed WASM hash and `guard-rpc` fetched WASM hash both resolved to:

```text
83d568de7ffdf7475bfc321d68ea6404f25e3285ad141ac874533d9b5f88707d
```

Validation cases:

- identical candidate `fixtures/basic-contract/target/wasm32v1-none/release/basic_contract.wasm`: `COMPATIBLE`, exit code `0`;
- breaking candidate `fixtures/phase3/live-breaking-candidate/target/wasm32v1-none/release/live_breaking_candidate.wasm`: `BREAKING`, exit code `1`.

The breaking candidate preserves `hello(name: Symbol)` and changes the return interface from `Symbol` to `String`.

## Boundaries

The RPC layer only retrieves deployed WASM and surfaces structured lookup errors. It does not analyze storage migrations, authorization changes, runtime behavior, or upgrade safety beyond the interface compatibility checks already implemented in `guard-core`.
