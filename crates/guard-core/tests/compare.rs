use guard_core::{
    CompatibilityImpact, CompatibilityResult, ContractInterface, EnumCase, EnumSpec, ErrorCase,
    ErrorSpec, EventParam, EventSpec, FindingCode, FunctionInput, FunctionSpec, StructField,
    StructSpec, TypeRef, UnionCase, UnionSpec, compare_interfaces, compare_wasm,
};

#[test]
fn identical_interfaces_are_compatible() {
    let report = compare_interfaces(&base_interface(), &base_interface());

    assert_eq!(report.result, CompatibilityResult::Compatible);
    assert!(report.findings.is_empty());
}

#[test]
fn function_addition_is_compatible() {
    let old = base_interface();
    let mut new = base_interface();
    new.functions
        .push(function("added", vec![], vec![ty("Symbol")]));

    let report = compare_interfaces(&old, &new);

    assert_finding(
        &report,
        FindingCode::FunctionAdded,
        CompatibilityImpact::Compatible,
        "function.added",
    );
    assert_eq!(report.result, CompatibilityResult::Compatible);
}

#[test]
fn function_removal_is_breaking() {
    let mut old = base_interface();
    old.functions.push(function("removed", vec![], vec![]));
    let new = base_interface();

    let report = compare_interfaces(&old, &new);

    assert_finding(
        &report,
        FindingCode::FunctionRemoved,
        CompatibilityImpact::Breaking,
        "function.removed",
    );
    assert_eq!(report.result, CompatibilityResult::Breaking);
}

#[test]
fn argument_changes_are_detected() {
    let old = interface_with_function(function(
        "set",
        vec![input("name", "Symbol"), input("amount", "u32")],
        vec![],
    ));
    let new = interface_with_function(function(
        "set",
        vec![input("name", "Symbol"), input("amount", "u64")],
        vec![],
    ));

    let report = compare_interfaces(&old, &new);

    assert_finding(
        &report,
        FindingCode::ArgumentTypeChanged,
        CompatibilityImpact::Breaking,
        "function.set.argument.amount.type",
    );
}

#[test]
fn argument_addition_and_removal_are_breaking() {
    let old = interface_with_function(function("set", vec![input("name", "Symbol")], vec![]));
    let new = interface_with_function(function(
        "set",
        vec![input("name", "Symbol"), input("amount", "u32")],
        vec![],
    ));

    let added = compare_interfaces(&old, &new);
    let removed = compare_interfaces(&new, &old);

    assert_finding(
        &added,
        FindingCode::ArgumentAdded,
        CompatibilityImpact::Breaking,
        "function.set.argument.amount",
    );
    assert_finding(
        &removed,
        FindingCode::ArgumentRemoved,
        CompatibilityImpact::Breaking,
        "function.set.argument.amount",
    );
}

#[test]
fn argument_rename_is_warning() {
    let old = interface_with_function(function("set", vec![input("name", "Symbol")], vec![]));
    let new = interface_with_function(function("set", vec![input("label", "Symbol")], vec![]));

    let report = compare_interfaces(&old, &new);

    assert_finding(
        &report,
        FindingCode::ArgumentRenamed,
        CompatibilityImpact::Warning,
        "function.set.argument.0.name",
    );
    assert_eq!(report.result, CompatibilityResult::Warning);
}

#[test]
fn argument_reorder_is_breaking() {
    let old = interface_with_function(function(
        "transfer",
        vec![input("from", "Address"), input("to", "Address")],
        vec![],
    ));
    let new = interface_with_function(function(
        "transfer",
        vec![input("to", "Address"), input("from", "Address")],
        vec![],
    ));

    let report = compare_interfaces(&old, &new);

    assert_finding(
        &report,
        FindingCode::ArgumentReordered,
        CompatibilityImpact::Breaking,
        "function.transfer.arguments",
    );
}

#[test]
fn return_type_change_is_breaking() {
    let old = interface_with_function(function("balance", vec![], vec![ty("i128")]));
    let new = interface_with_function(function("balance", vec![], vec![ty("u64")]));

    let report = compare_interfaces(&old, &new);

    assert_finding(
        &report,
        FindingCode::ReturnTypeChanged,
        CompatibilityImpact::Breaking,
        "function.balance.return",
    );
}

