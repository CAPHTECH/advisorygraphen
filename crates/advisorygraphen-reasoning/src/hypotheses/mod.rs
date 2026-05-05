//! Hypothesis layer: for each obstruction, emit competing structural
//! hypotheses with falsifiers and suggested completion candidates. This is the
//! `Observation -> Evidence -> Hypothesis -> Obstruction -> CompletionCandidate
//! -> Proposal` pipeline made explicit in AdvisoryGraphen, ahead of HG-side
//! support for the inquiry pattern.

use advisorygraphen_core::{AdvisoryResult, AdvisorySpaceEnvelope};
use serde_json::{json, Value};

mod api_route;
mod boundary;
mod cycle;
mod evidence;
mod owner;
mod requirement;

pub const HYPOTHESIS_LIFECYCLE_CANDIDATE: &str = "candidate";
pub const HYPOTHESIS_LIFECYCLE_SUPPORTED: &str = "supported";
pub const HYPOTHESIS_LIFECYCLE_ACCEPTED: &str = "accepted";
pub const HYPOTHESIS_LIFECYCLE_REJECTED: &str = "rejected";
pub const HYPOTHESIS_LIFECYCLE_FALSIFIED: &str = "falsified";

/// Emit hypotheses, falsifiers, and the supporting incidences for the
/// obstructions that the reasoning engine has produced.
pub fn build_hypotheses(
    space: &AdvisorySpaceEnvelope,
    obstructions: &[Value],
) -> AdvisoryResult<HypothesisBundle> {
    let mut bundle = HypothesisBundle::default();
    for obstruction in obstructions {
        let Some(obstruction_type) = obstruction.get("obstruction_type").and_then(Value::as_str)
        else {
            continue;
        };
        match obstruction_type {
            "boundary_violation" => boundary::emit(space, obstruction, &mut bundle)?,
            "api_route_missing_auth" => api_route::emit(obstruction, &mut bundle)?,
            "missing_owner" => owner::emit(space, obstruction, &mut bundle)?,
            "requirement_unverified" => requirement::emit(space, obstruction, &mut bundle)?,
            "insufficient_evidence" => evidence::emit(space, obstruction, &mut bundle)?,
            "circular_dependency" => cycle::emit(obstruction, &mut bundle)?,
            _ => {}
        }
    }
    Ok(bundle)
}

#[derive(Default)]
pub struct HypothesisBundle {
    pub hypotheses: Vec<Value>,
    pub falsifiers: Vec<Value>,
    pub incidences: Vec<Value>,
}

pub(super) fn hypothesis_cell(
    id: &str,
    context_ids: &[Value],
    evidence_ids: &[Value],
    summary: String,
    metadata: Value,
) -> Value {
    json!({
        "id": id,
        "cell_type": "hypothesis",
        "title": summary.clone(),
        "summary": summary,
        "context_ids": context_ids,
        "source_ids": evidence_ids,
        "structure_refs": [],
        "lifecycle_status": HYPOTHESIS_LIFECYCLE_CANDIDATE,
        "review_status": "unreviewed",
        "provenance": {
            "origin": "inferred",
            "actor": "advisorygraphen-reasoning",
            "review_status": "unreviewed",
            "confidence": 0.6
        },
        "metadata": metadata
    })
}

pub(super) fn falsifier_cell(
    id: &str,
    context_ids: &[Value],
    summary: String,
    falsifies_hypothesis_id: &str,
) -> Value {
    json!({
        "id": id,
        "cell_type": "falsifier",
        "title": summary.clone(),
        "summary": summary,
        "context_ids": context_ids,
        "source_ids": [],
        "structure_refs": [],
        "review_status": "unreviewed",
        "provenance": {
            "origin": "inferred",
            "actor": "advisorygraphen-reasoning",
            "review_status": "unreviewed",
            "confidence": 0.6
        },
        "metadata": {
            "falsifies": falsifies_hypothesis_id
        }
    })
}

pub(super) fn argumentation_incidence(
    id: &str,
    relation_type: &str,
    from_id: &str,
    to_id: &str,
    evidence_ids: &[Value],
) -> Value {
    json!({
        "id": id,
        "relation_type": relation_type,
        "from_id": from_id,
        "to_id": to_id,
        "context_ids": [],
        "evidence_ids": evidence_ids,
        "strength": "soft",
        "provenance": {
            "origin": "inferred",
            "actor": "advisorygraphen-reasoning",
            "review_status": "unreviewed",
            "confidence": 0.6
        },
        "metadata": {
            "argumentation_relation": true
        }
    })
}

pub(super) fn primary_evidence_strength(
    obstruction: &Value,
    evidence_ids: &[Value],
) -> &'static str {
    match obstruction
        .pointer("/metadata/specificity")
        .and_then(Value::as_str)
    {
        Some("source_derived") if !evidence_ids.is_empty() => "source_backed_incidence",
        Some("source_derived") => "source_derived_no_evidence_ids",
        Some("requirement_derived") => "requirement_derived",
        Some("code_derived") => "code_lexical_signal",
        Some("topology_derived") => "topology_derived_dfs",
        _ => "template_derived",
    }
}

pub(super) fn primary_explanatory_power(obstruction: &Value) -> &'static str {
    match obstruction
        .pointer("/metadata/specificity")
        .and_then(Value::as_str)
    {
        Some("source_derived") | Some("topology_derived") | Some("requirement_derived") => "high",
        Some("code_derived") => "medium_review_required",
        _ => "medium",
    }
}

pub(super) fn cell_title(space: &AdvisorySpaceEnvelope, cell_id: &str) -> String {
    space
        .cells
        .iter()
        .find(|cell| cell.get("id").and_then(Value::as_str) == Some(cell_id))
        .and_then(|cell| cell.get("title").and_then(Value::as_str))
        .unwrap_or(cell_id)
        .to_string()
}

pub(super) fn evidence_ids(obstruction: &Value) -> Vec<Value> {
    obstruction
        .get("evidence_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

pub(super) fn obstruction_id_str(obstruction: &Value) -> &str {
    obstruction
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("obstruction:unknown")
}
