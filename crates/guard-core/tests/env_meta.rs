//! Tests for `contractenvmetav0` parsing and comparison.
//!
//! The format is `SCEnvMetaEntry` XDR; the only entry kind in the bundled XDR is the interface
//! version (`protocol`, `pre_release`). Real bytes come from the committed fixtures, which were
//! built with protocol 28: `00000000 0000001c 00000000`.

use std::path::PathBuf;

use guard_core::{
    CompatibilityImpact, CompatibilityResult, EnvInterfaceVersion, EnvMeta, FindingCode,
    GuardError, compare_wasm, parse_contract_interface, read_env_meta,
};
use stellar_xdr::{Limits, ScEnvMetaEntry, ScEnvMetaEntryInterfaceVersion, WriteXdr};
use wasmparser::{Parser, Payload};

const REAL_ENV_META_HEX: &str = "000000000000001c00000000";

#[test]
fn real_fixture_reports_its_interface_version() {
    let wasm = fixture_wasm("compatible-function-added/old.wasm");

    let interface = parse_contract_interface(&wasm).expect("fixture parses");

    assert_eq!(
        interface.env_meta,
        EnvMeta::InterfaceVersion(EnvInterfaceVersion {
            protocol: 28,
            pre_release: 0
        })
    );
}

#[test]
fn encoded_entry_matches_the_bytes_in_real_fixtures() {
    let real = section_bytes(&fixture_wasm("compatible-function-added/old.wasm"));
    assert_eq!(hex(&real), REAL_ENV_META_HEX);

    assert_eq!(env_meta_bytes(28, 0), real);
}

#[test]
fn real_fixture_pair_has_no_env_findings() {
    let report = compare_wasm(
        &fixture_wasm("compatible-function-added/old.wasm"),
        &fixture_wasm("compatible-function-added/new.wasm"),
    )
    .expect("fixtures compare");

    assert!(
        report
            .findings
            .iter()
            .all(|finding| !format!("{:?}", finding.code).starts_with("Env"))
    );
}

#[test]
fn protocol_change_is_reported_as_unknown() {
    let report = compare_wasm(
        &wasm_with_env_meta(Some(&env_meta_bytes(28, 0))),
        &wasm_with_env_meta(Some(&env_meta_bytes(29, 0))),
    )
    .expect("compare");

    assert_eq!(report.result, CompatibilityResult::Unknown);
    assert_eq!(report.findings.len(), 1);
    let finding = &report.findings[0];
    assert_eq!(finding.code, FindingCode::EnvInterfaceVersionChanged);
    assert_eq!(finding.impact, CompatibilityImpact::Unknown);
    assert_eq!(finding.path, "env.interface_version");
    assert_eq!(
        finding.message,
        "Env interface version changed: protocol 28, pre-release 0 -> protocol 29, pre-release 0"
    );
    assert_eq!(
        finding.old_value.as_deref(),
        Some("protocol 28, pre-release 0")
    );
    assert_eq!(
        finding.new_value.as_deref(),
        Some("protocol 29, pre-release 0")
    );
}

#[test]
fn pre_release_change_is_reported() {
    let report = compare_wasm(
        &wasm_with_env_meta(Some(&env_meta_bytes(28, 0))),
        &wasm_with_env_meta(Some(&env_meta_bytes(28, 3))),
    )
    .expect("compare");

    assert_eq!(report.findings.len(), 1);
    assert_eq!(
        report.findings[0].code,
        FindingCode::EnvInterfaceVersionChanged
    );
}

#[test]
fn identical_versions_are_compatible() {
    let wasm = wasm_with_env_meta(Some(&env_meta_bytes(28, 0)));

    let report = compare_wasm(&wasm, &wasm).expect("compare");

    assert_eq!(report.result, CompatibilityResult::Compatible);
    assert!(report.findings.is_empty());
}

#[test]
fn both_absent_is_not_a_finding() {
    let wasm = wasm_with_env_meta(None);

    let report = compare_wasm(&wasm, &wasm).expect("compare");

    assert_eq!(report.result, CompatibilityResult::Compatible);
    assert!(report.findings.is_empty());
}

#[test]
fn one_side_absent_cannot_be_compared() {
    let report = compare_wasm(
        &wasm_with_env_meta(None),
        &wasm_with_env_meta(Some(&env_meta_bytes(28, 0))),
    )
    .expect("compare");

    assert_eq!(report.result, CompatibilityResult::Unknown);
    assert_eq!(report.findings.len(), 1);
    let finding = &report.findings[0];
    assert_eq!(finding.code, FindingCode::EnvMetaUnavailable);
    assert_eq!(finding.path, "env");
    assert_eq!(
        finding.message,
        "Env metadata cannot be compared: old is absent, new is protocol 28, pre-release 0"
    );
}