#[test]
fn multiple_return_value_shape_is_compared_if_present() {
    let old = interface_with_function(function("pair", vec![], vec![ty("u32"), ty("Symbol")]));
    let new = interface_with_function(function("pair", vec![], vec![ty("u64"), ty("Symbol")]));

    let report = compare_interfaces(&old, &new);

    assert_finding(
        &report,
        FindingCode::ReturnTypeChanged,
        CompatibilityImpact::Breaking,
        "function.pair.return",
    );
}

#[test]
fn function_declaration_order_only_produces_no_findings() {
    let mut old = empty_interface();
    old.functions.push(function("b", vec![], vec![]));
    old.functions.push(function("a", vec![], vec![]));
    let mut new = empty_interface();
    new.functions.push(function("a", vec![], vec![]));
    new.functions.push(function("b", vec![], vec![]));

    let report = compare_interfaces(&old, &new);

    assert!(report.findings.is_empty());
}

#[test]
fn nested_composite_type_change_is_detected() {
    let old = interface_with_function(function(
        "set",
        vec![input("value", "Option<u32>")],
        vec![ty("Map<Symbol, Vec<(Address, BytesN<32>)>>")],
    ));
    let new = interface_with_function(function(
        "set",
        vec![input("value", "Option<u64>")],
        vec![ty("Map<Symbol, Vec<(Address, BytesN<32>)>>")],
    ));

    let report = compare_interfaces(&old, &new);

    assert_finding(
        &report,
        FindingCode::ArgumentTypeChanged,
        CompatibilityImpact::Breaking,
        "function.set.argument.value.type",
    );
}

#[test]
fn struct_field_type_change_is_breaking() {
    let mut old = empty_interface();
    old.structs
        .push(struct_spec("Account", vec![field("id", "u32")]));
    let mut new = empty_interface();
    new.structs
        .push(struct_spec("Account", vec![field("id", "u64")]));

    let report = compare_interfaces(&old, &new);

    assert_finding(
        &report,
        FindingCode::StructFieldTypeChanged,
        CompatibilityImpact::Breaking,
        "struct.Account.field.id.type",
    );
}

#[test]
fn struct_field_addition_is_unknown() {
    let mut old = empty_interface();
    old.structs
        .push(struct_spec("Account", vec![field("id", "u32")]));
    let mut new = empty_interface();
    new.structs.push(struct_spec(
        "Account",
        vec![field("id", "u32"), field("status", "u32")],
    ));

    let report = compare_interfaces(&old, &new);

    assert_finding(
        &report,
        FindingCode::StructFieldAdded,
        CompatibilityImpact::Unknown,
        "struct.Account.field.status",
    );
    assert_eq!(report.result, CompatibilityResult::Unknown);
}

#[test]
fn struct_add_remove_and_field_removal_are_classified_conservatively() {
    let mut old = empty_interface();
    old.structs
        .push(struct_spec("Account", vec![field("id", "u32")]));
    let new = empty_interface();

    let removed = compare_interfaces(&old, &new);
    let added = compare_interfaces(&new, &old);

    assert_finding(
        &removed,
        FindingCode::StructRemoved,
        CompatibilityImpact::Breaking,
        "struct.Account",
    );
    assert_finding(
        &added,
        FindingCode::StructAdded,
        CompatibilityImpact::Warning,
        "struct.Account",
    );

    let mut new_without_field = empty_interface();
    new_without_field
        .structs
        .push(struct_spec("Account", vec![]));
    let field_removed = compare_interfaces(&old, &new_without_field);
    assert_finding(
        &field_removed,
        FindingCode::StructFieldRemoved,
        CompatibilityImpact::Unknown,
        "struct.Account.field.id",
    );
}

#[test]
fn struct_field_rename_and_reorder_are_unknown() {
    let mut old = empty_interface();
    old.structs.push(struct_spec(
        "Account",
        vec![field("id", "u32"), field("status", "u32")],
    ));
    let mut renamed = empty_interface();
    renamed.structs.push(struct_spec(
        "Account",
        vec![field("account_id", "u32"), field("status", "u32")],
    ));
    let mut reordered = empty_interface();
    reordered.structs.push(struct_spec(
        "Account",
        vec![field("status", "u32"), field("id", "u32")],
    ));

    assert_finding(
        &compare_interfaces(&old, &renamed),
        FindingCode::StructFieldRenamed,
        CompatibilityImpact::Unknown,
        "struct.Account.field.0.name",
    );
    assert_finding(
        &compare_interfaces(&old, &reordered),
        FindingCode::StructFieldReordered,
        CompatibilityImpact::Unknown,
        "struct.Account.fields",
    );
}

