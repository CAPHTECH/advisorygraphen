use advisorygraphen_core::{
    json_id, sorted_values_by_id, AdvisoryResult, AdvisorySpaceEnvelope, ReportEnvelope,
};
use higher_graphen_core::{Confidence, Id};
use higher_graphen_reasoning::completion::{
    CompletionCandidate, CompletionDetectionResult, MissingType, SuggestedStructure,
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
            Some("boundary_violation") => candidates.extend(boundary_completion_candidates(space)?),
            Some("missing_owner") => {
                candidates.push(owner_completion_candidate(space, &obstruction)?)
            }
            Some("requirement_unverified") => {
                candidates.push(verification_completion_candidate(space, &obstruction)?)
            }
            _ => {}
        }
    }
    let higher_candidates = candidates
        .iter()
        .filter_map(|candidate| candidate.get("higher_graphen").cloned())
        .map(serde_json::from_value)
        .collect::<Result<Vec<CompletionCandidate>, _>>()?;
    let higher_detection = CompletionDetectionResult::new(
        hg_id(&space.space_id)?,
        space
            .contexts
            .iter()
            .map(|context| hg_id(json_id(context)))
            .collect::<AdvisoryResult<Vec<_>>>()?,
        higher_candidates,
    )
    .map_err(hg_err)?;
    candidates = sorted_values_by_id(candidates);
    Ok(ReportEnvelope::new(
        "completion_proposal",
        command,
        json!({
            "space_id": space.space_id,
            "from_report": from_report
        }),
        json!({
            "completion_candidates": candidates,
            "higher_graphen": higher_detection
        }),
    ))
}

fn boundary_completion_candidates(space: &AdvisorySpaceEnvelope) -> AdvisoryResult<Vec<Value>> {
    Ok(vec![
        completion_candidate(CandidateSpec {
            space,
            id: "candidate:billing-status-api".to_string(),
            candidate_type: "proposed_interface",
            title: "Add Billing status query API",
            rationale: "Remove cross-context direct database access while preserving billing status check.".to_string(),
            resolves_obstruction_ids: vec!["obstruction:order-service-direct-billing-db-access".to_string()],
            proposed_cell_ids: vec!["cell:billing-status-api".to_string()],
            source_ids: vec!["source:architecture-note".to_string()],
            confidence: 0.82,
            missing_type: MissingType::Cell,
            suggested_structure_type: "interface_cell",
        })?,
        completion_candidate(CandidateSpec {
            space,
            id: "candidate:replace-order-service-db-read".to_string(),
            candidate_type: "proposed_refactor_action",
            title: "Replace Order Service direct DB read with Billing API call",
            rationale: "Order Service should depend on Billing Service interface instead of Billing DB ownership boundary.".to_string(),
            resolves_obstruction_ids: vec!["obstruction:order-service-direct-billing-db-access".to_string()],
            proposed_cell_ids: vec!["cell:action-replace-direct-db-read".to_string()],
            source_ids: vec!["source:architecture-note".to_string()],
            confidence: 0.78,
            missing_type: MissingType::Cell,
            suggested_structure_type: "refactor_action_cell",
        })?,
    ])
}

fn owner_completion_candidate(
    space: &AdvisorySpaceEnvelope,
    obstruction: &Value,
) -> AdvisoryResult<Value> {
    completion_candidate(CandidateSpec {
        space,
        id: format!(
            "candidate:{}-owner",
            json_id(obstruction).trim_start_matches("obstruction:")
        ),
        candidate_type: "ownership_clarification",
        title: "Clarify action owner",
        rationale: obstruction
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Action requires owner.")
            .to_string(),
        resolves_obstruction_ids: vec![json_id(obstruction).to_string()],
        proposed_cell_ids: Vec::new(),
        source_ids: Vec::new(),
        confidence: 0.7,
        missing_type: MissingType::Cell,
        suggested_structure_type: "owner_cell",
    })
}

fn verification_completion_candidate(
    space: &AdvisorySpaceEnvelope,
    obstruction: &Value,
) -> AdvisoryResult<Value> {
    completion_candidate(CandidateSpec {
        space,
        id: format!(
            "candidate:{}-verification",
            json_id(obstruction).trim_start_matches("obstruction:")
        ),
        candidate_type: "proposed_test",
        title: "Define verification method",
        rationale: obstruction
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Requirement needs verification.")
            .to_string(),
        resolves_obstruction_ids: vec![json_id(obstruction).to_string()],
        proposed_cell_ids: Vec::new(),
        source_ids: Vec::new(),
        confidence: 0.7,
        missing_type: MissingType::Cell,
        suggested_structure_type: "verification_cell",
    })
}

struct CandidateSpec<'a> {
    space: &'a AdvisorySpaceEnvelope,
    id: String,
    candidate_type: &'a str,
    title: &'a str,
    rationale: String,
    resolves_obstruction_ids: Vec<String>,
    proposed_cell_ids: Vec<String>,
    source_ids: Vec<String>,
    confidence: f64,
    missing_type: MissingType,
    suggested_structure_type: &'a str,
}

fn completion_candidate(spec: CandidateSpec<'_>) -> AdvisoryResult<Value> {
    let id = spec.id;
    let rationale = spec.rationale;
    let related_ids = spec
        .resolves_obstruction_ids
        .iter()
        .chain(spec.proposed_cell_ids.iter())
        .map(|id| hg_id(id))
        .collect::<AdvisoryResult<Vec<_>>>()?;
    let suggested_structure = SuggestedStructure::new(spec.suggested_structure_type, spec.title)
        .map_err(hg_err)?
        .with_related_ids(related_ids);
    let suggested_structure = match spec.proposed_cell_ids.first() {
        Some(cell_id) => suggested_structure.with_structure_id(hg_id(cell_id)?),
        None => suggested_structure,
    };
    let higher_candidate = CompletionCandidate::new(
        hg_id(&id)?,
        hg_id(&spec.space.space_id)?,
        spec.missing_type,
        suggested_structure,
        spec.resolves_obstruction_ids
            .iter()
            .map(|id| hg_id(id))
            .collect::<AdvisoryResult<Vec<_>>>()?,
        rationale.clone(),
        Confidence::new(spec.confidence).map_err(hg_err)?,
    )
    .map_err(hg_err)?;

    Ok(json!({
        "id": id,
        "candidate_type": spec.candidate_type,
        "title": spec.title,
        "rationale": rationale,
        "resolves_obstruction_ids": spec.resolves_obstruction_ids,
        "proposed_cell_ids": spec.proposed_cell_ids,
        "source_ids": spec.source_ids,
        "confidence": spec.confidence,
        "review_status": "unreviewed",
        "metadata": {},
        "higher_graphen": higher_candidate
    }))
}

fn hg_id(value: &str) -> AdvisoryResult<Id> {
    Id::new(value).map_err(hg_err)
}

fn hg_err(error: higher_graphen_core::CoreError) -> advisorygraphen_core::AdvisoryError {
    advisorygraphen_core::AdvisoryError::Validation(format!("higher-graphen completion: {error}"))
}
