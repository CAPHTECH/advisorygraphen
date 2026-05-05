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
        let invariant_ids = obstruction_invariant_ids(check_report, json_id(&obstruction));
        match obstruction.get("obstruction_type").and_then(Value::as_str) {
            Some("boundary_violation") => candidates.extend(boundary_completion_candidates(
                space,
                &obstruction,
                &invariant_ids,
            )?),
            Some("missing_owner") => candidates.push(owner_completion_candidate(
                space,
                &obstruction,
                &invariant_ids,
            )?),
            Some("requirement_unverified") => candidates.push(verification_completion_candidate(
                space,
                &obstruction,
                &invariant_ids,
            )?),
            Some("api_route_missing_auth") => candidates.push(auth_guard_completion_candidate(
                space,
                &obstruction,
                &invariant_ids,
            )?),
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
    invariant_ids: &[String],
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
            affected_invariant_ids: invariant_ids.to_vec(),
            witness_ids: obstruction_string_array(&obstruction, "witness_ids"),
            blocked_ids: obstruction_string_array(&obstruction, "blocked_ids"),
            proposed_incidence_ids: Vec::new(),
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
            affected_invariant_ids: invariant_ids.to_vec(),
            witness_ids: obstruction_string_array(&obstruction, "witness_ids"),
            blocked_ids: obstruction_string_array(&obstruction, "blocked_ids"),
            proposed_incidence_ids: Vec::new(),
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
    invariant_ids: &[String],
) -> AdvisoryResult<Value> {
    let stem = json_id(obstruction).trim_start_matches("obstruction:");
    let h_unassigned = format!("hypothesis:{stem}-no-team-holds-action");
    let source_ids = completion_source_ids(space, obstruction);
    let blocked_cell = blocked_cell(space, obstruction);
    if let Some((blocked, owner)) = blocked_cell.and_then(|blocked| {
        best_related_cell(space, blocked, &["owner"]).map(|owner| (blocked, owner))
    }) {
        let blocked_suffix = id_suffix(json_id(blocked));
        let owner_suffix = id_suffix(json_id(owner));
        return completion_candidate(CandidateSpec {
            space,
            id: format!("candidate:{stem}-assign-{owner_suffix}"),
            candidate_type: "owner_assignment",
            title: format!("Assign {} as owner for {}", title(owner), title(blocked)),
            rationale: format!(
                "{} shares source or context with the unowned action {} and can be reviewed as the explicit owner.",
                title(owner),
                title(blocked)
            ),
            resolves_obstruction_ids: vec![json_id(obstruction).to_string()],
            proposed_cell_ids: Vec::new(),
            proposed_incidence_ids: vec![format!(
                "incidence:{owner_suffix}-owns-{blocked_suffix}"
            )],
            source_ids,
            affected_invariant_ids: invariant_ids.to_vec(),
            witness_ids: obstruction_string_array(obstruction, "witness_ids"),
            blocked_ids: obstruction_string_array(obstruction, "blocked_ids"),
            confidence: related_cell_confidence(blocked, owner, 0.76),
            missing_type: MissingType::Incidence,
            suggested_structure_type: "ownership_incidence",
            metadata: json!({
                "specificity": "source_derived",
                "evidence_strength": "related_owner_cell",
                "precision_note": "Derived by matching an existing owner cell to the blocked action through shared context or source IDs.",
                "derived_from_hypothesis_id": format!("hypothesis:{stem}-de-facto-owner-link-missing"),
                "owner_cell_id": json_id(owner),
                "blocked_cell_id": json_id(blocked)
            }),
        });
    }
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
        proposed_incidence_ids: Vec::new(),
        source_ids,
        affected_invariant_ids: invariant_ids.to_vec(),
        witness_ids: obstruction_string_array(obstruction, "witness_ids"),
        blocked_ids: obstruction_string_array(obstruction, "blocked_ids"),
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
    invariant_ids: &[String],
) -> AdvisoryResult<Value> {
    let stem = json_id(obstruction).trim_start_matches("obstruction:");
    let h_genuinely_missing = format!("hypothesis:{stem}-verification-genuinely-missing");
    let source_ids = completion_source_ids(space, obstruction);
    let blocked_cell = blocked_cell(space, obstruction);
    if let Some((blocked, verification)) = blocked_cell.and_then(|blocked| {
        best_related_cell(space, blocked, &["test_or_verification", "metric"])
            .map(|verification| (blocked, verification))
    }) {
        let blocked_suffix = id_suffix(json_id(blocked));
        let verification_suffix = id_suffix(json_id(verification));
        return completion_candidate(CandidateSpec {
            space,
            id: format!("candidate:{stem}-link-{verification_suffix}"),
            candidate_type: "lift_verification_link",
            title: format!("Link {} as verification for {}", title(verification), title(blocked)),
            rationale: format!(
                "{} appears related to the unverified requirement {} and can be reviewed as its verifies relation.",
                title(verification),
                title(blocked)
            ),
            resolves_obstruction_ids: vec![json_id(obstruction).to_string()],
            proposed_cell_ids: Vec::new(),
            proposed_incidence_ids: vec![format!(
                "incidence:{verification_suffix}-verifies-{blocked_suffix}"
            )],
            source_ids,
            affected_invariant_ids: invariant_ids.to_vec(),
            witness_ids: obstruction_string_array(obstruction, "witness_ids"),
            blocked_ids: obstruction_string_array(obstruction, "blocked_ids"),
            confidence: related_cell_confidence(blocked, verification, 0.78),
            missing_type: MissingType::Incidence,
            suggested_structure_type: "verification_incidence",
            metadata: json!({
                "specificity": "source_derived",
                "evidence_strength": "related_verification_cell",
                "precision_note": "Derived by matching an existing test, metric, or verification cell to the blocked requirement through shared context or source IDs.",
                "derived_from_hypothesis_id": format!("hypothesis:{stem}-verification-link-not-lifted"),
                "verification_cell_id": json_id(verification),
                "blocked_cell_id": json_id(blocked)
            }),
        });
    }
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
        proposed_incidence_ids: Vec::new(),
        source_ids,
        affected_invariant_ids: invariant_ids.to_vec(),
        witness_ids: obstruction_string_array(obstruction, "witness_ids"),
        blocked_ids: obstruction_string_array(obstruction, "blocked_ids"),
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
    invariant_ids: &[String],
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
        proposed_incidence_ids: Vec::new(),
        source_ids,
        affected_invariant_ids: invariant_ids.to_vec(),
        witness_ids: obstruction_string_array(obstruction, "witness_ids"),
        blocked_ids: obstruction_string_array(obstruction, "blocked_ids"),
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
    proposed_incidence_ids: Vec<String>,
    source_ids: Vec<String>,
    affected_invariant_ids: Vec<String>,
    witness_ids: Vec<String>,
    blocked_ids: Vec<String>,
    confidence: f64,
    missing_type: MissingType,
    suggested_structure_type: &'a str,
    metadata: Value,
}

