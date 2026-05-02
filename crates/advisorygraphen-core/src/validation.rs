use crate::{
    json_id, optional_string_array, string_field, AdvisoryError, AdvisoryResult,
    AdvisorySpaceEnvelope, ValidationReport, PROJECTION_REQUEST_SCHEMA, REPORT_SCHEMA,
    REVIEW_EVENT_SCHEMA, SNAPSHOT_SCHEMA, SPACE_SCHEMA,
};
use indexmap::IndexSet;
use serde_json::Value;
use std::fmt;

pub fn validate_document(
    value: &Value,
    expected_schema: Option<&str>,
) -> AdvisoryResult<ValidationReport> {
    let schema = string_field(value, "schema")?;
    if let Some(expected) = expected_schema {
        if schema != expected {
            return Err(AdvisoryError::SchemaMismatch {
                expected: expected.to_string(),
                actual: schema.to_string(),
            });
        }
    }

    let mut errors = Vec::new();
    let document_type = dispatch_schema_validation(schema, value, &mut errors)?;
    if errors.is_empty() {
        Ok(ValidationReport {
            schema: schema.to_string(),
            document_type: document_type.to_string(),
            valid: true,
            errors,
        })
    } else {
        Err(AdvisoryError::Validation(errors.join("; ")))
    }
}

pub fn validate_space(space: &AdvisorySpaceEnvelope) -> AdvisoryResult<()> {
    let value = serde_json::to_value(space)?;
    validate_document(&value, Some(SPACE_SCHEMA)).map(|_| ())
}

fn dispatch_schema_validation<'a>(
    schema: &str,
    value: &Value,
    errors: &mut Vec<String>,
) -> AdvisoryResult<&'a str> {
    let document_type = match schema {
        SNAPSHOT_SCHEMA => {
            validate_snapshot(value, errors);
            "engagement_snapshot"
        }
        SPACE_SCHEMA => {
            validate_space_value(value, errors);
            "advisory_space"
        }
        REPORT_SCHEMA => {
            require_fields(value, REPORT_FIELDS, errors);
            "report"
        }
        PROJECTION_REQUEST_SCHEMA => {
            require_fields(value, PROJECTION_REQUEST_FIELDS, errors);
            "projection_request"
        }
        REVIEW_EVENT_SCHEMA => {
            validate_review_event(value, errors);
            "review_event"
        }
        other => {
            return Err(AdvisoryError::SchemaMismatch {
                expected: "known AdvisoryGraphen schema".to_string(),
                actual: other.to_string(),
            });
        }
    };
    Ok(document_type)
}

fn validate_snapshot(value: &Value, errors: &mut Vec<String>) {
    require_fields(value, SNAPSHOT_FIELDS, errors);
    let source_ids = collect_unique_ids(value, "sources", errors);
    for source in value["sources"].as_array().into_iter().flatten() {
        if source.get("classification").and_then(Value::as_str) == Some("secret") {
            errors.push(format!(
                "secret source is not ingestible: {}",
                json_id(source)
            ));
        }
    }
    let _record_ids = collect_unique_ids(value, "records", errors);
    for record in value["records"].as_array().into_iter().flatten() {
        validate_provenance(record, errors);
        validate_source_refs(record, &source_ids, errors);
        if let Some(relation) = record.get("relation").filter(|v| !v.is_null()) {
            require_fields(relation, RELATION_FIELDS, errors);
        }
    }
}

fn validate_space_value(value: &Value, errors: &mut Vec<String>) {
    require_fields(value, SPACE_FIELDS, errors);
    let cell_ids = collect_unique_ids(value, "cells", errors);
    let context_ids = collect_unique_ids(value, "contexts", errors);
    let incidence_ids = collect_unique_ids(value, "incidences", errors);
    let mut known_ids = cell_ids.clone();
    known_ids.extend(context_ids.iter().cloned());
    known_ids.extend(incidence_ids);
    validate_cells(value, &context_ids, errors);
    validate_contexts(value, errors);
    validate_incidences(value, &context_ids, &known_ids, errors);
}

fn validate_cells(value: &Value, context_ids: &IndexSet<String>, errors: &mut Vec<String>) {
    for cell in value["cells"].as_array().into_iter().flatten() {
        require_fields(cell, CELL_FIELDS, errors);
        validate_provenance(cell, errors);
        for context_id in optional_string_array(cell, "context_ids") {
            if !context_ids.contains(&context_id) {
                errors.push(format!(
                    "cell {} references unknown context {context_id}",
                    json_id(cell)
                ));
            }
        }
    }
}

fn validate_contexts(value: &Value, errors: &mut Vec<String>) {
    for context in value["contexts"].as_array().into_iter().flatten() {
        require_fields(context, CONTEXT_FIELDS, errors);
        validate_provenance(context, errors);
    }
}

fn validate_incidences(
    value: &Value,
    context_ids: &IndexSet<String>,
    known_ids: &IndexSet<String>,
    errors: &mut Vec<String>,
) {
    for incidence in value["incidences"].as_array().into_iter().flatten() {
        require_fields(incidence, INCIDENCE_FIELDS, errors);
        validate_provenance(incidence, errors);
        validate_incidence_endpoints(incidence, known_ids, errors);
        validate_incidence_refs(incidence, context_ids, known_ids, errors);
    }
}

