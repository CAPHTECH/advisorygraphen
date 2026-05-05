use serde_json::{json, Value};
use std::collections::BTreeMap;

pub fn mark_orphaned_candidates(candidates: &mut [Value], hypotheses: &[Value]) {
    let lifecycle = lifecycle_index(hypotheses);
    for candidate in candidates {
        let Some(parent_id) = candidate
            .pointer("/metadata/derived_from_hypothesis_id")
            .and_then(Value::as_str)
            .map(str::to_owned)
        else {
            continue;
        };
        let Some(parent_status) = lifecycle.get(&parent_id) else {
            continue;
        };
        if parent_status != "falsified" {
            continue;
        }
        let Some(object) = candidate.as_object_mut() else {
            continue;
        };
        if object.get("review_status").and_then(Value::as_str) == Some("unreviewed") {
            object.insert(
                "review_status".to_string(),
                Value::String("superseded".to_string()),
            );
        }
        let metadata = object.entry("metadata").or_insert_with(|| json!({}));
        if let Some(metadata_object) = metadata.as_object_mut() {
            metadata_object.insert(
                "parent_hypothesis_status".to_string(),
                Value::String("falsified".to_string()),
            );
        }
    }
}

pub fn extend_candidates_from_supported_hypotheses(
    candidates: &mut Vec<Value>,
    hypotheses: &[Value],
    obstructions: &[Value],
) {
    let existing_types: BTreeMap<(String, String), bool> = candidates
        .iter()
        .filter_map(|candidate| {
            let parent = candidate
                .pointer("/metadata/derived_from_hypothesis_id")
                .and_then(Value::as_str)?
                .to_owned();
            let candidate_type = candidate
                .get("candidate_type")
                .and_then(Value::as_str)?
                .to_owned();
            Some(((parent, candidate_type), true))
        })
        .collect();
    let obstruction_index: BTreeMap<&str, &Value> = obstructions
        .iter()
        .filter_map(|obstruction| {
            obstruction
                .get("id")
                .and_then(Value::as_str)
                .map(|id| (id, obstruction))
        })
        .collect();
    for hypothesis in hypotheses {
        if hypothesis.get("lifecycle_status").and_then(Value::as_str) != Some("supported") {
            continue;
        }
        let Some(hypothesis_id) = hypothesis.get("id").and_then(Value::as_str) else {
            continue;
        };
        let suggests = hypothesis
            .pointer("/metadata/suggests_completion_types")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let explains = hypothesis
            .pointer("/metadata/explains")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for candidate_type_value in &suggests {
            let Some(candidate_type) = candidate_type_value.as_str() else {
                continue;
            };
            if existing_types.contains_key(&(hypothesis_id.to_string(), candidate_type.to_string()))
            {
                continue;
            }
            for obstruction_id_value in &explains {
                let Some(obstruction_id) = obstruction_id_value.as_str() else {
                    continue;
                };
                let Some(obstruction) = obstruction_index.get(obstruction_id) else {
                    continue;
                };
                candidates.push(supporting_candidate(
                    hypothesis,
                    obstruction,
                    candidate_type,
                ));
            }
        }
    }
}

