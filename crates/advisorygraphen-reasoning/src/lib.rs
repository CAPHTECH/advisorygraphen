use advisorygraphen_core::{
    json_id, sorted_values_by_id, AdvisoryError, AdvisoryResult, AdvisorySpaceEnvelope,
    HigherGraphenAdvisorySpace, ReportEnvelope, Severity,
};
use advisorygraphen_interpretation::load_ruleset;
use higher_graphen_core::Id as HigherId;
use serde_json::{json, Value};

mod completions;
pub use completions::propose_completions;

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
    let higher_space = space.to_higher_graphen()?;
    let mut invariant_results = Vec::new();
    let mut obstructions = Vec::new();

    evaluate_boundary(
        space,
        &higher_space,
        &mut invariant_results,
        &mut obstructions,
    );
    evaluate_recommendation_evidence(space, &mut invariant_results, &mut obstructions);
    evaluate_action_owners(
        space,
        &higher_space,
        &mut invariant_results,
        &mut obstructions,
    );
    evaluate_required_verification(
        space,
        &higher_space,
        &mut invariant_results,
        &mut obstructions,
    );

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
            "obstructions": obstructions,
            "higher_graphen": higher_space.summary_json()
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
    higher_space: &HigherGraphenAdvisorySpace,
    invariant_results: &mut Vec<Value>,
    obstructions: &mut Vec<Value>,
) {
    for incidence in &space.incidences {
        let Some(higher_incidence) = higher_space.incidence(json_id(incidence)) else {
            continue;
        };
        if higher_incidence.relation_type != "accesses" {
            continue;
        }
        if incidence
            .pointer("/metadata/access_type")
            .and_then(Value::as_str)
            != Some("direct_database_read")
        {
            continue;
        }
        let Some(from) = higher_space.cell(higher_incidence.from_cell_id.as_str()) else {
            continue;
        };
        let Some(to) = higher_space.cell(higher_incidence.to_cell_id.as_str()) else {
            continue;
        };
        if to.cell_type != "data_store" {
            continue;
        }
        let from_contexts = from
            .context_ids
            .iter()
            .map(HigherId::as_str)
            .collect::<Vec<_>>();
        let to_contexts = to
            .context_ids
            .iter()
            .map(HigherId::as_str)
            .collect::<Vec<_>>();
        if !is_cross_context(&from_contexts, &to_contexts) {
            continue;
        }
        let obstruction_id = "obstruction:order-service-direct-billing-db-access";
        let Some(from_advisory) = find_cell(space, Some(higher_incidence.from_cell_id.as_str()))
        else {
            continue;
        };
        let Some(to_advisory) = find_cell(space, Some(higher_incidence.to_cell_id.as_str())) else {
            continue;
        };
        invariant_results.push(json!({
            "id": BOUNDARY_INVARIANT,
            "invariant_id": BOUNDARY_INVARIANT,
            "status": "violated",
            "severity": "high",
            "witness_ids": [from_advisory["id"].clone(), to_advisory["id"].clone(), incidence["id"].clone()],
            "obstruction_ids": [obstruction_id],
            "message": format!("{} accesses {} owned by Billing context.", title(from_advisory), title(to_advisory))
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
            "message": format!("{} directly reads {} across ownership boundary.", title(from_advisory), title(to_advisory))
        }));
    }
}

fn evaluate_action_owners(
    space: &AdvisorySpaceEnvelope,
    higher_space: &HigherGraphenAdvisorySpace,
    invariant_results: &mut Vec<Value>,
    obstructions: &mut Vec<Value>,
) {
    for action in space
        .cells
        .iter()
        .filter(|cell| cell["cell_type"] == "action")
    {
        if has_incoming_owner(higher_space, json_id(action)) {
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
    higher_space: &HigherGraphenAdvisorySpace,
    invariant_results: &mut Vec<Value>,
    obstructions: &mut Vec<Value>,
) {
    for requirement in space
        .cells
        .iter()
        .filter(|cell| cell["cell_type"] == "requirement")
    {
        if !requires_verification(requirement)
            || has_verification(higher_space, json_id(requirement))
        {
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

fn find_cell<'a>(space: &'a AdvisorySpaceEnvelope, id: Option<&str>) -> Option<&'a Value> {
    let id = id?;
    space.cells.iter().find(|cell| json_id(cell) == id)
}

fn is_cross_context(left: &[&str], right: &[&str]) -> bool {
    !left.is_empty() && !right.is_empty() && left.iter().all(|id| !right.contains(id))
}

fn title(value: &Value) -> &str {
    value
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_else(|| json_id(value))
}

fn has_incoming_owner(higher_space: &HigherGraphenAdvisorySpace, action_id: &str) -> bool {
    higher_space.incidence_records().iter().any(|incidence| {
        incidence.relation_type == "owns" && incidence.to_cell_id.as_str() == action_id
    })
}

fn has_verification(higher_space: &HigherGraphenAdvisorySpace, requirement_id: &str) -> bool {
    higher_space.incidence_records().iter().any(|incidence| {
        matches!(incidence.relation_type.as_str(), "verifies" | "implements")
            && (incidence.from_cell_id.as_str() == requirement_id
                || incidence.to_cell_id.as_str() == requirement_id)
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
