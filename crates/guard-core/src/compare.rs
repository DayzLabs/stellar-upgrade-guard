use std::collections::{BTreeMap, BTreeSet};

use crate::{
    error::GuardError,
    model::{
        CompatibilityFinding, CompatibilityImpact, CompatibilityReport, ContractInterface,
        EnumCase, EnvMeta, ErrorCase, EventParam, FindingCode, FunctionInput, FunctionSpec,
        StructField, TypeRef, UnionCase,
    },
    parse_contract_interface,
};

pub fn compare_wasm(old: &[u8], new: &[u8]) -> Result<CompatibilityReport, GuardError> {
    let old = parse_contract_interface(old)?;
    let new = parse_contract_interface(new)?;
    Ok(compare_interfaces(&old, &new))
}

pub fn compare_interfaces(old: &ContractInterface, new: &ContractInterface) -> CompatibilityReport {
    let mut findings = Vec::new();

    compare_functions(old, new, &mut findings);
    compare_structs(old, new, &mut findings);
    compare_enums(old, new, &mut findings);
    compare_errors(old, new, &mut findings);
    compare_unions(old, new, &mut findings);
    compare_events(old, new, &mut findings);
    compare_env_meta(old, new, &mut findings);

    findings.sort_by(|left, right| {
        impact_rank(left.impact)
            .cmp(&impact_rank(right.impact))
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.message.cmp(&right.message))
    });

    CompatibilityReport::new(findings)
}

fn compare_functions(
    old: &ContractInterface,
    new: &ContractInterface,
    findings: &mut Vec<CompatibilityFinding>,
) {
    let old_functions = by_name(&old.functions, |function| &function.name);
    let new_functions = by_name(&new.functions, |function| &function.name);

    for name in old_functions.keys() {
        if !new_functions.contains_key(name) {
            findings.push(finding(
                FindingCode::FunctionRemoved,
                CompatibilityImpact::Breaking,
                format!("function.{name}"),
                format!("Function `{name}` removed"),
                Some(name.to_string()),
                None,
            ));
        }
    }

    for name in new_functions.keys() {
        if !old_functions.contains_key(name) {
            findings.push(finding(
                FindingCode::FunctionAdded,
                CompatibilityImpact::Compatible,
                format!("function.{name}"),
                format!("Function `{name}` added"),
                None,
                Some(name.to_string()),
            ));
        }
    }

    for (name, old_function) in old_functions {
        let Some(new_function) = new_functions.get(name) else {
            continue;
        };
        compare_function_inputs(name, old_function, new_function, findings);
        if old_function.outputs != new_function.outputs {
            findings.push(finding(
                FindingCode::ReturnTypeChanged,
                CompatibilityImpact::Breaking,
                format!("function.{name}.return"),
                format!(
                    "Function `{name}` return type changed: {} -> {}",
                    format_types(&old_function.outputs),
                    format_types(&new_function.outputs)
                ),
                Some(format_types(&old_function.outputs)),
                Some(format_types(&new_function.outputs)),
            ));
        }
    }
}

