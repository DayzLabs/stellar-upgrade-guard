# Release Checklist

Use this checklist for the `v0.1.0` release. Do not create releases from a dirty worktree.

## Before Tagging

- [x] Start from clean `main`.
- [x] Confirm required GitHub workflows are green:
  - CI / Rust
  - Action Test / Composite Action
- [x] Confirm `v0.1.0` is a GitHub source release first; crates.io publication is deferred.
- [x] Confirm branch protection and required checks are configured.
- [x] Confirm GitHub private vulnerability reporting is enabled.
- [x] Confirm `CHANGELOG.md` has the release date and entries.
- [x] Confirm crate versions intended for release are updated.
- [x] Confirm README installation and action references are accurate for the planned tag.
- [x] Confirm no `target/`, `.build-target/`, logs, local identities, or generated release artifacts are tracked.
- [x] Run local checks:

```text
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

- [x] Run dependency review:

```text
cargo tree
cargo tree -d
```

- [x] Run `cargo audit` if available.
- [x] Run a secret scan.
- [x] Perform source, Git, release-binary, and smoke-test validation.
- [x] Perform a Testnet smoke test if deployed comparison changed or live validation is required.

## Release

- [ ] Create the release commit.
- [ ] Create the tag only after review approval.
- [ ] Draft GitHub release notes from `CHANGELOG.md` and `docs/RELEASE_NOTES_V0.1.0.md`.
- [ ] Attach release artifacts only if the distribution process has been defined.
- [ ] Update action examples or tags if an action tag is part of the release.

## After Release

- [ ] Verify installation instructions from the created tag.
- [ ] Verify the GitHub Action reference from the created tag.
- [ ] Open follow-up issues for deferred release blockers.
- [ ] Move `CHANGELOG.md` back to an `[Unreleased]` development state.
