use advisorygraphen_core::{
    array_field, json_object, optional_string_array, record_to_cell_id, sorted_values_by_id,
    stable_context_id, string_field, validate_document, validate_space, AdvisoryResult,
    AdvisorySpaceEnvelope, JsonMap, PACKAGE_TECHNICAL_ADVISORY_MVP, SNAPSHOT_SCHEMA,
};
use advisorygraphen_interpretation::InterpretationPackage;
use higher_graphen_core::{Confidence, Id, Provenance, ReviewStatus, SourceKind, SourceRef};
use higher_graphen_structure::morphism::{CellMapping, Morphism, MorphismType, RelationMapping};
use serde_json::{json, Value};
use std::collections::BTreeMap;

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
        },
        "schema_morphisms": [
            schema_morphism(snapshot, package)
        ]
    });

    let space_id = format!(
        "space:advisory:{}",
        string_field(snapshot, "snapshot_id")?.trim_start_matches("snapshot:")
    );
    let source_to_space_morphism = source_to_space_morphism(snapshot, records, &space_id)?;
    let source_to_space_preservation = source_to_space_morphism.check_preservation(
        package
            .invariant_records()
            .into_iter()
            .filter_map(|invariant| {
                invariant
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .map(Id::new)
            .collect::<Result<Vec<_>, _>>()
            .map_err(higher_error)?,
    );

    let space = AdvisorySpaceEnvelope {
        schema: advisorygraphen_core::SPACE_SCHEMA.to_string(),
        space_id: space_id.clone(),
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
            "to_id": space_id,
            "provenance": source_adapter_provenance(),
            "schema_morphism": schema_morphism(snapshot, package),
            "higher_graphen": {
                "morphism": source_to_space_morphism,
                "preservation_report": source_to_space_preservation
            }
        })],
        invariants: package.invariant_records(),
        policies: package.policy_records(),
        metadata: metadata.as_object().cloned().unwrap_or_else(JsonMap::new),
    };
    validate_space(&space)?;
    Ok(space)
}

fn schema_morphism(snapshot: &Value, package: &InterpretationPackage) -> Value {
    json!({
        "id": "schema-morphism:engagement-snapshot-to-advisory-space",
        "source_schema": SNAPSHOT_SCHEMA,
        "target_schema": advisorygraphen_core::SPACE_SCHEMA,
        "interpretation_package_id": package.package_id,
        "mapping_kind": "lift_adapter",
        "compatibility": "compatible_with_declared_loss",
        "mappings": [
            { "from": "/sources[]", "to": "/cells[cell_type=evidence]", "loss": "source prose is represented by evidence cells and source metadata" },
            { "from": "/records[]", "to": "/cells[] or /incidences[]", "loss": "record prose is normalized into typed advisory structure" },
            { "from": "/source_boundary", "to": "/metadata/source_boundary", "loss": "boundary notes remain advisory metadata rather than accepted facts" }
        ],
        "affected_objects": {
            "source_count": snapshot.get("sources").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
            "record_count": snapshot.get("records").and_then(Value::as_array).map(Vec::len).unwrap_or(0)
        },
        "verification": {
            "status": "checked_by_lift_validation",
            "validator": "advisorygraphen-lift",
            "review_status": "accepted"
        },
        "loss_claims": snapshot.pointer("/source_boundary/extraction_loss").cloned().unwrap_or_else(|| json!([])),
        "provenance": source_adapter_provenance(),
        "review_status": "accepted"
    })
}