fn completion_candidate(spec: CandidateSpec<'_>) -> AdvisoryResult<Value> {
    let proposal_content = proposal_content(&spec.id, &spec.rationale, &spec);
    let id = spec.id;
    let rationale = spec.rationale;
    let related_ids = spec
        .resolves_obstruction_ids
        .iter()
        .chain(spec.proposed_cell_ids.iter())
        .chain(spec.proposed_incidence_ids.iter())
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
        "proposed_incidence_ids": spec.proposed_incidence_ids,
        "source_ids": spec.source_ids,
        "confidence": spec.confidence,
        "review_status": "unreviewed",
        "proposal_content": proposal_content,
        "metadata": spec.metadata,
        "higher_graphen": higher_candidate
    }))
}

fn proposal_content(id: &str, rationale: &str, spec: &CandidateSpec<'_>) -> Value {
    let stem = id_suffix(id);
    let scenario_id = format!("scenario:{stem}-planned");
    let morphism_id = format!("morphism:{stem}-as-is-to-proposed");
    let derivation_id = format!("derivation:{stem}-proposal");
    let valuation_id = format!("valuation:{stem}-proposal");
    let policy_id = format!("policy:{stem}-review-gate");
    let candidate_ref = json!({ "object_type": "completion_candidate", "id": id });
    let required_witnesses = required_witnesses(spec);
    let known_witnesses = known_witnesses(spec);
    let content_obstructions = proposal_content_obstructions(spec, &required_witnesses);

    json!({
        "schema": "advisorygraphen.proposal_content.v1",
        "review_status": "unreviewed",
        "scenario": {
            "id": scenario_id,
            "base_space": spec.space.space_id,
            "scenario_kind": "planned",
            "assumptions": hypothesis_assumptions(&spec.metadata),
            "changed_structures": {
                "added": proposed_structure_ids(spec),
                "removed": [],
                "modified": spec.blocked_ids
            },
            "reachable_from": {
                "ref": spec.space.space_id,
                "via_morphisms": [morphism_id.clone()]
            },
            "affected_invariants": spec.affected_invariant_ids,
            "expected_obstructions": spec.resolves_obstruction_ids,
            "required_witnesses": required_witnesses,
            "valuations": [valuation_id.clone()],
            "status": if content_obstructions.is_empty() { "candidate" } else { "blocked" },
            "review_status": "candidate"
        },
        "morphism": {
            "id": morphism_id,
            "morphism_type": "as_is_to_to_be",
            "source_space": spec.space.space_id,
            "target_scenario": scenario_id,
            "repairs_obstructions": spec.resolves_obstruction_ids,
            "preserved_invariants": spec.affected_invariant_ids,
            "changed_cell_ids": spec.proposed_cell_ids,
            "changed_incidence_ids": spec.proposed_incidence_ids,
            "distortion": proposal_distortion(spec),
            "composition_constraints": [
                "candidate_to_accepted_structure requires an explicit completion review event",
                "accepted completion must be materialized before the blocker is treated as resolved"
            ],
            "review_status": "unreviewed"
        },
        "invariant_checks": proposal_invariant_checks(spec),
        "derivation": {
            "id": derivation_id,
            "conclusion": id,
            "premises": proposal_premises(spec),
            "inference_rule": {
                "id": format!("rule:completion-{}", spec.candidate_type),
                "name": format!("Generate {} completion candidate from obstruction", spec.candidate_type),
                "interpretation_package": "technical_advisory_mvp"
            },
            "warrants": known_witnesses,
            "excluded_premises": [],
            "counterexamples": [],
            "verifier": null,
            "verification_status": "unverified",
            "failure_mode": if spec.source_ids.is_empty() { "missing_premise" } else { "none" },
            "rationale": rationale,
            "review_status": "candidate"
        },
        "witnesses": witness_records(spec),
        "valuation": {
            "id": valuation_id,
            "target": candidate_ref,
            "valuation_context": spec.space.space_id,
            "order_type": "partial_order",
            "criteria": [
                { "criterion_id": "obstruction_resolution", "name": "Obstruction resolution", "direction": "maximize", "required": true },
                { "criterion_id": "evidence_backing", "name": "Evidence backing", "direction": "maximize", "required": true },
                { "criterion_id": "review_safety", "name": "Review safety", "direction": "preserve", "required": true }
            ],
            "values": [
                { "criterion_id": "obstruction_resolution", "value": spec.resolves_obstruction_ids.len(), "evidence": known_or_synthetic_witness(spec) },
                { "criterion_id": "evidence_backing", "value": spec.source_ids.len(), "evidence": known_or_synthetic_witness(spec) },
                { "criterion_id": "review_safety", "value": true, "evidence": policy_id }
            ],
            "tradeoffs": proposal_tradeoffs(spec),
            "confidence": spec.confidence,
            "review_status": "candidate"
        },
        "policy": {
            "id": policy_id,
            "policy_type": "completion_review_gate",
            "target": candidate_ref,
            "rules": [
                "AI agents may propose this content but must not accept it as current state",
                "Acceptance requires explicit review and blocker application requirements",
                "Missing required witnesses remain proposal-content obstructions"
            ],
            "required_witnesses": required_witnesses,
            "review_status": "candidate"
        },
        "content_obstructions": content_obstructions
    })
}