fn compare_function_inputs(
    function_name: &str,
    old_function: &FunctionSpec,
    new_function: &FunctionSpec,
    findings: &mut Vec<CompatibilityFinding>,
) {
    if old_function.inputs.len() == new_function.inputs.len()
        && old_function.inputs != new_function.inputs
        && sorted_values(&old_function.inputs) == sorted_values(&new_function.inputs)
    {
        findings.push(finding(
            FindingCode::ArgumentReordered,
            CompatibilityImpact::Breaking,
            format!("function.{function_name}.arguments"),
            format!("Function `{function_name}` arguments reordered"),
            Some(format_inputs(&old_function.inputs)),
            Some(format_inputs(&new_function.inputs)),
        ));
        return;
    }

    if old_function.inputs.len() == new_function.inputs.len()
        && old_function
            .inputs
            .iter()
            .zip(new_function.inputs.iter())
            .all(|(old_input, new_input)| old_input.type_ref == new_input.type_ref)
    {
        for (index, (old_input, new_input)) in old_function
            .inputs
            .iter()
            .zip(new_function.inputs.iter())
            .enumerate()
        {
            if old_input.name != new_input.name {
                findings.push(finding(
                    FindingCode::ArgumentRenamed,
                    CompatibilityImpact::Warning,
                    format!("function.{function_name}.argument.{index}.name"),
                    format!(
                        "Function `{function_name}` argument renamed: `{}` -> `{}`",
                        old_input.name, new_input.name
                    ),
                    Some(old_input.name.clone()),
                    Some(new_input.name.clone()),
                ));
            }
        }
        return;
    }

    let old_inputs = by_name(&old_function.inputs, |input| &input.name);
    let new_inputs = by_name(&new_function.inputs, |input| &input.name);

    for name in old_inputs.keys() {
        if !new_inputs.contains_key(name) {
            findings.push(finding(
                FindingCode::ArgumentRemoved,
                CompatibilityImpact::Breaking,
                format!("function.{function_name}.argument.{name}"),
                format!("Function `{function_name}` argument `{name}` removed"),
                Some(name.to_string()),
                None,
            ));
        }
    }

    for name in new_inputs.keys() {
        if !old_inputs.contains_key(name) {
            findings.push(finding(
                FindingCode::ArgumentAdded,
                CompatibilityImpact::Breaking,
                format!("function.{function_name}.argument.{name}"),
                format!("Function `{function_name}` argument `{name}` added"),
                None,
                Some(name.to_string()),
            ));
        }
    }

    for (name, old_input) in old_inputs {
        let Some(new_input) = new_inputs.get(name) else {
            continue;
        };
        if old_input.type_ref != new_input.type_ref {
            findings.push(finding(
                FindingCode::ArgumentTypeChanged,
                CompatibilityImpact::Breaking,
                format!("function.{function_name}.argument.{name}.type"),
                format!(
                    "Function `{function_name}` argument `{name}` type changed: {} -> {}",
                    old_input.type_ref.0, new_input.type_ref.0
                ),
                Some(old_input.type_ref.0.clone()),
                Some(new_input.type_ref.0.clone()),
            ));
        }
    }
}

fn compare_structs(
    old: &ContractInterface,
    new: &ContractInterface,
    findings: &mut Vec<CompatibilityFinding>,
) {
    let old_structs = by_name(&old.structs, |item| &item.name);
    let new_structs = by_name(&new.structs, |item| &item.name);

    compare_named_items(
        &old_structs,
        &new_structs,
        findings,
        NamedItemRules {
            added: FindingCode::StructAdded,
            removed: FindingCode::StructRemoved,
            added_impact: CompatibilityImpact::Warning,
            removed_impact: CompatibilityImpact::Breaking,
            kind: "Struct",
            path_kind: "struct",
        },
    );

    for (name, old_struct) in old_structs {
        let Some(new_struct) = new_structs.get(name) else {
            continue;
        };
        compare_fields(name, &old_struct.fields, &new_struct.fields, findings);
    }
}

