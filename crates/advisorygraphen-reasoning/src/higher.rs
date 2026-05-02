use advisorygraphen_core::{json_id, AdvisoryError, AdvisoryResult};
use higher_graphen_core::{
    Confidence, Id, Provenance, ReviewStatus, Severity, SourceKind, SourceRef,
};
use higher_graphen_evidence::confidence::{
    update_confidence, ConfidenceEvidence, ConfidenceUpdateInput, ConfidenceUpdateRecord,
    EvidenceLikelihood,
};
use higher_graphen_reasoning::invariant::{CheckResult, CheckStatus, CheckTargetKind, Violation};
use higher_graphen_reasoning::obstruction::{
    Counterexample, Obstruction, ObstructionExplanation, ObstructionType, RequiredResolution,
};
use serde_json::{json, Value};

pub struct AdvisoryFinding {
    pub invariant_result: Value,
    pub obstruction: Value,
}

pub struct FindingInput<'a> {
    pub space_id: &'a str,
    pub invariant_id: &'a str,
    pub obstruction_id: &'a str,
    pub obstruction_type: &'a str,
    pub severity: &'a str,
    pub message: String,
    pub witness_ids: Vec<String>,
    pub blocked_ids: Vec<Value>,
    pub evidence_ids: Vec<Value>,
    pub recommended_completion_types: Vec<&'a str>,
    pub resolution: &'a str,
}

pub fn violation_finding(input: FindingInput<'_>) -> AdvisoryResult<AdvisoryFinding> {
    let severity = severity(input.severity)?;
    let location_cell_ids = input
        .witness_ids
        .iter()
        .filter(|id| id.starts_with("cell:"))
        .cloned()
        .collect::<Vec<_>>();
    let location_hg_ids = hg_ids(&location_cell_ids)?;
    let check_result = CheckResult::violated(
        CheckTargetKind::Invariant,
        hg_id(input.invariant_id)?,
        Violation::new(input.message.clone(), severity)
            .with_location_cells(location_hg_ids.clone()),
    );
    let obstruction = obstruction_from_input(&input, severity, location_hg_ids)?;

    Ok(AdvisoryFinding {
        invariant_result: advisory_invariant_result(&check_result, &input)?,
        obstruction: advisory_obstruction(&obstruction, &input)?,
    })
}

pub fn evidence_record_for_cell(cell: &Value) -> AdvisoryResult<ConfidenceUpdateRecord> {
    let source_ids = cell
        .get("source_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let evidence = source_ids
        .iter()
        .map(|source_id| supporting_evidence(source_id))
        .collect::<AdvisoryResult<Vec<_>>>()?;
    let input =
        ConfidenceUpdateInput::new(hg_id(json_id(cell))?, Confidence::new(0.5).map_err(hg_err)?)
            .with_supporting_evidence(evidence);
    let record = update_confidence(input).map_err(hg_err)?;
    let review_status = cell
        .pointer("/provenance/review_status")
        .and_then(Value::as_str)
        .map(review_status)
        .transpose()?
        .unwrap_or_default();
    Ok(record.with_review_status(review_status))
}

pub fn has_accepted_supporting_evidence(cell: &Value) -> AdvisoryResult<bool> {
    let record = evidence_record_for_cell(cell)?;
    Ok(record.is_review_accepted() && !record.supporting_evidence.is_empty())
}

fn supporting_evidence(source_id: &str) -> AdvisoryResult<ConfidenceEvidence> {
    let source_hg_id = hg_id(source_id)?;
    let likelihood = EvidenceLikelihood::new(
        Confidence::new(0.8).map_err(hg_err)?,
        Confidence::new(0.2).map_err(hg_err)?,
    )
    .map_err(hg_err)?;
    ConfidenceEvidence::new(
        hg_id(&format!("evidence-support:{source_id}"))?,
        format!("source-backed support from {source_id}"),
        likelihood,
    )
    .map(|evidence| evidence.with_source_ids(vec![source_hg_id]))
    .map_err(hg_err)
}

fn obstruction_from_input(
    input: &FindingInput<'_>,
    severity: Severity,
    witness_hg_ids: Vec<Id>,
) -> AdvisoryResult<Obstruction> {
    let mut resolution = RequiredResolution::new(input.resolution).map_err(hg_err)?;
    for witness_id in &witness_hg_ids {
        resolution = resolution.with_target_cell(witness_id.clone());
    }
    let mut obstruction = Obstruction::new(
        hg_id(input.obstruction_id)?,
        hg_id(input.space_id)?,
        ObstructionType::custom(input.obstruction_type).map_err(hg_err)?,
        ObstructionExplanation::new(input.message.clone()).map_err(hg_err)?,
        severity,
        advisory_provenance(),
    )
    .with_counterexample(witness_hg_ids.iter().cloned().fold(
        Counterexample::new(input.message.clone()).map_err(hg_err)?,
        |counterexample, id| counterexample.with_path_cell(id),
    ))
    .with_required_resolution(resolution);
    for witness_id in witness_hg_ids {
        obstruction = obstruction.with_location_cell(witness_id);
    }
    Ok(obstruction)
}

fn advisory_invariant_result(
    result: &CheckResult,
    input: &FindingInput<'_>,
) -> AdvisoryResult<Value> {
    let violation = result
        .violation()
        .ok_or_else(|| AdvisoryError::Validation("missing HigherGraphen violation".to_string()))?;
    Ok(json!({
        "id": result.target_id().as_str(),
        "invariant_id": result.target_id().as_str(),
        "status": check_status(result.status()),
        "severity": violation.severity.as_str(),
        "witness_ids": input.witness_ids.clone(),
        "obstruction_ids": [input.obstruction_id],
        "message": violation.message,
        "higher_graphen": serde_json::to_value(result)?
    }))
}

fn advisory_obstruction(
    obstruction: &Obstruction,
    input: &FindingInput<'_>,
) -> AdvisoryResult<Value> {
    Ok(json!({
        "id": obstruction.id.as_str(),
        "obstruction_type": input.obstruction_type,
        "severity": obstruction.severity.as_str(),
        "blocked_ids": input.blocked_ids.clone(),
        "witness_ids": input.witness_ids.clone(),
        "evidence_ids": input.evidence_ids.clone(),
        "recommended_completion_types": input.recommended_completion_types.clone(),
        "review_status": "unreviewed",
        "message": obstruction.explanation.summary,
        "higher_graphen": serde_json::to_value(obstruction)?
    }))
}

fn check_status(status: CheckStatus) -> &'static str {
    match status {
        CheckStatus::Satisfied => "satisfied",
        CheckStatus::Violated => "violated",
        CheckStatus::Unsupported => "unsupported",
    }
}

fn severity(value: &str) -> AdvisoryResult<Severity> {
    value.try_into().map_err(hg_err)
}

fn review_status(value: &str) -> AdvisoryResult<ReviewStatus> {
    value.try_into().map_err(hg_err)
}

fn hg_ids(ids: &[String]) -> AdvisoryResult<Vec<Id>> {
    ids.iter().map(|id| hg_id(id)).collect()
}

fn hg_id(value: &str) -> AdvisoryResult<Id> {
    Id::new(value).map_err(hg_err)
}

fn advisory_provenance() -> Provenance {
    Provenance::new(
        SourceRef::new(SourceKind::Code),
        Confidence::new(1.0).expect("literal confidence is valid"),
    )
    .with_review_status(ReviewStatus::Unreviewed)
}

fn hg_err(error: higher_graphen_core::CoreError) -> AdvisoryError {
    AdvisoryError::Validation(format!("higher-graphen reasoning: {error}"))
}
