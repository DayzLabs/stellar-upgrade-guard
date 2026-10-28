use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityImpact {
    Compatible,
    Warning,
    Breaking,
    Unknown,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityResult {
    Compatible,
    Warning,
    Breaking,
    Unknown,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingCode {
    FunctionAdded,
    FunctionRemoved,
    ArgumentAdded,
    ArgumentRemoved,
    ArgumentRenamed,
    ArgumentReordered,
    ArgumentTypeChanged,
    ReturnTypeChanged,
    StructAdded,
    StructRemoved,
    StructFieldAdded,
    StructFieldRemoved,
    StructFieldRenamed,
    StructFieldReordered,
    StructFieldTypeChanged,
    EnumAdded,
    EnumRemoved,
    EnumCaseAdded,
    EnumCaseRemoved,
    EnumCaseValueChanged,
    ErrorAdded,
    ErrorRemoved,
    ErrorCaseAdded,
    ErrorCaseRemoved,
    ErrorCodeChanged,
    UnionAdded,
    UnionRemoved,
    UnionCaseAdded,
    UnionCaseRemoved,
    UnionCasePayloadChanged,
    EventAdded,
    EventRemoved,
    EventParametersChanged,
    EventDataFormatChanged,
    EnvInterfaceVersionChanged,
    EnvMetaUnavailable,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompatibilityFinding {
    pub code: FindingCode,
    pub impact: CompatibilityImpact,
    pub path: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_value: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompatibilitySummary {
    pub compatible: usize,
    pub warning: usize,
    pub breaking: usize,
    pub unknown: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompatibilityReport {
    pub result: CompatibilityResult,
    pub summary: CompatibilitySummary,
    pub findings: Vec<CompatibilityFinding>,
}

impl CompatibilityReport {
    pub fn new(findings: Vec<CompatibilityFinding>) -> Self {
        let summary = CompatibilitySummary::from_findings(&findings);
        let result = summary.result();
        Self {
            result,
            summary,
            findings,
        }
    }
}

impl CompatibilitySummary {
    pub fn from_findings(findings: &[CompatibilityFinding]) -> Self {
        let mut summary = Self {
            compatible: 0,
            warning: 0,
            breaking: 0,
            unknown: 0,
        };
        for finding in findings {
            match finding.impact {
                CompatibilityImpact::Compatible => summary.compatible += 1,
                CompatibilityImpact::Warning => summary.warning += 1,
                CompatibilityImpact::Breaking => summary.breaking += 1,
                CompatibilityImpact::Unknown => summary.unknown += 1,
            }
        }
        summary
    }

    pub fn result(&self) -> CompatibilityResult {
        if self.breaking > 0 {
            CompatibilityResult::Breaking
        } else if self.unknown > 0 {
            CompatibilityResult::Unknown
        } else if self.warning > 0 {
            CompatibilityResult::Warning
        } else {
            CompatibilityResult::Compatible
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContractInterface {
    pub functions: Vec<FunctionSpec>,
    pub structs: Vec<StructSpec>,
    pub enums: Vec<EnumSpec>,
    pub unions: Vec<UnionSpec>,
    pub errors: Vec<ErrorSpec>,
    pub events: Vec<EventSpec>,
    #[serde(default)]
    pub env_meta: EnvMeta,
}

/// The host environment interface version a contract was built against, taken from the
/// `contractenvmetav0` custom section (`SCEnvMetaEntry` interface-version entry).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct EnvInterfaceVersion {
    pub protocol: u32,
    pub pre_release: u32,
}

impl std::fmt::Display for EnvInterfaceVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "protocol {}, pre-release {}",
            self.protocol, self.pre_release
        )
    }
}

/// State of the `contractenvmetav0` custom section in a WASM file.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvMeta {
    /// The section is missing or contains no entries.
    #[default]
    Absent,
    InterfaceVersion(EnvInterfaceVersion),
    /// The section exists but is not a valid `SCEnvMetaEntry` stream for the bundled XDR version.
    Unreadable,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct FunctionSpec {
    pub name: String,
    pub inputs: Vec<FunctionInput>,
    pub outputs: Vec<TypeRef>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct FunctionInput {
    pub name: String,
    pub type_ref: TypeRef,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct StructSpec {
    pub name: String,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct StructField {
    pub name: String,
    pub type_ref: TypeRef,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct EnumSpec {
    pub name: String,
    pub cases: Vec<EnumCase>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct EnumCase {
    pub name: String,
    pub value: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct UnionSpec {
    pub name: String,
    pub cases: Vec<UnionCase>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct UnionCase {
    pub name: String,
    pub values: Vec<TypeRef>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ErrorSpec {
    pub name: String,
    pub cases: Vec<ErrorCase>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ErrorCase {
    pub name: String,
    pub value: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct EventSpec {
    pub name: String,
    pub parameters: Vec<EventParam>,
    pub data_format: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct EventParam {
    pub name: String,
    pub type_ref: TypeRef,
    pub location: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TypeRef(pub String);
