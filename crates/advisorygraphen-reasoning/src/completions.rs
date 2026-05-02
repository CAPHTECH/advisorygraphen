use advisorygraphen_core::{
    json_id, sorted_values_by_id, AdvisoryResult, AdvisorySpaceEnvelope, ReportEnvelope,
};
use serde_json::{json, Value};

pub fn propose_completions(
    space: &AdvisorySpaceEnvelope,
    check_report: &ReportEnvelope,
    from_report: &str,
    command: Option<&str>,
) -> AdvisoryResult<ReportEnvelope> {
    let mut candidates = Vec::new();
    let obstructions = check_report
        .result
        .get("obstructions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for obstruction in obstructions {
        match obstruction.get("obstruction_type").and_then(Value::as_str) {
            Some("boundary_violation") => candidates.extend(boundary_completion_candidates()),
            Some("missing_owner") => candidates.push(owner_completion_candidate(&obstruction)),
            Some("requirement_unverified") => {
                candidates.push(verification_completion_candidate(&obstruction))
            }
            _ => {}
        }
    }
    candidates = sorted_values_by_id(candidates);
    Ok(ReportEnvelope::new(
        "completion_proposal",
        command,
        json!({
            "space_id": space.space_id,
            "from_report": from_report
        }),
        json!({ "completion_candidates": candidates }),
    ))
}

fn boundary_completion_candidates() -> Vec<Value> {
    vec![
        json!({
            "id": "candidate:billing-status-api",
            "candidate_type": "proposed_interface",
            "title": "Add Billing status query API",
            "rationale": "Remove cross-context direct database access while preserving billing status check.",
            "resolves_obstruction_ids": ["obstruction:order-service-direct-billing-db-access"],
            "proposed_cell_ids": ["cell:billing-status-api"],
            "source_ids": ["source:architecture-note"],
            "confidence": 0.82,
            "review_status": "unreviewed",
            "metadata": {}
        }),
        json!({
            "id": "candidate:replace-order-service-db-read",
            "candidate_type": "proposed_refactor_action",
            "title": "Replace Order Service direct DB read with Billing API call",
            "rationale": "Order Service should depend on Billing Service interface instead of Billing DB ownership boundary.",
            "resolves_obstruction_ids": ["obstruction:order-service-direct-billing-db-access"],
            "proposed_cell_ids": ["cell:action-replace-direct-db-read"],
            "source_ids": ["source:architecture-note"],
            "confidence": 0.78,
            "review_status": "unreviewed",
            "metadata": {}
        }),
    ]
}

fn owner_completion_candidate(obstruction: &Value) -> Value {
    json!({
        "id": format!("candidate:{}-owner", json_id(obstruction).trim_start_matches("obstruction:")),
        "candidate_type": "ownership_clarification",
        "title": "Clarify action owner",
        "rationale": obstruction.get("message").cloned().unwrap_or_else(|| json!("Action requires owner.")),
        "resolves_obstruction_ids": [obstruction["id"].clone()],
        "proposed_cell_ids": [],
        "source_ids": [],
        "confidence": 0.7,
        "review_status": "unreviewed",
        "metadata": {}
    })
}

fn verification_completion_candidate(obstruction: &Value) -> Value {
    json!({
        "id": format!("candidate:{}-verification", json_id(obstruction).trim_start_matches("obstruction:")),
        "candidate_type": "proposed_test",
        "title": "Define verification method",
        "rationale": obstruction.get("message").cloned().unwrap_or_else(|| json!("Requirement needs verification.")),
        "resolves_obstruction_ids": [obstruction["id"].clone()],
        "proposed_cell_ids": [],
        "source_ids": [],
        "confidence": 0.7,
        "review_status": "unreviewed",
        "metadata": {}
    })
}
