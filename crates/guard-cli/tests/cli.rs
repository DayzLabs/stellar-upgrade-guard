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

/// Writes an old/new WASM pair whose comparison yields exactly one finding of the named severity.
fn severity_pair(severity: &str) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let old = dir.path().join("old.wasm");
    let new = dir.path().join("new.wasm");
    let (old_spec, new_spec) = match severity {
        // A struct appearing in the new interface is a `warning` finding.
        "warning" => (
            Vec::new(),
            struct_spec_xdr(vec![field("id", ScSpecTypeDef::U32)]),
        ),
        // A struct field added is an `unknown` finding.
        "unknown" => (
            struct_spec_xdr(vec![field("id", ScSpecTypeDef::U32)]),
            struct_spec_xdr(vec![
                field("id", ScSpecTypeDef::U32),
                field("status", ScSpecTypeDef::U32),
            ]),
        ),
        // A struct field type change is a `breaking` finding.
        "breaking" => (
            struct_spec_xdr(vec![field("id", ScSpecTypeDef::U32)]),
            struct_spec_xdr(vec![field("id", ScSpecTypeDef::U64)]),
        ),
        other => panic!("unknown severity {other}"),
    };
    std::fs::write(&old, wasm_with_contract_spec(&old_spec)).expect("write old wasm");
    std::fs::write(&new, wasm_with_contract_spec(&new_spec)).expect("write new wasm");
    (dir, old, new)
}

fn exit_code_for(severity: &str, fail_on: Option<&str>) -> Option<i32> {
    let (_dir, old, new) = severity_pair(severity);
    let mut args = vec![
        "compare".to_owned(),
        "--old".to_owned(),
        old.to_str().unwrap().to_owned(),
        "--new".to_owned(),
        new.to_str().unwrap().to_owned(),
    ];
    if let Some(fail_on) = fail_on {
        args.push("--fail-on".to_owned());
        args.push(fail_on.to_owned());
    }
    command()
        .args(args)
        .output()
        .expect("compare command runs")
        .status
        .code()
}

#[test]
fn fail_on_thresholds_map_severity_to_exit_codes() {
    // (finding severity, --fail-on value, expected exit code)
    let cases = [
        ("warning", "breaking", 0),
        ("warning", "unknown", 0),
        ("warning", "warning", 1),
        ("unknown", "breaking", 0),
        ("unknown", "unknown", 1),
        ("unknown", "warning", 1),
        ("breaking", "breaking", 1),
        ("breaking", "unknown", 1),
        ("breaking", "warning", 1),
    ];
    for (severity, fail_on, expected) in cases {
        assert_eq!(
            exit_code_for(severity, Some(fail_on)),
            Some(expected),
            "{severity} finding with --fail-on {fail_on}"
        );
    }
}

#[test]
fn fail_on_defaults_to_unknown() {
    assert_eq!(exit_code_for("warning", None), Some(0));
    assert_eq!(exit_code_for("unknown", None), Some(1));
    assert_eq!(exit_code_for("breaking", None), Some(1));
}

#[test]
fn fail_on_warning_does_not_fail_compatible_changes() {
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
            "--fail-on",
            "warning",
        ])
        .output()
        .expect("compare command runs");

    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn fail_on_rejects_unknown_values() {
    let (_dir, old, new) = severity_pair("warning");
    let output = command()
        .args([
            "compare",
            "--old",
            old.to_str().unwrap(),
            "--new",
            new.to_str().unwrap(),
            "--fail-on",
            "compatible",
        ])
        .output()
        .expect("compare command runs");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--fail-on"));
}

const COMPARE_JSON_KEYS: [&str; 5] = ["findings", "result", "schema_version", "source", "summary"];

fn sorted_keys(value: &serde_json::Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value
        .as_object()
        .expect("json object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

/// Serves `bodies` as sequential JSON responses, one per connection, on a loopback port.
fn stub_rpc(bodies: Vec<String>) -> String {
    use std::io::{Read, Write};

    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}", listener.local_addr().expect("addr"));
    std::thread::spawn(move || {
        for body in bodies {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut request = Vec::new();
            let mut chunk = [0_u8; 4096];
            let body_start = loop {
                let read = stream.read(&mut chunk).expect("read request");
                request.extend_from_slice(&chunk[..read]);
                if let Some(pos) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    break pos + 4;
                }
            };
            let headers = String::from_utf8_lossy(&request[..body_start]).to_lowercase();
            let length = headers
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            while request.len() < body_start + length {
                let read = stream.read(&mut chunk).expect("read body");
                request.extend_from_slice(&chunk[..read]);
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
                 Connection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).expect("write reply");
        }
    });
    url
}

fn ledger_entries_body(xdr: String) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "latestLedger": 100,
            "entries": [{ "xdr": xdr, "lastModifiedLedger": 42 }]
        }
    })
    .to_string()
}

