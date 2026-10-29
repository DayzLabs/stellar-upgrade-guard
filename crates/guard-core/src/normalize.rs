use stellar_xdr::{
    ScSpecEntry, ScSpecEventDataFormat, ScSpecEventParamLocationV0, ScSpecTypeDef,
    ScSpecUdtUnionCaseV0,
};

use crate::model::{
    ContractInterface, EnumCase, EnumSpec, EnvMeta, ErrorCase, ErrorSpec, EventParam, EventSpec,
    FunctionInput, FunctionSpec, StructField, StructSpec, TypeRef, UnionCase, UnionSpec,
};

pub fn normalize_entries(entries: Vec<ScSpecEntry>) -> ContractInterface {
    let mut interface = ContractInterface {
        functions: Vec::new(),
        structs: Vec::new(),
        enums: Vec::new(),
        unions: Vec::new(),
        errors: Vec::new(),
        events: Vec::new(),
        env_meta: EnvMeta::Absent,
    };

    for entry in entries {
        match entry {
            ScSpecEntry::FunctionV0(function) => {
                interface.functions.push(FunctionSpec {
                    name: function.name.to_string(),
                    inputs: function
                        .inputs
                        .into_iter()
                        .map(|input| FunctionInput {
                            name: input.name.to_string(),
                            type_ref: normalize_type(&input.type_),
                        })
                        .collect(),
                    outputs: function.outputs.iter().map(normalize_type).collect(),
                });
            }
            ScSpecEntry::UdtStructV0(struct_spec) => {
                interface.structs.push(StructSpec {
                    name: struct_spec.name.to_string(),
                    fields: struct_spec
                        .fields
                        .into_iter()
                        .map(|field| StructField {
                            name: field.name.to_string(),
                            type_ref: normalize_type(&field.type_),
                        })
                        .collect(),
                });
            }
            ScSpecEntry::UdtUnionV0(union_spec) => {
                interface.unions.push(UnionSpec {
                    name: union_spec.name.to_string(),
                    cases: union_spec
                        .cases
                        .into_iter()
                        .map(|case| match case {
                            ScSpecUdtUnionCaseV0::VoidV0(case) => UnionCase {
                                name: case.name.to_string(),
                                values: Vec::new(),
                            },
                            ScSpecUdtUnionCaseV0::TupleV0(case) => UnionCase {
                                name: case.name.to_string(),
                                values: case.type_.iter().map(normalize_type).collect(),
                            },
                        })
                        .collect(),
                });
            }
            ScSpecEntry::UdtEnumV0(enum_spec) => {
                interface.enums.push(EnumSpec {
                    name: enum_spec.name.to_string(),
                    cases: enum_spec
                        .cases
                        .into_iter()
                        .map(|case| EnumCase {
                            name: case.name.to_string(),
                            value: case.value,
                        })
                        .collect(),
                });
            }
            ScSpecEntry::UdtErrorEnumV0(error_spec) => {
                interface.errors.push(ErrorSpec {
                    name: error_spec.name.to_string(),
                    cases: error_spec
                        .cases
                        .into_iter()
                        .map(|case| ErrorCase {
                            name: case.name.to_string(),
                            value: case.value,
                        })
                        .collect(),
                });
            }
            ScSpecEntry::EventV0(event_spec) => {
                interface.events.push(EventSpec {
                    name: event_spec.name.to_string(),
                    parameters: event_spec
                        .params
                        .into_iter()
                        .map(|param| EventParam {
                            name: param.name.to_string(),
                            type_ref: normalize_type(&param.type_),
                            location: normalize_event_location(param.location),
                        })
                        .collect(),
                    data_format: normalize_event_data_format(event_spec.data_format),
                });
            }
        }
    }

    interface.functions.sort();
    interface.structs.sort();
    interface.enums.sort();
    interface.unions.sort();
    interface.errors.sort();
    interface.events.sort();
    interface
}

fn normalize_type(type_def: &ScSpecTypeDef) -> TypeRef {
    let text = match type_def {
        ScSpecTypeDef::Val => "Val".to_owned(),
        ScSpecTypeDef::Bool => "bool".to_owned(),
        ScSpecTypeDef::Void => "void".to_owned(),
        ScSpecTypeDef::Error => "Error".to_owned(),
        ScSpecTypeDef::U32 => "u32".to_owned(),
        ScSpecTypeDef::I32 => "i32".to_owned(),
        ScSpecTypeDef::U64 => "u64".to_owned(),
        ScSpecTypeDef::I64 => "i64".to_owned(),
        ScSpecTypeDef::Timepoint => "timepoint".to_owned(),
        ScSpecTypeDef::Duration => "duration".to_owned(),
        ScSpecTypeDef::U128 => "u128".to_owned(),
        ScSpecTypeDef::I128 => "i128".to_owned(),
        ScSpecTypeDef::U256 => "u256".to_owned(),
        ScSpecTypeDef::I256 => "i256".to_owned(),
        ScSpecTypeDef::Bytes => "Bytes".to_owned(),
        ScSpecTypeDef::String => "String".to_owned(),
        ScSpecTypeDef::Symbol => "Symbol".to_owned(),
        ScSpecTypeDef::Address => "Address".to_owned(),
        ScSpecTypeDef::MuxedAddress => "MuxedAddress".to_owned(),
        ScSpecTypeDef::Option(option) => {
            format!("Option<{}>", normalize_type(&option.value_type).0)
        }
        ScSpecTypeDef::Result(result) => format!(
            "Result<{}, {}>",
            normalize_type(&result.ok_type).0,
            normalize_type(&result.error_type).0
        ),
        ScSpecTypeDef::Vec(vec) => format!("Vec<{}>", normalize_type(&vec.element_type).0),
        ScSpecTypeDef::Map(map) => format!(
            "Map<{}, {}>",
            normalize_type(&map.key_type).0,
            normalize_type(&map.value_type).0
        ),
        ScSpecTypeDef::Tuple(tuple) => {
            let values = tuple
                .value_types
                .iter()
                .map(|value| normalize_type(value).0)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({values})")
        }
        ScSpecTypeDef::BytesN(bytes) => format!("BytesN<{}>", bytes.n),
        ScSpecTypeDef::Udt(udt) => udt.name.to_string(),
    };
    TypeRef(text)
}

fn normalize_event_location(location: ScSpecEventParamLocationV0) -> String {
    match location {
        ScSpecEventParamLocationV0::Data => "data",
        ScSpecEventParamLocationV0::TopicList => "topic_list",
    }
    .to_owned()
}

fn normalize_event_data_format(format: ScSpecEventDataFormat) -> String {
    match format {
        ScSpecEventDataFormat::SingleValue => "single_value",
        ScSpecEventDataFormat::Vec => "vec",
        ScSpecEventDataFormat::Map => "map",
    }
    .to_owned()
}
