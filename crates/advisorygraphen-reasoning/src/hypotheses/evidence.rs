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
    let recommendation_id = obstruction
        .get("blocked_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .next()
        .unwrap_or("");
    let recommendation_title = cell_title(space, recommendation_id);
    let stem = obstruction_id.trim_start_matches("obstruction:");
    let primary_strength = primary_evidence_strength(obstruction, &evidence_ids);
    let primary_power = primary_explanatory_power(obstruction);

    let h_genuinely_missing = format!("hypothesis:{stem}-source-evidence-genuinely-missing");
    let h_link_missing = format!("hypothesis:{stem}-evidence-exists-link-missing");
    let h_judgment_call = format!("hypothesis:{stem}-accepted-as-judgment-call");
    let f_genuinely_missing = format!("falsifier:{stem}-source-evidence-cell-found");
    let f_link_missing = format!("falsifier:{stem}-no-evidence-cell-matches");
    let f_judgment_call = format!("falsifier:{stem}-no-judgment-policy-documented");

    bundle.hypotheses.push(hypothesis_cell(
        &h_genuinely_missing,
        &context_ids,
        &evidence_ids,
        format!(
            "{recommendation_title} was accepted but no source-backed or review-promoted evidence has been lifted to support it."
        ),
        json!({
            "explanatory_power": primary_power,
            "evidence_strength": primary_strength,
            "specificity": "structural",
            "verification_cost": "low",
            "risk_if_true": "high",
            "competes_with": [h_link_missing.clone(), h_judgment_call.clone()],
            "explains": [obstruction_id],
            "falsified_by": [f_genuinely_missing.clone()],
            "suggests_completion_types": ["source_backed_evidence", "review_promote_evidence"]
        }),
    ));
    bundle.hypotheses.push(hypothesis_cell(
        &h_link_missing,
        &context_ids,
        &evidence_ids,
        format!(
            "Evidence supporting {recommendation_title} exists in the source corpus, but the supports incidence was not lifted."
        ),
        json!({
            "explanatory_power": "medium",
            "evidence_strength": "lift_completeness_signal",
            "specificity": "structural",
            "verification_cost": "low",
            "risk_if_true": "low",
            "competes_with": [h_genuinely_missing.clone(), h_judgment_call.clone()],
            "explains": [obstruction_id],
            "falsified_by": [f_link_missing.clone()],
            "suggests_completion_types": ["lift_supporting_incidence", "review_promote_evidence"]
        }),
    ));
    bundle.hypotheses.push(hypothesis_cell(
        &h_judgment_call,
        &context_ids,
        &evidence_ids,
        format!(
            "{recommendation_title} was accepted as an explicit judgment call, and the missing evidence is policy-permitted."
        ),
        json!({
            "explanatory_power": "medium",
            "evidence_strength": "policy_absence",
            "specificity": "policy_derived",
            "verification_cost": "low",
            "risk_if_true": "medium",
            "competes_with": [h_genuinely_missing.clone(), h_link_missing.clone()],
            "explains": [obstruction_id],
            "falsified_by": [f_judgment_call.clone()],
            "suggests_completion_types": ["documented_judgment_policy", "policy_review"]
        }),
    ));
    bundle.falsifiers.push(falsifier_cell(
        &f_genuinely_missing,
        &context_ids,
        format!(
            "A source-backed evidence cell or review-promoted morphism supporting {recommendation_title} exists in another revision or external corpus."
        ),
        &h_genuinely_missing,
    ));
    bundle.falsifiers.push(falsifier_cell(
        &f_link_missing,
        &context_ids,
        format!(
            "Manual review confirms no evidence cell in the corpus matches {recommendation_title}."
        ),
        &h_link_missing,
    ));
    bundle.falsifiers.push(falsifier_cell(
        &f_judgment_call,
        &context_ids,
        format!(
            "No judgment-call or evidence-waiver policy is documented for the package or engagement that owns {recommendation_title}."
        ),
        &h_judgment_call,
    ));

    for hypothesis_id in [&h_genuinely_missing, &h_link_missing, &h_judgment_call] {
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
        (&h_judgment_call, &f_judgment_call),
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
        &format!("incidence:{h_genuinely_missing}-competes-{h_judgment_call}"),
        "competes_with",
        &h_genuinely_missing,
        &h_judgment_call,
        &[],
    ));

    Ok(())
}
