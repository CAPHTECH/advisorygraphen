use serde_json::{json, Value};

/// Request schema for `micro review`.
///
/// `micro review` does not classify natural-language claims itself. Deciding
/// whether a sentence is overconfident, an assumption, or evidence-backed is a
/// semantic judgement that belongs to the calling agent. This command takes the
/// agent's self-classified claims and enforces *structural* honesty
/// deterministically: a claim cannot be marked evidence-backed without citing a
/// concrete witness, declared strong claims must carry a falsification path, and
/// unsupported or high-blast-radius claims decide whether to escalate to the
/// full workflow. This mirrors the full workflow's
/// `supported_hypothesis_missing_support` invariant at small scope.
pub const REQUEST_SCHEMA: &str = "advisorygraphen.micro_review.request.v1";

/// Classifications the agent may assign to a claim. Any other value is a hard
/// validation error rather than a silently tolerated category.
const CLASSIFICATIONS: &[&str] = &[
    "test_backed",
    "source_backed",
    "assumption",
    "unsupported_strong_claim",
    "unsupported",
];

fn is_evidence_backed(classification: &str) -> bool {
    matches!(classification, "test_backed" | "source_backed")
}

struct Tally {
    evidence_referenced: usize,
    supported_without_evidence: usize,
    unsupported_strong_claim: usize,
    assumption: usize,
    unsupported: usize,
    high_blast_radius_unsupported: usize,
}