fn proposal_premises(spec: &CandidateSpec<'_>) -> Vec<String> {
    let mut premises = spec.resolves_obstruction_ids.clone();
    premises.extend(spec.proposed_cell_ids.clone());
    premises.extend(spec.proposed_incidence_ids.clone());
    premises.extend(spec.witness_ids.clone());
    premises.extend(spec.source_ids.clone());
    premises.sort();
    premises.dedup();
    premises
}

fn known_witnesses(spec: &CandidateSpec<'_>) -> Vec<String> {
    let mut witnesses = spec.source_ids.clone();
    witnesses.extend(spec.witness_ids.clone());
    witnesses.sort();
    witnesses.dedup();
    witnesses
}

fn required_witnesses(spec: &CandidateSpec<'_>) -> Vec<String> {
    let mut witnesses = vec![format!("witness:{}-review", id_suffix(&spec.id))];
    match spec.candidate_type {
        "proposed_interface" => witnesses.push(format!(
            "witness:{}-interface-contract",
            id_suffix(&spec.id)
        )),
        "proposed_refactor_action" => {
            witnesses.push(format!("witness:{}-migration-plan", id_suffix(&spec.id)))
        }
        "ownership_clarification" | "owner_assignment" => witnesses.push(format!(
            "witness:{}-owner-confirmation",
            id_suffix(&spec.id)
        )),
        "proposed_test" | "lift_verification_link" => witnesses.push(format!(
            "witness:{}-verification-method",
            id_suffix(&spec.id)
        )),
        "proposed_auth_guard" => witnesses.push(format!(
            "witness:{}-auth-behavior-check",
            id_suffix(&spec.id)
        )),
        _ => {}
    }
    witnesses
}

