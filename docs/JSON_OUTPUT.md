# JSON Output

`stellar-upgrade-guard compare --format json` prints one JSON document to stdout. Errors go to stderr with exit code `2` and nothing is written to stdout.

Local and deployed comparison produce the same document shape. Only the contents of `source` differ.

## Schema Version 1

```json
{
  "schema_version": 1,
  "source": { "kind": "local_files", "old": "old.wasm", "new": "new.wasm" },
  "result": "breaking",
  "summary": { "compatible": 0, "warning": 0, "breaking": 1, "unknown": 0 },
  "findings": [
    {
      "code": "return_type_changed",
      "impact": "breaking",
      "path": "function.value.return",
      "message": "Function `value` return type changed: u32 -> u64",
      "old_value": "u32",
      "new_value": "u64"
    }
  ]
}
```

| Field | Type | Notes |
| --- | --- | --- |
| `schema_version` | integer | `1`. Incremented only for changes that can break existing consumers. |
| `source` | object | Tagged by `kind`; see below. |
| `result` | string | `compatible`, `warning`, `unknown`, or `breaking`: the most severe finding. Independent of `--fail-on`. |
| `summary` | object | Finding counts per impact: `compatible`, `warning`, `breaking`, `unknown`. |
| `findings` | array | Sorted by impact (breaking, unknown, warning, compatible), then `code`, `path`, `message`. |

Each finding has `code`, `impact`, `path`, and `message`. `old_value` and `new_value` are omitted when not applicable (for example `old_value` for an added item).

### `source` for local comparison

| Field | Type |
| --- | --- |
| `kind` | `"local_files"` |
| `old` | string, path as passed to `--old` |
| `new` | string, path as passed to `--new` |

### `source` for deployed comparison

| Field | Type |
| --- | --- |
| `kind` | `"deployed_contract"` |
| `contract_id` | string |
| `network` | `"testnet"`, `"mainnet"`, or `null` when only `--rpc-url` was given |
| `candidate` | string, path as passed to `--candidate` |
| `wasm_hash` | string, hex hash of the deployed contract code |
| `last_modified_ledger` | integer or `null` |

## Compatibility Policy

- Adding fields is not a breaking change. Consumers should ignore unknown fields.
- Removing a field, renaming a field, or changing a field's type or meaning increments `schema_version`.
- Adding a value to `code` or `impact` is not breaking; consumers should handle unrecognized codes.

## Change From v0.1.0

The change is additive. Every field printed by `v0.1.0` is still present with the same meaning:

- local comparison output gains `schema_version` and a `source` object (previously it had neither);
- deployed comparison output gains `schema_version`; its `source` object is unchanged.

Consumers written against `v0.1.0` output continue to work. Consumers that want to branch on mode can now read `source.kind` in both modes.

`inspect --format json` is not part of this schema and is unchanged.