fn validate_incidence_endpoints(
    incidence: &Value,
    known_ids: &IndexSet<String>,
    errors: &mut Vec<String>,
) {
    for endpoint in ["from_id", "to_id"] {
        if let Some(id) = incidence.get(endpoint).and_then(Value::as_str) {
            if !known_ids.contains(id) {
                errors.push(format!(
                    "incidence {} has unresolved {endpoint} {id}",
                    json_id(incidence)
                ));
            }
        }
    }
}

fn validate_incidence_refs(
    incidence: &Value,
    context_ids: &IndexSet<String>,
    known_ids: &IndexSet<String>,
    errors: &mut Vec<String>,
) {
    for context_id in optional_string_array(incidence, "context_ids") {
        if !context_ids.contains(&context_id) {
            errors.push(format!(
                "incidence {} references unknown context {context_id}",
                json_id(incidence)
            ));
        }
    }
    for evidence_id in optional_string_array(incidence, "evidence_ids") {
        if !known_ids.contains(&evidence_id) {
            errors.push(format!(
                "incidence {} references unknown evidence {evidence_id}",
                json_id(incidence)
            ));
        }
    }
}

fn validate_review_event(value: &Value, errors: &mut Vec<String>) {
    require_fields(value, REVIEW_EVENT_FIELDS, errors);
    if value
        .get("reason")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        errors.push("review event reason is required".to_string());
    }
    if value
        .get("target_ids")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        errors.push("review event must target at least one id".to_string());
    }
}

fn validate_source_refs(record: &Value, source_ids: &IndexSet<String>, errors: &mut Vec<String>) {
    for source_id in optional_string_array(record, "source_ids") {
        if !source_ids.contains(&source_id) {
            errors.push(format!(
                "record {} references unknown source {source_id}",
                json_id(record)
            ));
        }
    }
}

fn validate_provenance(value: &Value, errors: &mut Vec<String>) {
    let Some(provenance) = value.get("provenance") else {
        errors.push(format!("{} is missing provenance", json_id(value)));
        return;
    };
    require_fields(provenance, PROVENANCE_FIELDS, errors);
    if provenance.get("origin").and_then(Value::as_str) == Some("inferred")
        && provenance.get("review_status").and_then(Value::as_str) == Some("accepted")
    {
        errors.push(format!(
            "{} cannot accept inferred provenance without review",
            json_id(value)
        ));
    }
}

fn collect_unique_ids(
    value: &Value,
    array_field: &str,
    errors: &mut Vec<String>,
) -> IndexSet<String> {
    let mut ids = IndexSet::new();
    for item in value[array_field].as_array().into_iter().flatten() {
        let Some(id) = item.get("id").and_then(Value::as_str) else {
            errors.push(format!("{array_field} item missing id"));
            continue;
        };
        if !ids.insert(id.to_string()) {
            errors.push(format!("duplicate id: {id}"));
        }
    }
    ids
}

fn require_fields(value: &Value, fields: &[&str], errors: &mut Vec<String>) {
    for field in fields {
        if value.get(*field).is_none() {
            errors.push(format!("missing required field `{field}`"));
        }
    }
}

impl fmt::Display for ValidationReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} valid={}", self.document_type, self.valid)
    }
}

const SNAPSHOT_FIELDS: &[&str] = &[
    "schema",
    "snapshot_id",
    "engagement_id",
    "captured_at",
    "source_boundary",
    "sources",
    "records",
    "metadata",
];
const SPACE_FIELDS: &[&str] = &[
    "schema",
    "space_id",
    "engagement_id",
    "snapshot_id",
    "package_id",
    "cells",
    "contexts",
    "incidences",
    "morphisms",
    "invariants",
    "policies",
    "metadata",
];
const REPORT_FIELDS: &[&str] = &[
    "schema",
    "report_type",
    "report_version",
    "tool",
    "input",
    "result",
    "projection",
    "warnings",
];
const PROJECTION_REQUEST_FIELDS: &[&str] = &[
    "schema",
    "projection_id",
    "space_id",
    "audience",
    "purpose",
    "include_ids",
    "exclude_ids",
    "policy_ids",
    "metadata",
];
const CELL_FIELDS: &[&str] = &[
    "id",
    "cell_type",
    "title",
    "context_ids",
    "source_ids",
    "provenance",
    "metadata",
];
const CONTEXT_FIELDS: &[&str] = &["id", "context_type", "title", "provenance", "metadata"];
const INCIDENCE_FIELDS: &[&str] = &[
    "id",
    "relation_type",
    "from_id",
    "to_id",
    "context_ids",
    "evidence_ids",
    "strength",
    "provenance",
    "metadata",
];
const REVIEW_EVENT_FIELDS: &[&str] = &[
    "schema",
    "review_event_id",
    "engagement_id",
    "target_ids",
    "outcome",
    "reviewer_id",
    "reviewed_at",
    "reason",
    "evidence_ids",
    "metadata",
];
const RELATION_FIELDS: &[&str] = &["relation_type", "from_record_id", "to_record_id"];
const PROVENANCE_FIELDS: &[&str] = &["origin", "actor", "review_status"];
