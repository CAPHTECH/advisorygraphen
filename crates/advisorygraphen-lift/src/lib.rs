use advisorygraphen_core::{
    array_field, json_object, optional_string_array, record_to_cell_id, sorted_values_by_id,
    stable_context_id, string_field, validate_document, validate_space, AdvisoryResult,
    AdvisorySpaceEnvelope, JsonMap, PACKAGE_TECHNICAL_ADVISORY_MVP, SNAPSHOT_SCHEMA,
};
use advisorygraphen_interpretation::InterpretationPackage;
use serde_json::{json, Value};

pub fn lift_snapshot(
    snapshot: &Value,
    package: &InterpretationPackage,
) -> AdvisoryResult<AdvisorySpaceEnvelope> {
    validate_document(snapshot, Some(SNAPSHOT_SCHEMA))?;
    let sources = array_field(snapshot, "sources")?;
    let records = array_field(snapshot, "records")?;
    let contexts = build_contexts(records);
    let evidence_cells = build_evidence_cells(sources);
    let record_cells = build_record_cells(records);
    let incidences = build_incidences(records);

    let mut cells = evidence_cells;
    cells.extend(record_cells);

    let metadata = json!({
        "source_boundary": snapshot.get("source_boundary").cloned().unwrap_or_else(|| json!({})),
        "lift": {
            "adapter": "json_snapshot",
            "package_id": package.package_id,
            "higher_graphen_interpretation": package.higher_graphen_package_value()?
        }
    });

    let space = AdvisorySpaceEnvelope {
        schema: advisorygraphen_core::SPACE_SCHEMA.to_string(),
        space_id: format!(
            "space:advisory:{}",
            string_field(snapshot, "snapshot_id")?.trim_start_matches("snapshot:")
        ),
        engagement_id: string_field(snapshot, "engagement_id")?.to_string(),
        snapshot_id: string_field(snapshot, "snapshot_id")?.to_string(),
        package_id: PACKAGE_TECHNICAL_ADVISORY_MVP.to_string(),
        cells: sorted_values_by_id(cells),
        contexts: sorted_values_by_id(contexts),
        incidences: sorted_values_by_id(incidences),
        morphisms: vec![json!({
            "id": "morphism:source-to-advisory-space",
            "morphism_type": "source_to_advisory_space",
            "from_id": string_field(snapshot, "snapshot_id")?,
            "to_id": format!("space:advisory:{}", string_field(snapshot, "snapshot_id")?.trim_start_matches("snapshot:")),
            "provenance": source_adapter_provenance()
        })],
        invariants: package.invariant_records(),
        policies: package.policy_records(),
        metadata: metadata.as_object().cloned().unwrap_or_else(JsonMap::new),
    };
    validate_space(&space)?;
    Ok(space)
}

fn build_contexts(records: &[Value]) -> Vec<Value> {
    let mut hints = records
        .iter()
        .flat_map(|record| optional_string_array(record, "context_hints"))
        .collect::<Vec<_>>();
    hints.sort();
    hints.dedup();
    hints
        .into_iter()
        .map(|hint| {
            json!({
                "id": stable_context_id(&hint),
                "context_type": infer_context_type(&hint),
                "title": title_case(&hint),
                "summary": null,
                "provenance": source_adapter_provenance(),
                "metadata": { "source_hint": hint }
            })
        })
        .collect()
}

fn build_evidence_cells(sources: &[Value]) -> Vec<Value> {
    sources
        .iter()
        .filter_map(|source| {
            let source_id = source.get("id")?.as_str()?;
            let suffix = source_id.trim_start_matches("source:");
            Some(json!({
                "id": format!("cell:evidence-{suffix}"),
                "cell_type": "evidence",
                "title": source.get("title").and_then(Value::as_str).unwrap_or(source_id),
                "summary": source.get("source_type").and_then(Value::as_str),
                "context_ids": [],
                "source_ids": [source_id],
                "structure_refs": [],
                "provenance": source_adapter_provenance(),
                "metadata": {
                    "source_id": source_id,
                    "classification": source.get("classification").cloned().unwrap_or_else(|| json!("public"))
                }
            }))
        })
        .collect()
}

fn build_record_cells(records: &[Value]) -> Vec<Value> {
    records
        .iter()
        .filter(|record| record.get("relation").is_none_or(Value::is_null))
        .map(|record| {
            let context_ids = optional_string_array(record, "context_hints")
                .into_iter()
                .map(|hint| stable_context_id(&hint))
                .collect::<Vec<_>>();
            json_object([
                (
                    "id",
                    json!(record_to_cell_id(record["id"].as_str().unwrap_or_default())),
                ),
                (
                    "cell_type",
                    json!(map_record_type(
                        record["record_type"].as_str().unwrap_or("claim")
                    )),
                ),
                (
                    "title",
                    record
                        .get("title")
                        .cloned()
                        .unwrap_or_else(|| json!("Untitled")),
                ),
                (
                    "summary",
                    record.get("summary").cloned().unwrap_or(Value::Null),
                ),
                ("context_ids", json!(context_ids)),
                (
                    "source_ids",
                    record
                        .get("source_ids")
                        .cloned()
                        .unwrap_or_else(|| json!([])),
                ),
                ("structure_refs", json!([])),
                (
                    "provenance",
                    record
                        .get("provenance")
                        .cloned()
                        .unwrap_or_else(source_adapter_provenance),
                ),
                (
                    "metadata",
                    record.get("metadata").cloned().unwrap_or_else(|| json!({})),
                ),
            ])
        })
        .collect()
}

fn build_incidences(records: &[Value]) -> Vec<Value> {
    records
        .iter()
        .filter_map(|record| {
            let relation = record.get("relation").filter(|value| !value.is_null())?;
            let from = relation.get("from_record_id")?.as_str()?;
            let to = relation.get("to_record_id")?.as_str()?;
            let evidence_ids = optional_string_array(record, "source_ids")
                .into_iter()
                .map(|source_id| format!("cell:evidence-{}", source_id.trim_start_matches("source:")))
                .collect::<Vec<_>>();
            let context_ids = optional_string_array(record, "context_hints")
                .into_iter()
                .map(|hint| stable_context_id(&hint))
                .collect::<Vec<_>>();
            Some(json!({
                "id": format!("incidence:{}", record["id"].as_str().unwrap_or_default().trim_start_matches("record:")),
                "relation_type": relation.get("relation_type").and_then(Value::as_str).unwrap_or("related"),
                "from_id": record_to_cell_id(from),
                "to_id": record_to_cell_id(to),
                "context_ids": context_ids,
                "evidence_ids": evidence_ids,
                "strength": "hard",
                "provenance": record.get("provenance").cloned().unwrap_or_else(source_adapter_provenance),
                "metadata": record.get("metadata").cloned().unwrap_or_else(|| json!({}))
            }))
        })
        .collect()
}

fn map_record_type(record_type: &str) -> &str {
    match record_type {
        "component" => "component",
        "data_store" => "data_store",
        "requirement" => "requirement",
        "test" | "verification" | "test_or_verification" => "test_or_verification",
        "owner" => "owner",
        "metric" => "metric",
        "action" => "action",
        _ => "claim",
    }
}

fn infer_context_type(hint: &str) -> &str {
    match hint {
        "orders" | "billing" => "technical_boundary",
        _ => "review_scope",
    }
}

fn title_case(value: &str) -> String {
    value
        .split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn source_adapter_provenance() -> Value {
    json!({
        "origin": "source_backed",
        "actor": "source-adapter:json",
        "confidence": 1.0,
        "review_status": "accepted"
    })
}
