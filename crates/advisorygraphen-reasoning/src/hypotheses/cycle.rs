use super::{
    argumentation_incidence, evidence_ids, falsifier_cell, hypothesis_cell, obstruction_id_str,
    primary_evidence_strength, primary_explanatory_power, HypothesisBundle,
};
use advisorygraphen_core::AdvisoryResult;
use serde_json::{json, Value};

pub(super) fn emit(obstruction: &Value, bundle: &mut HypothesisBundle) -> AdvisoryResult<()> {
    let obstruction_id = obstruction_id_str(obstruction);
    let evidence_ids = evidence_ids(obstruction);
    let context_ids: Vec<Value> = Vec::new();
    let stem = obstruction_id.trim_start_matches("obstruction:");
    let primary_strength = primary_evidence_strength(obstruction, &evidence_ids);
    let primary_power = primary_explanatory_power(obstruction);

    let h_real_cycle = format!("hypothesis:{stem}-true-runtime-cycle");
    let h_misclassified = format!("hypothesis:{stem}-edge-misclassified");
    let h_runtime_break = format!("hypothesis:{stem}-cycle-broken-by-runtime-mechanism");
    let f_real_cycle = format!("falsifier:{stem}-runtime-trace-shows-no-cycle");
    let f_misclassified = format!("falsifier:{stem}-edge-classification-correct");
    let f_runtime_break = format!("falsifier:{stem}-no-runtime-break-mechanism");

    bundle.hypotheses.push(hypothesis_cell(
        &h_real_cycle,
        &context_ids,
        &evidence_ids,
        "The cycle is a true runtime/structural dependency cycle that needs interface inversion or an asynchronous boundary.".to_string(),
        json!({
            "explanatory_power": primary_power,
            "evidence_strength": primary_strength,
            "specificity": "topology_derived",
            "verification_cost": "medium",
            "risk_if_true": "high",
            "competes_with": [h_misclassified.clone(), h_runtime_break.clone()],
            "explains": [obstruction_id],
            "falsified_by": [f_real_cycle.clone()],
            "suggests_completion_types": ["proposed_dependency_break", "architecture_review"]
        }),
    ));
    bundle.hypotheses.push(hypothesis_cell(
        &h_misclassified,
        &context_ids,
        &evidence_ids,
        "One or more edges on the cycle are incorrectly classified (the relation_type is wrong) and the cycle is an artifact of mis-lifting.".to_string(),
        json!({
            "explanatory_power": "medium",
            "evidence_strength": "lift_classification_signal",
            "specificity": "structural",
            "verification_cost": "low",
            "risk_if_true": "low",
            "competes_with": [h_real_cycle.clone(), h_runtime_break.clone()],
            "explains": [obstruction_id],
            "falsified_by": [f_misclassified.clone()],
            "suggests_completion_types": ["relift_edge_classification", "architecture_review"]
        }),
    ));
    bundle.hypotheses.push(hypothesis_cell(
        &h_runtime_break,
        &context_ids,
        &evidence_ids,
        "The cycle exists in the static graph but is broken at runtime by lazy initialization, dependency injection, or a published interface that the snapshot did not surface.".to_string(),
        json!({
            "explanatory_power": "medium",
            "evidence_strength": "policy_absence",
            "specificity": "policy_derived",
            "verification_cost": "medium",
            "risk_if_true": "medium",
            "competes_with": [h_real_cycle.clone(), h_misclassified.clone()],
            "explains": [obstruction_id],
            "falsified_by": [f_runtime_break.clone()],
            "suggests_completion_types": ["documented_runtime_break", "architecture_review"]
        }),
    ));
    bundle.falsifiers.push(falsifier_cell(
        &f_real_cycle,
        &context_ids,
        "Runtime trace, integration test, or build-time analysis demonstrates that the cycle is not actually executed at runtime.".to_string(),
        &h_real_cycle,
    ));
    bundle.falsifiers.push(falsifier_cell(
        &f_misclassified,
        &context_ids,
        "Source review confirms that every edge on the cycle is correctly classified as a runtime/structural dependency.".to_string(),
        &h_misclassified,
    ));
    bundle.falsifiers.push(falsifier_cell(
        &f_runtime_break,
        &context_ids,
        "No documented lazy-init, DI container, or published interface mediates the cycle in the production codebase.".to_string(),
        &h_runtime_break,
    ));

    for hypothesis_id in [&h_real_cycle, &h_misclassified, &h_runtime_break] {
        bundle.incidences.push(argumentation_incidence(
            &format!("incidence:{hypothesis_id}-explains-{obstruction_id}"),
            "explains",
            hypothesis_id,
            obstruction_id,
            &evidence_ids,
        ));
    }
    for (hypothesis_id, falsifier_id) in [
        (&h_real_cycle, &f_real_cycle),
        (&h_misclassified, &f_misclassified),
        (&h_runtime_break, &f_runtime_break),
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
        &format!("incidence:{h_real_cycle}-competes-{h_misclassified}"),
        "competes_with",
        &h_real_cycle,
        &h_misclassified,
        &[],
    ));
    bundle.incidences.push(argumentation_incidence(
        &format!("incidence:{h_real_cycle}-competes-{h_runtime_break}"),
        "competes_with",
        &h_real_cycle,
        &h_runtime_break,
        &[],
    ));

    Ok(())
}