fn compare_fields(
    struct_name: &str,
    old_fields: &[StructField],
    new_fields: &[StructField],
    findings: &mut Vec<CompatibilityFinding>,
) {
    if old_fields.len() == new_fields.len()
        && old_fields != new_fields
        && sorted_values(old_fields) == sorted_values(new_fields)
    {
        findings.push(finding(
            FindingCode::StructFieldReordered,
            CompatibilityImpact::Unknown,
            format!("struct.{struct_name}.fields"),
            format!("Struct `{struct_name}` fields reordered"),
            Some(format_fields(old_fields)),
            Some(format_fields(new_fields)),
        ));
        return;
    }

    let old_by_name = by_name(old_fields, |field| &field.name);
    let new_by_name = by_name(new_fields, |field| &field.name);

    for name in old_by_name.keys() {
        if !new_by_name.contains_key(name) {
            findings.push(finding(
                FindingCode::StructFieldRemoved,
                CompatibilityImpact::Unknown,
                format!("struct.{struct_name}.field.{name}"),
                format!("Struct `{struct_name}` field `{name}` removed"),
                Some(name.to_string()),
                None,
            ));
        }
    }

    for name in new_by_name.keys() {
        if !old_by_name.contains_key(name) {
            findings.push(finding(
                FindingCode::StructFieldAdded,
                CompatibilityImpact::Unknown,
                format!("struct.{struct_name}.field.{name}"),
                format!("Struct `{struct_name}` field `{name}` added"),
                None,
                Some(name.to_string()),
            ));
        }
    }

    for (name, old_field) in old_by_name {
        let Some(new_field) = new_by_name.get(name) else {
            continue;
        };
        if old_field.type_ref != new_field.type_ref {
            findings.push(finding(
                FindingCode::StructFieldTypeChanged,
                CompatibilityImpact::Breaking,
                format!("struct.{struct_name}.field.{name}.type"),
                format!(
                    "Struct `{struct_name}` field `{name}` type changed: {} -> {}",
                    old_field.type_ref.0, new_field.type_ref.0
                ),
                Some(old_field.type_ref.0.clone()),
                Some(new_field.type_ref.0.clone()),
            ));
        }
    }

    for (index, (old_field, new_field)) in old_fields.iter().zip(new_fields.iter()).enumerate() {
        if old_field.name != new_field.name && old_field.type_ref == new_field.type_ref {
            findings.push(finding(
                FindingCode::StructFieldRenamed,
                CompatibilityImpact::Unknown,
                format!("struct.{struct_name}.field.{index}.name"),
                format!(
                    "Struct `{struct_name}` field renamed: `{}` -> `{}`",
                    old_field.name, new_field.name
                ),
                Some(old_field.name.clone()),
                Some(new_field.name.clone()),
            ));
        }
    }
}

fn compare_enums(
    old: &ContractInterface,
    new: &ContractInterface,
    findings: &mut Vec<CompatibilityFinding>,
) {
    let old_enums = by_name(&old.enums, |item| &item.name);
    let new_enums = by_name(&new.enums, |item| &item.name);

    compare_named_items(
        &old_enums,
        &new_enums,
        findings,
        NamedItemRules {
            added: FindingCode::EnumAdded,
            removed: FindingCode::EnumRemoved,
            added_impact: CompatibilityImpact::Warning,
            removed_impact: CompatibilityImpact::Breaking,
            kind: "Enum",
            path_kind: "enum",
        },
    );

    for (name, old_enum) in old_enums {
        let Some(new_enum) = new_enums.get(name) else {
            continue;
        };
        compare_enum_cases(name, &old_enum.cases, &new_enum.cases, findings);
    }
}

fn compare_enum_cases(
    enum_name: &str,
    old_cases: &[EnumCase],
    new_cases: &[EnumCase],
    findings: &mut Vec<CompatibilityFinding>,
) {
    let old_by_name = by_name(old_cases, |case| &case.name);
    let new_by_name = by_name(new_cases, |case| &case.name);

    for name in old_by_name.keys() {
        if !new_by_name.contains_key(name) {
            findings.push(finding(
                FindingCode::EnumCaseRemoved,
                CompatibilityImpact::Breaking,
                format!("enum.{enum_name}.case.{name}"),
                format!("Enum `{enum_name}` case `{name}` removed"),
                Some(name.to_string()),
                None,
            ));
        }
    }

    for name in new_by_name.keys() {
        if !old_by_name.contains_key(name) {
            findings.push(finding(
                FindingCode::EnumCaseAdded,
                CompatibilityImpact::Unknown,
                format!("enum.{enum_name}.case.{name}"),
                format!("Enum `{enum_name}` case `{name}` added"),
                None,
                Some(name.to_string()),
            ));
        }
    }

    for (name, old_case) in old_by_name {
        let Some(new_case) = new_by_name.get(name) else {
            continue;
        };
        if old_case.value != new_case.value {
            findings.push(finding(
                FindingCode::EnumCaseValueChanged,
                CompatibilityImpact::Breaking,
                format!("enum.{enum_name}.case.{name}.value"),
                format!(
                    "Enum `{enum_name}` case `{name}` value changed: {} -> {}",
                    old_case.value, new_case.value
                ),
                Some(old_case.value.to_string()),
                Some(new_case.value.to_string()),
            ));
        }
    }
}