fn proposal_invariant_checks(spec: &CandidateSpec<'_>) -> Vec<Value> {
    let mut checks = spec
        .affected_invariant_ids
        .iter()
        .map(|id| {
            json!({
                "invariant_id": id,
                "status": "candidate_repair",
                "target_candidate_id": spec.id,
                "review_status": "unreviewed"
            })
        })
        .collect::<Vec<_>>();
    checks.push(json!({
        "invariant_id": "invariant:completion-candidate-review-gated",
        "status": "preserved",
        "target_candidate_id": spec.id,
        "review_status": "unreviewed"
    }));
    checks
}

fn witness_records(spec: &CandidateSpec<'_>) -> Vec<Value> {
    known_witnesses(spec)
        .into_iter()
        .map(|id| {
            let witness_type = if id.starts_with("source:") {
                "source_reference"
            } else {
                "structure_reference"
            };
            json!({
                "id": id,
                "witness_type": witness_type,
                "supports": [spec.id],
                "validity_contexts": [spec.space.space_id],
                "review_status": "candidate"
            })
        })
        .collect()
}

fn proposal_tradeoffs(spec: &CandidateSpec<'_>) -> Vec<Value> {
    match spec.candidate_type {
        "proposed_interface" => vec![json!({
            "gains": "Replaces direct ownership-boundary access with an explicit interface.",
            "losses": "Adds an interface contract that still needs owner, compatibility, and verification witnesses.",
            "affected_invariants": spec.affected_invariant_ids
        })],
        "proposed_refactor_action" => vec![json!({
            "gains": "Moves the violating caller toward the proposed boundary-preserving structure.",
            "losses": "Requires migration sequencing and regression evidence before acceptance.",
            "affected_invariants": spec.affected_invariant_ids
        })],
        "owner_assignment" => vec![json!({
            "gains": "Reuses an existing owner cell and proposes only the missing ownership relation.",
            "losses": "The owner match still needs review because shared context or source IDs are suggestive, not proof of responsibility.",
            "affected_invariants": spec.affected_invariant_ids
        })],
        "ownership_clarification" => vec![json!({
            "gains": "Makes execution accountability explicit.",
            "losses": "Does not infer a concrete owner without a reviewed ownership witness.",
            "affected_invariants": spec.affected_invariant_ids
        })],
        "lift_verification_link" => vec![json!({
            "gains": "Reuses an existing verification structure and proposes the missing verifies relation.",
            "losses": "The verification match still needs review because shared context or source IDs do not prove coverage.",
            "affected_invariants": spec.affected_invariant_ids
        })],
        "proposed_test" => vec![json!({
            "gains": "Turns an unverified requirement into a checkable obligation.",
            "losses": "The concrete test or metric still needs design and review.",
            "affected_invariants": spec.affected_invariant_ids
        })],
        "proposed_auth_guard" => vec![json!({
            "gains": "Adds an explicit security control candidate for a database-touching route.",
            "losses": "Shared middleware and intended-public exceptions still need reviewed witnesses.",
            "affected_invariants": spec.affected_invariant_ids
        })],
        _ => vec![json!({
            "gains": "May resolve the linked obstruction.",
            "losses": "Proposal content requires review before acceptance.",
            "affected_invariants": spec.affected_invariant_ids
        })],
    }
}

fn proposal_distortion(spec: &CandidateSpec<'_>) -> Vec<Value> {
    let mut distortion = Vec::new();
    if proposed_structure_ids(spec).is_empty() {
        distortion.push(json!({
            "distortion_type": "underspecified_structure",
            "summary": "Candidate describes a missing structure type but does not yet name the concrete cell to materialize."
        }));
    }
    if spec.source_ids.is_empty() {
        distortion.push(json!({
            "distortion_type": "weak_source_backing",
            "summary": "Candidate is derived from an obstruction without direct source IDs."
        }));
    }
    distortion
}

