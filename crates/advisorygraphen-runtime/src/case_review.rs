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

pub fn blocker_resolution_state(blockers: &[Value], candidates: &[Value]) -> Vec<Value> {
    blockers
        .iter()
        .filter_map(|blocker| {
            let obstruction_id = blocker.get("id").and_then(Value::as_str)?;
            let resolving = resolving_candidates(obstruction_id, candidates);
            let accepted = resolving
                .iter()
                .filter(|candidate| {
                    candidate.get("review_status").and_then(Value::as_str) == Some("accepted")
                })
                .cloned()
                .collect::<Vec<_>>();
            let status = if !accepted.is_empty() {
                "accepted_candidate_pending_application"
            } else if resolving.is_empty() {
                "no_candidate"
            } else if resolving.iter().all(|candidate| {
                candidate.get("review_status").and_then(Value::as_str) == Some("rejected")
            }) {
                "all_candidates_rejected"
            } else {
                "candidate_review_pending"
            };
            Some(json!({
                "obstruction_id": obstruction_id,
                "resolution_status": status,
                "candidate_ids": ids(&resolving),
                "accepted_candidate_ids": ids(&accepted),
                "close_effect": "does_not_clear_obstruction_until_structure_changes"
            }))
        })
        .collect()
}

fn resolving_candidates<'a>(obstruction_id: &str, candidates: &'a [Value]) -> Vec<&'a Value> {
    candidates
        .iter()
        .filter(|candidate| {
            candidate
                .get("resolves_obstruction_ids")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .any(|id| id.as_str() == Some(obstruction_id))
        })
        .collect()
}

fn ids(candidates: &[&Value]) -> Vec<String> {
    candidates
        .iter()
        .filter_map(|candidate| {
            candidate
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect()
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
