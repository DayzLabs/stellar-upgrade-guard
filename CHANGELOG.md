# Changelog

All notable changes to this project will be documented in this file.

This project follows Keep a Changelog style and intends to use Semantic Versioning.

## [Unreleased]

### Added

- `contractenvmetav0` support: the env interface version (`protocol`, `pre_release`) is read, shown by `inspect` (text and JSON `env_meta` field), and compared. A change, or metadata present on only one side or undecodable, is reported as an `unknown` finding (`env_interface_version_changed`, `env_meta_unavailable`). Contracts rebuilt with a newer SDK/protocol therefore now produce an `unknown` result; pass `--fail-on breaking` to report without failing.
- `compare --format json` now emits `schema_version` and a `source` object in both local and deployed modes, so both share one document shape. The change is additive; see [docs/JSON_OUTPUT.md](docs/JSON_OUTPUT.md).

## [0.1.0] - 2026-10-09

### Added

- Soroban `contractspecv0` parsing from compiled WASM files.
- Deterministic contract interface normalization.
- Deterministic compatibility engine for functions, structs, enums, error enums, unions, and events.
- Local WASM comparison through `stellar-upgrade-guard compare --old --new`.
- Deployed contract comparison through Stellar RPC with contract instance and contract code lookup.
- Text and JSON CLI output.
- GitHub Action for local and deployed comparison modes.
- Repository CI and action-test workflows.
- Real Testnet validation for deployed comparison.
- Phase 2 and Phase 3 Soroban fixtures.
- Documentation for development, RPC architecture, compatibility rules, and release preparation.
- Open-source project documentation for contributing, security, governance, roadmap, release readiness, and maintainer guidance.
- Issue and pull request templates for public collaboration.
