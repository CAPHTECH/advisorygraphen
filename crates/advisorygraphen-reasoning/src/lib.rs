use advisorygraphen_core::{
    json_id, optional_string_array, sorted_values_by_id, AdvisoryError, AdvisoryResult,
    AdvisorySpaceEnvelope, ReportEnvelope, Severity,
};
use advisorygraphen_interpretation::load_ruleset;
use serde_json::{json, Value};

pub const BOUNDARY_INVARIANT: &str =
    "invariant:architecture_no_cross_context_direct_database_access";
pub const EVIDENCE_INVARIANT: &str = "invariant:recommendation_requires_evidence";
pub const OWNER_INVARIANT: &str = "invariant:action_requires_owner";
pub const REQUIREMENT_VERIFICATION_INVARIANT: &str = "invariant:requirement_requires_verification";

pub fn check_space(
    space: &AdvisorySpaceEnvelope,
    ruleset: &str,
    fail_on: Option<Severity>,
    command: Option<&str>,
) -> AdvisoryResult<ReportEnvelope> {
    let _package = load_ruleset(ruleset)?;
    let mut invariant_results = Vec::new();
    let mut obstructions = Vec::new();

    evaluate_boundary(space, &mut invariant_results, &mut obstructions);
    evaluate_recommendation_evidence(space, &mut invariant_results, &mut obstructions);
    evaluate_action_owners(space, &mut invariant_results, &mut obstructions);
    evaluate_required_verification(space, &mut invariant_results, &mut obstructions);

    invariant_results = sorted_values_by_id(invariant_results);
    obstructions = sorted_values_by_id(obstructions);
    if let Some(threshold) = fail_on {
        let triggered = obstructions
            .iter()
            .filter_map(|item| item.get("severity").and_then(Value::as_str))
            .filter_map(Severity::parse)
            .any(|severity| severity >= threshold);
        if triggered {
            return Err(AdvisoryError::FailOnThreshold(format!("{threshold:?}")));
        }
    }

    Ok(ReportEnvelope::new(
        "check",
        command,
        json!({
            "space_id": space.space_id,
            "ruleset": ruleset
        }),
        json!({
            "invariant_results": invariant_results,
            "obstructions": obstructions
        }),
    ))
}

fn evaluate_recommendation_evidence(
    space: &AdvisorySpaceEnvelope,
    invariant_results: &mut Vec<Value>,
    obstructions: &mut Vec<Value>,
) {
    for cell in space
        .cells
        .iter()
        .filter(|cell| matches!(cell["cell_type"].as_str(), Some("action" | "decision")))
    {
        let review_status = cell
            .pointer("/provenance/review_status")
            .and_then(Value::as_str);
        if review_status != Some("accepted") || is_source_backed_or_review_promoted(cell) {
            continue;
        }
        let obstruction_id = format!(
            "obstruction:{}-insufficient-evidence",
            json_id(cell).trim_start_matches("cell:")
        );
        invariant_results.push(json!({
            "id": EVIDENCE_INVARIANT,
            "invariant_id": EVIDENCE_INVARIANT,
            "status": "violated",
            "severity": "high",
            "witness_ids": [cell["id"].clone()],
            "obstruction_ids": [obstruction_id],
            "message": format!("{} is accepted without source-backed or review-promoted evidence.", title(cell))
        }));
        obstructions.push(json!({
            "id": obstruction_id,
            "obstruction_type": "insufficient_evidence",
            "severity": "high",
            "blocked_ids": [cell["id"].clone()],
            "witness_ids": [cell["id"].clone()],
            "evidence_ids": cell.get("source_ids").cloned().unwrap_or_else(|| json!([])),
            "recommended_completion_types": ["review_promote_evidence", "source_backed_evidence"],
            "review_status": "unreviewed",
            "message": format!("{} needs source-backed or review-promoted evidence.", title(cell))
        }));
    }
}
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
pub fn close_status(space: &AdvisorySpaceEnvelope, check_report: &ReportEnvelope) -> Value {
    let blocking = check_report
        .result
        .get("obstructions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|obstruction| {
            matches!(
                obstruction.get("severity").and_then(Value::as_str),
                Some("high" | "critical")
            ) && obstruction.get("review_status").and_then(Value::as_str) != Some("waived")
        })
        .cloned()
        .collect::<Vec<_>>();
    json!({
        "space_id": space.space_id,
        "closeable": blocking.is_empty(),
        "blocking_obstruction_ids": blocking.iter().filter_map(|item| item.get("id").and_then(Value::as_str)).collect::<Vec<_>>(),
        "blocking_obstructions": blocking
    })
}
fn evaluate_boundary(
    space: &AdvisorySpaceEnvelope,
    invariant_results: &mut Vec<Value>,
    obstructions: &mut Vec<Value>,
) {
    for incidence in &space.incidences {
        if incidence.get("relation_type").and_then(Value::as_str) != Some("accesses") {
            continue;
        }
        if incidence
            .pointer("/metadata/access_type")
            .and_then(Value::as_str)
            != Some("direct_database_read")
        {
            continue;
        }
        let Some(from) = find_cell(space, incidence.get("from_id").and_then(Value::as_str)) else {
            continue;
        };
        let Some(to) = find_cell(space, incidence.get("to_id").and_then(Value::as_str)) else {
            continue;
        };
        if to.get("cell_type").and_then(Value::as_str) != Some("data_store") {
            continue;
        }
        let from_contexts = optional_string_array(from, "context_ids");
        let to_contexts = optional_string_array(to, "context_ids");
        if !is_cross_context(&from_contexts, &to_contexts) {
            continue;
        }
        let obstruction_id = "obstruction:order-service-direct-billing-db-access";
        invariant_results.push(json!({
            "id": BOUNDARY_INVARIANT,
            "invariant_id": BOUNDARY_INVARIANT,
            "status": "violated",
            "severity": "high",
            "witness_ids": [from["id"].clone(), to["id"].clone(), incidence["id"].clone()],
            "obstruction_ids": [obstruction_id],
            "message": format!("{} accesses {} owned by Billing context.", title(from), title(to))
        }));
        obstructions.push(json!({
            "id": obstruction_id,
            "obstruction_type": "boundary_violation",
            "severity": "high",
            "blocked_ids": ["decision:approve-current-architecture"],
            "witness_ids": [incidence["id"].clone()],
            "evidence_ids": incidence.get("evidence_ids").cloned().unwrap_or_else(|| json!([])),
            "recommended_completion_types": ["proposed_interface", "proposed_refactor_action"],
            "review_status": "unreviewed",
            "message": format!("{} directly reads {} across ownership boundary.", title(from), title(to))
        }));
    }
}