#[test]
fn unknown_entry_kind_is_unreadable_not_an_error() {
    // Discriminant 99 does not exist in the bundled XDR (a newer environment could add one).
    let unreadable = [0, 0, 0, 99, 0, 0, 0, 0];

    let interface = parse_contract_interface(&wasm_with_env_meta(Some(&unreadable)))
        .expect("wasm still parses");
    assert_eq!(interface.env_meta, EnvMeta::Unreadable);

    let report = compare_wasm(
        &wasm_with_env_meta(Some(&env_meta_bytes(28, 0))),
        &wasm_with_env_meta(Some(&unreadable)),
    )
    .expect("compare does not fail");
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].code, FindingCode::EnvMetaUnavailable);
    assert_eq!(
        report.findings[0].message,
        "Env metadata cannot be compared: old is protocol 28, pre-release 0, new is unreadable"
    );
}

#[test]
fn truncated_entry_is_unreadable() {
    let truncated = &env_meta_bytes(28, 0)[..6];

    let meta = read_env_meta(&wasm_with_env_meta(Some(truncated))).expect("wasm parses");

    assert_eq!(meta, EnvMeta::Unreadable);
}

#[test]
fn empty_section_is_absent() {
    let meta = read_env_meta(&wasm_with_env_meta(Some(&[]))).expect("wasm parses");

    assert_eq!(meta, EnvMeta::Absent);
}

#[test]
fn only_the_first_env_meta_section_is_used() {
    let mut wasm = wasm_with_env_meta(Some(&env_meta_bytes(28, 0)));
    wasm.extend(custom_section("contractenvmetav0", &env_meta_bytes(99, 0)));

    let meta = read_env_meta(&wasm).expect("wasm parses");

    assert_eq!(
        meta,
        EnvMeta::InterfaceVersion(EnvInterfaceVersion {
            protocol: 28,
            pre_release: 0
        })
    );
}

#[test]
fn structurally_invalid_wasm_after_the_spec_is_an_error() {
    let mut wasm = wasm_with_env_meta(None);
    // A section id with a length that runs past the end of the file.
    wasm.extend([0x01, 0x7f]);

    let err = parse_contract_interface(&wasm).expect_err("truncated wasm fails");

    assert!(matches!(err, GuardError::InvalidWasm(_)), "got {err:?}");
}

#[test]
fn env_meta_appears_in_inspect_json() {
    let interface = parse_contract_interface(&wasm_with_env_meta(Some(&env_meta_bytes(28, 0))))
        .expect("parses");

    let json = serde_json::to_value(&interface).expect("serializes");

    assert_eq!(
        json["env_meta"],
        serde_json::json!({ "interface_version": { "protocol": 28, "pre_release": 0 } })
    );
}

fn fixture_wasm(relative: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/phase2")
        .join(relative);
    std::fs::read(path).expect("fixture exists")
}

fn section_bytes(wasm: &[u8]) -> Vec<u8> {
    for payload in Parser::new(0).parse_all(wasm) {
        if let Payload::CustomSection(section) = payload.expect("valid wasm")
            && section.name() == "contractenvmetav0"
        {
            return section.data().to_vec();
        }
    }
    panic!("no contractenvmetav0 section");
}

fn env_meta_bytes(protocol: u32, pre_release: u32) -> Vec<u8> {
    ScEnvMetaEntry::ScEnvMetaKindInterfaceVersion(ScEnvMetaEntryInterfaceVersion {
        protocol,
        pre_release,
    })
    .to_xdr(Limits::none())
    .expect("env meta serializes")
}

/// A minimal WASM with an empty `contractspecv0` section and, optionally, `contractenvmetav0`.
fn wasm_with_env_meta(env_meta: Option<&[u8]>) -> Vec<u8> {
    let mut wasm = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    wasm.extend(custom_section("contractspecv0", &[]));
    if let Some(env_meta) = env_meta {
        wasm.extend(custom_section("contractenvmetav0", env_meta));
    }
    wasm
}

fn custom_section(name: &str, data: &[u8]) -> Vec<u8> {
    let mut payload = Vec::new();
    leb_u32(u32::try_from(name.len()).expect("short name"), &mut payload);
    payload.extend(name.as_bytes());
    payload.extend(data);

    let mut section = vec![0];
    leb_u32(
        u32::try_from(payload.len()).expect("short section"),
        &mut section,
    );
    section.extend(payload);
    section
}

fn leb_u32(mut value: u32, out: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
