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
    let from_id = obstruction
        .pointer("/metadata/from_cell_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let to_id = obstruction
        .pointer("/metadata/to_cell_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let from_title = cell_title(space, from_id);
    let to_title = cell_title(space, to_id);
    let obstruction_id = obstruction_id_str(obstruction);
    let evidence_ids = evidence_ids(obstruction);
    let context_ids: Vec<Value> = obstruction
        .pointer("/metadata/from_context_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let stem = obstruction_id.trim_start_matches("obstruction:");
    let primary_strength = primary_evidence_strength(obstruction, &evidence_ids);
    let primary_power = primary_explanatory_power(obstruction);

    let h_primary = format!("hypothesis:{stem}-implicit-interface");
    let h_alt_misclassified = format!("hypothesis:{stem}-context-misclassified");
    let h_alt_exception = format!("hypothesis:{stem}-undocumented-exception");
    let f_primary = format!("falsifier:{stem}-explicit-policy-exists");
    let f_misclass = format!("falsifier:{stem}-cells-share-context");

    bundle.hypotheses.push(hypothesis_cell(
        &h_primary,
        &context_ids,
        &evidence_ids,
        format!(
            "{from_title} accessing {to_title} across contexts indicates an implicit cross-context interface that should be made explicit."
        ),
        json!({
            "explanatory_power": primary_power,
            "evidence_strength": primary_strength,
            "specificity": "structural",
            "verification_cost": "low",
            "risk_if_true": "high",
            "competes_with": [h_alt_misclassified.clone(), h_alt_exception.clone()],
            "explains": [obstruction_id],
            "falsified_by": [f_primary.clone()],
            "suggests_completion_types": ["proposed_interface", "proposed_refactor_action"]
        }),
    ));
    bundle.hypotheses.push(hypothesis_cell(
        &h_alt_misclassified,
        &context_ids,
        &evidence_ids,
        format!(
            "The cross-context label is incorrect: {from_title} and {to_title} actually share an ownership context."
        ),
        json!({
            "explanatory_power": "medium",
            "evidence_strength": "context_taxonomy",
            "specificity": "structural",
            "verification_cost": "low",
            "risk_if_true": "medium",
            "competes_with": [h_primary.clone(), h_alt_exception.clone()],
            "explains": [obstruction_id],
            "falsified_by": [f_misclass.clone()],
            "suggests_completion_types": ["context_remap"]
        }),
    ));
    bundle.hypotheses.push(hypothesis_cell(
        &h_alt_exception,
        &context_ids,
        &evidence_ids,
        format!(
            "{from_title} accessing {to_title} is an intentional exception that lacks a documented exception policy."
        ),
        json!({
            "explanatory_power": "medium",
            "evidence_strength": "policy_absence",
            "specificity": "policy_derived",
            "verification_cost": "low",
            "risk_if_true": "medium",
            "competes_with": [h_primary.clone(), h_alt_misclassified.clone()],
            "explains": [obstruction_id],
            "suggests_completion_types": ["documented_exception_policy"]
        }),
    ));
    bundle.falsifiers.push(falsifier_cell(
        &f_primary,
        &context_ids,
        format!(
            "An explicit, accepted policy or ADR documenting {from_title} -> {to_title} as an allowed direct access exists."
        ),
        &h_primary,
    ));
    bundle.falsifiers.push(falsifier_cell(
        &f_misclass,
        &context_ids,
        format!(
            "Reviewed source documents confirm {from_title} and {to_title} are owned by the same context."
        ),
        &h_alt_misclassified,
    ));

    for hypothesis_id in [&h_primary, &h_alt_misclassified, &h_alt_exception] {
        bundle.incidences.push(argumentation_incidence(
            &format!("incidence:{hypothesis_id}-explains-{obstruction_id}"),
            "explains",
            hypothesis_id,
            obstruction_id,
            &evidence_ids,
        ));
    }
    if !evidence_ids.is_empty() {
        for evidence_value in &evidence_ids {
            let Some(evidence_id) = evidence_value.as_str() else {
                continue;
            };
            bundle.incidences.push(argumentation_incidence(
                &format!("incidence:{evidence_id}-supports-{h_primary}"),
                "supported_by",
                &h_primary,
                evidence_id,
                &[],
            ));
        }
    }
    bundle.incidences.push(argumentation_incidence(
        &format!("incidence:{f_primary}-falsifies-{h_primary}"),
        "falsified_by",
        &h_primary,
        &f_primary,
        &[],
    ));
    bundle.incidences.push(argumentation_incidence(
        &format!("incidence:{f_misclass}-falsifies-{h_alt_misclassified}"),
        "falsified_by",
        &h_alt_misclassified,
        &f_misclass,
        &[],
    ));
    bundle.incidences.push(argumentation_incidence(
        &format!("incidence:{h_primary}-competes-{h_alt_misclassified}"),
        "competes_with",
        &h_primary,
        &h_alt_misclassified,
        &[],
    ));
    bundle.incidences.push(argumentation_incidence(
        &format!("incidence:{h_primary}-competes-{h_alt_exception}"),
        "competes_with",
        &h_primary,
        &h_alt_exception,
        &[],
    ));

    Ok(())
}