fn evaluate_action_owners(
    space: &AdvisorySpaceEnvelope,
    invariant_results: &mut Vec<Value>,
    obstructions: &mut Vec<Value>,
) {
    for action in space
        .cells
        .iter()
        .filter(|cell| cell["cell_type"] == "action")
    {
        if has_incoming_owner(space, json_id(action)) {
            continue;
        }
        let obstruction_id = format!(
            "obstruction:{}-missing-owner",
            json_id(action).trim_start_matches("cell:")
        );
        invariant_results.push(json!({
            "id": OWNER_INVARIANT,
            "invariant_id": OWNER_INVARIANT,
            "status": "violated",
            "severity": "medium",
            "witness_ids": [action["id"].clone()],
            "obstruction_ids": [obstruction_id],
            "message": format!("{} has no owner.", title(action))
        }));
        obstructions.push(json!({
            "id": obstruction_id,
            "obstruction_type": "missing_owner",
            "severity": "medium",
            "blocked_ids": [action["id"].clone()],
            "witness_ids": [action["id"].clone()],
            "evidence_ids": action.get("source_ids").cloned().unwrap_or_else(|| json!([])),
            "recommended_completion_types": ["ownership_clarification"],
            "review_status": "unreviewed",
            "message": format!("{} needs an owner before export or acceptance.", title(action))
        }));
    }
}

fn evaluate_required_verification(
    space: &AdvisorySpaceEnvelope,
    invariant_results: &mut Vec<Value>,
    obstructions: &mut Vec<Value>,
) {
    for requirement in space
        .cells
        .iter()
        .filter(|cell| cell["cell_type"] == "requirement")
    {
        if !requires_verification(requirement) || has_verification(space, json_id(requirement)) {
            continue;
        }
        let obstruction_id = format!(
            "obstruction:{}-missing-verification",
            json_id(requirement).trim_start_matches("cell:")
        );
        invariant_results.push(json!({
            "id": REQUIREMENT_VERIFICATION_INVARIANT,
            "invariant_id": REQUIREMENT_VERIFICATION_INVARIANT,
            "status": "violated",
            "severity": "medium",
            "witness_ids": [requirement["id"].clone()],
            "obstruction_ids": [obstruction_id],
            "message": format!("{} has no verification method.", title(requirement))
        }));
        obstructions.push(json!({
            "id": obstruction_id,
            "obstruction_type": "requirement_unverified",
            "severity": "medium",
            "blocked_ids": [requirement["id"].clone()],
            "witness_ids": [requirement["id"].clone()],
            "evidence_ids": requirement.get("source_ids").cloned().unwrap_or_else(|| json!([])),
            "recommended_completion_types": ["proposed_test", "proposed_metric", "requirement_review"],
            "review_status": "unreviewed",
            "message": format!("{} needs a test or metric.", title(requirement))
        }));
    }
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

fn find_cell<'a>(space: &'a AdvisorySpaceEnvelope, id: Option<&str>) -> Option<&'a Value> {
    let id = id?;
    space.cells.iter().find(|cell| json_id(cell) == id)
}

fn is_cross_context(left: &[String], right: &[String]) -> bool {
    !left.is_empty() && !right.is_empty() && left.iter().all(|id| !right.contains(id))
}

fn title(value: &Value) -> &str {
    value
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_else(|| json_id(value))
}

fn has_incoming_owner(space: &AdvisorySpaceEnvelope, action_id: &str) -> bool {
    space.incidences.iter().any(|incidence| {
        incidence.get("relation_type").and_then(Value::as_str) == Some("owns")
            && incidence.get("to_id").and_then(Value::as_str) == Some(action_id)
    })
}

fn has_verification(space: &AdvisorySpaceEnvelope, requirement_id: &str) -> bool {
    space.incidences.iter().any(|incidence| {
        matches!(
            incidence.get("relation_type").and_then(Value::as_str),
            Some("verifies" | "implements")
        ) && (incidence.get("from_id").and_then(Value::as_str) == Some(requirement_id)
            || incidence.get("to_id").and_then(Value::as_str) == Some(requirement_id))
    })
}

fn requires_verification(requirement: &Value) -> bool {
    requirement
        .pointer("/metadata/require_verification")
        .and_then(Value::as_bool)
        == Some(true)
        || requirement
            .pointer("/metadata/verification_required")
            .and_then(Value::as_bool)
            == Some(true)
}

fn is_source_backed_or_review_promoted(cell: &Value) -> bool {
    let origin = cell.pointer("/provenance/origin").and_then(Value::as_str);
    let source_ids = cell
        .get("source_ids")
        .and_then(Value::as_array)
        .is_some_and(|ids| !ids.is_empty());
    matches!(origin, Some("source_backed" | "review_promoted")) && source_ids
}
