# Compatibility Rules

Phase 2 status: implemented local compatibility rules. These rules compare normalized Soroban contract interfaces extracted from official `contractspecv0` WASM data. They do not analyze runtime behavior or storage migration safety.

## Impact Levels

- `Compatible`: the change should not break existing callers that use the old interface.
- `Warning`: the change may affect generated clients, documentation, indexing, or operational expectations but is not clearly call-breaking.
- `Breaking`: the change changes or removes a callable shape or serialized type shape that existing callers may rely on.
- `Unknown`: the official spec exposes a change, but the tool cannot confidently classify runtime/client compatibility yet.

## Compatibility Matrix

| Change | Proposed impact | Confidence | Reasoning |
| --- | --- | --- | --- |
| Function added | Compatible | High | Existing callers can keep invoking old functions. New exported function is additive. |
| Function removed | Breaking | High | Old callers cannot invoke the removed `SCSymbol` function name. |
| Function renamed | Breaking | High | Equivalent to old function removed plus new function added. |
| Argument added | Breaking | High | `SCSpecFunctionV0.inputs` defines expected call arguments. Old callers will not provide the new argument. |
| Argument removed | Breaking | High | Old callers will provide an argument no longer in the function signature. |
| Argument renamed | Warning by default | Medium | Soroban invocation is positional at the low level, but generated CLIs/clients and human workflows use parameter names. |
| Argument reordered | Breaking | High | Positional argument order maps values to different parameters/types. |
| Argument type changed | Breaking | High | Callers encode values according to `ScSpecTypeDef`; a different type changes expected `SCVal` shape. |
| Return type changed | Breaking | High | Clients decode return values from the expected output type. |
| Return removed or added | Breaking | High | `SCSpecFunctionV0.outputs<1>` allows zero or one output; changing presence changes decoding expectations. |
| Struct added | Warning | Medium | Additive UDTs may affect generated clients/docs but do not remove an existing callable interface element. |
| Struct removed | Breaking | Medium | Existing functions or clients may reference the removed UDT. |
| Struct field added | Unknown | Medium | Structs are spec-visible UDTs. Recent SDK/CAP-86 behavior may tolerate extra/missing fields in some stored-data cases, but function argument/client compatibility needs more validation. |
| Struct field removed | Unknown | Medium | May break clients constructing values or decoding returned values; exact tolerance depends on representation and SDK behavior. |
| Struct field renamed | Unknown | Medium | If serialized as map keys, rename can change data shape; generated clients also change. More fixture validation is needed. |
| Struct field reordered | Unknown | Medium | Named struct fields are likely map-shaped, so order may not be semantically relevant, but this remains conservative in Phase 2. |
| Struct field type changed | Breaking | High | The field's `ScSpecTypeDef` changes expected encoded value. |
| Enum added | Warning | Medium | Additive UDTs may affect generated clients/docs. |
| Enum removed | Breaking | Medium | Existing functions or clients may reference the removed UDT. |
| Enum case added | Unknown | Medium | Existing old values may still decode, but generated clients and exhaustive handling behavior need more validation. |
| Enum case removed | Breaking | High | Existing values using removed numeric case may no longer be valid. |
| Enum discriminant/value changed | Breaking | High | SEP-48 states UDT enums map to `SCV_U32` values; changing the value changes wire meaning. |
| Union added | Warning | Medium | Additive UDTs may affect generated clients/docs. |
| Union removed | Breaking | Medium | Existing functions or clients may reference the removed UDT. |
| Union case added | Unknown | Medium | Existing values may remain representable, but union compatibility semantics need more fixture evidence. |
| Union case removed | Breaking | High | Existing callers/returned values using removed case lose a valid variant. |
| Union case payload changed | Breaking | High | Tuple case payload types define encoded values. |
| Error enum added | Warning | Medium | Existing success calls remain valid, but clients handling errors may need regeneration. |
| Error enum removed | Warning | Medium | Removing an error code may not break callers, but clients/documentation may rely on it. |
| Error enum case removed | Unknown | Medium | Removing an exposed error case may affect generated clients and error handling; classify conservatively until validated further. |
| Error code changed | Breaking | High | Contract errors are represented by numeric contract codes. Changing a code changes machine interpretation. |
| Metadata added | Warning | Medium | Metadata is off-chain informational/build data unless a policy declares a key meaningful. |
| Metadata removed | Warning | Medium | May affect provenance, verification, or release workflows but not callable interface. |
| Metadata value changed | Warning | Medium | Version/build metadata changes are expected across releases; policy can later escalate selected keys. |
| Env interface version changed (`contractenvmetav0`) | Unknown | Medium | The interface version (`protocol`, `pre_release`) names the host environment the contract was built against. Whether a version runs on a network depends on that network's protocol, which this tool does not know, so a difference is surfaced as `Unknown` rather than guessed at. Implemented. |
| Env metadata absent on one side, or undecodable | Unknown | Medium | The two versions cannot be compared. A section that cannot be decoded with the bundled XDR is reported, not treated as a failure. Implemented. |
| Event added | Unknown | Medium | Event consumers and indexers may care about emitted shapes; Phase 2.5 does not claim safety. |
| Event removed | Unknown | Medium | Event consumers and indexers may rely on removed event shapes. |
| Event renamed | Unknown | Medium | Event consumers may rely on name/prefix topics. |
| Event parameter added | Unknown | Medium | Event decoding/indexing shape changes. |
| Event parameter removed | Unknown | Medium | Event decoding/indexing shape changes. |
| Event parameter type/location changed | Unknown | Medium | Topic/data decoding changes and indexers may break. |
| Event data format changed | Unknown | Medium | Consumers decode event data differently. |