#[test]
fn enum_and_error_changes_are_detected() {
    let mut old = empty_interface();
    old.enums.push(EnumSpec {
        name: "Status".to_owned(),
        cases: vec![EnumCase {
            name: "Open".to_owned(),
            value: 1,
        }],
    });
    old.errors.push(ErrorSpec {
        name: "ContractError".to_owned(),
        cases: vec![ErrorCase {
            name: "Unauthorized".to_owned(),
            value: 1,
        }],
    });
    let mut new = empty_interface();
    new.enums.push(EnumSpec {
        name: "Status".to_owned(),
        cases: vec![EnumCase {
            name: "Open".to_owned(),
            value: 2,
        }],
    });
    new.errors.push(ErrorSpec {
        name: "ContractError".to_owned(),
        cases: vec![ErrorCase {
            name: "Unauthorized".to_owned(),
            value: 7,
        }],
    });

    let report = compare_interfaces(&old, &new);

    assert_finding(
        &report,
        FindingCode::EnumCaseValueChanged,
        CompatibilityImpact::Breaking,
        "enum.Status.case.Open.value",
    );
    assert_finding(
        &report,
        FindingCode::ErrorCodeChanged,
        CompatibilityImpact::Breaking,
        "error.ContractError.case.Unauthorized.code",
    );
}

#[test]
fn enum_order_only_produces_no_findings_and_case_addition_is_unknown() {
    let mut old = empty_interface();
    old.enums.push(EnumSpec {
        name: "Status".to_owned(),
        cases: vec![
            EnumCase {
                name: "Open".to_owned(),
                value: 1,
            },
            EnumCase {
                name: "Closed".to_owned(),
                value: 2,
            },
        ],
    });
    let mut reordered = empty_interface();
    reordered.enums.push(EnumSpec {
        name: "Status".to_owned(),
        cases: vec![
            EnumCase {
                name: "Closed".to_owned(),
                value: 2,
            },
            EnumCase {
                name: "Open".to_owned(),
                value: 1,
            },
        ],
    });
    let mut added = old.clone();
    added.enums[0].cases.push(EnumCase {
        name: "Paused".to_owned(),
        value: 3,
    });

    assert!(compare_interfaces(&old, &reordered).findings.is_empty());
    assert_finding(
        &compare_interfaces(&old, &added),
        FindingCode::EnumCaseAdded,
        CompatibilityImpact::Unknown,
        "enum.Status.case.Paused",
    );
}

#[test]
fn union_and_event_uncertain_changes_are_unknown() {
    let mut old = empty_interface();
    old.unions.push(UnionSpec {
        name: "Value".to_owned(),
        cases: vec![UnionCase {
            name: "Int".to_owned(),
            values: vec![ty("u32")],
        }],
    });
    old.events.push(EventSpec {
        name: "changed".to_owned(),
        parameters: vec![EventParam {
            name: "id".to_owned(),
            type_ref: ty("u32"),
            location: "topic_list".to_owned(),
        }],
        data_format: "single_value".to_owned(),
    });
    let mut new = old.clone();
    new.unions[0].cases.push(UnionCase {
        name: "Text".to_owned(),
        values: vec![ty("String")],
    });
    new.events[0].parameters[0].type_ref = ty("u64");
    new.events[0].data_format = "vec".to_owned();

    let report = compare_interfaces(&old, &new);

    assert_finding(
        &report,
        FindingCode::UnionCaseAdded,
        CompatibilityImpact::Unknown,
        "union.Value.case.Text",
    );
    assert_finding(
        &report,
        FindingCode::EventParametersChanged,
        CompatibilityImpact::Unknown,
        "event.changed.parameters",
    );
    assert_finding(
        &report,
        FindingCode::EventDataFormatChanged,
        CompatibilityImpact::Unknown,
        "event.changed.data_format",
    );
}

