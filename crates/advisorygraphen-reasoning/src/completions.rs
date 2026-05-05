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
            Some("boundary_violation") => {
                candidates.extend(boundary_completion_candidates(space, &obstruction)?)
            }
            Some("missing_owner") => {
                candidates.push(owner_completion_candidate(space, &obstruction)?)
            }
            Some("requirement_unverified") => {
                candidates.push(verification_completion_candidate(space, &obstruction)?)
            }
            Some("api_route_missing_auth") => {
                candidates.push(auth_guard_completion_candidate(space, &obstruction)?)
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

fn boundary_completion_candidates(
    space: &AdvisorySpaceEnvelope,
    obstruction: &Value,
) -> AdvisoryResult<Vec<Value>> {
    let from_id = obstruction
        .pointer("/metadata/from_cell_id")
        .and_then(Value::as_str)
        .or_else(|| witness_cell_id(space, obstruction, |cell| cell["cell_type"] != "data_store"));
    let to_id = obstruction
        .pointer("/metadata/to_cell_id")
        .and_then(Value::as_str)
        .or_else(|| witness_cell_id(space, obstruction, |cell| cell["cell_type"] == "data_store"));
    let Some(from_cell) = find_cell(space, from_id) else {
        return Ok(Vec::new());
    };
    let Some(to_cell) = find_cell(space, to_id) else {
        return Ok(Vec::new());
    };
    let obstruction_id = json_id(obstruction).to_string();
    let from_title = title(from_cell);
    let to_title = title(to_cell);
    let domain_title = data_store_domain_title(to_title);
    let domain_id = id_suffix(json_id(to_cell))
        .trim_end_matches("-db")
        .to_string();
    let source_ids = completion_source_ids(space, obstruction);
    let evidence_strength = if source_ids.is_empty() {
        "rule_derived_without_source_ids"
    } else {
        "source_backed_obstruction"
    };
    let stem = obstruction_id.trim_start_matches("obstruction:");
    let h_implicit_interface = format!("hypothesis:{stem}-implicit-interface");
    Ok(vec![
        completion_candidate(CandidateSpec {
            space,
            id: format!("candidate:{domain_id}-status-api"),
            candidate_type: "proposed_interface",
            title: format!("Add {domain_title} status query API"),
            rationale: format!(
                "Remove cross-context direct database access while preserving {} status check.",
                domain_title.to_ascii_lowercase()
            ),
            resolves_obstruction_ids: vec![obstruction_id.clone()],
            proposed_cell_ids: vec![format!("cell:{domain_id}-status-api")],
            source_ids: source_ids.clone(),
            confidence: 0.82,
            missing_type: MissingType::Cell,
            suggested_structure_type: "interface_cell",
            metadata: json!({
                "specificity": "source_derived",
                "evidence_strength": evidence_strength,
                "precision_note": "Derived from boundary violation witness cells and obstruction evidence_ids.",
                "derived_from_hypothesis_id": h_implicit_interface,
                "from_cell_id": json_id(from_cell),
                "to_cell_id": json_id(to_cell)
            }),
        })?,
        completion_candidate(CandidateSpec {
            space,
            id: format!("candidate:replace-{}-db-read", id_suffix(json_id(from_cell))),
            candidate_type: "proposed_refactor_action",
            title: format!("Replace {from_title} direct DB read with {domain_title} API call"),
            rationale: format!(
                "{from_title} should depend on {domain_title} Service interface instead of {to_title} ownership boundary."
            ),
            resolves_obstruction_ids: vec![obstruction_id],
            proposed_cell_ids: vec![format!("cell:action-replace-{}-direct-db-read", id_suffix(json_id(from_cell)))],
            source_ids,
            confidence: 0.78,
            missing_type: MissingType::Cell,
            suggested_structure_type: "refactor_action_cell",
            metadata: json!({
                "specificity": "source_derived",
                "evidence_strength": evidence_strength,
                "precision_note": "Derived from boundary violation witness cells and obstruction evidence_ids.",
                "derived_from_hypothesis_id": h_implicit_interface,
                "from_cell_id": json_id(from_cell),
                "to_cell_id": json_id(to_cell)
            }),
        })?,
    ])
}

fn owner_completion_candidate(
    space: &AdvisorySpaceEnvelope,
    obstruction: &Value,
) -> AdvisoryResult<Value> {
    let stem = json_id(obstruction).trim_start_matches("obstruction:");
    let h_unassigned = format!("hypothesis:{stem}-no-team-holds-action");
    completion_candidate(CandidateSpec {
        space,
        id: format!("candidate:{stem}-owner"),
        candidate_type: "ownership_clarification",
        title: "Clarify action owner".to_string(),
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
        metadata: json!({
            "specificity": "generic",
            "evidence_strength": "obstruction_message",
            "precision_note": "Identifies the missing owner relation but does not infer a specific owner.",
            "derived_from_hypothesis_id": h_unassigned
        }),
    })
}

fn verification_completion_candidate(
    space: &AdvisorySpaceEnvelope,
    obstruction: &Value,
) -> AdvisoryResult<Value> {
    let stem = json_id(obstruction).trim_start_matches("obstruction:");
    let h_genuinely_missing = format!("hypothesis:{stem}-verification-genuinely-missing");
    completion_candidate(CandidateSpec {
        space,
        id: format!("candidate:{stem}-verification"),
        candidate_type: "proposed_test",
        title: "Define verification method".to_string(),
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
        metadata: json!({
            "specificity": "requirement_derived",
            "evidence_strength": "obstruction_message",
            "precision_note": "Identifies the verification gap but does not infer a concrete test implementation.",
            "derived_from_hypothesis_id": h_genuinely_missing
        }),
    })
}

fn auth_guard_completion_candidate(
    space: &AdvisorySpaceEnvelope,
    obstruction: &Value,
) -> AdvisoryResult<Value> {
    let route_path = obstruction
        .pointer("/metadata/route_path")
        .and_then(Value::as_str)
        .unwrap_or("API route");
    let source_ids = completion_source_ids(space, obstruction);
    let evidence_strength = if source_ids.is_empty() {
        "rule_derived_without_source_ids"
    } else {
        "source_backed_obstruction"
    };
    let stem = json_id(obstruction).trim_start_matches("obstruction:");
    let h_unprotected = format!("hypothesis:{stem}-truly-unprotected");
    completion_candidate(CandidateSpec {
        space,
        id: format!("candidate:{stem}-auth-guard"),
        candidate_type: "proposed_auth_guard",
        title: format!("Add authentication guard to {route_path}"),
        rationale: obstruction
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Database-touching API route requires authentication.")
            .to_string(),
        resolves_obstruction_ids: vec![json_id(obstruction).to_string()],
        proposed_cell_ids: Vec::new(),
        source_ids,
        confidence: 0.72,
        missing_type: MissingType::Cell,
        suggested_structure_type: "auth_guard_cell",
        metadata: json!({
            "specificity": "code_derived",
            "evidence_strength": evidence_strength,
            "precision_note": "Derived from code snapshot route metadata. The candidate must be reviewed because lexical detection can miss shared middleware or dynamic auth wrappers.",
            "derived_from_hypothesis_id": h_unprotected,
            "route_path": route_path,
            "http_methods": obstruction.pointer("/metadata/http_methods").cloned().unwrap_or_else(|| json!([]))
        }),
    })
}

struct CandidateSpec<'a> {
    space: &'a AdvisorySpaceEnvelope,
    id: String,
    candidate_type: &'a str,
    title: String,
    rationale: String,
    resolves_obstruction_ids: Vec<String>,
    proposed_cell_ids: Vec<String>,
    source_ids: Vec<String>,
    confidence: f64,
    missing_type: MissingType,
    suggested_structure_type: &'a str,
    metadata: Value,
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
    let suggested_structure = SuggestedStructure::new(spec.suggested_structure_type, &spec.title)
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
        "metadata": spec.metadata,
        "higher_graphen": higher_candidate
    }))
}

