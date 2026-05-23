use advisorygraphen_core::{AdvisoryError, AdvisoryResult, AdvisorySpaceEnvelope};
use higher_graphen_core::{Id, ParticipantRef};
use higher_graphen_projection::{project_correspondence, ProjectionAudience, ProjectionPurpose};
use higher_graphen_reasoning::correspondence::{
    derive_correspondence_candidates, CorrespondenceDetectionInput, CorrespondenceScope,
    CorrespondenceSubject, InvariantSatisfaction, InvariantState, TypedRelation,
};
use higher_graphen_reasoning::gluing::attempt_gluing;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const MAX_REVIEW_FOCUS_CORRESPONDENCES: usize = 24;

pub fn correspondence_analysis(
    space: &AdvisorySpaceEnvelope,
    obstructions: &[Value],
    hypotheses: &[Value],
    falsifiers: &[Value],
    candidates: &[Value],
    argumentation_incidences: &[Value],
) -> AdvisoryResult<Value> {
    let relations = relation_triples(space, candidates, falsifiers, argumentation_incidences)?;
    let subjects = subject_seeds(
        space,
        obstructions,
        hypotheses,
        falsifiers,
        candidates,
        &relations,
    )?
    .into_values()
    .map(SubjectSeed::into_subject)
    .collect::<AdvisoryResult<Vec<_>>>()?;
    let context = id("context:advisory-correspondence-analysis")?;
    let provenance = id("source:advisorygraphen-correspondence-analysis")?;
    let subject_count = subjects.len();
    let result = derive_correspondence_candidates(
        CorrespondenceDetectionInput::new(context, provenance, subjects)
            .with_scope(CorrespondenceScope::All),
    )
    .map_err(hg_err)?;

    let mut ranked = Vec::new();
    let mut failure_count = 0_u64;
    let mut review_count = 0_u64;
    let mut success_count = 0_u64;

    for mut candidate in result.candidates {
        let gluing = attempt_gluing(&candidate).map_err(hg_err)?;
        let gluing_value = serde_json::to_value(&gluing)?;
        let gluing_kind = gluing_value["result"]["kind"]
            .as_str()
            .unwrap_or("unknown")
            .to_owned();
        match gluing_kind.as_str() {
            "failure" => failure_count += 1,
            "candidate" => review_count += 1,
            "success" => success_count += 1,
            _ => {}
        }
        candidate.gluing = Some(gluing);
        let projection = serde_json::to_value(project_correspondence(
            &candidate,
            ProjectionAudience::AiAgent,
            ProjectionPurpose::Review,
        ))?;
        let candidate_value = serde_json::to_value(candidate)?;
        let (selection_score, selection_reasons) = selection_score(&candidate_value, &gluing_kind);
        ranked.push(RankedCorrespondence {
            candidate: candidate_value,
            projection,
            gluing_kind,
            selection_score,
            selection_reasons,
        });
    }
    let total_candidate_count = ranked.len();
    ranked.sort_by(|left, right| {
        right
            .selection_score
            .cmp(&left.selection_score)
            .then_with(|| left.id().cmp(&right.id()))
    });
    let selected = ranked
        .into_iter()
        .take(MAX_REVIEW_FOCUS_CORRESPONDENCES)
        .collect::<Vec<_>>();
    let selected_count = selected.len();
    let omitted_count = total_candidate_count.saturating_sub(selected_count);
    let review_focus_summaries = selected
        .iter()
        .enumerate()
        .map(|(index, candidate)| candidate.summary(index + 1))
        .collect::<Vec<_>>();
    let candidate_values = selected
        .iter()
        .map(|candidate| candidate.candidate.clone())
        .collect::<Vec<_>>();
    let projections = selected
        .iter()
        .map(|candidate| candidate.projection.clone())
        .collect::<Vec<_>>();

    Ok(json!({
        "schema": "advisorygraphen.correspondence_analysis.v1",
        "source": "highergraphen_0_5_correspondence_overlap_gluing",
        "subject_count": subject_count,
        "candidate_count": total_candidate_count,
        "emitted_candidate_count": selected_count,
        "omitted_candidate_count": omitted_count,
        "max_emitted_candidates": MAX_REVIEW_FOCUS_CORRESPONDENCES,
        "selection_policy": [
            "Always rank gluing failures first.",
            "Then rank gluing review candidates and blocking differences.",
            "Then rank major differences and non-surface structural witnesses.",
            "Omit low-signal success-only surface or evidence overlaps when the candidate set is large."
        ],
        "gluing_summary": {
            "failure": failure_count,
            "review_candidate": review_count,
            "success": success_count,
            "rule": "Gluing failures and review candidates are structural review prompts, not accepted advisory facts."
        },
        "review_focus_candidates": review_focus_summaries,
        "candidates": candidate_values,
        "ai_agent_projections": projections,
        "agent_rule": "Use review_focus_candidates first. Do not inspect omitted success-only correspondence candidates unless a reviewer asks for full trace expansion."
    }))
}

