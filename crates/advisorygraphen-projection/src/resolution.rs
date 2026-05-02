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
