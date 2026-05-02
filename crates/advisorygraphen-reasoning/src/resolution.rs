use serde_json::{json, Value};

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
                .copied()
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
                "obstruction_type": blocker
                    .get("obstruction_type")
                    .cloned()
                    .unwrap_or_else(|| json!("unknown")),
                "severity": blocker
                    .get("severity")
                    .cloned()
                    .unwrap_or_else(|| json!("unknown")),
                "blocked_ids": blocker
                    .get("blocked_ids")
                    .cloned()
                    .unwrap_or_else(|| json!([])),
                "recommended_completion_types": blocker
                    .get("recommended_completion_types")
                    .cloned()
                    .unwrap_or_else(|| json!([])),
                "resolution_status": status,
                "candidate_ids": candidate_ids(&resolving),
                "accepted_candidate_ids": candidate_ids(&accepted),
                "application_requirements": application_requirements(
                    blocker,
                    &applicable_candidates(&accepted, &resolving),
                ),
                "close_effect": "does_not_clear_obstruction_until_structure_changes"
            }))
        })
        .collect()
}

pub fn frontier_items(resolution_state: &[Value]) -> Vec<Value> {
    resolution_state
        .iter()
        .filter_map(|item| {
            let obstruction_id = item.get("obstruction_id").and_then(Value::as_str)?;
            match item.get("resolution_status").and_then(Value::as_str) {
                Some("accepted_candidate_pending_application") => Some(json!({
                    "item_type": "apply_accepted_candidate_structure",
                    "obstruction_id": obstruction_id,
                    "candidate_ids": item
                        .get("accepted_candidate_ids")
                        .cloned()
                        .unwrap_or_else(|| json!([])),
                    "application_requirements": item
                        .get("application_requirements")
                        .cloned()
                        .unwrap_or_else(|| json!([])),
                    "next_operation": "apply required cells/incidences, rerun check, then rerun case reason"
                })),
                Some("no_candidate") => Some(json!({
                    "item_type": "propose_completion_candidate",
                    "obstruction_id": obstruction_id,
                    "candidate_ids": [],
                    "blocked_ids": item
                        .get("blocked_ids")
                        .cloned()
                        .unwrap_or_else(|| json!([])),
                    "recommended_completion_types": item
                        .get("recommended_completion_types")
                        .cloned()
                        .unwrap_or_else(|| json!([])),
                    "application_requirements": [],
                    "next_operation": "run completions propose or add bounded source structure"
                })),
                _ => None,
            }
        })
        .collect()
}

pub fn waiting_items(resolution_state: &[Value]) -> Vec<Value> {
    resolution_state
        .iter()
        .filter_map(|item| {
            let obstruction_id = item.get("obstruction_id").and_then(Value::as_str)?;
            match item.get("resolution_status").and_then(Value::as_str) {
                Some("candidate_review_pending") => Some(json!({
                    "item_type": "candidate_review_pending",
                    "obstruction_id": obstruction_id,
                    "candidate_ids": item
                        .get("candidate_ids")
                        .cloned()
                        .unwrap_or_else(|| json!([])),
                    "recommended_completion_types": item
                        .get("recommended_completion_types")
                        .cloned()
                        .unwrap_or_else(|| json!([])),
                    "waiting_on": "explicit accept/reject review for candidate structure"
                })),
                Some("all_candidates_rejected") => Some(json!({
                    "item_type": "all_candidates_rejected",
                    "obstruction_id": obstruction_id,
                    "candidate_ids": item
                        .get("candidate_ids")
                        .cloned()
                        .unwrap_or_else(|| json!([])),
                    "recommended_completion_types": item
                        .get("recommended_completion_types")
                        .cloned()
                        .unwrap_or_else(|| json!([])),
                    "waiting_on": "new bounded source structure or human direction"
                })),
                _ => None,
            }
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

fn candidate_ids(candidates: &[&Value]) -> Vec<String> {
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

fn applicable_candidates<'a>(accepted: &[&'a Value], resolving: &[&'a Value]) -> Vec<&'a Value> {
    if !accepted.is_empty() {
        return accepted.to_vec();
    }
    resolving
        .iter()
        .copied()
        .filter(|candidate| {
            candidate.get("review_status").and_then(Value::as_str) != Some("rejected")
        })
        .collect()
}

fn application_requirements(blocker: &Value, candidates: &[&Value]) -> Vec<Value> {
    candidates
        .iter()
        .filter_map(|candidate| {
            let candidate_id = candidate.get("id").and_then(Value::as_str)?;
            let candidate_type = candidate.get("candidate_type").and_then(Value::as_str)?;
            let (cells, relations, next_step) = application_contract(candidate_type);
            Some(json!({
                "candidate_id": candidate_id,
                "candidate_type": candidate_type,
                "review_status": candidate
                    .get("review_status")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                "required_cell_types": cells,
                "required_relation_types": relations,
                "target_blocked_ids": blocker
                    .get("blocked_ids")
                    .cloned()
                    .unwrap_or_else(|| json!([])),
                "next_structural_step": next_step
            }))
        })
        .collect()
}

fn application_contract(
    candidate_type: &str,
) -> (
    &'static [&'static str],
    &'static [&'static str],
    &'static str,
) {
    match candidate_type {
        "ownership_clarification" => (
            &["owner"],
            &["owns"],
            "add owner cell and owns incidence to the blocked action, then rerun check and case reason",
        ),
        "proposed_test" | "define_metric" => (
            &["test_or_verification"],
            &["verifies"],
            "add verification cell and verifies incidence to the blocked requirement, then rerun check and case reason",
        ),
        "proposed_interface" => (
            &["interface"],
            &["uses", "provides"],
            "add interface cell and boundary-safe incidences, then rerun check and case reason",
        ),
        "proposed_refactor_action" => (
            &["refactor_action"],
            &["replaces"],
            "add refactor action cell and replacement incidence, then rerun check and case reason",
        ),
        _ => (
            &["reviewed_structure"],
            &["resolves"],
            "add reviewed structure linked to the obstruction, then rerun check and case reason",
        ),
    }
}
