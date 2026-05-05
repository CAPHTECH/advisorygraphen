use advisorygraphen_core::{AdvisoryResult, HYPOTHESIS_EVENT_SCHEMA};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::Path};

pub fn apply_hypothesis_events(
    store: &Path,
    space_id: &str,
    hypotheses: &mut [Value],
) -> AdvisoryResult<()> {
    let events = hypothesis_events(store, space_id)?;
    if events.is_empty() {
        return Ok(());
    }
    let mut competitor_supports: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut competitor_rejects: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for hypothesis in hypotheses.iter() {
        let Some(hypothesis_id) = hypothesis.get("id").and_then(Value::as_str) else {
            continue;
        };
        if let Some(event) = events.get(hypothesis_id) {
            let outcome = event["outcome"].as_str().unwrap_or("");
            let competitors: Vec<String> = hypothesis
                .pointer("/metadata/competes_with")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect();
            match outcome {
                "falsified" => {
                    for competitor in competitors {
                        competitor_supports
                            .entry(competitor)
                            .or_default()
                            .push(event.clone());
                    }
                }
                "accepted" => {
                    for competitor in competitors {
                        competitor_rejects
                            .entry(competitor)
                            .or_default()
                            .push(event.clone());
                    }
                }
                _ => {}
            }
        }
    }
    for hypothesis in hypotheses {
        let Some(hypothesis_id) = hypothesis.get("id").and_then(Value::as_str) else {
            continue;
        };
        let direct = events.get(hypothesis_id).cloned();
        let supports = competitor_supports.get(hypothesis_id).cloned();
        let rejects = competitor_rejects.get(hypothesis_id).cloned();
        let Some(object) = hypothesis.as_object_mut() else {
            continue;
        };
        if let Some(event) = direct {
            object.insert("lifecycle_status".to_string(), event["outcome"].clone());
            let metadata = object.entry("metadata").or_insert_with(|| json!({}));
            if let Some(metadata_object) = metadata.as_object_mut() {
                metadata_object.insert("latest_hypothesis_event".to_string(), event);
            }
        } else if let Some(events) = rejects {
            object.insert(
                "lifecycle_status".to_string(),
                Value::String("rejected".to_string()),
            );
            let metadata = object.entry("metadata").or_insert_with(|| json!({}));
            if let Some(metadata_object) = metadata.as_object_mut() {
                metadata_object.insert(
                    "rejected_by_competitor_acceptance".to_string(),
                    Value::Array(events),
                );
            }
        } else if let Some(events) = supports {
            object.insert(
                "lifecycle_status".to_string(),
                Value::String("supported".to_string()),
            );
            let metadata = object.entry("metadata").or_insert_with(|| json!({}));
            if let Some(metadata_object) = metadata.as_object_mut() {
                metadata_object.insert(
                    "supporting_falsifications".to_string(),
                    Value::Array(events),
                );
            }
        }
    }
    Ok(())
}

fn hypothesis_events(store: &Path, space_id: &str) -> AdvisoryResult<BTreeMap<String, Value>> {
    let mut events = BTreeMap::new();
    for log_path in [root_log_path(store), space_log_path(store, space_id)] {
        let Ok(contents) = fs::read_to_string(log_path) else {
            continue;
        };
        for line in contents.lines().filter(|line| !line.trim().is_empty()) {
            let entry: Value = serde_json::from_str(line)?;
            if entry.get("case_space_id").and_then(Value::as_str) != Some(space_id) {
                continue;
            }
            let Some(payload) = entry.get("payload") else {
                continue;
            };
            if payload.get("schema").and_then(Value::as_str) != Some(HYPOTHESIS_EVENT_SCHEMA) {
                continue;
            }
            let Some(target) = payload.get("target_hypothesis_id").and_then(Value::as_str) else {
                continue;
            };
            events.insert(target.to_string(), event_summary(payload));
        }
    }
    Ok(events)
}

fn event_summary(payload: &Value) -> Value {
    json!({
        "hypothesis_event_id": payload.get("hypothesis_event_id"),
        "outcome": payload.get("outcome"),
        "reviewer_id": payload.get("reviewer_id"),
        "reviewed_at": payload.get("reviewed_at"),
        "reason": payload.get("reason"),
        "evidence_ids": payload.get("evidence_ids")
    })
}

fn root_log_path(store: &Path) -> std::path::PathBuf {
    store.join("logs/morphism-log.jsonl")
}

fn space_log_path(store: &Path, space_id: &str) -> std::path::PathBuf {
    store
        .join("spaces")
        .join(space_id.replace([':', '/'], "-"))
        .join("logs/morphism-log.jsonl")
}
