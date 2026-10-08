use std::{path::PathBuf, process::Command};

use stellar_xdr::{
    Limits, ScSpecEntry, ScSpecTypeDef, ScSpecUdtStructFieldV0, ScSpecUdtStructV0, StringM, VecM,
    WriteXdr,
};

#[test]
fn compatible_compare_exits_zero() {
    let output = command()
        .args([
            "compare",
            "--old",
            fixture("fixtures/phase2/compatible-function-added/old.wasm")
                .to_str()
                .unwrap(),
            "--new",
            fixture("fixtures/phase2/compatible-function-added/new.wasm")
                .to_str()
                .unwrap(),
        ])
        .output()
        .expect("compare command runs");

    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn breaking_compare_exits_one() {
    let output = command()
        .args([
            "compare",
            "--old",
            fixture("fixtures/phase2/breaking-return-type-changed/old.wasm")
                .to_str()
                .unwrap(),
            "--new",
            fixture("fixtures/phase2/breaking-return-type-changed/new.wasm")
                .to_str()
                .unwrap(),
        ])
        .output()
        .expect("compare command runs");

    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn unknown_compare_exits_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    let old = dir.path().join("old.wasm");
    let new = dir.path().join("new.wasm");
    std::fs::write(
        &old,
        wasm_with_contract_spec(&struct_spec_xdr(vec![field("id", ScSpecTypeDef::U32)])),
    )
    .expect("write old wasm");
    std::fs::write(
        &new,
        wasm_with_contract_spec(&struct_spec_xdr(vec![
            field("id", ScSpecTypeDef::U32),
            field("status", ScSpecTypeDef::U32),
        ])),
    )
    .expect("write new wasm");

    let output = command()
        .args([
            "compare",
            "--old",
            old.to_str().unwrap(),
            "--new",
            new.to_str().unwrap(),
        ])
        .output()
        .expect("compare command runs");

    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn parse_error_exits_two_and_json_stdout_is_clean() {
    let dir = tempfile::tempdir().expect("tempdir");
    let old = dir.path().join("old.wasm");
    let new = fixture("fixtures/phase2/compatible-function-added/new.wasm");
    std::fs::write(&old, b"not wasm").expect("write invalid wasm");

    let output = command()
        .args([
            "compare",
            "--old",
            old.to_str().unwrap(),
            "--new",
            new.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("compare command runs");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).starts_with("error: "));
}

#[test]
fn json_compare_stdout_is_structured_without_stderr_noise() {
    let output = command()
        .args([
            "compare",
            "--old",
            fixture("fixtures/phase2/breaking-return-type-changed/old.wasm")
                .to_str()
                .unwrap(),
            "--new",
            fixture("fixtures/phase2/breaking-return-type-changed/new.wasm")
                .to_str()
                .unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("compare command runs");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is valid json");
    assert_eq!(json["result"], "breaking");
    assert_eq!(json["summary"]["breaking"], 1);
    assert_eq!(json["findings"][0]["code"], "return_type_changed");
}

#[test]
fn compare_rejects_mixing_local_and_deployed_modes() {
    let output = command()
        .args([
            "compare",
            "--old",
            fixture("fixtures/phase2/compatible-function-added/old.wasm")
                .to_str()
                .unwrap(),
            "--new",
            fixture("fixtures/phase2/compatible-function-added/new.wasm")
                .to_str()
                .unwrap(),
            "--contract",
            "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4",
            "--candidate",
            fixture("fixtures/phase2/compatible-function-added/new.wasm")
                .to_str()
                .unwrap(),
        ])
        .output()
        .expect("compare command runs");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot be combined"));
}

#[test]
fn mainnet_deployed_compare_requires_rpc_url() {
    let output = command()
        .args([
            "compare",
            "--contract",
            "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4",
            "--candidate",
            fixture("fixtures/phase2/compatible-function-added/new.wasm")
                .to_str()
                .unwrap(),
            "--network",
            "mainnet",
        ])
        .output()
        .expect("compare command runs");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires --rpc-url"));
}

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_stellar-upgrade-guard"))
}

fn fixture(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(path)
}

fn struct_spec_xdr(fields: Vec<ScSpecUdtStructFieldV0>) -> Vec<u8> {
    ScSpecEntry::UdtStructV0(ScSpecUdtStructV0 {
        doc: string_m(""),
        lib: string_m(""),
        name: string_m("Account"),
        fields: VecM::try_from(fields).expect("bounded fields"),
    })
    .to_xdr(Limits::none())
    .expect("struct spec serializes")
}

fn field(name: &str, type_: ScSpecTypeDef) -> ScSpecUdtStructFieldV0 {
    ScSpecUdtStructFieldV0 {
        doc: string_m(""),
        name: string_m(name),
        type_,
    }
}

fn string_m<const N: u32>(value: &str) -> StringM<N> {
    StringM::try_from(value.to_owned()).expect("bounded string")
}

fn empty_wasm() -> Vec<u8> {
    vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]
}

fn wasm_with_contract_spec(spec: &[u8]) -> Vec<u8> {
    let name = b"contractspecv0";
    let mut custom_payload = Vec::new();
    encode_leb_u32(name.len() as u32, &mut custom_payload);
    custom_payload.extend(name);
    custom_payload.extend(spec);

    let mut wasm = empty_wasm();
    wasm.push(0);
    encode_leb_u32(custom_payload.len() as u32, &mut wasm);
    wasm.extend(custom_payload);
    wasm
}

fn encode_leb_u32(mut value: u32, out: &mut Vec<u8>) {
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