struct RankedCorrespondence {
    candidate: Value,
    projection: Value,
    gluing_kind: String,
    selection_score: i64,
    selection_reasons: Vec<String>,
}

impl RankedCorrespondence {
    fn id(&self) -> String {
        self.candidate
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned()
    }

    fn summary(&self, rank: usize) -> Value {
        json!({
            "rank": rank,
            "candidate_id": self.candidate.get("id"),
            "selection_score": self.selection_score,
            "selection_reasons": self.selection_reasons,
            "gluing_kind": self.gluing_kind,
            "participants": participant_ids(&self.candidate),
            "overlap_witness_kinds": witness_kinds(&self.candidate),
            "difference_severities": difference_severities(&self.candidate),
            "review_status": self.candidate.get("reviewStatus")
        })
    }
}

#[derive(Clone)]
struct SubjectSeed {
    id: String,
    role: String,
    label: Option<String>,
    modality: Option<String>,
    contexts: BTreeSet<String>,
    evidence: BTreeSet<String>,
    invariants: BTreeSet<String>,
    invariant_states: BTreeMap<String, InvariantSatisfaction>,
    relations: BTreeSet<RelationTriple>,
}

impl SubjectSeed {
    fn new(id: &str, role: &str) -> Self {
        Self {
            id: id.to_owned(),
            role: role.to_owned(),
            label: None,
            modality: None,
            contexts: BTreeSet::new(),
            evidence: BTreeSet::new(),
            invariants: BTreeSet::new(),
            invariant_states: BTreeMap::new(),
            relations: BTreeSet::new(),
        }
    }

    fn absorb_value(&mut self, value: &Value, modality: Option<&str>) {
        self.label = self.label.clone().or_else(|| normalized_label(value));
        self.modality = self
            .modality
            .clone()
            .or_else(|| modality.map(str::to_owned));
        extend_strings(
            &mut self.contexts,
            value,
            &["context_ids", "location_context_ids"],
        );
        extend_strings(&mut self.evidence, value, &["source_ids", "evidence_ids"]);
        extend_strings(
            &mut self.invariants,
            value,
            &["affected_invariant_ids", "invariant_ids"],
        );
        for field in ["violated_invariant_id", "invariant_id"] {
            if let Some(invariant) = value.get(field).and_then(Value::as_str) {
                self.invariants.insert(invariant.to_owned());
                self.invariant_states
                    .insert(invariant.to_owned(), InvariantSatisfaction::Failed);
            }
        }
    }