fn proposal_content_obstructions(
    spec: &CandidateSpec<'_>,
    required_witnesses: &[String],
) -> Vec<Value> {
    let mut obstructions = Vec::new();
    if proposed_structure_ids(spec).is_empty() {
        obstructions.push(json!({
            "obstruction_type": "proposal_content_underspecified",
            "message": "Proposal content does not yet identify concrete structures to add.",
            "required_resolution": "Add concrete proposed cells or incidences before treating the candidate as structurally complete.",
            "review_status": "unreviewed"
        }));
    }
    if spec.source_ids.is_empty() {
        obstructions.push(json!({
            "obstruction_type": "proposal_content_missing_source_witness",
            "message": "Proposal content lacks source-backed witnesses.",
            "required_witnesses": required_witnesses,
            "review_status": "unreviewed"
        }));
    }
    obstructions
}

fn proposed_structure_ids(spec: &CandidateSpec<'_>) -> Vec<String> {
    let mut ids = spec.proposed_cell_ids.clone();
    ids.extend(spec.proposed_incidence_ids.clone());
    ids
}

fn hypothesis_assumptions(metadata: &Value) -> Vec<String> {
    metadata
        .get("derived_from_hypothesis_id")
        .and_then(Value::as_str)
        .map(|id| vec![id.to_string()])
        .unwrap_or_default()
}

fn known_or_synthetic_witness(spec: &CandidateSpec<'_>) -> String {
    known_witnesses(spec)
        .into_iter()
        .next()
        .unwrap_or_else(|| format!("witness:{}-proposal-generated", id_suffix(&spec.id)))
}

fn find_cell<'a>(space: &'a AdvisorySpaceEnvelope, id: Option<&str>) -> Option<&'a Value> {
    let id = id?;
    space.cells.iter().find(|cell| json_id(cell) == id)
}

fn blocked_cell<'a>(space: &'a AdvisorySpaceEnvelope, obstruction: &Value) -> Option<&'a Value> {
    obstruction
        .get("blocked_ids")
        .and_then(Value::as_array)?
        .iter()
        .filter_map(Value::as_str)
        .find_map(|id| find_cell(space, Some(id)))
}

fn best_related_cell<'a>(
    space: &'a AdvisorySpaceEnvelope,
    blocked: &Value,
    cell_types: &[&str],
) -> Option<&'a Value> {
    space
        .cells
        .iter()
        .filter(|cell| {
            let cell_type = cell.get("cell_type").and_then(Value::as_str);
            cell_type.is_some_and(|value| cell_types.contains(&value))
        })
        .filter(|cell| related_cell_score(blocked, cell) > 0)
        .max_by_key(|cell| related_cell_score(blocked, cell))
}

fn related_cell_confidence(blocked: &Value, candidate: &Value, base: f64) -> f64 {
    let score = related_cell_score(blocked, candidate);
    let bump = if score >= 3 {
        0.08
    } else if score >= 2 {
        0.04
    } else {
        0.0
    };
    (base + bump).min(0.88)
}

fn related_cell_score(blocked: &Value, candidate: &Value) -> usize {
    let context_overlap = overlap_count(
        &optional_strings(blocked, "context_ids"),
        &optional_strings(candidate, "context_ids"),
    );
    let source_overlap = overlap_count(
        &optional_strings(blocked, "source_ids"),
        &optional_strings(candidate, "source_ids"),
    );
    (context_overlap * 2) + source_overlap
}

fn overlap_count(left: &[String], right: &[String]) -> usize {
    left.iter().filter(|item| right.contains(item)).count()
}

fn optional_strings(value: &Value, field: &str) -> Vec<String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
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

fn obstruction_string_array(obstruction: &Value, field: &str) -> Vec<String> {
    match obstruction.get(field) {
        Some(Value::String(value)) => vec![value.to_string()],
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn obstruction_invariant_ids(check_report: &ReportEnvelope, obstruction_id: &str) -> Vec<String> {
    let mut invariant_ids = check_report
        .result
        .get("invariant_results")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|result| {
            result
                .get("obstruction_ids")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .any(|id| id.as_str() == Some(obstruction_id))
        })
        .filter_map(|result| result.get("invariant_id").and_then(Value::as_str))
        .map(str::to_string)
        .collect::<Vec<_>>();
    invariant_ids.sort();
    invariant_ids.dedup();
    invariant_ids
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
