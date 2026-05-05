use advisorygraphen_core::{
    json_id, sorted_values_by_id, AdvisoryResult, AdvisorySpaceEnvelope, ReportEnvelope,
};
use serde_json::{json, Value};

pub fn propose_hypothesis_lifecycle(
    space: &AdvisorySpaceEnvelope,
    check_report: &ReportEnvelope,
    from_report: &str,
    command: Option<&str>,
) -> AdvisoryResult<ReportEnvelope> {
    let hypotheses = check_report
        .result
        .get("hypotheses")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut skipped = 0usize;
    let mut proposals = Vec::new();

    for hypothesis in &hypotheses {
        let support = support_signals(space, hypothesis);
        let falsify = falsify_signals(space, hypothesis);
        if support.is_empty() && falsify.is_empty() {
            skipped += 1;
            continue;
        }
        proposals.push(lifecycle_proposal(hypothesis, support, falsify));
    }

    proposals = sorted_values_by_id(proposals);
    Ok(ReportEnvelope::new(
        "hypothesis_lifecycle_proposal",
        command,
        json!({
            "space_id": space.space_id,
            "from_report": from_report
        }),
        json!({
            "lifecycle_proposals": proposals,
            "proposal_count": proposals.len(),
            "skipped_hypothesis_count": skipped,
            "authority_boundary": {
                "proposal_status": "unreviewed",
                "may_apply_events": false,
                "review_gated_commands": [
                    "hypothesis support",
                    "hypothesis falsify",
                    "hypothesis accept",
                    "hypothesis reject"
                ],
                "reason": "Agent-observed signals can propose lifecycle transitions, but cannot promote, accept, reject, support, or falsify hypotheses without an explicit review event."
            }
        }),
    ))
}

fn lifecycle_proposal(hypothesis: &Value, support: Vec<Value>, falsify: Vec<Value>) -> Value {
    let hypothesis_id = json_id(hypothesis);
    let slug = hypothesis_id.trim_start_matches("hypothesis:");
    let proposed_outcome = match (support.is_empty(), falsify.is_empty()) {
        (false, true) => "supported",
        (true, false) => "falsified",
        (false, false) => "review_conflict",
        (true, true) => "none",
    };
    let evidence_ids = support
        .iter()
        .chain(falsify.iter())
        .filter_map(|signal| signal.get("evidence_id").and_then(Value::as_str))
        .map(str::to_string)
        .collect::<Vec<_>>();
    json!({
        "id": format!("hypothesis-lifecycle-proposal:{slug}-{proposed_outcome}"),
        "proposal_type": "hypothesis_lifecycle_transition",
        "target_hypothesis_id": hypothesis_id,
        "target_hypothesis_status": hypothesis
            .get("lifecycle_status")
            .and_then(Value::as_str)
            .unwrap_or("candidate"),
        "proposed_outcome": proposed_outcome,
        "review_status": "unreviewed",
        "confidence": proposal_confidence(&support, &falsify),
        "supporting_signals": support,
        "falsifying_signals": falsify,
        "evidence_ids": evidence_ids,
        "metadata": {
            "can_be_applied_by": match proposed_outcome {
                "supported" => "hypothesis support",
                "falsified" => "hypothesis falsify",
                "review_conflict" => "human review required before lifecycle command",
                _ => "none"
            },
            "event_preview": match proposed_outcome {
                "supported" | "falsified" => json!({
                    "schema": advisorygraphen_core::HYPOTHESIS_EVENT_SCHEMA,
                    "target_hypothesis_id": hypothesis_id,
                    "outcome": proposed_outcome,
                    "evidence_ids": evidence_ids,
                    "review_required": true
                }),
                _ => json!(null)
            }
        }
    })
}

fn proposal_confidence(support: &[Value], falsify: &[Value]) -> f64 {
    if !support.is_empty() && !falsify.is_empty() {
        return 0.4;
    }
    let signals = if support.is_empty() { falsify } else { support };
    if signals.iter().any(|signal| {
        signal.get("trust_level").and_then(Value::as_str) == Some("reviewed_or_source_backed")
    }) {
        0.78
    } else {
        0.62
    }
}