    fn into_subject(self) -> AdvisoryResult<CorrespondenceSubject> {
        Ok(CorrespondenceSubject {
            participant: ParticipantRef::from_compact_id(id(&self.id)?),
            role: Some(self.role),
            normalized_label: self.label,
            modality: self.modality,
            contexts: ids(self.contexts)?,
            evidence: ids(self.evidence)?,
            invariants: ids(self.invariants)?,
            invariant_states: self
                .invariant_states
                .into_iter()
                .map(|(invariant, satisfaction)| {
                    Ok(InvariantState::new(id(&invariant)?, satisfaction))
                })
                .collect::<AdvisoryResult<Vec<_>>>()?,
            typed_relations: self
                .relations
                .into_iter()
                .map(|relation| {
                    TypedRelation::new(relation.subject, relation.relation, relation.object)
                        .map_err(hg_err)
                })
                .collect::<AdvisoryResult<Vec<_>>>()?,
        })
    }
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
struct RelationTriple {
    subject: String,
    relation: String,
    object: String,
}

fn subject_seeds(
    space: &AdvisorySpaceEnvelope,
    obstructions: &[Value],
    hypotheses: &[Value],
    falsifiers: &[Value],
    candidates: &[Value],
    relations: &[RelationTriple],
) -> AdvisoryResult<BTreeMap<String, SubjectSeed>> {
    let mut seeds = BTreeMap::new();
    for cell in &space.cells {
        if let Some(id) = cell.get("id").and_then(Value::as_str) {
            seed(&mut seeds, id, cell_type(cell)).absorb_value(cell, cell_modality(cell));
        }
    }
    for obstruction in obstructions {
        if let Some(id) = obstruction.get("id").and_then(Value::as_str) {
            seed(&mut seeds, id, "obstruction").absorb_value(obstruction, None);
        }
    }
    for hypothesis in hypotheses {
        if let Some(id) = item_id(hypothesis, &["id", "hypothesis_id"]) {
            let modality =
                hypothesis_status(hypothesis).map(|status| format!("hypothesis:{status}"));
            seed(&mut seeds, id, "hypothesis").absorb_value(hypothesis, modality.as_deref());
        }
    }
    for falsifier in falsifiers {
        if let Some(id) = falsifier.get("id").and_then(Value::as_str) {
            seed(&mut seeds, id, "falsifier").absorb_value(falsifier, Some("falsifier"));
        }
    }
    for candidate in candidates {
        if let Some(id) = candidate.get("id").and_then(Value::as_str) {
            let seed = seed(&mut seeds, id, "completion_candidate");
            seed.absorb_value(candidate, None);
            for invariant in string_values(candidate, "affected_invariant_ids") {
                seed.invariant_states
                    .insert(invariant, InvariantSatisfaction::Satisfied);
            }
        }
    }
    for relation in relations {
        if let Some(seed) = seeds.get_mut(&relation.subject) {
            seed.relations.insert(relation.clone());
        }
        if let Some(seed) = seeds.get_mut(&relation.object) {
            seed.relations.insert(incoming_relation_triple(relation));
        }
    }
    Ok(seeds)
}

fn relation_triples(
    space: &AdvisorySpaceEnvelope,
    candidates: &[Value],
    falsifiers: &[Value],
    argumentation_incidences: &[Value],
) -> AdvisoryResult<Vec<RelationTriple>> {
    let mut relations = Vec::new();
    for incidence in space.incidences.iter().chain(argumentation_incidences) {
        if let (Some(subject), Some(relation), Some(object)) = (
            incidence.get("from_id").and_then(Value::as_str),
            incidence.get("relation_type").and_then(Value::as_str),
            incidence.get("to_id").and_then(Value::as_str),
        ) {
            relations.push(relation_triple(subject, relation, object));
        }
    }
    for candidate in candidates {
        let Some(candidate_id) = candidate.get("id").and_then(Value::as_str) else {
            continue;
        };
        for obstruction_id in string_values(candidate, "resolves_obstruction_ids") {
            relations.push(relation_triple(candidate_id, "resolves", &obstruction_id));
        }
        for hypothesis_id in candidate
            .pointer("/metadata/derived_from_hypothesis_id")
            .and_then(Value::as_str)
            .into_iter()
            .chain(
                candidate
                    .get("supported_hypothesis_ids")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str),
            )
        {
            relations.push(relation_triple(candidate_id, "derives_from", hypothesis_id));
        }
    }
    for falsifier in falsifiers {
        if let (Some(falsifier_id), Some(hypothesis_id)) = (
            falsifier.get("id").and_then(Value::as_str),
            falsifier
                .pointer("/metadata/falsifies")
                .and_then(Value::as_str),
        ) {
            relations.push(relation_triple(falsifier_id, "falsifies", hypothesis_id));
        }
    }
    relations.sort();
    relations.dedup();
    Ok(relations)
}

fn seed<'a>(
    seeds: &'a mut BTreeMap<String, SubjectSeed>,
    id: &str,
    role: &str,
) -> &'a mut SubjectSeed {
    seeds
        .entry(id.to_owned())
        .or_insert_with(|| SubjectSeed::new(id, role))
}

fn relation_triple(subject: &str, relation: &str, object: &str) -> RelationTriple {
    RelationTriple {
        subject: subject.to_owned(),
        relation: relation.to_owned(),
        object: object.to_owned(),
    }
}

fn incoming_relation_triple(relation: &RelationTriple) -> RelationTriple {
    relation_triple(
        &relation.object,
        &format!("incoming:{}", relation.relation),
        &relation.subject,
    )
}

