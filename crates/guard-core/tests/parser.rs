use guard_core::{
    ContractInterface, GuardError, TypeRef, parse_contract_interface,
    parse_contract_interface_file, read_contract_spec,
};
use stellar_xdr::{
    Limits, ScSpecEntry, ScSpecFunctionInputV0, ScSpecFunctionV0, ScSpecTypeBytesN, ScSpecTypeDef,
    ScSpecTypeMap, ScSpecTypeOption, ScSpecTypeResult, ScSpecTypeTuple, ScSpecTypeUdt,
    ScSpecTypeVec, ScSymbol, StringM, VecM, WriteXdr,
};

#[test]
fn parses_valid_soroban_spec_wasm() {
    let wasm = wasm_with_contract_spec(&basic_spec_xdr());

    let interface = parse_contract_interface(&wasm).expect("valid spec parses");

    assert_eq!(
        interface,
        ContractInterface {
            functions: vec![guard_core::FunctionSpec {
                name: "hello".to_owned(),
                inputs: vec![guard_core::FunctionInput {
                    name: "name".to_owned(),
                    type_ref: TypeRef("Symbol".to_owned()),
                }],
                outputs: vec![TypeRef("Symbol".to_owned())],
            }],
            structs: vec![],
            enums: vec![],
            unions: vec![],
            errors: vec![],
            events: vec![],
        }
    );
}

#[test]
fn finds_contractspecv0_entries() {
    let wasm = wasm_with_contract_spec(&basic_spec_xdr());

    let entries = read_contract_spec(&wasm).expect("contract spec entries");

    assert_eq!(entries.len(), 1);
    assert!(matches!(entries[0], ScSpecEntry::FunctionV0(_)));
}

#[test]
fn normalizes_output_deterministically() {
    let spec = basic_function("z_last");
    let first = basic_function("a_first");
    let mut bytes = Vec::new();
    bytes.extend(
        spec.to_xdr(Limits::none())
            .expect("function spec serializes to xdr"),
    );
    bytes.extend(
        first
            .to_xdr(Limits::none())
            .expect("function spec serializes to xdr"),
    );
    let wasm = wasm_with_contract_spec(&bytes);

    let names = parse_contract_interface(&wasm)
        .expect("valid spec parses")
        .functions
        .into_iter()
        .map(|function| function.name)
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["a_first", "z_last"]);
}

#[test]
fn handles_wasm_with_no_contract_spec() {
    let err = parse_contract_interface(&empty_wasm()).expect_err("missing spec should error");

    assert!(matches!(err, GuardError::MissingContractSpec));
}

#[test]
fn handles_invalid_wasm() {
    let err = parse_contract_interface(b"not wasm").expect_err("invalid wasm should error");

    assert!(matches!(err, GuardError::InvalidWasm(_)));
}

#[test]
fn handles_malformed_contract_spec_data_without_panic() {
    let wasm = wasm_with_contract_spec(b"not xdr");
    let err = parse_contract_interface(&wasm).expect_err("malformed spec should error");

    assert!(matches!(err, GuardError::InvalidContractSpec(_)));
}

#[test]
fn reads_wasm_file_from_disk() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("contract.wasm");
    std::fs::write(&path, wasm_with_contract_spec(&basic_spec_xdr())).expect("write fixture wasm");

    let interface = parse_contract_interface_file(&path).expect("file parses");

    assert_eq!(interface.functions[0].name, "hello");
}