fn support_signals(space: &AdvisorySpaceEnvelope, hypothesis: &Value) -> Vec<Value> {
    let mut signals = Vec::new();
    let hypothesis_id = json_id(hypothesis);
    for item in hypothesis
        .pointer("/metadata/supported_by")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        signals.push(json!({
            "signal_type": "support",
            "signal_source": "hypothesis.metadata.supported_by",
            "evidence_id": item.get("id").and_then(Value::as_str),
            "summary": item.get("summary").and_then(Value::as_str).unwrap_or("Hypothesis has attached supporting observation."),
            "trust_level": provenance_trust(item)
        }));
    }
    for cell in &space.cells {
        if cell
            .pointer("/metadata/supports_hypothesis_id")
            .and_then(Value::as_str)
            == Some(hypothesis_id)
        {
            signals.push(cell_signal(
                "support",
                "cell.metadata.supports_hypothesis_id",
                cell,
            ));
        }
    }
    for incidence in &space.incidences {
        let relation_type = incidence
            .get("relation_type")
            .and_then(Value::as_str)
            .unwrap_or("");
        let supports = matches!(relation_type, "supports" | "supports_hypothesis")
            && incidence.get("to_id").and_then(Value::as_str) == Some(hypothesis_id);
        let supported_by = relation_type == "supported_by"
            && incidence.get("from_id").and_then(Value::as_str) == Some(hypothesis_id);
        if supports || supported_by {
            signals.push(incidence_signal("support", incidence));
        }
    }
    sorted_values_by_id(signals)
}

fn falsify_signals(space: &AdvisorySpaceEnvelope, hypothesis: &Value) -> Vec<Value> {
    let mut signals = Vec::new();
    let hypothesis_id = json_id(hypothesis);
    for cell in &space.cells {
        if generated_falsifier_template(cell) {
            continue;
        }
        let explicit = cell
            .pointer("/metadata/falsifies_hypothesis_id")
            .and_then(Value::as_str)
            == Some(hypothesis_id);
        let legacy =
            cell.pointer("/metadata/falsifies").and_then(Value::as_str) == Some(hypothesis_id);
        if explicit || legacy {
            signals.push(cell_signal(
                "falsify",
                "cell.metadata.falsifies_hypothesis_id",
                cell,
            ));
        }
    }
    for incidence in &space.incidences {
        let relation_type = incidence
            .get("relation_type")
            .and_then(Value::as_str)
            .unwrap_or("");
        let falsifies = relation_type == "falsifies"
            && incidence.get("to_id").and_then(Value::as_str) == Some(hypothesis_id);
        let falsified_by = relation_type == "falsified_by"
            && incidence.get("from_id").and_then(Value::as_str) == Some(hypothesis_id);
        if falsifies || falsified_by {
            signals.push(incidence_signal("falsify", incidence));
        }
    }
    sorted_values_by_id(signals)
}

fn cell_signal(signal_type: &str, signal_source: &str, cell: &Value) -> Value {
    json!({
        "id": format!("signal:{}:{}", signal_type, json_id(cell).trim_start_matches("cell:")),
        "signal_type": signal_type,
        "signal_source": signal_source,
        "evidence_id": json_id(cell),
        "summary": cell
            .get("summary")
            .and_then(Value::as_str)
            .or_else(|| cell.get("title").and_then(Value::as_str))
            .unwrap_or("Lifecycle signal cell."),
        "trust_level": provenance_trust(cell)
    })
}

fn incidence_signal(signal_type: &str, incidence: &Value) -> Value {
    json!({
        "id": format!("signal:{}:{}", signal_type, json_id(incidence).trim_start_matches("incidence:")),
        "signal_type": signal_type,
        "signal_source": "space.incidence",
        "evidence_id": json_id(incidence),
        "summary": incidence
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("Lifecycle signal incidence."),
        "trust_level": provenance_trust(incidence)
    })
}

fn generated_falsifier_template(cell: &Value) -> bool {
    cell.get("cell_type").and_then(Value::as_str) == Some("falsifier")
        && cell.pointer("/provenance/actor").and_then(Value::as_str)
            == Some("advisorygraphen-reasoning")
}

fn provenance_trust(value: &Value) -> &'static str {
    let review_status = value
        .pointer("/provenance/review_status")
        .and_then(Value::as_str)
        .or_else(|| value.get("review_status").and_then(Value::as_str));
    let origin = value.pointer("/provenance/origin").and_then(Value::as_str);
    if review_status == Some("accepted") || matches!(origin, Some("source_backed" | "reviewed")) {
        "reviewed_or_source_backed"
    } else if origin == Some("inferred") {
        "agent_inferred"
    } else {
        "unclassified"
    }
}