fn find_cell<'a>(space: &'a AdvisorySpaceEnvelope, id: Option<&str>) -> Option<&'a Value> {
    let id = id?;
    space.cells.iter().find(|cell| json_id(cell) == id)
}

fn witness_cell_id<'a>(
    space: &'a AdvisorySpaceEnvelope,
    obstruction: &'a Value,
    predicate: impl Fn(&Value) -> bool,
) -> Option<&'a str> {
    obstruction
        .get("witness_ids")
        .and_then(Value::as_array)?
        .iter()
        .filter_map(Value::as_str)
        .find(|id| find_cell(space, Some(id)).map(&predicate).unwrap_or(false))
}

fn title(value: &Value) -> &str {
    value
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_else(|| json_id(value))
}

fn data_store_domain_title(title: &str) -> String {
    title
        .trim_end_matches(" Database")
        .trim_end_matches(" database")
        .trim_end_matches(" DB")
        .trim_end_matches(" db")
        .to_string()
}

fn completion_source_ids(space: &AdvisorySpaceEnvelope, obstruction: &Value) -> Vec<String> {
    let mut source_ids = obstruction
        .get("evidence_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .flat_map(|id| {
            if id.starts_with("source:") {
                vec![id.to_string()]
            } else {
                evidence_cell_source_ids(space, id)
            }
        })
        .collect::<Vec<_>>();
    source_ids.sort();
    source_ids.dedup();
    source_ids
}

fn evidence_cell_source_ids(space: &AdvisorySpaceEnvelope, evidence_id: &str) -> Vec<String> {
    let Some(cell) = find_cell(space, Some(evidence_id)) else {
        return Vec::new();
    };
    cell.pointer("/metadata/source_id")
        .and_then(Value::as_str)
        .map(|id| vec![id.to_string()])
        .unwrap_or_else(|| {
            cell.get("source_ids")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
}

fn id_suffix(id: &str) -> String {
    id.rsplit_once(':')
        .map(|(_, suffix)| suffix)
        .unwrap_or(id)
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn hg_id(value: &str) -> AdvisoryResult<Id> {
    Id::new(value).map_err(hg_err)
}

fn hg_err(error: higher_graphen_core::CoreError) -> advisorygraphen_core::AdvisoryError {
    advisorygraphen_core::AdvisoryError::Validation(format!("higher-graphen completion: {error}"))
}
