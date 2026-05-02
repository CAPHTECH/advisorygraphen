use advisorygraphen_core::{
    json_id, sorted_values_by_id, AdvisoryError, AdvisoryResult, AdvisorySpaceEnvelope,
    HigherGraphenAdvisorySpace, ReportEnvelope, Severity,
};
use advisorygraphen_interpretation::load_ruleset;
use higher_graphen_core::Id as HigherId;
use serde_json::{json, Value};

mod completions;
mod higher;
mod resolution;
pub use completions::propose_completions;
use higher::{has_accepted_supporting_evidence, violation_finding, FindingInput};
pub use resolution::{blocker_resolution_state, frontier_items, waiting_items};

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
    )?;
    evaluate_recommendation_evidence(space, &mut invariant_results, &mut obstructions)?;
    evaluate_action_owners(
        space,
        &higher_space,
        &mut invariant_results,
        &mut obstructions,
    )?;
    evaluate_required_verification(
        space,
        &higher_space,
        &mut invariant_results,
        &mut obstructions,
    )?;

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
) -> AdvisoryResult<()> {
    for cell in space
        .cells
        .iter()
        .filter(|cell| matches!(cell["cell_type"].as_str(), Some("action" | "decision")))
    {
        let review_status = cell
            .pointer("/provenance/review_status")
            .and_then(Value::as_str);
        if review_status != Some("accepted") || has_accepted_supporting_evidence(cell)? {
            continue;
        }
        let obstruction_id = format!(
            "obstruction:{}-insufficient-evidence",
            json_id(cell).trim_start_matches("cell:")
        );
        let finding = violation_finding(FindingInput {
            space_id: &space.space_id,
            invariant_id: EVIDENCE_INVARIANT,
            obstruction_id: &obstruction_id,
            obstruction_type: "insufficient_evidence",
            severity: "high",
            message: format!(
                "{} is accepted without source-backed or review-promoted evidence.",
                title(cell)
            ),
            witness_ids: vec![json_id(cell).to_string()],
            blocked_ids: vec![cell["id"].clone()],
            evidence_ids: cell
                .get("source_ids")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
            recommended_completion_types: vec!["review_promote_evidence", "source_backed_evidence"],
            resolution: "attach source-backed or review-promoted evidence",
        })?;
        invariant_results.push(finding.invariant_result);
        obstructions.push(finding.obstruction);
    }
    Ok(())
}
pub fn close_status(space: &AdvisorySpaceEnvelope, check_report: &ReportEnvelope) -> Value {
    let blocking = check_report
        .result
        .get("obstructions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|obstruction| {
            obstruction
                .get("severity")
                .and_then(Value::as_str)
                .and_then(Severity::parse)
                .is_some_and(|severity| severity >= Severity::Medium)
                && obstruction.get("review_status").and_then(Value::as_str) != Some("waived")
        })
        .cloned()
        .collect::<Vec<_>>();
    json!({
        "space_id": space.space_id,
        "blocking_threshold": "medium",
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
) -> AdvisoryResult<()> {
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
        let finding = violation_finding(FindingInput {
            space_id: &space.space_id,
            invariant_id: BOUNDARY_INVARIANT,
            obstruction_id,
            obstruction_type: "boundary_violation",
            severity: "high",
            message: format!(
                "{} directly reads {} across ownership boundary.",
                title(from_advisory),
                title(to_advisory)
            ),
            witness_ids: vec![
                json_id(from_advisory).to_string(),
                json_id(to_advisory).to_string(),
                json_id(incidence).to_string(),
            ],
            blocked_ids: vec![json!("decision:approve-current-architecture")],
            evidence_ids: incidence
                .get("evidence_ids")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
            recommended_completion_types: vec!["proposed_interface", "proposed_refactor_action"],
            resolution: "replace cross-context direct database access with an explicit interface",
        })?;
        invariant_results.push(finding.invariant_result);
        obstructions.push(finding.obstruction);
    }
    Ok(())
}

fn evaluate_action_owners(
    space: &AdvisorySpaceEnvelope,
    higher_space: &HigherGraphenAdvisorySpace,
    invariant_results: &mut Vec<Value>,
    obstructions: &mut Vec<Value>,
) -> AdvisoryResult<()> {
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
        let finding = violation_finding(FindingInput {
            space_id: &space.space_id,
            invariant_id: OWNER_INVARIANT,
            obstruction_id: &obstruction_id,
            obstruction_type: "missing_owner",
            severity: "medium",
            message: format!("{} has no owner.", title(action)),
            witness_ids: vec![json_id(action).to_string()],
            blocked_ids: vec![action["id"].clone()],
            evidence_ids: action
                .get("source_ids")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
            recommended_completion_types: vec!["ownership_clarification"],
            resolution: "clarify the action owner",
        })?;
        invariant_results.push(finding.invariant_result);
        obstructions.push(finding.obstruction);
    }
    Ok(())
}

fn evaluate_required_verification(
    space: &AdvisorySpaceEnvelope,
    higher_space: &HigherGraphenAdvisorySpace,
    invariant_results: &mut Vec<Value>,
    obstructions: &mut Vec<Value>,
) -> AdvisoryResult<()> {
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
        let finding = violation_finding(FindingInput {
            space_id: &space.space_id,
            invariant_id: REQUIREMENT_VERIFICATION_INVARIANT,
            obstruction_id: &obstruction_id,
            obstruction_type: "requirement_unverified",
            severity: "medium",
            message: format!("{} has no verification method.", title(requirement)),
            witness_ids: vec![json_id(requirement).to_string()],
            blocked_ids: vec![requirement["id"].clone()],
            evidence_ids: requirement
                .get("source_ids")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
            recommended_completion_types: vec![
                "proposed_test",
                "proposed_metric",
                "requirement_review",
            ],
            resolution: "define a test, metric, or review path for the requirement",
        })?;
        invariant_results.push(finding.invariant_result);
        obstructions.push(finding.obstruction);
    }
    Ok(())
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
