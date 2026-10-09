# v0.1.0 Release Readiness

## Summary

Stellar Upgrade Guard is ready for `v0.1.0` release review as a GitHub source release. Crates.io publication is deferred.

## Assessment

| Area | Status | Notes |
| --- | --- | --- |
| Functionality | Ready for v0.1 scope | Local and deployed interface comparison exist. |
| Tests | Strong for current scope | Parser, compatibility, CLI, RPC mock, and fixture tests exist. |
| CI | Ready | Repository CI and action-test workflows exist. |
| Documentation | Ready | OSS docs, action docs, changelog, and release notes are present. |
| Security | Ready for initial release | Security policy, private vulnerability reporting, branch protection, and secret-handling guidance are in place. |
| Packaging | Ready for source release | GitHub source release first; crates.io publication is explicitly deferred. |
| Installation | Source/Git-based | `cargo install --path crates/guard-cli` is verified; tag install is documented for `v0.1.0` once the tag exists. |
| API stability | Pre-1.0 | Internal APIs may change. |
| Action stability | Initial | Composite action is tested against local fixtures. |
| Known limitations | Documented | No storage, runtime, authorization, or migration safety proof. |

## Resolved Release Blockers

- Repository ownership has migrated to `DayzLabs/stellar-upgrade-guard`.
- Branch protection and required checks are configured for the release process.
- GitHub private vulnerability reporting is enabled.
- `CHANGELOG.md` is dated for `2026-10-09`.
- Crate versions and repository metadata are set for `0.1.0`.
- MSRV is documented as Rust `1.91.0`.
- Dependency audit and secret scan are part of the final release gate.
- GitHub Action examples use the planned `v0.1.0` tag.
- Crates.io publication is deferred; this release is a GitHub source release.

## Remaining Release Steps

- Merge the release preparation branch after review and green CI.
- Create the `v0.1.0` tag only after approval.
- Create the GitHub source release from the tag.
- Re-run install and action-reference verification after the tag exists.
- Publish crates.io packages in a future release only after a separate packaging decision.

## Recommendation

Proceed with `v0.1.0` review and tagging after the final local gates pass. Do not claim production safety; position the release as an early deterministic interface guard for Soroban contract upgrades.