pub fn analyze(request: &Value) -> Result<Value, String> {
    if let Some(schema) = request.get("schema").and_then(Value::as_str) {
        if schema != REQUEST_SCHEMA {
            return Err(format!(
                "micro review request schema must be {REQUEST_SCHEMA}, found {schema}"
            ));
        }
    }
    let claim_values = request
        .get("claims")
        .and_then(Value::as_array)
        .ok_or_else(|| "micro review request must contain a `claims` array".to_string())?;
    if claim_values.is_empty() {
        return Err("micro review request `claims` must not be empty".to_string());
    }

    let mut claims = Vec::new();
    let mut obstructions = Vec::new();
    let mut assumptions = Vec::new();
    let mut missing_checks = Vec::new();
    let mut alternative_hypotheses = Vec::new();
    let mut tally = Tally {
        evidence_referenced: 0,
        supported_without_evidence: 0,
        unsupported_strong_claim: 0,
        assumption: 0,
        unsupported: 0,
        high_blast_radius_unsupported: 0,
    };

    for (index, claim) in claim_values.iter().enumerate() {
        let claim_id = claim
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("claim:{:03}", index + 1));
        let text = claim
            .get("text")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .ok_or_else(|| format!("{claim_id} must contain non-empty `text`"))?
            .to_string();
        let classification = claim
            .get("classification")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{claim_id} must contain a `classification`"))?;
        if !CLASSIFICATIONS.contains(&classification) {
            return Err(format!(
                "{claim_id} has unknown classification `{classification}`; expected one of {}",
                CLASSIFICATIONS.join(", ")
            ));
        }
        let evidence_refs = string_array(claim, "evidence_refs");
        let risk_surface = string_array(claim, "risk_surface");

        let substantiated = is_evidence_backed(classification) && !evidence_refs.is_empty();
        let structural_status = if is_evidence_backed(classification) {
            if evidence_refs.is_empty() {
                tally.supported_without_evidence += 1;
                obstructions.push(json!({
                    "id": format!("obstruction:{claim_id}:supported-without-evidence"),
                    "claim_id": claim_id,
                    "obstruction_type": "claim_marked_supported_without_evidence",
                    "message": format!(
                        "Claim is classified `{classification}` but cites no witness in evidence_refs."
                    ),
                    "required_resolution": "Cite a concrete witness (file path, command output, test name, log, or source id) or downgrade the classification.",
                    "review_status": "unreviewed"
                }));
                missing_checks.push(json!({
                    "id": format!("check:{:03}", missing_checks.len() + 1),
                    "claim_id": claim_id,
                    "reason": "claim asserts evidence backing without a cited witness",
                    "suggested_observation": "Attach the exact witness (file path, command output, test name, log, or source id) that supports this claim."
                }));
                "supported_without_evidence"
            } else {
                tally.evidence_referenced += 1;
                "evidence_referenced"
            }
        } else if classification == "assumption" {
            tally.assumption += 1;
            assumptions.push(json!({
                "claim_id": claim_id,
                "text": text,
                "status": "requires_confirmation_or_downgrade"
            }));
            "assumption"
        } else if classification == "unsupported_strong_claim" {
            tally.unsupported_strong_claim += 1;
            obstructions.push(json!({
                "id": format!("obstruction:{claim_id}:unsupported-strong-claim"),
                "claim_id": claim_id,
                "obstruction_type": "unsupported_strong_claim",
                "message": "Claim is classified as a strong/overconfident claim with no cited evidence.",
                "required_resolution": "Provide a bounded witness that supports the claim, add a falsification check, or downgrade it to a hypothesis.",
                "review_status": "unreviewed"
            }));
            missing_checks.push(json!({
                "id": format!("check:{:03}", missing_checks.len() + 1),
                "claim_id": claim_id,
                "reason": "strong/overconfident claim lacks bounded evidence",
                "suggested_observation": "Try to falsify the strong claim with a focused negative test or counterexample before treating it as accepted."
            }));
            "unsupported_strong_claim"
        } else {
            tally.unsupported += 1;
            "unsupported"
        };

        if !risk_surface.is_empty() && !substantiated {
            tally.high_blast_radius_unsupported += 1;
            obstructions.push(json!({
                "id": format!("obstruction:{claim_id}:high-blast-radius"),
                "claim_id": claim_id,
                "obstruction_type": "high_blast_radius_claim_without_evidence",
                "message": format!(
                    "Claim touches a high-blast-radius surface ({}) but is not evidence-backed.",
                    risk_surface.join(", ")
                ),
                "required_resolution": "Collect one bounded source-backed observation for this surface before treating the claim as accepted.",
                "review_status": "unreviewed"
            }));
        }

        for hypothesis in claim
            .get("alternative_hypotheses")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            alternative_hypotheses.push(json!({
                "id": format!("hypothesis:alternative-{:03}", alternative_hypotheses.len() + 1),
                "source_claim_id": claim_id,
                "alternative": hypothesis.get("alternative").and_then(Value::as_str).unwrap_or_default(),
                "falsifier": hypothesis.get("falsifier").and_then(Value::as_str).unwrap_or_default()
            }));
        }
        for check in string_array(claim, "missing_checks") {
            missing_checks.push(json!({
                "id": format!("check:{:03}", missing_checks.len() + 1),
                "claim_id": claim_id,
                "reason": "agent-declared check not yet run",
                "suggested_observation": check
            }));
        }

        claims.push(json!({
            "id": claim_id,
            "text": text,
            "classification": classification,
            "structural_status": structural_status,
            "evidence_refs": evidence_refs,
            "risk_surface": risk_surface
        }));
    }

    let escalation_reasons = escalation_reasons(claims.len(), &tally);
    let recommended_mode = if escalation_reasons.is_empty() {
        "micro_review"
    } else {
        "full_advisory_workflow_recommended"
    };
    let recommended_next_observation = missing_checks.first().cloned().unwrap_or_else(|| {
        json!({
            "id": "check:000",
            "reason": "no structural obstruction detected in the self-classified claims",
            "suggested_observation": "Keep the current answer as a lightweight note; no full AdvisoryGraphen workflow is indicated."
        })
    });

    Ok(json!({
        "mode": {
            "recommended": recommended_mode,
            "escalate_to_full_workflow": !escalation_reasons.is_empty(),
            "escalation_reasons": escalation_reasons
        },
        "scale_signals": {
            "claim_count": claims.len(),
            "evidence_referenced_count": tally.evidence_referenced,
            "supported_without_evidence_count": tally.supported_without_evidence,
            "unsupported_strong_claim_count": tally.unsupported_strong_claim,
            "assumption_count": tally.assumption,
            "unsupported_claim_count": tally.unsupported,
            "high_blast_radius_unsupported_count": tally.high_blast_radius_unsupported
        },
        "small_scope_value": {
            "role": "ai_answer_self_review",
            "value_proposition": "Validate an agent's self-classified claims: enforce that any claim marked evidence-backed cites a concrete witness, surface declared strong claims as obstructions with falsification checks, flag unsupported high-blast-radius claims, and decide whether to escalate to the full workflow. Semantic classification is the agent's responsibility; this command enforces structural honesty deterministically and does not pattern-match prose."
        },
        "claims": claims,
        "obstructions": obstructions,
        "assumptions": assumptions,
        "missing_checks": missing_checks,
        "alternative_hypotheses": alternative_hypotheses,
        "recommended_next_observation": recommended_next_observation
    }))
}

fn string_array(claim: &Value, field: &str) -> Vec<String> {
    claim
        .get(field)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn escalation_reasons(claim_count: usize, tally: &Tally) -> Vec<Value> {
    let mut reasons = Vec::new();
    if claim_count > 8 {
        reasons.push(json!({
            "reason": "many_claims",
            "detail": "More than eight claims were submitted; full hypothesis and completion review may be cheaper than ad hoc checking."
        }));
    }
    if tally.unsupported_strong_claim >= 2 {
        reasons.push(json!({
            "reason": "dominant_unsupported_certainty",
            "detail": "Two or more claims are self-classified as strong/overconfident without evidence."
        }));
    }
    if tally.high_blast_radius_unsupported > 0 {
        reasons.push(json!({
            "reason": "high_blast_radius_claims",
            "detail": "A high-blast-radius claim is not evidence-backed."
        }));
    }
    let unsupported_total =
        tally.unsupported + tally.unsupported_strong_claim + tally.supported_without_evidence;
    if unsupported_total > 5 {
        reasons.push(json!({
            "reason": "many_unsupported_claims",
            "detail": "More than five claims lack cited evidence."
        }));
    }
    reasons
}