fn selection_score(candidate: &Value, gluing_kind: &str) -> (i64, Vec<String>) {
    let mut score = 0_i64;
    let mut reasons = Vec::new();
    match gluing_kind {
        "failure" => {
            score += 10_000;
            reasons.push("gluing_failure".to_owned());
        }
        "candidate" => {
            score += 9_000;
            reasons.push("gluing_review_candidate".to_owned());
        }
        "success" => {
            score += 100;
            reasons.push("gluing_success_context".to_owned());
        }
        other => {
            reasons.push(format!("gluing_{other}"));
        }
    }
    let severities = difference_severities(candidate);
    if severities.iter().any(|severity| severity == "blocking") {
        score += 8_000;
        reasons.push("blocking_difference".to_owned());
    }
    if severities.iter().any(|severity| severity == "major") {
        score += 6_000;
        reasons.push("major_difference".to_owned());
    }
    if !severities.is_empty() {
        score += 2_000;
        reasons.push("has_difference_witness".to_owned());
    }
    let witness_kinds = witness_kinds(candidate);
    if witness_kinds.iter().any(|kind| {
        matches!(
            kind.as_str(),
            "PredicateSet" | "NormalizedClaim" | "ConstraintSet"
        )
    }) {
        score += 3_000;
        reasons.push("structural_or_constraint_overlap".to_owned());
    }
    let roles = participant_roles(candidate);
    if roles.iter().any(|role| role == "requirement") {
        score += 7_000;
        reasons.push("direct_requirement_participant".to_owned());
    }
    if roles.iter().any(|role| role == "obstruction") {
        score += 7_000;
        reasons.push("direct_obstruction_participant".to_owned());
    }
    if roles.iter().any(|role| role == "evidence") {
        score += 3_500;
        reasons.push("direct_evidence_participant".to_owned());
    }
    if roles.iter().any(|role| {
        matches!(
            role.as_str(),
            "obstruction" | "completion_candidate" | "hypothesis" | "falsifier"
        )
    }) {
        score += 1_000;
        reasons.push("review_relevant_participant_role".to_owned());
    }
    if is_generic_candidate_similarity(&roles, &witness_kinds) {
        score -= 13_000;
        reasons.push("generic_candidate_similarity_deprioritized".to_owned());
    }
    (score, reasons)
}

fn is_generic_candidate_similarity(roles: &[String], witness_kinds: &[String]) -> bool {
    roles.len() == 2
        && roles.iter().all(|role| role == "completion_candidate")
        && witness_kinds.len() == 1
        && witness_kinds[0] == "FeatureSet"
}

fn participant_ids(candidate: &Value) -> Vec<String> {
    candidate
        .get("participants")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|participant| participant.pointer("/ref/id").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

fn participant_roles(candidate: &Value) -> Vec<String> {
    candidate
        .get("participants")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|participant| participant.get("role").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

fn witness_kinds(candidate: &Value) -> Vec<String> {
    unique_strings_from_array(candidate, "overlapWitnesses", "witnessKind")
}

fn difference_severities(candidate: &Value) -> Vec<String> {
    unique_strings_from_array(candidate, "differenceWitnesses", "severity")
}

fn unique_strings_from_array(
    candidate: &Value,
    array_field: &str,
    value_field: &str,
) -> Vec<String> {
    candidate
        .get(array_field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get(value_field).and_then(Value::as_str))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn normalized_label(value: &Value) -> Option<String> {
    ["title", "summary", "message", "rationale"]
        .into_iter()
        .filter_map(|field| value.get(field).and_then(Value::as_str))
        .map(|text| text.trim().to_ascii_lowercase())
        .find(|text| !text.is_empty())
}

fn extend_strings(target: &mut BTreeSet<String>, value: &Value, fields: &[&str]) {
    for field in fields {
        target.extend(string_values(value, field));
    }
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

fn item_id<'a>(value: &'a Value, fields: &[&str]) -> Option<&'a str> {
    fields
        .iter()
        .find_map(|field| value.get(*field).and_then(Value::as_str))
}

fn cell_type(value: &Value) -> &str {
    value
        .get("cell_type")
        .and_then(Value::as_str)
        .unwrap_or("cell")
}

fn cell_modality(value: &Value) -> Option<&str> {
    match cell_type(value) {
        "hypothesis" => hypothesis_status(value),
        "falsifier" => Some("falsifier"),
        _ => None,
    }
}

fn hypothesis_status(value: &Value) -> Option<&str> {
    value
        .pointer("/metadata/hypothesis_status")
        .and_then(Value::as_str)
        .or_else(|| value.get("lifecycle_status").and_then(Value::as_str))
        .or_else(|| value.get("status").and_then(Value::as_str))
}

fn ids(values: BTreeSet<String>) -> AdvisoryResult<Vec<Id>> {
    values.into_iter().map(|value| id(&value)).collect()
}

fn id(value: &str) -> AdvisoryResult<Id> {
    Id::new(value).map_err(hg_err)
}

fn hg_err(error: higher_graphen_core::CoreError) -> AdvisoryError {
    AdvisoryError::Validation(format!("higher-graphen correspondence: {error}"))
}