#[test]
fn local_json_report_has_the_unified_schema() {
    let old = fixture("fixtures/phase2/breaking-return-type-changed/old.wasm");
    let new = fixture("fixtures/phase2/breaking-return-type-changed/new.wasm");
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

    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(sorted_keys(&json), COMPARE_JSON_KEYS);
    assert_eq!(json["schema_version"], 1);
    assert_eq!(sorted_keys(&json["source"]), ["kind", "new", "old"]);
    assert_eq!(json["source"]["kind"], "local_files");
    assert_eq!(json["source"]["old"], old.display().to_string());
    assert_eq!(json["source"]["new"], new.display().to_string());
    assert_eq!(
        sorted_keys(&json["summary"]),
        ["breaking", "compatible", "unknown", "warning"]
    );
    assert_eq!(json["result"], "breaking");
    assert_eq!(
        sorted_keys(&json["findings"][0]),
        [
            "code",
            "impact",
            "message",
            "new_value",
            "old_value",
            "path"
        ]
    );
}

#[test]
fn deployed_json_report_has_the_same_top_level_schema_as_local() {
    use stellar_xdr::{
        BytesM, ContractCodeEntry, ContractCodeEntryExt, ContractDataDurability, ContractDataEntry,
        ContractExecutable, ContractId, ExtensionPoint, Hash, LedgerEntryData, ScAddress,
        ScContractInstance, ScVal,
    };

    let deployed_wasm = std::fs::read(fixture(
        "fixtures/phase2/breaking-return-type-changed/old.wasm",
    ))
    .expect("read fixture");
    let hash = [7_u8; 32];
    let instance = LedgerEntryData::ContractData(ContractDataEntry {
        ext: ExtensionPoint::V0,
        contract: ScAddress::Contract(ContractId(Hash([1; 32]))),
        key: ScVal::LedgerKeyContractInstance,
        durability: ContractDataDurability::Persistent,
        val: ScVal::ContractInstance(ScContractInstance {
            executable: ContractExecutable::Wasm(Hash(hash)),
            storage: None,
        }),
    })
    .to_xdr_base64(Limits::none())
    .expect("instance xdr");
    let code = LedgerEntryData::ContractCode(ContractCodeEntry {
        ext: ContractCodeEntryExt::V0,
        hash: Hash(hash),
        code: BytesM::try_from(deployed_wasm).expect("bounded wasm"),
    })
    .to_xdr_base64(Limits::none())
    .expect("code xdr");
    let url = stub_rpc(vec![
        ledger_entries_body(instance),
        ledger_entries_body(code),
    ]);

    let candidate = fixture("fixtures/phase2/breaking-return-type-changed/new.wasm");
    let output = command()
        .args([
            "compare",
            "--contract",
            "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4",
            "--candidate",
            candidate.to_str().unwrap(),
            "--network",
            "testnet",
            "--rpc-url",
            &url,
            "--format",
            "json",
        ])
        .output()
        .expect("compare command runs");

    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(sorted_keys(&json), COMPARE_JSON_KEYS);
    assert_eq!(json["schema_version"], 1);
    assert_eq!(
        sorted_keys(&json["source"]),
        [
            "candidate",
            "contract_id",
            "kind",
            "last_modified_ledger",
            "network",
            "wasm_hash"
        ]
    );
    assert_eq!(json["source"]["kind"], "deployed_contract");
    assert_eq!(json["source"]["network"], "testnet");
    assert_eq!(json["source"]["wasm_hash"], "07".repeat(32));
    assert_eq!(json["source"]["last_modified_ledger"], 42);
    assert_eq!(json["result"], "breaking");
    assert_eq!(json["findings"][0]["code"], "return_type_changed");
}

#[test]
fn deployed_compare_times_out_against_unresponsive_rpc() {
    // A loopback server that accepts the connection but never answers.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}", listener.local_addr().expect("addr"));
    let holder = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        std::thread::sleep(std::time::Duration::from_secs(4));
        drop(stream);
    });

    let started = std::time::Instant::now();
    let output = command()
        .args([
            "compare",
            "--contract",
            "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4",
            "--candidate",
            fixture("fixtures/phase2/compatible-function-added/new.wasm")
                .to_str()
                .unwrap(),
            "--rpc-url",
            &url,
            "--rpc-timeout",
            "1",
        ])
        .output()
        .expect("compare command runs");

    assert_eq!(output.status.code(), Some(2));
    assert!(started.elapsed() < std::time::Duration::from_secs(4));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "error: rpc request timed out after 1s"
    );
    assert!(output.stdout.is_empty());
    holder.join().expect("holder thread");
}

#[test]
fn rpc_timeout_must_be_positive() {
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
            "testnet",
            "--rpc-timeout",
            "0",
        ])
        .output()
        .expect("compare command runs");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--rpc-timeout"));
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
