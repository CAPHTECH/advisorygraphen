use super::{
    argumentation_incidence, evidence_ids, falsifier_cell, hypothesis_cell, obstruction_id_str,
    primary_evidence_strength, trusted_reviewed_structure, HypothesisBundle,
};
use advisorygraphen_core::{json_id, AdvisoryResult, AdvisorySpaceEnvelope};
use serde_json::{json, Value};

pub(super) fn emit(
    space: &AdvisorySpaceEnvelope,
    obstruction: &Value,
    bundle: &mut HypothesisBundle,
) -> AdvisoryResult<()> {
    let obstruction_id = obstruction_id_str(obstruction);
    let evidence_ids = evidence_ids(obstruction);
    let context_ids: Vec<Value> = Vec::new();
    let route_path = obstruction
        .pointer("/metadata/route_path")
        .and_then(Value::as_str)
        .unwrap_or("API route");
    let stem = obstruction_id.trim_start_matches("obstruction:");
    let primary_strength = primary_evidence_strength(obstruction, &evidence_ids);

    let h_unprotected = format!("hypothesis:{stem}-truly-unprotected");
    let h_shared_middleware = format!("hypothesis:{stem}-shared-middleware-auth");
    let h_intentionally_public = format!("hypothesis:{stem}-intentionally-public");
    let f_unprotected = format!("falsifier:{stem}-route-or-middleware-auth-found");
    let f_shared_middleware = format!("falsifier:{stem}-no-shared-auth-covers-route");
    let f_intentionally_public = format!("falsifier:{stem}-route-not-intended-public");
    let route_cell_id = obstruction
        .get("witness_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .find(|id| id.starts_with("cell:"));
    let shared_middleware_support = route_hypothesis_support(
        space,
        route_cell_id,
        &["shared_middleware_auth", "auth_present"],
    );
    let intentionally_public_support = route_hypothesis_support(
        space,
        route_cell_id,
        &["intentionally_public", "public_endpoint"],
    );

    bundle.hypotheses.push(hypothesis_cell(
        &h_unprotected,
        &context_ids,
        &evidence_ids,
        format!("{route_path} touches the database without any authentication guard."),
        json!({
            "explanatory_power": "high",
            "evidence_strength": primary_strength,
            "specificity": "code_derived",
            "verification_cost": "medium",
            "risk_if_true": "high",
            "competes_with": [h_shared_middleware.clone(), h_intentionally_public.clone()],
            "explains": [obstruction_id],
            "falsified_by": [f_unprotected.clone()],
            "suggests_completion_types": ["proposed_auth_guard", "route_security_review"]
        }),
    ));
    bundle.hypotheses.push(hypothesis_cell(
        &h_shared_middleware,
        &context_ids,
        &with_support_evidence(&evidence_ids, &shared_middleware_support),
        format!(
            "{route_path} is protected by shared middleware that the lexical scan did not detect."
        ),
        json!({
            "explanatory_power": "medium_review_required",
            "evidence_strength": support_evidence_strength(&shared_middleware_support, "lexical_scan_blind_spot"),
            "specificity": "code_derived",
            "verification_cost": "medium",
            "risk_if_true": "low",
            "competes_with": [h_unprotected.clone(), h_intentionally_public.clone()],
            "explains": [obstruction_id],
            "falsified_by": [f_shared_middleware.clone()],
            "supported_by": shared_middleware_support,
            "suggests_completion_types": ["source_backed_evidence", "route_security_review"]
        }),
    ));
    bundle.hypotheses.push(hypothesis_cell(
        &h_intentionally_public,
        &context_ids,
        &with_support_evidence(&evidence_ids, &intentionally_public_support),
        format!(
            "{route_path} is intentionally public but the public_endpoint or anonymous_allowed metadata flag is missing."
        ),
        json!({
            "explanatory_power": "medium",
            "evidence_strength": support_evidence_strength(&intentionally_public_support, "metadata_absence"),
            "specificity": "code_derived",
            "verification_cost": "low",
            "risk_if_true": "low",
            "competes_with": [h_unprotected.clone(), h_shared_middleware.clone()],
            "explains": [obstruction_id],
            "falsified_by": [f_intentionally_public.clone()],
            "supported_by": intentionally_public_support,
            "suggests_completion_types": ["source_backed_evidence", "route_security_review"]
        }),
    ));
    bundle.falsifiers.push(falsifier_cell(
        &f_unprotected,
        &context_ids,
        format!(
            "Route-level decorator, framework guard, or shared middleware that authenticates {route_path} is found in the source snapshot."
        ),
        &h_unprotected,
    ));
    bundle.falsifiers.push(falsifier_cell(
        &f_shared_middleware,
        &context_ids,
        format!(
            "Middleware analysis confirms no shared authentication wrapper covers {route_path}."
        ),
        &h_shared_middleware,
    ));
    bundle.falsifiers.push(falsifier_cell(
        &f_intentionally_public,
        &context_ids,
        format!("Reviewed PR or ADR confirms {route_path} is not intended to be public."),
        &h_intentionally_public,
    ));

    for hypothesis_id in [
        &h_unprotected,
        &h_shared_middleware,
        &h_intentionally_public,
    ] {
        bundle.incidences.push(argumentation_incidence(
            &format!("incidence:{hypothesis_id}-explains-{obstruction_id}"),
            "explains",
            hypothesis_id,
            obstruction_id,
            &evidence_ids,
        ));
    }
    for (hypothesis_id, falsifier_id) in [
        (&h_unprotected, &f_unprotected),
        (&h_shared_middleware, &f_shared_middleware),
        (&h_intentionally_public, &f_intentionally_public),
    ] {
        bundle.incidences.push(argumentation_incidence(
            &format!("incidence:{falsifier_id}-falsifies-{hypothesis_id}"),
            "falsified_by",
            hypothesis_id,
            falsifier_id,
            &[],
        ));
    }
    for evidence_id in route_hypothesis_support(
        space,
        route_cell_id,
        &["shared_middleware_auth", "auth_present"],
    ) {
        bundle.incidences.push(argumentation_incidence(
            &format!("incidence:{h_shared_middleware}-supported-by-{evidence_id}"),
            "supported_by",
            &h_shared_middleware,
            &evidence_id,
            &[Value::String(evidence_id.clone())],
        ));
    }
    for evidence_id in route_hypothesis_support(
        space,
        route_cell_id,
        &["intentionally_public", "public_endpoint"],
    ) {
        bundle.incidences.push(argumentation_incidence(
            &format!("incidence:{h_intentionally_public}-supported-by-{evidence_id}"),
            "supported_by",
            &h_intentionally_public,
            &evidence_id,
            &[Value::String(evidence_id.clone())],
        ));
    }
    bundle.incidences.push(argumentation_incidence(
        &format!("incidence:{h_unprotected}-competes-{h_shared_middleware}"),
        "competes_with",
        &h_unprotected,
        &h_shared_middleware,
        &[],
    ));
    bundle.incidences.push(argumentation_incidence(
        &format!("incidence:{h_unprotected}-competes-{h_intentionally_public}"),
        "competes_with",
        &h_unprotected,
        &h_intentionally_public,
        &[],
    ));

    Ok(())
}

fn route_hypothesis_support(
    space: &AdvisorySpaceEnvelope,
    route_cell_id: Option<&str>,
    supported_kinds: &[&str],
) -> Vec<String> {
    let Some(route_cell_id) = route_cell_id else {
        return Vec::new();
    };
    space
        .incidences
        .iter()
        .filter(|incidence| {
            incidence.get("relation_type").and_then(Value::as_str) == Some("supports")
                && incidence.get("to_id").and_then(Value::as_str) == Some(route_cell_id)
        })
        .filter_map(|incidence| incidence.get("from_id").and_then(Value::as_str))
        .filter(|cell_id| {
            space
                .cells
                .iter()
                .find(|cell| json_id(cell) == *cell_id)
                .is_some_and(|cell| {
                    let kind = cell
                        .pointer("/metadata/supports_hypothesis_type")
                        .and_then(Value::as_str);
                    kind.is_some_and(|kind| supported_kinds.contains(&kind))
                        && !trusted_reviewed_structure(cell)
                })
        })
        .map(str::to_string)
        .collect()
}

fn with_support_evidence(primary: &[Value], support: &[String]) -> Vec<Value> {
    primary
        .iter()
        .cloned()
        .chain(support.iter().cloned().map(Value::String))
        .collect()
}

fn support_evidence_strength(support: &[String], fallback: &'static str) -> &'static str {
    if support.is_empty() {
        fallback
    } else {
        "agent_observation_unreviewed"
    }
}
