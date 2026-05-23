use advisorygraphen_core::{json_id, AdvisoryError, AdvisoryResult, AdvisorySpaceEnvelope};
use higher_graphen_core::{Id, ParticipantRef};
use higher_graphen_reasoning::correspondence::{
    derive_correspondence_candidates, CorrespondenceDetectionInput, CorrespondenceScope,
    CorrespondenceSubject, InvariantSatisfaction, InvariantState, TypedRelation,
};
use higher_graphen_reasoning::gluing::attempt_gluing;
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub fn candidate_gluing_review(
    space: &AdvisorySpaceEnvelope,
    candidate: &Value,
    before_obstructions: &[Value],
    after_obstructions: &[Value],
    cells: &[Value],
    incidences: &[Value],
    removed_incidence_ids: &[String],
) -> AdvisoryResult<Value> {
    let subjects = subjects(
        space,
        candidate,
        before_obstructions,
        after_obstructions,
        cells,
        incidences,
        removed_incidence_ids,
    )?;
    let subject_count = subjects.len();
    let result = derive_correspondence_candidates(
        CorrespondenceDetectionInput::new(
            id("context:completion-dry-run-gluing")?,
            id("source:advisorygraphen-completion-dry-run")?,
            subjects,
        )
        .with_scope(CorrespondenceScope::All),
    )
    .map_err(hg_err)?;

    let mut correspondences = Vec::new();
    let mut failure_count = 0_u64;
    let mut review_count = 0_u64;
    let mut success_count = 0_u64;
    let mut blocking_difference_ids = BTreeSet::new();
    let mut preserved_structure_ids = BTreeSet::new();
    let mut preserved_invariant_ids = BTreeSet::new();

    for mut correspondence in result.candidates {
        let gluing = attempt_gluing(&correspondence).map_err(hg_err)?;
        let gluing_value = serde_json::to_value(&gluing)?;
        match gluing_value["result"]["kind"].as_str().unwrap_or("unknown") {
            "failure" => failure_count += 1,
            "candidate" => review_count += 1,
            "success" => success_count += 1,
            _ => {}
        }
        collect_strings(
            &mut preserved_structure_ids,
            &gluing_value["preservationReport"]["preservedStructures"],
        );
        collect_strings(
            &mut preserved_invariant_ids,
            &gluing_value["preservationReport"]["preservedInvariants"],
        );
        for difference in &correspondence.difference_witnesses {
            let value = serde_json::to_value(difference)?;
            if value["severity"] == "blocking" {
                if let Some(id) = value["id"].as_str() {
                    blocking_difference_ids.insert(id.to_owned());
                }
            }
        }
        correspondence.gluing = Some(gluing);
        correspondences.push(serde_json::to_value(correspondence)?);
    }

    let has_blocking_differences = !blocking_difference_ids.is_empty();
    let policy_blockers =
        policy_blockers_from_counts(failure_count, review_count, has_blocking_differences);

    Ok(json!({
        "schema": "advisorygraphen.completion_dry_run.gluing_review.v1",
        "source": "highergraphen_0_5_correspondence_overlap_gluing",
        "subject_count": subject_count,
        "correspondence_count": correspondences.len(),
        "gluing_summary": {
            "failure": failure_count,
            "review_candidate": review_count,
            "success": success_count
        },
        "preserved_structure_ids": preserved_structure_ids.into_iter().collect::<Vec<_>>(),
        "preserved_invariant_ids": preserved_invariant_ids.into_iter().collect::<Vec<_>>(),
        "blocking_difference_ids": blocking_difference_ids.into_iter().collect::<Vec<_>>(),
        "correspondences": correspondences,
        "policy_blockers": policy_blockers,
        "review_rule": "Dry-run gluing evidence is a review input. It does not accept the completion candidate or mutate the case store."
    }))
}

pub fn skipped_candidate_gluing_review(candidate: &Value, reason: &str) -> Value {
    json!({
        "schema": "advisorygraphen.completion_dry_run.gluing_review.v1",
        "source": "highergraphen_0_5_correspondence_overlap_gluing",
        "subject_count": if json_id(candidate).is_empty() { 0 } else { 1 },
        "correspondence_count": 0,
        "gluing_summary": {
            "failure": 0,
            "review_candidate": 0,
            "success": 0
        },
        "preserved_structure_ids": [],
        "preserved_invariant_ids": [],
        "blocking_difference_ids": [],
        "correspondences": [],
        "policy_blockers": ["dry_run_candidate_not_materialized"],
        "skip_reason": reason,
        "review_rule": "No gluing attempt was run because the candidate was not materialized."
    })
}