## Rules Implemented In Phase 2

The engine currently implements:

- function added: `Compatible`;
- function removed or renamed: `Breaking`;
- function argument added/removed/reordered/type changed: `Breaking`;
- function argument renamed: `Warning`;
- return type or return presence changed: `Breaking`;
- struct added: `Warning`;
- struct removed: `Breaking`;
- struct field added/removed/renamed/reordered: `Unknown`;
- struct field type changed: `Breaking`;
- enum added: `Warning`;
- enum removed: `Breaking`;
- enum case added: `Unknown`;
- enum discriminant/value changed: `Breaking`;
- enum case removed: `Breaking`;
- union added: `Warning`;
- union removed: `Breaking`;
- union case added: `Unknown`;
- union case removed or payload type changed: `Breaking`;
- error enum added/removed: `Warning`;
- error case added: `Warning`;
- error case removed: `Unknown`;
- error code changed: `Breaking`;
- event added/removed/parameter changed/data format changed: `Unknown`;
- env interface version changed, or env metadata present on only one side or undecodable: `Unknown`. Only the first `contractenvmetav0` section and its first entry are read. Because this makes a routine SDK/protocol upgrade `Unknown`, use `--fail-on breaking` if such changes should be reported without failing the build;
- malformed/missing spec: execution failure, not a compatibility finding.

Findings are sorted deterministically by severity, finding code, path, and message. Severity order is `Breaking`, `Unknown`, `Warning`, then `Compatible`.

Overall result policy:

- any `Breaking` finding => `Breaking`;
- else any `Unknown` finding => `Unknown`;
- else any `Warning` finding => `Warning`;
- else `Compatible`.

## Rules That Remain Uncertain

These need real compiled fixtures and generated-client checks before final classification:

- struct field added/removed/renamed/reordered;
- enum case added and enum case rename where numeric value is unchanged;
- union case added;
- event changes across SDK versions that may or may not emit `ScSpecEventV0`;
- env-meta changes and protocol-version compatibility policy;
- treatment of `lib` field differences in UDT identity;
- how to classify Stellar Asset Contract comparisons.

## Explicit Exclusions From MVP

v0.1 should not analyze:

- storage key schemas;
- data migrations;
- authorization semantics;
- emitted event coverage or behavioral guarantees;
- gas/budget/cost changes;
- host function usage changes;
- source-level Rust changes;
- arbitrary WASM bytecode semantic equivalence;
- security vulnerabilities;
- deployed contract archival/restore workflows;
- Stellar Asset Contract special handling unless it is trivial through official embedded spec APIs.

## Evidence

SEP-48 defines the contract interface as `SCSpecEntry` values stored in the `contractspecv0` custom section, with entries for functions, UDT structs, UDT unions, UDT enums, UDT error enums, and events. The XDR source defines `SCSpecFunctionV0` with ordered `inputs<>` and `outputs<1>`, and defines event parameters, data formats, enum values, union cases, and UDT fields as part of the spec.

References: [SEP-48 Contract Interface Spec](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0048.md), [Stellar-contract-spec.x](https://github.com/stellar/stellar-xdr/blob/main/Stellar-contract-spec.x), [Fully Typed Contracts](https://developers.stellar.org/docs/learn/fundamentals/contract-development/types/fully-typed-contracts).