#[test]
fn normalizes_composite_and_primitive_types_without_flattening() {
    let function = ScSpecEntry::FunctionV0(ScSpecFunctionV0 {
        doc: string_m(""),
        name: symbol("types"),
        inputs: VecM::try_from(vec![
            ScSpecFunctionInputV0 {
                doc: string_m(""),
                name: string_m("maybe"),
                type_: ScSpecTypeDef::Option(Box::new(ScSpecTypeOption {
                    value_type: Box::new(ScSpecTypeDef::U32),
                })),
            },
            ScSpecFunctionInputV0 {
                doc: string_m(""),
                name: string_m("items"),
                type_: ScSpecTypeDef::Vec(Box::new(ScSpecTypeVec {
                    element_type: Box::new(ScSpecTypeDef::String),
                })),
            },
            ScSpecFunctionInputV0 {
                doc: string_m(""),
                name: string_m("map"),
                type_: ScSpecTypeDef::Map(Box::new(ScSpecTypeMap {
                    key_type: Box::new(ScSpecTypeDef::Symbol),
                    value_type: Box::new(ScSpecTypeDef::I128),
                })),
            },
            ScSpecFunctionInputV0 {
                doc: string_m(""),
                name: string_m("pair"),
                type_: ScSpecTypeDef::Tuple(Box::new(ScSpecTypeTuple {
                    value_types: VecM::try_from(vec![
                        ScSpecTypeDef::Address,
                        ScSpecTypeDef::BytesN(ScSpecTypeBytesN { n: 32 }),
                    ])
                    .expect("bounded tuple values"),
                })),
            },
            ScSpecFunctionInputV0 {
                doc: string_m(""),
                name: string_m("outcome"),
                type_: ScSpecTypeDef::Result(Box::new(ScSpecTypeResult {
                    ok_type: Box::new(ScSpecTypeDef::U64),
                    error_type: Box::new(ScSpecTypeDef::Error),
                })),
            },
            ScSpecFunctionInputV0 {
                doc: string_m(""),
                name: string_m("custom"),
                type_: ScSpecTypeDef::Udt(ScSpecTypeUdt {
                    name: string_m("Account"),
                }),
            },
            ScSpecFunctionInputV0 {
                doc: string_m(""),
                name: string_m("nested_option"),
                type_: ScSpecTypeDef::Option(Box::new(ScSpecTypeOption {
                    value_type: Box::new(ScSpecTypeDef::Vec(Box::new(ScSpecTypeVec {
                        element_type: Box::new(ScSpecTypeDef::U32),
                    }))),
                })),
            },
            ScSpecFunctionInputV0 {
                doc: string_m(""),
                name: string_m("nested_map"),
                type_: ScSpecTypeDef::Map(Box::new(ScSpecTypeMap {
                    key_type: Box::new(ScSpecTypeDef::Address),
                    value_type: Box::new(ScSpecTypeDef::Vec(Box::new(ScSpecTypeVec {
                        element_type: Box::new(ScSpecTypeDef::U64),
                    }))),
                })),
            },
            ScSpecFunctionInputV0 {
                doc: string_m(""),
                name: string_m("nested_user_type"),
                type_: ScSpecTypeDef::Option(Box::new(ScSpecTypeOption {
                    value_type: Box::new(ScSpecTypeDef::Udt(ScSpecTypeUdt {
                        name: string_m("Account"),
                    })),
                })),
            },
            ScSpecFunctionInputV0 {
                doc: string_m(""),
                name: string_m("signed_32"),
                type_: ScSpecTypeDef::I32,
            },
            ScSpecFunctionInputV0 {
                doc: string_m(""),
                name: string_m("signed_64"),
                type_: ScSpecTypeDef::I64,
            },
            ScSpecFunctionInputV0 {
                doc: string_m(""),
                name: string_m("unsigned_128"),
                type_: ScSpecTypeDef::U128,
            },
            ScSpecFunctionInputV0 {
                doc: string_m(""),
                name: string_m("signed_256"),
                type_: ScSpecTypeDef::I256,
            },
            ScSpecFunctionInputV0 {
                doc: string_m(""),
                name: string_m("unsigned_256"),
                type_: ScSpecTypeDef::U256,
            },
        ])
        .expect("bounded inputs"),
        outputs: VecM::try_from(vec![ScSpecTypeDef::Bytes]).expect("bounded outputs"),
    });
    let wasm = wasm_with_contract_spec(
        &function
            .to_xdr(Limits::none())
            .expect("function spec serializes to xdr"),
    );

    let interface = parse_contract_interface(&wasm).expect("composite spec parses");
    let repeated_interface = parse_contract_interface(&wasm).expect("composite spec parses again");
    assert_eq!(interface, repeated_interface);
    let inputs = &interface.functions[0].inputs;

    assert_eq!(inputs[0].type_ref, TypeRef("Option<u32>".to_owned()));
    assert_eq!(inputs[1].type_ref, TypeRef("Vec<String>".to_owned()));
    assert_eq!(inputs[2].type_ref, TypeRef("Map<Symbol, i128>".to_owned()));
    assert_eq!(
        inputs[3].type_ref,
        TypeRef("(Address, BytesN<32>)".to_owned())
    );
    assert_eq!(inputs[4].type_ref, TypeRef("Result<u64, Error>".to_owned()));
    assert_eq!(inputs[5].type_ref, TypeRef("Account".to_owned()));
    assert_eq!(inputs[6].type_ref, TypeRef("Option<Vec<u32>>".to_owned()));
    assert_eq!(
        inputs[7].type_ref,
        TypeRef("Map<Address, Vec<u64>>".to_owned())
    );
    assert_eq!(inputs[8].type_ref, TypeRef("Option<Account>".to_owned()));
    assert_eq!(inputs[9].type_ref, TypeRef("i32".to_owned()));
    assert_eq!(inputs[10].type_ref, TypeRef("i64".to_owned()));
    assert_eq!(inputs[11].type_ref, TypeRef("u128".to_owned()));
    assert_eq!(inputs[12].type_ref, TypeRef("i256".to_owned()));
    assert_eq!(inputs[13].type_ref, TypeRef("u256".to_owned()));
    assert_eq!(
        serde_json::to_value(&inputs[6].type_ref).expect("nested type serializes"),
        serde_json::Value::String("Option<Vec<u32>>".to_owned())
    );
    assert_eq!(
        interface.functions[0].outputs,
        vec![TypeRef("Bytes".to_owned())]
    );
}

fn basic_spec_xdr() -> Vec<u8> {
    basic_function("hello")
        .to_xdr(Limits::none())
        .expect("function spec serializes to xdr")
}

fn basic_function(name: &str) -> ScSpecEntry {
    ScSpecEntry::FunctionV0(ScSpecFunctionV0 {
        doc: string_m(""),
        name: symbol(name),
        inputs: VecM::try_from(vec![ScSpecFunctionInputV0 {
            doc: string_m(""),
            name: string_m("name"),
            type_: ScSpecTypeDef::Symbol,
        }])
        .expect("bounded inputs"),
        outputs: VecM::try_from(vec![ScSpecTypeDef::Symbol]).expect("bounded outputs"),
    })
}

fn string_m<const N: u32>(value: &str) -> StringM<N> {
    StringM::try_from(value.to_owned()).expect("bounded string")
}

fn symbol(value: &str) -> ScSymbol {
    ScSymbol::try_from(value.to_owned()).expect("bounded symbol")
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