fn compare_errors(
    old: &ContractInterface,
    new: &ContractInterface,
    findings: &mut Vec<CompatibilityFinding>,
) {
    let old_errors = by_name(&old.errors, |item| &item.name);
    let new_errors = by_name(&new.errors, |item| &item.name);

    compare_named_items(
        &old_errors,
        &new_errors,
        findings,
        NamedItemRules {
            added: FindingCode::ErrorAdded,
            removed: FindingCode::ErrorRemoved,
            added_impact: CompatibilityImpact::Warning,
            removed_impact: CompatibilityImpact::Warning,
            kind: "Error enum",
            path_kind: "error",
        },
    );

    for (name, old_error) in old_errors {
        let Some(new_error) = new_errors.get(name) else {
            continue;
        };
        compare_error_cases(name, &old_error.cases, &new_error.cases, findings);
    }
}

fn compare_error_cases(
    error_name: &str,
    old_cases: &[ErrorCase],
    new_cases: &[ErrorCase],
    findings: &mut Vec<CompatibilityFinding>,
) {
    let old_by_name = by_name(old_cases, |case| &case.name);
    let new_by_name = by_name(new_cases, |case| &case.name);

    for name in old_by_name.keys() {
        if !new_by_name.contains_key(name) {
            findings.push(finding(
                FindingCode::ErrorCaseRemoved,
                CompatibilityImpact::Unknown,
                format!("error.{error_name}.case.{name}"),
                format!("Error enum `{error_name}` case `{name}` removed"),
                Some(name.to_string()),
                None,
            ));
        }
    }

    for name in new_by_name.keys() {
        if !old_by_name.contains_key(name) {
            findings.push(finding(
                FindingCode::ErrorCaseAdded,
                CompatibilityImpact::Warning,
                format!("error.{error_name}.case.{name}"),
                format!("Error enum `{error_name}` case `{name}` added"),
                None,
                Some(name.to_string()),
            ));
        }
    }

    for (name, old_case) in old_by_name {
        let Some(new_case) = new_by_name.get(name) else {
            continue;
        };
        if old_case.value != new_case.value {
            findings.push(finding(
                FindingCode::ErrorCodeChanged,
                CompatibilityImpact::Breaking,
                format!("error.{error_name}.case.{name}.code"),
                format!(
                    "Error enum `{error_name}` case `{name}` code changed: {} -> {}",
                    old_case.value, new_case.value
                ),
                Some(old_case.value.to_string()),
                Some(new_case.value.to_string()),
            ));
        }
    }
}

fn compare_unions(
    old: &ContractInterface,
    new: &ContractInterface,
    findings: &mut Vec<CompatibilityFinding>,
) {
    let old_unions = by_name(&old.unions, |item| &item.name);
    let new_unions = by_name(&new.unions, |item| &item.name);

    compare_named_items(
        &old_unions,
        &new_unions,
        findings,
        NamedItemRules {
            added: FindingCode::UnionAdded,
            removed: FindingCode::UnionRemoved,
            added_impact: CompatibilityImpact::Warning,
            removed_impact: CompatibilityImpact::Breaking,
            kind: "Union",
            path_kind: "union",
        },
    );

    for (name, old_union) in old_unions {
        let Some(new_union) = new_unions.get(name) else {
            continue;
        };
        compare_union_cases(name, &old_union.cases, &new_union.cases, findings);
    }
}