fn source_to_space_morphism(
    snapshot: &Value,
    records: &[Value],
    space_id: &str,
) -> AdvisoryResult<Morphism> {
    let mut cell_mapping: CellMapping = BTreeMap::new();
    let mut relation_mapping: RelationMapping = BTreeMap::new();

    for source in array_field(snapshot, "sources")? {
        let Some(source_id) = source.get("id").and_then(Value::as_str) else {
            continue;
        };
        cell_mapping.insert(
            hg_id(source_id)?,
            hg_id(&format!(
                "cell:evidence-{}",
                source_id.trim_start_matches("source:")
            ))?,
        );
    }

    for record in records {
        let Some(record_id) = record.get("id").and_then(Value::as_str) else {
            continue;
        };
        if record
            .get("relation")
            .is_some_and(|relation| !relation.is_null())
        {
            relation_mapping.insert(
                hg_id(record_id)?,
                hg_id(&format!(
                    "incidence:{}",
                    record_id.trim_start_matches("record:")
                ))?,
            );
        } else {
            cell_mapping.insert(hg_id(record_id)?, hg_id(&record_to_cell_id(record_id))?);
        }
    }

    Ok(Morphism {
        id: hg_id("morphism:source-to-advisory-space")?,
        source_space_id: hg_id(string_field(snapshot, "snapshot_id")?)?,
        target_space_id: hg_id(space_id)?,
        name: "JSON snapshot to AdvisoryGraphen space".to_string(),
        morphism_type: MorphismType::Lift,
        cell_mapping,
        relation_mapping,
        preserved_invariant_ids: Vec::new(),
        lost_structure: Vec::new(),
        distortion: Vec::new(),
        composable_with: Vec::new(),
        provenance: hg_source_adapter_provenance()?,
    })
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
            let record_type = record["record_type"].as_str().unwrap_or("claim");
            let context_ids = optional_string_array(record, "context_hints")
                .into_iter()
                .map(|hint| stable_context_id(&hint))
                .collect::<Vec<_>>();
            json_object([
                (
                    "id",
                    json!(record_to_cell_id(record["id"].as_str().unwrap_or_default())),
                ),
                ("cell_type", json!(map_record_type(record_type))),
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
                ("metadata", lifted_record_metadata(record_type, record)),
            ])
        })
        .collect()
}

fn lifted_record_metadata(record_type: &str, record: &Value) -> Value {
    let mut metadata = record
        .get("metadata")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(JsonMap::new);

    match record_type {
        "hypothesis" | "hypothesis_seed" | "hypothesis_refinement" => {
            metadata
                .entry("hypothesis".to_string())
                .or_insert_with(|| json!(true));
            metadata
                .entry("hypothesis_status".to_string())
                .or_insert_with(|| json!("candidate"));
            let structuring_phase = if record_type == "hypothesis_refinement" {
                "hypothesis_refinement"
            } else {
                "hypothesis_first"
            };
            metadata
                .entry("structuring_phase".to_string())
                .or_insert_with(|| json!(structuring_phase));
            if record_type == "hypothesis_refinement" {
                metadata
                    .entry("hypothesis_refinement".to_string())
                    .or_insert_with(|| json!(true));
            }
        }
        "proposal" | "structure_proposal" => {
            metadata
                .entry("structure_proposal".to_string())
                .or_insert_with(|| json!(true));
            metadata
                .entry("structuring_phase".to_string())
                .or_insert_with(|| json!("derived_from_hypothesis"));
            metadata
                .entry("priority".to_string())
                .or_insert_with(|| json!("p2"));
        }
        _ => {}
    }

    Value::Object(metadata)
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
        "action" | "proposal" | "structure_proposal" => "action",
        "hypothesis" | "hypothesis_seed" | "hypothesis_refinement" => "hypothesis",
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

fn hg_source_adapter_provenance() -> AdvisoryResult<Provenance> {
    Ok(Provenance::new(
        SourceRef::new(SourceKind::Document)
            .with_title("source-adapter:json")
            .map_err(higher_error)?,
        Confidence::new(1.0).map_err(higher_error)?,
    )
    .with_review_status(ReviewStatus::Accepted))
}

fn hg_id(value: &str) -> AdvisoryResult<Id> {
    Id::new(value).map_err(higher_error)
}

fn higher_error(error: higher_graphen_core::CoreError) -> advisorygraphen_core::AdvisoryError {
    advisorygraphen_core::AdvisoryError::Validation(format!("higher-graphen lift: {error}"))
}