pub fn policy_blockers(review: &Value) -> Vec<Value> {
    review
        .get("policy_blockers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn subjects(
    space: &AdvisorySpaceEnvelope,
    candidate: &Value,
    before_obstructions: &[Value],
    after_obstructions: &[Value],
    cells: &[Value],
    incidences: &[Value],
    removed_incidence_ids: &[String],
) -> AdvisoryResult<Vec<CorrespondenceSubject>> {
    let candidate_invariants = candidate_invariants(candidate);
    let candidate_evidence = string_values(candidate, "source_ids");
    let mut subjects = vec![subject(
        json_id(candidate),
        "completion_candidate",
        candidate,
        Some("completion_candidate"),
        &candidate_invariants,
        Some(InvariantSatisfaction::Satisfied),
        relations_for_candidate(candidate),
    )?];

    let resolved_ids = string_values(candidate, "resolves_obstruction_ids")
        .into_iter()
        .collect::<BTreeSet<_>>();
    for obstruction in before_obstructions {
        if resolved_ids.contains(json_id(obstruction)) {
            subjects.push(subject(
                json_id(obstruction),
                "pre_apply_obstruction",
                obstruction,
                Some("obstruction_failed"),
                &candidate_invariants,
                Some(InvariantSatisfaction::Failed),
                Vec::new(),
            )?);
        }
    }

    let before_ids = before_obstructions
        .iter()
        .map(json_id)
        .collect::<BTreeSet<_>>();
    for obstruction in after_obstructions {
        if !before_ids.contains(json_id(obstruction)) {
            subjects.push(subject(
                json_id(obstruction),
                "introduced_obstruction",
                obstruction,
                Some("obstruction_failed"),
                &candidate_invariants,
                Some(InvariantSatisfaction::Failed),
                Vec::new(),
            )?);
        }
    }

    for cell in cells {
        subjects.push(subject(
            json_id(cell),
            "dry_run_cell",
            cell,
            Some("dry_run_added"),
            &candidate_invariants,
            Some(InvariantSatisfaction::Satisfied),
            Vec::new(),
        )?);
    }
    for incidence in incidences {
        subjects.push(subject(
            json_id(incidence),
            "dry_run_incidence",
            incidence,
            Some("dry_run_added"),
            &candidate_invariants,
            Some(InvariantSatisfaction::Satisfied),
            incidence_relation(incidence),
        )?);
    }
    for incidence_id in removed_incidence_ids {
        subjects.push(CorrespondenceSubject {
            participant: ParticipantRef::from_compact_id(id(incidence_id)?),
            role: Some("removed_incidence".to_owned()),
            normalized_label: Some(incidence_id.to_owned()),
            modality: Some("dry_run_removed".to_owned()),
            contexts: Vec::new(),
            evidence: ids(candidate_evidence.iter())?,
            invariants: ids(candidate_invariants.iter())?,
            invariant_states: invariant_states(
                &candidate_invariants,
                InvariantSatisfaction::Satisfied,
            )?,
            typed_relations: Vec::new(),
        });
    }

    if subjects.len() == 1 {
        subjects.extend(space.cells.iter().take(1).filter_map(|cell| {
            subject(
                json_id(cell),
                "space_context_cell",
                cell,
                Some("existing"),
                &[],
                None,
                Vec::new(),
            )
            .ok()
        }));
    }

    Ok(subjects)
}

fn subject(
    participant_id: &str,
    role: &str,
    value: &Value,
    modality: Option<&str>,
    fallback_invariants: &[String],
    invariant_state: Option<InvariantSatisfaction>,
    typed_relations: Vec<TypedRelation>,
) -> AdvisoryResult<CorrespondenceSubject> {
    let invariants = own_or_fallback_invariants(value, fallback_invariants);
    Ok(CorrespondenceSubject {
        participant: ParticipantRef::from_compact_id(id(participant_id)?),
        role: Some(role.to_owned()),
        normalized_label: normalized_label(value),
        modality: modality.map(str::to_owned),
        contexts: ids(string_values(value, "context_ids").iter())?,
        evidence: ids(string_values(value, "source_ids")
            .iter()
            .chain(string_values(value, "evidence_ids").iter()))?,
        invariants: ids(invariants.iter())?,
        invariant_states: match invariant_state {
            Some(state) => invariant_states(&invariants, state)?,
            None => Vec::new(),
        },
        typed_relations,
    })
}

fn candidate_invariants(candidate: &Value) -> Vec<String> {
    let mut invariants = string_values(candidate, "affected_invariant_ids");
    invariants.extend(string_values_at(
        candidate,
        "/proposal_content/morphism/preserved_invariants",
    ));
    invariants.extend(string_values_at(
        candidate,
        "/proposal_content/scenario/affected_invariants",
    ));
    invariants.sort();
    invariants.dedup();
    invariants
}

fn own_or_fallback_invariants(value: &Value, fallback: &[String]) -> Vec<String> {
    let mut invariants = string_values(value, "affected_invariant_ids");
    for field in ["violated_invariant_id", "invariant_id"] {
        if let Some(id) = value.get(field).and_then(Value::as_str) {
            invariants.push(id.to_owned());
        }
    }
    if invariants.is_empty() {
        invariants.extend(fallback.iter().cloned());
    }
    invariants.sort();
    invariants.dedup();
    invariants
}

fn relations_for_candidate(candidate: &Value) -> Vec<TypedRelation> {
    let candidate_id = json_id(candidate);
    string_values(candidate, "resolves_obstruction_ids")
        .into_iter()
        .filter_map(|obstruction_id| {
            TypedRelation::new(candidate_id, "resolves", obstruction_id).ok()
        })
        .collect()
}

fn incidence_relation(incidence: &Value) -> Vec<TypedRelation> {
    let Some(from_id) = incidence.get("from_id").and_then(Value::as_str) else {
        return Vec::new();
    };
    let Some(relation_type) = incidence.get("relation_type").and_then(Value::as_str) else {
        return Vec::new();
    };
    let Some(to_id) = incidence.get("to_id").and_then(Value::as_str) else {
        return Vec::new();
    };
    TypedRelation::new(from_id, relation_type, to_id)
        .ok()
        .into_iter()
        .collect()
}

fn invariant_states(
    invariants: &[String],
    state: InvariantSatisfaction,
) -> AdvisoryResult<Vec<InvariantState>> {
    invariants
        .iter()
        .map(|invariant| Ok(InvariantState::new(id(invariant)?, state)))
        .collect()
}

fn normalized_label(value: &Value) -> Option<String> {
    ["title", "summary", "message", "rationale"]
        .into_iter()
        .filter_map(|field| value.get(field).and_then(Value::as_str))
        .map(|text| text.trim().to_ascii_lowercase())
        .find(|text| !text.is_empty())
}

fn string_values(value: &Value, field: &str) -> Vec<String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn string_values_at(value: &Value, pointer: &str) -> Vec<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn ids<'a>(values: impl IntoIterator<Item = &'a String>) -> AdvisoryResult<Vec<Id>> {
    values.into_iter().map(|value| id(value)).collect()
}

fn collect_strings(target: &mut BTreeSet<String>, value: &Value) {
    target.extend(
        value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned),
    );
}

fn policy_blockers_from_counts(
    failure_count: u64,
    review_count: u64,
    has_blocking_differences: bool,
) -> Vec<&'static str> {
    let mut blockers = Vec::new();
    if failure_count > 0 {
        blockers.push("gluing_failure_requires_explicit_review");
    }
    if review_count > 0 {
        blockers.push("gluing_candidate_requires_review");
    }
    if has_blocking_differences {
        blockers.push("blocking_difference_requires_revision_or_override");
    }
    blockers
}

fn id(value: &str) -> AdvisoryResult<Id> {
    Id::new(value).map_err(hg_err)
}

fn hg_err(error: higher_graphen_core::CoreError) -> AdvisoryError {
    AdvisoryError::Validation(format!("higher-graphen dry-run gluing: {error}"))
}