fn compare_union_cases(
    union_name: &str,
    old_cases: &[UnionCase],
    new_cases: &[UnionCase],
    findings: &mut Vec<CompatibilityFinding>,
) {
    let old_by_name = by_name(old_cases, |case| &case.name);
    let new_by_name = by_name(new_cases, |case| &case.name);

    for name in old_by_name.keys() {
        if !new_by_name.contains_key(name) {
            findings.push(finding(
                FindingCode::UnionCaseRemoved,
                CompatibilityImpact::Breaking,
                format!("union.{union_name}.case.{name}"),
                format!("Union `{union_name}` case `{name}` removed"),
                Some(name.to_string()),
                None,
            ));
        }
    }

    for name in new_by_name.keys() {
        if !old_by_name.contains_key(name) {
            findings.push(finding(
                FindingCode::UnionCaseAdded,
                CompatibilityImpact::Unknown,
                format!("union.{union_name}.case.{name}"),
                format!("Union `{union_name}` case `{name}` added"),
                None,
                Some(name.to_string()),
            ));
        }
    }

    for (name, old_case) in old_by_name {
        let Some(new_case) = new_by_name.get(name) else {
            continue;
        };
        if old_case.values != new_case.values {
            findings.push(finding(
                FindingCode::UnionCasePayloadChanged,
                CompatibilityImpact::Breaking,
                format!("union.{union_name}.case.{name}.payload"),
                format!("Union `{union_name}` case `{name}` payload changed"),
                Some(format_types(&old_case.values)),
                Some(format_types(&new_case.values)),
            ));
        }
    }
}

fn compare_events(
    old: &ContractInterface,
    new: &ContractInterface,
    findings: &mut Vec<CompatibilityFinding>,
) {
    let old_events = by_name(&old.events, |item| &item.name);
    let new_events = by_name(&new.events, |item| &item.name);

    compare_named_items(
        &old_events,
        &new_events,
        findings,
        NamedItemRules {
            added: FindingCode::EventAdded,
            removed: FindingCode::EventRemoved,
            added_impact: CompatibilityImpact::Unknown,
            removed_impact: CompatibilityImpact::Unknown,
            kind: "Event",
            path_kind: "event",
        },
    );

    for (name, old_event) in old_events {
        let Some(new_event) = new_events.get(name) else {
            continue;
        };
        if old_event.data_format != new_event.data_format {
            findings.push(finding(
                FindingCode::EventDataFormatChanged,
                CompatibilityImpact::Unknown,
                format!("event.{name}.data_format"),
                format!("Event `{name}` data format changed"),
                Some(old_event.data_format.clone()),
                Some(new_event.data_format.clone()),
            ));
        }
        compare_event_params(name, &old_event.parameters, &new_event.parameters, findings);
    }
}

fn compare_event_params(
    event_name: &str,
    old_params: &[EventParam],
    new_params: &[EventParam],
    findings: &mut Vec<CompatibilityFinding>,
) {
    if old_params != new_params {
        findings.push(finding(
            FindingCode::EventParametersChanged,
            CompatibilityImpact::Unknown,
            format!("event.{event_name}.parameters"),
            format!("Event `{event_name}` parameters changed"),
            Some(format_event_params(old_params)),
            Some(format_event_params(new_params)),
        ));
    }
}

