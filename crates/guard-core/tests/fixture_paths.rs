//! Meta-check: integration tests must reference canonical fixture paths.
//!
//! The canonical fixtures are `fixtures/phase2/<scenario>/old.wasm` and
//! `fixtures/phase2/<scenario>/new.wasm`. Test sources must not point at the
//! `.build-target` output directory or the removed `wasm` directory under
//! `fixtures/phase2`; this regression check protects the canonical layout.

use std::fs;
use std::path::PathBuf;

/// Forbidden path fragments, assembled at runtime so that this file itself
/// does not contain the literal strings the scan looks for.
fn forbidden_fragments() -> Vec<String> {
    let base = "fixtures/phase2";
    [".build-target", "wasm"]
        .into_iter()
        .map(|segment| format!("{base}/{segment}"))
        .collect()
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/guard-core sits two levels below the workspace root")
        .to_path_buf()
}

#[test]
fn integration_tests_reference_canonical_fixture_paths() {
    let root = workspace_root();
    let test_dirs = [
        root.join("crates/guard-core/tests"),
        root.join("crates/guard-cli/tests"),
    ];
    let forbidden = forbidden_fragments();

    let mut offenders: Vec<String> = Vec::new();
    for dir in test_dirs {
        let mut sources: Vec<PathBuf> = fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
            .collect();
        sources.sort();

        for path in sources {
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            for fragment in &forbidden {
                if source.contains(fragment.as_str()) {
                    offenders.push(format!("{} references {}", path.display(), fragment));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "integration tests must reference canonical fixtures \
         (fixtures/phase2/<scenario>/old.wasm and new.wasm), not build output:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn phase2_scenario_dirs_hold_a_complete_fixture_pair() {
    let phase2 = workspace_root().join("fixtures/phase2");

    let mut scenarios = 0;
    let mut entries: Vec<PathBuf> = fs::read_dir(&phase2)
        .unwrap_or_else(|e| panic!("read {}: {e}", phase2.display()))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .collect();
    entries.sort();

    for scenario in entries {
        let has_old = scenario.join("old.wasm").is_file();
        let has_new = scenario.join("new.wasm").is_file();
        if !has_old && !has_new {
            // Not a fixture dir (e.g. a shared helper directory); skip.
            continue;
        }
        scenarios += 1;
        assert!(
            has_old && has_new,
            "{} must contain both old.wasm and new.wasm",
            scenario.display()
        );
    }

    assert!(
        scenarios > 0,
        "expected at least one fixtures/phase2 scenario with a canonical pair"
    );
}
