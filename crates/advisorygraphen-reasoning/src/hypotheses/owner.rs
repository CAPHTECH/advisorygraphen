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
    let action_id = obstruction
        .get("blocked_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .next()
        .unwrap_or("");
    let action_title = cell_title(space, action_id);
    let stem = obstruction_id.trim_start_matches("obstruction:");
    let primary_strength = primary_evidence_strength(obstruction, &evidence_ids);
    let primary_power = primary_explanatory_power(obstruction);

    let h_unassigned = format!("hypothesis:{stem}-no-team-holds-action");
    let h_de_facto = format!("hypothesis:{stem}-de-facto-owner-link-missing");
    let h_collective = format!("hypothesis:{stem}-collective-ownership");
    let f_unassigned = format!("falsifier:{stem}-owner-cell-or-link-found");
    let f_de_facto = format!("falsifier:{stem}-history-shows-no-clear-contributor");
    let f_collective = format!("falsifier:{stem}-no-shared-ownership-policy");

    bundle.hypotheses.push(hypothesis_cell(
        &h_unassigned,
        &context_ids,
        &evidence_ids,
        format!(
            "{action_title} has no owner because no team or individual currently holds responsibility for it."
        ),
        json!({
            "explanatory_power": primary_power,
            "evidence_strength": primary_strength,
            "specificity": "structural",
            "verification_cost": "low",
            "risk_if_true": "high",
            "competes_with": [h_de_facto.clone(), h_collective.clone()],
            "explains": [obstruction_id],
            "falsified_by": [f_unassigned.clone()],
            "suggests_completion_types": ["ownership_clarification", "owner_assignment"]
        }),
    ));
    bundle.hypotheses.push(hypothesis_cell(
        &h_de_facto,
        &context_ids,
        &evidence_ids,
        "A de-facto owner exists (recent contributor, on-call team, or module owner) but the explicit owns relation was not lifted.".to_string(),
        json!({
            "explanatory_power": "medium",
            "evidence_strength": "history_or_roster_signal",
            "specificity": "structural",
            "verification_cost": "low",
            "risk_if_true": "medium",
            "competes_with": [h_unassigned.clone(), h_collective.clone()],
            "explains": [obstruction_id],
            "falsified_by": [f_de_facto.clone()],
            "suggests_completion_types": ["derive_owner_from_history", "ownership_clarification"]
        }),
    ));
    bundle.hypotheses.push(hypothesis_cell(
        &h_collective,
        &context_ids,
        &evidence_ids,
        format!(
            "{action_title} belongs to a shared service whose ownership is collective; a single owner placeholder is the wrong shape."
        ),
        json!({
            "explanatory_power": "medium",
            "evidence_strength": "policy_absence",
            "specificity": "policy_derived",
            "verification_cost": "low",
            "risk_if_true": "medium",
            "competes_with": [h_unassigned.clone(), h_de_facto.clone()],
            "explains": [obstruction_id],
            "falsified_by": [f_collective.clone()],
            "suggests_completion_types": ["shared_ownership_policy", "policy_review"]
        }),
    ));
    bundle.falsifiers.push(falsifier_cell(
        &f_unassigned,
        &context_ids,
        format!(
            "An owner cell or owns incidence pointing at {action_title} exists in another revision or in an external roster."
        ),
        &h_unassigned,
    ));
    bundle.falsifiers.push(falsifier_cell(
        &f_de_facto,
        &context_ids,
        format!(
            "Reviewed git history, on-call rota, or team roster does not surface a single de-facto owner for {action_title}."
        ),
        &h_de_facto,
    ));
    bundle.falsifiers.push(falsifier_cell(
        &f_collective,
        &context_ids,
        format!(
            "No documented shared-ownership policy exists for the service that owns {action_title}."
        ),
        &h_collective,
    ));

    for hypothesis_id in [&h_unassigned, &h_de_facto, &h_collective] {
        bundle.incidences.push(argumentation_incidence(
            &format!("incidence:{hypothesis_id}-explains-{obstruction_id}"),
            "explains",
            hypothesis_id,
            obstruction_id,
            &evidence_ids,
        ));
    }
    for (hypothesis_id, falsifier_id) in [
        (&h_unassigned, &f_unassigned),
        (&h_de_facto, &f_de_facto),
        (&h_collective, &f_collective),
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
        &format!("incidence:{h_unassigned}-competes-{h_de_facto}"),
        "competes_with",
        &h_unassigned,
        &h_de_facto,
        &[],
    ));
    bundle.incidences.push(argumentation_incidence(
        &format!("incidence:{h_unassigned}-competes-{h_collective}"),
        "competes_with",
        &h_unassigned,
        &h_collective,
        &[],
    ));

    Ok(())
}