/// Compares `contractenvmetav0`.
///
/// The interface version names the host environment a contract was built against. Whether a
/// given version runs on a given network depends on the network's protocol, which this tool
/// does not know, so every difference is `Unknown` rather than a guess at compatibility.
fn compare_env_meta(
    old: &ContractInterface,
    new: &ContractInterface,
    findings: &mut Vec<CompatibilityFinding>,
) {
    match (old.env_meta, new.env_meta) {
        (EnvMeta::Absent, EnvMeta::Absent) => {}
        (EnvMeta::InterfaceVersion(old_version), EnvMeta::InterfaceVersion(new_version)) => {
            if old_version != new_version {
                findings.push(finding(
                    FindingCode::EnvInterfaceVersionChanged,
                    CompatibilityImpact::Unknown,
                    "env.interface_version".to_owned(),
                    format!("Env interface version changed: {old_version} -> {new_version}"),
                    Some(old_version.to_string()),
                    Some(new_version.to_string()),
                ));
            }
        }
        (old_meta, new_meta) => {
            let (old_state, new_state) = (describe_env_meta(old_meta), describe_env_meta(new_meta));
            findings.push(finding(
                FindingCode::EnvMetaUnavailable,
                CompatibilityImpact::Unknown,
                "env".to_owned(),
                format!("Env metadata cannot be compared: old is {old_state}, new is {new_state}"),
                Some(old_state),
                Some(new_state),
            ));
        }
    }
}

fn describe_env_meta(meta: EnvMeta) -> String {
    match meta {
        EnvMeta::Absent => "absent".to_owned(),
        EnvMeta::InterfaceVersion(version) => version.to_string(),
        EnvMeta::Unreadable => "unreadable".to_owned(),
    }
}

struct NamedItemRules {
    added: FindingCode,
    removed: FindingCode,
    added_impact: CompatibilityImpact,
    removed_impact: CompatibilityImpact,
    kind: &'static str,
    path_kind: &'static str,
}

fn compare_named_items<T>(
    old: &BTreeMap<&String, &T>,
    new: &BTreeMap<&String, &T>,
    findings: &mut Vec<CompatibilityFinding>,
    rules: NamedItemRules,
) {
    for name in old.keys() {
        if !new.contains_key(name) {
            findings.push(finding(
                rules.removed,
                rules.removed_impact,
                format!("{}.{}", rules.path_kind, name),
                format!("{} `{}` removed", rules.kind, name),
                Some((*name).clone()),
                None,
            ));
        }
    }

    for name in new.keys() {
        if !old.contains_key(name) {
            findings.push(finding(
                rules.added,
                rules.added_impact,
                format!("{}.{}", rules.path_kind, name),
                format!("{} `{}` added", rules.kind, name),
                None,
                Some((*name).clone()),
            ));
        }
    }
}

fn by_name<'a, T>(
    items: &'a [T],
    name: impl Fn(&'a T) -> &'a String,
) -> BTreeMap<&'a String, &'a T> {
    items.iter().map(|item| (name(item), item)).collect()
}

fn sorted_values<T>(items: &[T]) -> BTreeSet<T>
where
    T: Clone + Ord,
{
    items.iter().cloned().collect()
}

fn finding(
    code: FindingCode,
    impact: CompatibilityImpact,
    path: String,
    message: String,
    old_value: Option<String>,
    new_value: Option<String>,
) -> CompatibilityFinding {
    CompatibilityFinding {
        code,
        impact,
        path,
        message,
        old_value,
        new_value,
    }
}

fn format_inputs(inputs: &[FunctionInput]) -> String {
    inputs
        .iter()
        .map(|input| format!("{}: {}", input.name, input.type_ref.0))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_fields(fields: &[StructField]) -> String {
    fields
        .iter()
        .map(|field| format!("{}: {}", field.name, field.type_ref.0))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_types(types: &[TypeRef]) -> String {
    if types.is_empty() {
        "void".to_owned()
    } else {
        types
            .iter()
            .map(|type_ref| type_ref.0.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn format_event_params(params: &[EventParam]) -> String {
    params
        .iter()
        .map(|param| format!("{}: {} ({})", param.name, param.type_ref.0, param.location))
        .collect::<Vec<_>>()
        .join(", ")
}

fn impact_rank(impact: CompatibilityImpact) -> u8 {
    match impact {
        CompatibilityImpact::Breaking => 0,
        CompatibilityImpact::Unknown => 1,
        CompatibilityImpact::Warning => 2,
        CompatibilityImpact::Compatible => 3,
    }
}