pub fn reframe_obstructions(obstructions: &mut [Value], hypotheses: &[Value]) {
    let by_obstruction = explanations_by_obstruction(hypotheses);
    for obstruction in obstructions {
        let Some(obstruction_id) = obstruction
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_owned)
        else {
            continue;
        };
        let Some(explainers) = by_obstruction.get(&obstruction_id) else {
            continue;
        };
        let any_falsified = explainers
            .iter()
            .any(|hypothesis| status_of(hypothesis) == "falsified");
        let supporting: Vec<&Value> = explainers
            .iter()
            .copied()
            .filter(|hypothesis| status_of(hypothesis) == "supported")
            .collect();
        if !any_falsified || supporting.is_empty() {
            continue;
        }
        let supporting_ids: Vec<Value> = supporting
            .iter()
            .filter_map(|hypothesis| {
                hypothesis
                    .get("id")
                    .and_then(Value::as_str)
                    .map(|id| Value::String(id.to_string()))
            })
            .collect();
        let mut effective_completion_types: Vec<Value> = Vec::new();
        for hypothesis in &supporting {
            for type_value in hypothesis
                .pointer("/metadata/suggests_completion_types")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if !effective_completion_types.contains(type_value) {
                    effective_completion_types.push(type_value.clone());
                }
            }
        }
        let original_severity = obstruction
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let effective_severity = match original_severity.as_str() {
            "critical" => "high",
            "high" => "medium",
            "medium" => "low",
            other => other,
        };
        let Some(object) = obstruction.as_object_mut() else {
            continue;
        };
        let metadata = object.entry("metadata").or_insert_with(|| json!({}));
        if let Some(metadata_object) = metadata.as_object_mut() {
            metadata_object.insert(
                "reframe".to_string(),
                json!({
                    "primary_hypothesis_falsified": true,
                    "supporting_hypothesis_ids": supporting_ids,
                    "effective_completion_types": effective_completion_types,
                    "original_severity": original_severity,
                    "effective_severity": effective_severity
                }),
            );
            metadata_object.insert(
                "effective_severity".to_string(),
                Value::String(effective_severity.to_string()),
            );
        }
    }
}

fn lifecycle_index(hypotheses: &[Value]) -> BTreeMap<String, String> {
    hypotheses
        .iter()
        .filter_map(|hypothesis| {
            let id = hypothesis.get("id").and_then(Value::as_str)?.to_owned();
            let status = hypothesis
                .get("lifecycle_status")
                .and_then(Value::as_str)
                .unwrap_or("candidate")
                .to_owned();
            Some((id, status))
        })
        .collect()
}

fn explanations_by_obstruction(hypotheses: &[Value]) -> BTreeMap<String, Vec<&Value>> {
    let mut map: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for hypothesis in hypotheses {
        for obstruction_id_value in hypothesis
            .pointer("/metadata/explains")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(obstruction_id) = obstruction_id_value.as_str() {
                map.entry(obstruction_id.to_string())
                    .or_default()
                    .push(hypothesis);
            }
        }
    }
    map
}

fn status_of(hypothesis: &Value) -> &str {
    hypothesis
        .get("lifecycle_status")
        .and_then(Value::as_str)
        .unwrap_or("candidate")
}

fn supporting_candidate(hypothesis: &Value, obstruction: &Value, candidate_type: &str) -> Value {
    let hypothesis_id = hypothesis
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("hypothesis:unknown");
    let obstruction_id = obstruction
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("obstruction:unknown");
    let stem = obstruction_id.trim_start_matches("obstruction:");
    let candidate_id = format!("candidate:{stem}-{candidate_type}");
    let source_ids = obstruction
        .get("evidence_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let context_ids = hypothesis
        .get("context_ids")
        .cloned()
        .unwrap_or_else(|| json!([]));
    json!({
        "id": candidate_id,
        "candidate_type": candidate_type,
        "title": format!("{candidate_type} suggested by supported hypothesis"),
        "summary": hypothesis
            .get("summary")
            .cloned()
            .unwrap_or_else(|| Value::String(String::new())),
        "context_ids": context_ids,
        "source_ids": source_ids,
        "structure_refs": [],
        "review_status": "unreviewed",
        "resolves_obstruction_ids": [obstruction_id],
        "proposed_cell_ids": [],
        "missing_type": "cell",
        "suggested_structure_type": format!("{candidate_type}_cell"),
        "provenance": {
            "origin": "inferred",
            "actor": "advisorygraphen-hypothesis-propagation",
            "review_status": "unreviewed",
            "confidence": 0.55
        },
        "metadata": {
            "specificity": "hypothesis_derived",
            "evidence_strength": "supported_competitor_hypothesis",
            "derived_from_hypothesis_id": hypothesis_id,
            "parent_hypothesis_status": "supported",
            "precision_note": "Generated because the primary hypothesis explaining this obstruction was falsified and this competitor was promoted to supported."
        }
    })
}
