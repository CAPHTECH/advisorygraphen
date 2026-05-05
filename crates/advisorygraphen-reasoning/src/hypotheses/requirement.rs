use super::{
    argumentation_incidence, cell_title, evidence_ids, falsifier_cell, hypothesis_cell,
    obstruction_id_str, primary_evidence_strength, primary_explanatory_power, HypothesisBundle,
};
use advisorygraphen_core::{AdvisoryResult, AdvisorySpaceEnvelope};
use serde_json::{json, Value};

pub(super) fn emit(
    space: &AdvisorySpaceEnvelope,
    obstruction: &Value,
    bundle: &mut HypothesisBundle,
) -> AdvisoryResult<()> {
    let obstruction_id = obstruction_id_str(obstruction);
    let evidence_ids = evidence_ids(obstruction);
    let context_ids: Vec<Value> = Vec::new();
    let requirement_id = obstruction
        .get("blocked_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .next()
        .unwrap_or("");
    let requirement_title = cell_title(space, requirement_id);
    let stem = obstruction_id.trim_start_matches("obstruction:");
    let primary_strength = primary_evidence_strength(obstruction, &evidence_ids);
    let primary_power = primary_explanatory_power(obstruction);

    let h_genuinely_missing = format!("hypothesis:{stem}-verification-genuinely-missing");
    let h_link_missing = format!("hypothesis:{stem}-test-exists-link-missing");
    let h_exploratory = format!("hypothesis:{stem}-requirement-is-exploratory");
    let f_genuinely_missing = format!("falsifier:{stem}-verifies-or-implements-link-found");
    let f_link_missing = format!("falsifier:{stem}-no-test-cell-matches-requirement");
    let f_exploratory = format!("falsifier:{stem}-stakeholder-confirms-not-exploratory");

    bundle.hypotheses.push(hypothesis_cell(
        &h_genuinely_missing,
        &context_ids,
        &evidence_ids,
        format!(
            "{requirement_title} has no verification because no test, metric, or implementation cell exists yet."
        ),
        json!({
            "explanatory_power": primary_power,
            "evidence_strength": primary_strength,
            "specificity": "requirement_derived",
            "verification_cost": "medium",
            "risk_if_true": "high",
            "competes_with": [h_link_missing.clone(), h_exploratory.clone()],
            "explains": [obstruction_id],
            "falsified_by": [f_genuinely_missing.clone()],
            "suggests_completion_types": ["proposed_test", "proposed_metric"]
        }),
    ));
    bundle.hypotheses.push(hypothesis_cell(
        &h_link_missing,
        &context_ids,
        &evidence_ids,
        format!(
            "A test or metric for {requirement_title} exists, but the verifies or implements incidence was not lifted."
        ),
        json!({
            "explanatory_power": "medium",
            "evidence_strength": "lift_completeness_signal",
            "specificity": "structural",
            "verification_cost": "low",
            "risk_if_true": "low",
            "competes_with": [h_genuinely_missing.clone(), h_exploratory.clone()],
            "explains": [obstruction_id],
            "falsified_by": [f_link_missing.clone()],
            "suggests_completion_types": ["lift_verification_link", "requirement_review"]
        }),
    ));
    bundle.hypotheses.push(hypothesis_cell(
        &h_exploratory,
        &context_ids,
        &evidence_ids,
        format!(
            "{requirement_title} is exploratory and the verification gap reflects intent rather than oversight."
        ),
        json!({
            "explanatory_power": "medium",
            "evidence_strength": "metadata_absence",
            "specificity": "policy_derived",
            "verification_cost": "low",
            "risk_if_true": "low",
            "competes_with": [h_genuinely_missing.clone(), h_link_missing.clone()],
            "explains": [obstruction_id],
            "falsified_by": [f_exploratory.clone()],
            "suggests_completion_types": ["mark_exploratory_requirement", "requirement_review"]
        }),
    ));
    bundle.falsifiers.push(falsifier_cell(
        &f_genuinely_missing,
        &context_ids,
        format!(
            "A verifies or implements incidence pointing to {requirement_title} exists in another revision or external test catalog."
        ),
        &h_genuinely_missing,
    ));
    bundle.falsifiers.push(falsifier_cell(
        &f_link_missing,
        &context_ids,
        format!(
            "Manual review of test catalog confirms no existing test or metric matches {requirement_title}."
        ),
        &h_link_missing,
    ));
    bundle.falsifiers.push(falsifier_cell(
        &f_exploratory,
        &context_ids,
        format!(
            "Stakeholder note or PRD revision confirms {requirement_title} is not exploratory."
        ),
        &h_exploratory,
    ));

    for hypothesis_id in [&h_genuinely_missing, &h_link_missing, &h_exploratory] {
        bundle.incidences.push(argumentation_incidence(
            &format!("incidence:{hypothesis_id}-explains-{obstruction_id}"),
            "explains",
            hypothesis_id,
            obstruction_id,
            &evidence_ids,
        ));
    }
    for (hypothesis_id, falsifier_id) in [
        (&h_genuinely_missing, &f_genuinely_missing),
        (&h_link_missing, &f_link_missing),
        (&h_exploratory, &f_exploratory),
    ] {
        bundle.incidences.push(argumentation_incidence(
            &format!("incidence:{falsifier_id}-falsifies-{hypothesis_id}"),
            "falsified_by",
            hypothesis_id,
            falsifier_id,
            &[],
        ));
    }
    bundle.incidences.push(argumentation_incidence(
        &format!("incidence:{h_genuinely_missing}-competes-{h_link_missing}"),
        "competes_with",
        &h_genuinely_missing,
        &h_link_missing,
        &[],
    ));
    bundle.incidences.push(argumentation_incidence(
        &format!("incidence:{h_genuinely_missing}-competes-{h_exploratory}"),
        "competes_with",
        &h_genuinely_missing,
        &h_exploratory,
        &[],
    ));

    Ok(())
}
