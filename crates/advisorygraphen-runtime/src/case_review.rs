use advisorygraphen_core::AdvisoryResult;
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::Path};

pub fn apply_candidate_reviews(
    store: &Path,
    space_id: &str,
    candidates: &mut [Value],
) -> AdvisoryResult<()> {
    let reviews = review_events(store, space_id)?;
    for candidate in candidates {
        let Some(candidate_id) = candidate.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(review) = reviews.get(candidate_id) else {
            continue;
        };
        if let Some(candidate_object) = candidate.as_object_mut() {
            candidate_object.insert("review_status".to_string(), review["outcome"].clone());
            let metadata = candidate_object
                .entry("metadata")
                .or_insert_with(|| json!({}));
            if let Some(metadata_object) = metadata.as_object_mut() {
                metadata_object.insert("latest_review".to_string(), review.clone());
            }
        }
    }
    Ok(())
}

fn review_events(store: &Path, space_id: &str) -> AdvisoryResult<BTreeMap<String, Value>> {
    let mut reviews = BTreeMap::new();
    for log_path in [root_log_path(store), space_log_path(store, space_id)] {
        let Ok(contents) = fs::read_to_string(log_path) else {
            continue;
        };
        for line in contents.lines().filter(|line| !line.trim().is_empty()) {
            let entry: Value = serde_json::from_str(line)?;
            if entry
                .get("case_space_id")
                .and_then(Value::as_str)
                .is_some_and(|case_space_id| {
                    case_space_id != space_id && case_space_id != "space:unknown"
                })
            {
                continue;
            }
            let Some(payload) = entry.get("payload") else {
                continue;
            };
            if payload.get("schema").and_then(Value::as_str)
                != Some(advisorygraphen_core::REVIEW_EVENT_SCHEMA)
            {
                continue;
            }
            for target_id in payload
                .get("target_ids")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                reviews.insert(target_id.to_string(), review_summary(payload));
            }
        }
    }
    Ok(reviews)
}

fn review_summary(payload: &Value) -> Value {
    json!({
        "review_event_id": payload.get("review_event_id"),
        "outcome": payload.get("outcome"),
        "reviewer_id": payload.get("reviewer_id"),
        "reviewed_at": payload.get("reviewed_at"),
        "reason": payload.get("reason")
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