#[test]
fn duplicate_finding_for_rename_plus_type_change_is_deliberate() {
    let old = interface_with_function(function("foo", vec![input("a", "u32")], vec![]));
    let new = interface_with_function(function("foo", vec![input("b", "u64")], vec![]));

    let report = compare_interfaces(&old, &new);

    assert_eq!(report.findings.len(), 2);
    assert_finding(
        &report,
        FindingCode::ArgumentRemoved,
        CompatibilityImpact::Breaking,
        "function.foo.argument.a",
    );
    assert_finding(
        &report,
        FindingCode::ArgumentAdded,
        CompatibilityImpact::Breaking,
        "function.foo.argument.b",
    );
}

#[test]
fn findings_order_is_deterministic() {
    let mut old = empty_interface();
    old.functions.push(function("z_removed", vec![], vec![]));
    old.functions
        .push(function("a_changed", vec![], vec![ty("u32")]));
    let mut new = empty_interface();
    new.functions
        .push(function("a_changed", vec![], vec![ty("u64")]));
    new.functions.push(function("compatible", vec![], vec![]));

    let report = compare_interfaces(&old, &new);
    let paths = report
        .findings
        .iter()
        .map(|finding| finding.path.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        paths,
        vec![
            "function.z_removed",
            "function.a_changed.return",
            "function.compatible",
        ]
    );
}

#[test]
fn repeated_comparisons_and_json_are_deterministic() {
    let mut old = empty_interface();
    old.functions.push(function("z_removed", vec![], vec![]));
    old.functions
        .push(function("a_changed", vec![input("value", "u32")], vec![]));
    let mut new = empty_interface();
    new.functions
        .push(function("a_changed", vec![input("value", "u64")], vec![]));
    new.functions.push(function("compatible", vec![], vec![]));

    let first = compare_interfaces(&old, &new);
    let second = compare_interfaces(&old, &new);

    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_string(&first).expect("report serializes"),
        serde_json::to_string(&second).expect("report serializes")
    );
}

#[test]
fn old_new_directionality_is_preserved() {
    let mut old = empty_interface();
    old.functions.push(function("foo", vec![], vec![]));
    let new = empty_interface();

    assert_finding(
        &compare_interfaces(&old, &new),
        FindingCode::FunctionRemoved,
        CompatibilityImpact::Breaking,
        "function.foo",
    );
    assert_finding(
        &compare_interfaces(&new, &old),
        FindingCode::FunctionAdded,
        CompatibilityImpact::Compatible,
        "function.foo",
    );
}

#[test]
fn real_wasm_function_addition_is_compatible() {
    let old = include_bytes!("../../../fixtures/phase2/compatible-function-added/old.wasm");
    let new = include_bytes!("../../../fixtures/phase2/compatible-function-added/new.wasm");

    let report = compare_wasm(old, new).expect("real wasm fixtures compare");

    assert_finding(
        &report,
        FindingCode::FunctionAdded,
        CompatibilityImpact::Compatible,
        "function.goodbye",
    );
    assert_eq!(report.result, CompatibilityResult::Compatible);
}

#[test]
fn real_wasm_identical_contract_has_no_findings() {
    let wasm = include_bytes!("../../../fixtures/phase2/compatible-function-added/old.wasm");

    let report = compare_wasm(wasm, wasm).expect("real wasm fixture compares with itself");

    assert_eq!(report.result, CompatibilityResult::Compatible);
    assert_eq!(report.summary.compatible, 0);
    assert_eq!(report.summary.warning, 0);
    assert_eq!(report.summary.breaking, 0);
    assert_eq!(report.summary.unknown, 0);
    assert!(report.findings.is_empty());
}

#[test]
fn multiple_changes_report_counts_and_order_are_stable() {
    let mut old = empty_interface();
    old.functions.push(function("removed", vec![], vec![]));
    old.functions
        .push(function("changed", vec![input("value", "u32")], vec![]));
    old.enums.push(EnumSpec {
        name: "Status".to_owned(),
        cases: vec![EnumCase {
            name: "Open".to_owned(),
            value: 1,
        }],
    });
    let mut new = empty_interface();
    new.functions.push(function("added", vec![], vec![]));
    new.functions
        .push(function("changed", vec![input("value", "u64")], vec![]));
    new.enums.push(EnumSpec {
        name: "Status".to_owned(),
        cases: vec![EnumCase {
            name: "Open".to_owned(),
            value: 2,
        }],
    });

    let report = compare_interfaces(&old, &new);
    let codes = report
        .findings
        .iter()
        .map(|finding| finding.code)
        .collect::<Vec<_>>();

    assert_eq!(report.result, CompatibilityResult::Breaking);
    assert_eq!(report.summary.breaking, 3);
    assert_eq!(report.summary.compatible, 1);
    assert_eq!(
        codes,
        vec![
            FindingCode::FunctionRemoved,
            FindingCode::ArgumentTypeChanged,
            FindingCode::EnumCaseValueChanged,
            FindingCode::FunctionAdded,
        ]
    );
}

