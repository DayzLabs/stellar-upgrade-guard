# Maintainer Guide

This guide records recommended repository settings and maintenance practices. It does not change GitHub settings automatically.

## Branch Protection

Recommended settings for `main`:

- Require a pull request before merge.
- Require status checks:
  - `CI / Rust`
  - `Action Test / Composite Action`
- Block force pushes.
- Prevent branch deletion.
- Require conversation resolution when practical.
- Do not require multiple approvals while the project has one maintainer unless the maintainer wants the additional friction.

## Release Authority

The maintainer decides when a release is ready, updates versions and changelog entries, creates tags, and publishes releases. Do not create release tags from feature branches.

Keep GitHub private vulnerability reporting enabled and keep `SECURITY.md` aligned with the repository's private reporting path.

## Versioning

Use Semantic Versioning. Before `1.0.0`, minor versions may still include breaking changes, but release notes should call them out clearly.

For `v0.1.0`, release artifacts are expected to be:

- the repository source;
- CLI built from `crates/guard-cli`;
- the reusable action under `action/`.

Fixture crates are not release artifacts even though their source remains tracked for tests and validation. Crates.io publication is not required for the initial source release. If crates are published later, publish `guard-core` and `guard-rpc` before `guard-cli` because the CLI depends on those crates by version.

## Reviews

Review compatibility changes with extra care. Require evidence, tests, and deterministic output. Prefer conservative classifications when evidence is incomplete.

## Repository Hygiene

Before merging release or infrastructure PRs, check:

- no secrets or local identities;
- no `target/`, `.build-target/`, logs, or dependency caches;
- docs match behavior;
- normal CI does not depend on live Testnet.
