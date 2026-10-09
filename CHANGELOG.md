# Changelog

All notable changes to this project will be documented in this file.

This project follows Keep a Changelog style and intends to use Semantic Versioning.

## [Unreleased]

### Added

- No changes yet.

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