#[test]
fn real_wasm_function_removal_is_breaking() {
    let old = include_bytes!("../../../fixtures/phase2/breaking-function-removed/old.wasm");
    let new = include_bytes!("../../../fixtures/phase2/breaking-function-removed/new.wasm");

    let report = compare_wasm(old, new).expect("real wasm fixtures compare");

    assert_finding(
        &report,
        FindingCode::FunctionRemoved,
        CompatibilityImpact::Breaking,
        "function.removed",
    );
}

#[test]
fn real_wasm_argument_type_change_is_breaking() {
    let old = include_bytes!("../../../fixtures/phase2/breaking-argument-type-changed/old.wasm");
    let new = include_bytes!("../../../fixtures/phase2/breaking-argument-type-changed/new.wasm");

    let report = compare_wasm(old, new).expect("real wasm fixtures compare");

    assert_finding(
        &report,
        FindingCode::ArgumentTypeChanged,
        CompatibilityImpact::Breaking,
        "function.set.argument.value.type",
    );
}

#[test]
fn real_wasm_return_type_change_is_breaking() {
    let old = include_bytes!("../../../fixtures/phase2/breaking-return-type-changed/old.wasm");
    let new = include_bytes!("../../../fixtures/phase2/breaking-return-type-changed/new.wasm");

    let report = compare_wasm(old, new).expect("real wasm fixtures compare");

    assert_finding(
        &report,
        FindingCode::ReturnTypeChanged,
        CompatibilityImpact::Breaking,
        "function.value.return",
    );
}

#[test]
fn real_wasm_struct_field_change_is_breaking() {
    let old = include_bytes!("../../../fixtures/phase2/struct-field-changed/old.wasm");
    let new = include_bytes!("../../../fixtures/phase2/struct-field-changed/new.wasm");

    let report = compare_wasm(old, new).expect("real wasm fixtures compare");

    assert_finding(
        &report,
        FindingCode::StructFieldTypeChanged,
        CompatibilityImpact::Breaking,
        "struct.Account.field.id.type",
    );
}

fn assert_finding(
    report: &guard_core::CompatibilityReport,
    code: FindingCode,
    impact: CompatibilityImpact,
    path: &str,
) {
    assert!(
        report.findings.iter().any(|finding| finding.code == code
            && finding.impact == impact
            && finding.path == path),
        "missing finding {code:?} {impact:?} {path}; findings: {:#?}",
        report.findings
    );
}

fn base_interface() -> ContractInterface {
    interface_with_function(function(
        "hello",
        vec![input("name", "Symbol")],
        vec![ty("Symbol")],
    ))
}

fn interface_with_function(function: FunctionSpec) -> ContractInterface {
    let mut interface = empty_interface();
    interface.functions.push(function);
    interface
}

fn empty_interface() -> ContractInterface {
    ContractInterface {
        functions: vec![],
        structs: vec![],
        enums: vec![],
        unions: vec![],
        errors: vec![],
        events: vec![],
        env_meta: guard_core::EnvMeta::Absent,
    }
}

fn function(name: &str, inputs: Vec<FunctionInput>, outputs: Vec<TypeRef>) -> FunctionSpec {
    FunctionSpec {
        name: name.to_owned(),
        inputs,
        outputs,
    }
}

fn input(name: &str, type_name: &str) -> FunctionInput {
    FunctionInput {
        name: name.to_owned(),
        type_ref: ty(type_name),
    }
}

fn struct_spec(name: &str, fields: Vec<StructField>) -> StructSpec {
    StructSpec {
        name: name.to_owned(),
        fields,
    }
}

fn field(name: &str, type_name: &str) -> StructField {
    StructField {
        name: name.to_owned(),
        type_ref: ty(type_name),
    }
}

fn ty(type_name: &str) -> TypeRef {
    TypeRef(type_name.to_owned())
}
