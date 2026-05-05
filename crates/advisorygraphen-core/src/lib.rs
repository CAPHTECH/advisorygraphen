use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

mod higher;
mod validation;
pub use higher::HigherGraphenAdvisorySpace;
pub use validation::{validate_document, validate_space};

pub const TOOL_NAME: &str = "advisorygraphen";
pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SNAPSHOT_SCHEMA: &str = "advisorygraphen.engagement.snapshot.v1";
pub const SPACE_SCHEMA: &str = "advisorygraphen.space.v1";
pub const REPORT_SCHEMA: &str = "advisorygraphen.report.v1";
pub const PROJECTION_REQUEST_SCHEMA: &str = "advisorygraphen.projection.request.v1";
pub const REVIEW_EVENT_SCHEMA: &str = "advisorygraphen.review.event.v1";
pub const HYPOTHESIS_EVENT_SCHEMA: &str = "advisorygraphen.hypothesis.event.v1";
pub const PACKAGE_TECHNICAL_ADVISORY_MVP: &str = "package:technical_advisory_mvp";

#[derive(Debug, thiserror::Error)]
pub enum AdvisoryError {
    #[error("schema mismatch: expected {expected}, got {actual}")]
    SchemaMismatch { expected: String, actual: String },
    #[error("validation error: {0}")]
    Validation(String),
    #[error("duplicate id: {0}")]
    DuplicateId(String),
    #[error("unresolved reference: {0}")]
    UnresolvedReference(String),
    #[error("unsupported package: {0}")]
    UnsupportedPackage(String),
    #[error("unsupported ruleset: {0}")]
    UnsupportedRuleset(String),
    #[error("unsupported audience: {0}")]
    UnsupportedAudience(String),
    #[error("stale revision: expected {expected}, got {actual}")]
    StaleRevision { expected: String, actual: String },
    #[error("fail-on threshold triggered: {0}")]
    FailOnThreshold(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

impl AdvisoryError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::SchemaMismatch { .. }
            | Self::Validation(_)
            | Self::DuplicateId(_)
            | Self::UnresolvedReference(_) => 1,
            Self::Io(_) => 3,
            Self::UnsupportedPackage(_)
            | Self::UnsupportedRuleset(_)
            | Self::UnsupportedAudience(_) => 4,
            Self::StaleRevision { .. } => 5,
            Self::FailOnThreshold(_) => 6,
            Self::Json(_) => 1,
        }
    }
}

pub type AdvisoryResult<T> = Result<T, AdvisoryError>;
pub type JsonMap = serde_json::Map<String, Value>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub schema: String,
    pub document_type: String,
    pub valid: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvisorySpaceEnvelope {
    pub schema: String,
    pub space_id: String,
    pub engagement_id: String,
    pub snapshot_id: String,
    pub package_id: String,
    pub cells: Vec<Value>,
    pub contexts: Vec<Value>,
    pub incidences: Vec<Value>,
    pub morphisms: Vec<Value>,
    pub invariants: Vec<Value>,
    pub policies: Vec<Value>,
    pub metadata: JsonMap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportEnvelope {
    pub schema: String,
    pub report_type: String,
    pub report_version: u8,
    pub tool: Value,
    pub input: Value,
    pub result: Value,
    pub projection: Value,
    pub warnings: Vec<Value>,
}

impl ReportEnvelope {
    pub fn new(report_type: &str, command: Option<&str>, input: Value, result: Value) -> Self {
        Self {
            schema: REPORT_SCHEMA.to_string(),
            report_type: report_type.to_string(),
            report_version: 1,
            tool: tool_metadata(command),
            input,
            result,
            projection: json!({}),
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "info" => Some(Self::Info),
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }
}

pub fn tool_metadata(command: Option<&str>) -> Value {
    let mut tool = json!({
        "name": TOOL_NAME,
        "version": TOOL_VERSION,
        "git_revision": null
    });
    if let Some(command) = command {
        tool["command"] = Value::String(command.to_string());
    }
    tool
}

pub fn canonical_package_id(package: &str) -> AdvisoryResult<String> {
    match package {
        "technical_advisory" | "technical_advisory_mvp" | "package:technical_advisory_mvp" => {
            Ok(PACKAGE_TECHNICAL_ADVISORY_MVP.to_string())
        }
        other => Err(AdvisoryError::UnsupportedPackage(other.to_string())),
    }
}

pub fn sorted_values_by_id(mut values: Vec<Value>) -> Vec<Value> {
    values.sort_by(|left, right| json_id(left).cmp(json_id(right)));
    values
}

pub fn slugify_id(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

pub fn stable_context_id(hint: &str) -> String {
    format!("context:{}", slugify_id(hint))
}

pub fn record_to_cell_id(record_id: &str) -> String {
    prefixed_id(
        "cell",
        record_id.strip_prefix("record:").unwrap_or(record_id),
    )
}

pub fn prefixed_id(prefix: &str, raw: &str) -> String {
    let raw = raw.strip_prefix(&format!("{prefix}:")).unwrap_or(raw);
    format!("{prefix}:{}", slugify_id(raw))
}

pub fn string_field<'a>(value: &'a Value, field: &str) -> AdvisoryResult<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| AdvisoryError::Validation(format!("missing or non-string field `{field}`")))
}

pub fn array_field<'a>(value: &'a Value, field: &str) -> AdvisoryResult<&'a Vec<Value>> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| AdvisoryError::Validation(format!("missing or non-array field `{field}`")))
}

pub fn json_id(value: &Value) -> &str {
    value.get("id").and_then(Value::as_str).unwrap_or("")
}

pub fn optional_string_array(value: &Value, field: &str) -> Vec<String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

pub fn json_object(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    let mut map = JsonMap::new();
    for (key, value) in entries {
        map.insert(key.to_string(), value);
    }
    Value::Object(map)
}
