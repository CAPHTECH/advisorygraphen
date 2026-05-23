use advisorygraphen_core::{AdvisoryError, AdvisoryResult, AdvisorySpaceEnvelope};
use advisorygraphen_reasoning::{
    blocker_resolution_state, close_status, frontier_items, waiting_items,
};
use serde_json::{json, Value};

mod correspondence;
mod higher;
mod hypotheses;

use hypotheses::{argumentation_incidences, falsifiers, hypotheses, hypothesis_summary};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum OutputFormat {
    Json,
    Markdown,
}

impl OutputFormat {
    pub fn parse(value: &str) -> AdvisoryResult<Self> {
        match value {
            "json" => Ok(Self::Json),
            "markdown" => Ok(Self::Markdown),
            other => Err(AdvisoryError::Validation(format!(
                "unsupported format: {other}"
            ))),
        }
    }
}

pub fn project(
    space: &AdvisorySpaceEnvelope,
    report: &Value,
    audience: &str,
    format: OutputFormat,
) -> AdvisoryResult<String> {
    let projection = build_projection(space, report, audience)?;
    match format {
        OutputFormat::Json => Ok(serde_json::to_string_pretty(&projection)?),
        OutputFormat::Markdown => render_markdown(audience, &projection),
    }
}

pub fn build_projection(
    space: &AdvisorySpaceEnvelope,
    report: &Value,
    audience: &str,
) -> AdvisoryResult<Value> {
    match audience {
        "executive" => executive_projection(space, report, audience),
        "developer_action" => developer_projection(space, report, audience),
        "audit_trace" => audit_projection(space, report, audience),
        "ai_agent" => ai_agent_projection(space, report, audience),
        "client_review" | "cli" => executive_projection(space, report, audience),
        other => Err(AdvisoryError::UnsupportedAudience(other.to_string())),
    }
}

fn executive_projection(
    space: &AdvisorySpaceEnvelope,
    report: &Value,
    audience: &str,
) -> AdvisoryResult<Value> {
    let obstructions = obstructions(report);
    let represented_ids = represented_ids(report);
    let omitted_ids = source_ids(space);
    let high_severity_obstructions = obstructions_by_severity(&obstructions, "high");
    let medium_severity_obstructions = obstructions_by_severity(&obstructions, "medium");
    let close_status = close_status_value(space, report);
    let candidates = completion_candidates(report);
    let candidate_quality = candidate_quality_summary(&candidates);
    let proposal_content_summary = proposal_content_summary(&candidates);
    let recommendation_trace = recommendation_trace(&candidates);
    let observation_actions = observation_actions(&recommendation_trace);
    let falsifiers = falsifiers(report);
    let explicit_hypothesis_matrix = explicit_hypothesis_matrix(space);
    let hypotheses = merged_hypotheses(hypotheses(report), &explicit_hypothesis_matrix);
    let hypothesis_summary = hypothesis_summary(&hypotheses);
    let explicit_proposal_trace = explicit_proposal_trace(space);
    let projection_loss = projection_loss(space, report);
    let projection_loss_metrics = projection_loss_metrics(
        space,
        report,
        &represented_ids,
        &omitted_ids,
        &projection_loss,
    );
    let schema_morphisms = schema_morphisms(space);
    let higher_graphen = higher::projection_result_json(
        space,
        report,
        audience,
        represented_ids.clone(),
        omitted_ids.clone(),
    )?;
    Ok(json!({
        "schema": "advisorygraphen.projection.v1",
        "projection_id": format!("projection:executive:{}", space.space_id.trim_start_matches("space:advisory:")),
        "audience": "executive",
        "space_id": space.space_id,
        "represented_ids": represented_ids,
        "omitted_ids": omitted_ids,
        "summary": {
            "closeable": close_status["closeable"].clone(),
            "blocking_threshold": close_status["blocking_threshold"].clone(),
            "blocking_obstruction_ids": close_status["blocking_obstruction_ids"].clone(),
            "obstruction_counts": obstruction_counts(&obstructions),
            "high_severity_obstructions": high_severity_obstructions,
            "medium_severity_obstructions": medium_severity_obstructions,
            "unreviewed_candidates_are_not_accepted": true,
            "candidate_quality": candidate_quality,
            "proposal_content_summary": proposal_content_summary,
            "recommendation_trace": recommendation_trace,
            "observation_actions": observation_actions,
            "explicit_hypothesis_matrix": explicit_hypothesis_matrix,
            "explicit_proposal_trace": explicit_proposal_trace,
            "hypothesis_summary": hypothesis_summary
        },
        "hypotheses": hypotheses,
        "falsifiers": falsifiers,
        "source_boundary": space.metadata.get("source_boundary").cloned().unwrap_or_else(|| json!({})),
        "projection_loss": projection_loss,
        "projection_loss_metrics": projection_loss_metrics,
        "schema_morphisms": schema_morphisms,
        "higher_graphen": higher_graphen
    }))
}

fn developer_projection(
    space: &AdvisorySpaceEnvelope,
    report: &Value,
    audience: &str,
) -> AdvisoryResult<Value> {
    let represented_ids = represented_ids(report);
    let omitted_ids = source_ids(space);
    let projection_loss = projection_loss(space, report);
    let projection_loss_metrics = projection_loss_metrics(
        space,
        report,
        &represented_ids,
        &omitted_ids,
        &projection_loss,
    );
    let higher_graphen = higher::projection_result_json(
        space,
        report,
        audience,
        represented_ids.clone(),
        omitted_ids.clone(),
    )?;
    Ok(json!({
        "schema": "advisorygraphen.projection.v1",
        "projection_id": format!("projection:developer-action:{}", space.space_id.trim_start_matches("space:advisory:")),
        "audience": "developer_action",
        "space_id": space.space_id,
        "represented_ids": represented_ids,
        "omitted_ids": omitted_ids,
        "actions": completion_candidates(report),
        "projection_loss": projection_loss,
        "projection_loss_metrics": projection_loss_metrics,
        "schema_morphisms": schema_morphisms(space),
        "higher_graphen": higher_graphen
    }))
}

fn audit_projection(
    space: &AdvisorySpaceEnvelope,
    report: &Value,
    audience: &str,
) -> AdvisoryResult<Value> {
    let represented_ids = represented_ids(report);
    let omitted_ids = Vec::new();
    let projection_loss = projection_loss(space, report);
    let projection_loss_metrics = projection_loss_metrics(
        space,
        report,
        &represented_ids,
        &omitted_ids,
        &projection_loss,
    );
    let higher_graphen = higher::projection_result_json(
        space,
        report,
        audience,
        represented_ids.clone(),
        omitted_ids.clone(),
    )?;
    Ok(json!({
        "schema": "advisorygraphen.projection.v1",
        "projection_id": format!("projection:audit:{}", space.space_id.trim_start_matches("space:advisory:")),
        "audience": "audit_trace",
        "space_id": space.space_id,
        "represented_ids": represented_ids,
        "omitted_ids": omitted_ids,
        "source_boundary": space.metadata.get("source_boundary").cloned().unwrap_or_else(|| json!({})),
        "report": report,
        "projection_loss": projection_loss,
        "projection_loss_metrics": projection_loss_metrics,
        "schema_morphisms": schema_morphisms(space),
        "higher_graphen": higher_graphen
    }))
}

fn ai_agent_projection(
    space: &AdvisorySpaceEnvelope,
    report: &Value,
    audience: &str,
) -> AdvisoryResult<Value> {
    let represented_ids = represented_ids(report);
    let omitted_ids = source_ids(space);
    let open_obstructions = obstructions(report);
    let candidates = completion_candidates(report);
    let resolution_state = blocker_resolution_state(&open_obstructions, &candidates);
    let candidate_quality = candidate_quality_summary(&candidates);
    let proposal_content_summary = proposal_content_summary(&candidates);
    let recommendation_trace = recommendation_trace(&candidates);
    let observation_actions = observation_actions(&recommendation_trace);
    let hypothesis_promotion_workflow = hypothesis_promotion_workflow(&recommendation_trace);
    let (live_candidates, superseded_candidates) = partition_candidates(&candidates);
    let falsifiers = falsifiers(report);
    let argumentation_incidences = argumentation_incidences(report);
    let explicit_hypothesis_matrix = explicit_hypothesis_matrix(space);
    let hypotheses = merged_hypotheses(hypotheses(report), &explicit_hypothesis_matrix);
    let hypothesis_summary = hypothesis_summary(&hypotheses);
    let explicit_proposal_trace = explicit_proposal_trace(space);
    let projection_loss = projection_loss(space, report);
    let projection_loss_metrics = projection_loss_metrics(
        space,
        report,
        &represented_ids,
        &omitted_ids,
        &projection_loss,
    );
    let schema_morphisms = schema_morphisms(space);
    let higher_graphen = higher::projection_result_json(
        space,
        report,
        audience,
        represented_ids.clone(),
        omitted_ids.clone(),
    )?;
    let correspondence_analysis = correspondence::correspondence_analysis(
        space,
        &open_obstructions,
        &hypotheses,
        &falsifiers,
        &candidates,
        &argumentation_incidences,
    )?;
    Ok(json!({
        "schema": "advisorygraphen.projection.v1",
        "projection_id": format!("projection:ai-agent:{}", space.space_id.trim_start_matches("space:advisory:")),
        "audience": "ai_agent",
        "space_id": space.space_id,
        "represented_ids": represented_ids,
        "omitted_ids": omitted_ids,
        "hg_operation_model": {
            "primary_operator": "ai_agent",
            "human_role": "sets goals, reviews candidates, and accepts or rejects promotions",
            "human_ui_role": "projection_consumer",
            "source_of_truth": "advisory_space_case_log_and_review_events",
            "principle": "HigherGraphen structure is manipulated by agents; humans review projections and explicit promotion events."
        },
        "agent_operation_contract": {
            "allowed_commands": [
                "validate",
                "lift",
                "check",
                "completions propose",
                "hypothesis propose",
                "hypothesis apply-proposals with conservative policy",
                "project ai_agent",
                "project audit_trace",
                "case import",
                "case reason",
                "case close-check"
            ],
            "review_gated_commands": [
                "completions accept",
                "completions reject",
                "hypothesis falsify",
                "hypothesis support",
                "hypothesis accept",
                "hypothesis reject"
            ],
            "forbidden_operations": [
                "promote unreviewed candidate structure",
                "hide projection_loss",
                "hide projection_loss_metrics",
                "treat inferred evidence as accepted fact",
                "rewrite source material outside the bounded snapshot"
            ],
            "resume_protocol": [
                "read close_status",
                "inspect open_obstructions",
                "inspect candidate_review_state",
                "inspect correspondence_analysis for shared evidence, conflicts, and gluing failures",
                "inspect blocker_resolution_state.application_requirements when present",
                "inspect observation_actions before promoting unsupported hypotheses",
                "inspect projection_loss_metrics and schema_morphisms before summarizing",
                "propose missing owner or verification structure",
                "generate audit_trace before reporting final state"
            ]
        },
        "open_obstructions": open_obstructions,
        "hypotheses": hypotheses,
        "falsifiers": falsifiers,
        "argumentation_incidences": argumentation_incidences,
        "correspondence_analysis": correspondence_analysis,
        "hypothesis_summary": hypothesis_summary,
        "explicit_hypothesis_matrix": explicit_hypothesis_matrix,
        "explicit_proposal_trace": explicit_proposal_trace,
        "candidate_review_state": candidates,
        "live_candidates": live_candidates,
        "superseded_candidates": superseded_candidates,
        "candidate_quality": candidate_quality,
        "proposal_content_summary": proposal_content_summary,
        "recommendation_trace": recommendation_trace,
        "observation_actions": observation_actions,
        "hypothesis_promotion_workflow": hypothesis_promotion_workflow,
        "blocker_resolution_state": resolution_state,
        "frontier_items": frontier_items(&resolution_state),
        "waiting_items": waiting_items(&resolution_state),
        "next_safe_operations": [
            "review_obstructions",
            "inspect_application_requirements",
            "propose_or_review_candidates",
            "run_case_close_check_before_closure",
            "generate_audit_projection"
        ],
        "close_status": close_status_value(space, report),
        "projection_loss": projection_loss,
        "projection_loss_metrics": projection_loss_metrics,
        "schema_morphisms": schema_morphisms,
        "higher_graphen": higher_graphen
    }))
}

fn render_markdown(audience: &str, projection: &Value) -> AdvisoryResult<String> {
    let mut lines = vec![
        format!(
            "# AdvisoryGraphen {} Projection",
            audience.replace('_', " ")
        ),
        String::new(),
        format!(
            "Space: `{}`",
            projection["space_id"].as_str().unwrap_or("unknown")
        ),
        String::new(),
    ];
    if let Some(obstructions) = projection
        .pointer("/summary/high_severity_obstructions")
        .and_then(Value::as_array)
    {
        if let Some(closeable) = projection
            .pointer("/summary/closeable")
            .and_then(Value::as_bool)
        {
            lines.push("## Close status".to_string());
            lines.push(format!("- Closeable: `{closeable}`"));
            if let Some(blocking_ids) = projection
                .pointer("/summary/blocking_obstruction_ids")
                .and_then(Value::as_array)
            {
                lines.push(format!("- Blocking obstructions: {}", blocking_ids.len()));
            }
            lines.push(String::new());
        }
        if let Some(counts) = projection
            .pointer("/summary/obstruction_counts")
            .and_then(Value::as_object)
        {
            lines.push("## Obstruction summary".to_string());
            for severity in ["high", "medium", "low", "unknown"] {
                let count = counts.get(severity).and_then(Value::as_u64).unwrap_or(0);
                lines.push(format!("- {severity}: {count}"));
            }
            lines.push(String::new());
        }
        if let Some(quality) = projection.pointer("/summary/candidate_quality") {
            lines.push("## Candidate quality".to_string());
            lines.push(format!(
                "- Source-derived: {}",
                quality["source_derived"].as_u64().unwrap_or(0)
            ));
            lines.push(format!(
                "- Requirement-derived: {}",
                quality["requirement_derived"].as_u64().unwrap_or(0)
            ));
            lines.push(format!(
                "- Code-derived: {}",
                quality["code_derived"].as_u64().unwrap_or(0)
            ));
            lines.push(format!(
                "- Generic: {}",
                quality["generic"].as_u64().unwrap_or(0)
            ));
            lines.push(format!(
                "- Missing precision metadata: {}",
                quality["missing_precision_metadata"].as_u64().unwrap_or(0)
            ));
            lines.push(String::new());
        }
        if let Some(summary) = projection.pointer("/summary/proposal_content_summary") {
            lines.push("## Proposal content".to_string());
            lines.push(format!(
                "- With structured content: {}",
                summary["with_structured_content"].as_u64().unwrap_or(0)
            ));
            lines.push(format!(
                "- Blocked content: {}",
                summary["blocked_content"].as_u64().unwrap_or(0)
            ));
            lines.push(format!(
                "- Content obstructions: {}",
                summary["content_obstruction_count"].as_u64().unwrap_or(0)
            ));
            lines.push(String::new());
        }
        if let Some(trace) = projection.pointer("/summary/recommendation_trace") {
            lines.push("## Recommendation trace".to_string());
            lines.push(format!(
                "- Primary recommendations: {}",
                trace["primary_count"].as_u64().unwrap_or(0)
            ));
            lines.push(format!(
                "- Alternatives: {}",
                trace["alternative_count"].as_u64().unwrap_or(0)
            ));
            lines.push(format!(
                "- Follow-up observations: {}",
                trace["follow_up_observation_count"].as_u64().unwrap_or(0)
            ));
            if let Some(items) = trace
                .get("follow_up_observations")
                .and_then(Value::as_array)
            {
                for item in items.iter().take(5) {
                    lines.push(format!(
                        "- Follow-up: `{}` from `{}`: {}",
                        item["candidate_id"].as_str().unwrap_or("unknown"),
                        item["derived_hypothesis_id"].as_str().unwrap_or("missing"),
                        item["title"].as_str().unwrap_or("Untitled follow-up")
                    ));
                    for task in item
                        .get("ranked_observation_tasks")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .take(2)
                    {
                        lines.push(format!(
                            "  - Observation {}: {}",
                            task["rank"].as_u64().unwrap_or(0),
                            task["expected_observation"]
                                .as_str()
                                .unwrap_or("Collect evidence before promotion.")
                        ));
                    }
                }
            }
            lines.push(String::new());
        }
        lines.push("## High-severity obstructions".to_string());
        if obstructions.is_empty() {
            lines.push("- None".to_string());
        } else {
            for obstruction in obstructions {
                lines.push(format!(
                    "- `{}`: {}",
                    obstruction["id"].as_str().unwrap_or("unknown"),
                    obstruction["message"].as_str().unwrap_or("No message.")
                ));
            }
        }
        lines.push(String::new());
    }
    if let Some(obstructions) = projection
        .pointer("/summary/medium_severity_obstructions")
        .and_then(Value::as_array)
    {
        lines.push("## Medium-severity obstructions".to_string());
        if obstructions.is_empty() {
            lines.push("- None".to_string());
        } else {
            for obstruction in obstructions {
                lines.push(format!(
                    "- `{}`: {}",
                    obstruction["id"].as_str().unwrap_or("unknown"),
                    obstruction["message"].as_str().unwrap_or("No message.")
                ));
            }
        }
        lines.push(String::new());
    }
    let mut reframed_obstructions: Vec<&Value> = Vec::new();
    for obstruction in projection
        .pointer("/summary/high_severity_obstructions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .chain(
            projection
                .pointer("/summary/medium_severity_obstructions")
                .and_then(Value::as_array)
                .into_iter()
                .flatten(),
        )
    {
        if obstruction.pointer("/metadata/reframe").is_some() {
            reframed_obstructions.push(obstruction);
        }
    }
    if !reframed_obstructions.is_empty() {
        lines.push("## Reframed obstructions (primary hypothesis falsified)".to_string());
        for obstruction in reframed_obstructions {
            let id = obstruction["id"].as_str().unwrap_or("unknown");
            let original = obstruction
                .pointer("/metadata/reframe/original_severity")
                .and_then(Value::as_str)
                .unwrap_or("?");
            let effective = obstruction
                .pointer("/metadata/reframe/effective_severity")
                .and_then(Value::as_str)
                .unwrap_or("?");
            lines.push(format!(
                "- `{id}`: severity {original} → effective {effective}"
            ));
            if let Some(types) = obstruction
                .pointer("/metadata/reframe/effective_completion_types")
                .and_then(Value::as_array)
            {
                let names: Vec<&str> = types.iter().filter_map(Value::as_str).collect();
                lines.push(format!("  - now suggests: {}", names.join(", ")));
            }
        }
        lines.push(String::new());
    }
    if let Some(summary) = projection.pointer("/summary/hypothesis_summary") {
        let total = summary["total"].as_u64().unwrap_or(0);
        if total > 0 {
            lines.push("## Hypotheses".to_string());
            lines.push(format!("- Total: {}", total));
            for status in [
                "candidate",
                "supported",
                "accepted",
                "rejected",
                "falsified",
            ] {
                let count = summary[status].as_u64().unwrap_or(0);
                if count > 0 {
                    lines.push(format!("- {status}: {count}"));
                }
            }
            lines.push(String::new());
        }
    }
    if let Some(boundary) = projection.get("source_boundary") {
        lines.push("## Source boundary".to_string());
        if let Some(included) = boundary
            .get("included_source_ids")
            .and_then(Value::as_array)
        {
            lines.push(format!("- Included sources: {}", included.len()));
        }
        if let Some(excluded) = boundary.get("excluded_summary").and_then(Value::as_array) {
            for item in excluded {
                lines.push(format!(
                    "- Excluded: {}",
                    item.as_str().unwrap_or("unknown")
                ));
            }
        }
        lines.push(String::new());
    }
    lines.push("## Projection loss".to_string());
    for loss in projection["projection_loss"]
        .as_array()
        .into_iter()
        .flatten()
    {
        lines.push(format!(
            "- `{}`: {}",
            loss["loss_type"].as_str().unwrap_or("loss"),
            loss["description"]
                .as_str()
                .unwrap_or("Projection omitted or compressed information.")
        ));
    }
    Ok(lines.join("\n"))
}

fn represented_ids(report: &Value) -> Vec<String> {
    obstructions(report)
        .into_iter()
        .chain(completion_candidates(report))
        .filter_map(|item| item.get("id").and_then(Value::as_str).map(str::to_string))
        .collect()
}

fn source_ids(space: &AdvisorySpaceEnvelope) -> Vec<String> {
    let mut ids = space
        .cells
        .iter()
        .flat_map(|cell| advisorygraphen_core::optional_string_array(cell, "source_ids"))
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids
}

fn projection_loss(space: &AdvisorySpaceEnvelope, report: &Value) -> Vec<Value> {
    let mut entries = vec![json!({
        "loss_type": "omitted_source_text",
        "description": "Source material is represented by structured records and summarized for this projection.",
        "omitted_ids": source_ids(space),
        "severity": "low"
    })];
    let code_derived_obstruction_ids: Vec<Value> = obstructions(report)
        .into_iter()
        .filter(|obstruction| {
            obstruction
                .pointer("/metadata/specificity")
                .and_then(Value::as_str)
                == Some("code_derived")
        })
        .filter_map(|obstruction| obstruction.get("id").cloned())
        .collect();
    if !code_derived_obstruction_ids.is_empty() {
        entries.push(json!({
            "loss_type": "lexical_detection_caveat",
            "description": "Code-derived findings are produced by lexical analysis and may miss shared middleware, dynamic wrappers, or framework-specific conventions; review is required before treating them as accepted fact.",
            "omitted_ids": code_derived_obstruction_ids,
            "severity": "medium"
        }));
    }
    entries
}

fn projection_loss_metrics(
    space: &AdvisorySpaceEnvelope,
    report: &Value,
    represented_ids: &[String],
    omitted_ids: &[String],
    loss_entries: &[Value],
) -> Value {
    let source_ids = source_ids(space);
    let represented_items = obstructions(report)
        .into_iter()
        .chain(completion_candidates(report))
        .collect::<Vec<_>>();
    let source_trace_gap_ids = represented_items
        .iter()
        .filter(|item| {
            item.get("source_ids")
                .and_then(Value::as_array)
                .is_none_or(Vec::is_empty)
                && item
                    .get("evidence_ids")
                    .and_then(Value::as_array)
                    .is_none_or(Vec::is_empty)
        })
        .filter_map(|item| item.get("id").cloned())
        .collect::<Vec<_>>();
    let collapsed_source_distinction_count = source_ids.len().saturating_sub(represented_ids.len());
    json!({
        "id": format!("projection-loss-metric:{}", space.space_id.trim_start_matches("space:")),
        "metric_type": "projection_loss_metric",
        "source_cardinality": source_ids.len(),
        "projected_cardinality": represented_ids.len(),
        "omitted_source_count": omitted_ids.len(),
        "collapsed_source_distinction_count": collapsed_source_distinction_count,
        "source_trace_gap_count": source_trace_gap_ids.len(),
        "source_trace_gap_ids": source_trace_gap_ids,
        "loss_declaration_count": loss_entries.len(),
        "missing_loss_declaration": loss_entries.is_empty(),
        "ambiguity": if collapsed_source_distinction_count > 0 || !omitted_ids.is_empty() {
            "declared_loss"
        } else {
            "none_detected"
        },
        "review_status": "unreviewed",
        "rule": "Finite metric for what the projection collapses, omits, or leaves without source trace."
    })
}

fn schema_morphisms(space: &AdvisorySpaceEnvelope) -> Value {
    let mut morphisms = space
        .morphisms
        .iter()
        .filter_map(|morphism| morphism.get("schema_morphism").cloned())
        .collect::<Vec<_>>();
    morphisms.extend(
        space
            .metadata
            .get("schema_morphisms")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    );
    morphisms.sort_by_key(|morphism| {
        morphism
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    });
    morphisms.dedup_by(|a, b| a.get("id") == b.get("id"));
    json!({
        "count": morphisms.len(),
        "morphisms": morphisms,
        "rule": "Schema morphisms describe contract evolution or lift mappings with compatibility, verification, and explicit loss claims."
    })
}

fn obstructions(report: &Value) -> Vec<Value> {
    report
        .pointer("/result/obstructions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn completion_candidates(report: &Value) -> Vec<Value> {
    let mut candidates = report
        .pointer("/result/completion_candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    candidates.extend(
        report
            .pointer("/related_reports/completions/result/completion_candidates")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    );
    candidates
}

fn partition_candidates(candidates: &[Value]) -> (Vec<Value>, Vec<Value>) {
    let mut live = Vec::new();
    let mut superseded = Vec::new();
    for candidate in candidates {
        if candidate.get("review_status").and_then(Value::as_str) == Some("superseded") {
            superseded.push(candidate.clone());
        } else {
            live.push(candidate.clone());
        }
    }
    (live, superseded)
}

fn obstructions_by_severity(obstructions: &[Value], severity: &str) -> Vec<Value> {
    obstructions
        .iter()
        .filter(|item| item["severity"] == severity)
        .cloned()
        .collect()
}

fn obstruction_counts(obstructions: &[Value]) -> Value {
    let mut high = 0_u64;
    let mut medium = 0_u64;
    let mut low = 0_u64;
    let mut unknown = 0_u64;
    for obstruction in obstructions {
        match obstruction
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
        {
            "high" => high += 1,
            "medium" => medium += 1,
            "low" => low += 1,
            _ => unknown += 1,
        }
    }
    json!({
        "high": high,
        "medium": medium,
        "low": low,
        "unknown": unknown
    })
}

fn candidate_quality_summary(candidates: &[Value]) -> Value {
    let mut source_derived = 0_u64;
    let mut requirement_derived = 0_u64;
    let mut code_derived = 0_u64;
    let mut topology_derived = 0_u64;
    let mut generic = 0_u64;
    let mut missing_precision_metadata = 0_u64;
    let mut source_backed = 0_u64;
    for candidate in candidates {
        match candidate
            .pointer("/metadata/specificity")
            .and_then(Value::as_str)
            .unwrap_or("missing")
        {
            "source_derived" => source_derived += 1,
            "requirement_derived" => requirement_derived += 1,
            "code_derived" => code_derived += 1,
            "topology_derived" => topology_derived += 1,
            "generic" => generic += 1,
            _ => missing_precision_metadata += 1,
        }
        if candidate
            .get("source_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .next()
            .is_some()
        {
            source_backed += 1;
        }
    }
    json!({
        "total": candidates.len(),
        "source_derived": source_derived,
        "requirement_derived": requirement_derived,
        "code_derived": code_derived,
        "topology_derived": topology_derived,
        "generic": generic,
        "source_backed": source_backed,
        "missing_precision_metadata": missing_precision_metadata
    })
}

fn proposal_content_summary(candidates: &[Value]) -> Value {
    let mut with_structured_content = 0_u64;
    let mut blocked_content = 0_u64;
    let mut candidate_content = 0_u64;
    let mut content_obstruction_count = 0_u64;
    let mut obstruction_types = serde_json::Map::new();

    for candidate in candidates {
        let Some(content) = candidate.get("proposal_content") else {
            continue;
        };
        with_structured_content += 1;
        match content
            .pointer("/scenario/status")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
        {
            "blocked" => blocked_content += 1,
            "candidate" => candidate_content += 1,
            _ => {}
        }
        for obstruction in content
            .get("content_obstructions")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            content_obstruction_count += 1;
            let key = obstruction
                .get("obstruction_type")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let count = obstruction_types
                .get(key)
                .and_then(Value::as_u64)
                .unwrap_or(0)
                + 1;
            obstruction_types.insert(key.to_string(), json!(count));
        }
    }

    json!({
        "with_structured_content": with_structured_content,
        "candidate_content": candidate_content,
        "blocked_content": blocked_content,
        "content_obstruction_count": content_obstruction_count,
        "content_obstruction_types": obstruction_types
    })
}

fn explicit_hypothesis_matrix(space: &AdvisorySpaceEnvelope) -> Value {
    let hypotheses = space
        .cells
        .iter()
        .filter(|cell| is_explicit_hypothesis(cell))
        .map(|hypothesis| {
            let id = hypothesis.get("id").and_then(Value::as_str).unwrap_or("cell:unknown");
            json!({
                "hypothesis_id": id,
                "title": hypothesis.get("title").cloned().unwrap_or(Value::Null),
                "status": explicit_hypothesis_status(hypothesis),
                "refinement_parent_ids": refinement_parent_ids_for(space, id),
                "refinement_child_ids": refinement_child_ids_for(space, id),
                "refinement_depth": refinement_depth_for(space, id),
                "refinement_status": refinement_status_for(space, id, hypothesis),
                "expected_observations": hypothesis.pointer("/metadata/expected_observations").cloned().unwrap_or_else(|| json!([])),
                "falsifiers": hypothesis.pointer("/metadata/falsifiers").cloned().unwrap_or_else(|| json!([])),
                "supporting_incidence_ids": relation_ids_for(space, id, &["supports", "supported_by"]),
                "falsifying_incidence_ids": relation_ids_for(space, id, &["falsifies", "falsified_by"]),
                "competing_hypothesis_ids": competing_ids_for(space, id),
                "remaining_uncertainty": remaining_hypothesis_uncertainty(space, hypothesis)
            })
        })
        .collect::<Vec<_>>();
    let mut counts = serde_json::Map::new();
    for hypothesis in &hypotheses {
        let status = hypothesis
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let current = counts.get(status).and_then(Value::as_u64).unwrap_or(0);
        counts.insert(status.to_string(), json!(current + 1));
    }
    json!({
        "count": hypotheses.len(),
        "status_counts": counts,
        "hypotheses": hypotheses,
        "rule": "Hypotheses should carry expected observations, falsifiers, support/falsify incidences, competing alternatives, and refinement lineage before driving proposals."
    })
}

fn merged_hypotheses(mut report_hypotheses: Vec<Value>, explicit_matrix: &Value) -> Vec<Value> {
    let mut known_ids = report_hypotheses
        .iter()
        .filter_map(|hypothesis| hypothesis.get("id").and_then(Value::as_str))
        .map(str::to_string)
        .collect::<Vec<_>>();
    known_ids.extend(
        report_hypotheses
            .iter()
            .filter_map(|hypothesis| hypothesis.get("hypothesis_id").and_then(Value::as_str))
            .map(str::to_string),
    );

    let explicit = explicit_matrix
        .get("hypotheses")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for hypothesis in explicit {
        let Some(id) = hypothesis.get("hypothesis_id").and_then(Value::as_str) else {
            continue;
        };
        if known_ids.iter().any(|known| known == id) {
            continue;
        }
        known_ids.push(id.to_string());
        report_hypotheses.push(json!({
            "id": id,
            "hypothesis_id": id,
            "title": hypothesis.get("title").cloned().unwrap_or(Value::Null),
            "lifecycle_status": hypothesis.get("status").cloned().unwrap_or_else(|| json!("candidate")),
            "status": hypothesis.get("status").cloned().unwrap_or_else(|| json!("candidate")),
            "source": "explicit_advisory_space",
            "expected_observations": hypothesis.get("expected_observations").cloned().unwrap_or_else(|| json!([])),
            "falsifiers": hypothesis.get("falsifiers").cloned().unwrap_or_else(|| json!([])),
            "supporting_incidence_ids": hypothesis.get("supporting_incidence_ids").cloned().unwrap_or_else(|| json!([])),
            "falsifying_incidence_ids": hypothesis.get("falsifying_incidence_ids").cloned().unwrap_or_else(|| json!([])),
            "competing_hypothesis_ids": hypothesis.get("competing_hypothesis_ids").cloned().unwrap_or_else(|| json!([])),
            "refinement_parent_ids": hypothesis.get("refinement_parent_ids").cloned().unwrap_or_else(|| json!([])),
            "refinement_child_ids": hypothesis.get("refinement_child_ids").cloned().unwrap_or_else(|| json!([])),
            "refinement_depth": hypothesis.get("refinement_depth").cloned().unwrap_or_else(|| json!(0)),
            "refinement_status": hypothesis.get("refinement_status").cloned().unwrap_or_else(|| json!("seed")),
            "remaining_uncertainty": hypothesis.get("remaining_uncertainty").cloned().unwrap_or_else(|| json!([]))
        }));
    }
    report_hypotheses
}

fn explicit_proposal_trace(space: &AdvisorySpaceEnvelope) -> Value {
    let proposals = space
        .cells
        .iter()
        .filter(|cell| cell["cell_type"] == "action")
        .map(|action| {
            let action_id = action.get("id").and_then(Value::as_str).unwrap_or("cell:unknown");
            let derived = explicit_derived_hypothesis_ids(space, action_id, action);
            json!({
                "action_id": action_id,
                "title": action.get("title").cloned().unwrap_or(Value::Null),
                "priority": action.pointer("/metadata/priority").cloned().unwrap_or(Value::Null),
                "derived_hypothesis_ids": derived,
                "derived_hypothesis_statuses": derived.iter().map(|id| {
                    json!({
                        "hypothesis_id": id,
                        "status": space.cells.iter()
                            .find(|cell| cell.get("id").and_then(Value::as_str) == Some(id.as_str()))
                            .map(explicit_hypothesis_status)
                            .unwrap_or("missing")
                    })
                }).collect::<Vec<_>>(),
                "required_verification": action.pointer("/metadata/required_verification").cloned().unwrap_or(Value::Null),
                "owner_state": if relation_ids_for(space, action_id, &["owns"]).is_empty() { "missing" } else { "present" },
                "proposal_quality_notes": proposal_quality_notes(space, action)
            })
        })
        .collect::<Vec<_>>();
    json!({
        "count": proposals.len(),
        "proposals": proposals,
        "rule": "Proposal trace is problem -> hypothesis -> evidence -> classification -> proposal -> required verification/owner."
    })
}

fn is_explicit_hypothesis(cell: &Value) -> bool {
    cell["cell_type"] == "hypothesis"
        || cell
            .pointer("/metadata/hypothesis")
            .and_then(Value::as_bool)
            == Some(true)
        || cell.pointer("/metadata/hypothesis_status").is_some()
        || cell.get("lifecycle_status").is_some()
}

fn explicit_hypothesis_status(cell: &Value) -> &str {
    cell.pointer("/metadata/hypothesis_status")
        .and_then(Value::as_str)
        .or_else(|| cell.get("lifecycle_status").and_then(Value::as_str))
        .unwrap_or("candidate")
}

fn relation_ids_for(
    space: &AdvisorySpaceEnvelope,
    target_id: &str,
    relation_types: &[&str],
) -> Vec<Value> {
    space
        .incidences
        .iter()
        .filter(|incidence| {
            let relation_type = incidence
                .get("relation_type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            relation_types.contains(&relation_type)
                && (incidence.get("from_id").and_then(Value::as_str) == Some(target_id)
                    || incidence.get("to_id").and_then(Value::as_str) == Some(target_id))
        })
        .filter_map(|incidence| incidence.get("id").cloned())
        .collect()
}

fn competing_ids_for(space: &AdvisorySpaceEnvelope, hypothesis_id: &str) -> Vec<Value> {
    space
        .incidences
        .iter()
        .filter(|incidence| {
            incidence.get("relation_type").and_then(Value::as_str) == Some("competes_with")
        })
        .filter_map(|incidence| {
            let from = incidence.get("from_id").and_then(Value::as_str)?;
            let to = incidence.get("to_id").and_then(Value::as_str)?;
            if from == hypothesis_id {
                Some(json!(to))
            } else if to == hypothesis_id {
                Some(json!(from))
            } else {
                None
            }
        })
        .collect()
}

fn refinement_parent_ids_for(space: &AdvisorySpaceEnvelope, hypothesis_id: &str) -> Vec<Value> {
    refinement_related_ids(space, hypothesis_id, RefinementDirection::Parent)
}

fn refinement_child_ids_for(space: &AdvisorySpaceEnvelope, hypothesis_id: &str) -> Vec<Value> {
    refinement_related_ids(space, hypothesis_id, RefinementDirection::Child)
}

enum RefinementDirection {
    Parent,
    Child,
}

fn refinement_related_ids(
    space: &AdvisorySpaceEnvelope,
    hypothesis_id: &str,
    direction: RefinementDirection,
) -> Vec<Value> {
    let mut ids = space
        .incidences
        .iter()
        .filter(|incidence| is_refinement_relation(incidence))
        .filter_map(|incidence| {
            let from = incidence.get("from_id").and_then(Value::as_str)?;
            let to = incidence.get("to_id").and_then(Value::as_str)?;
            match direction {
                RefinementDirection::Parent if from == hypothesis_id => Some(json!(to)),
                RefinementDirection::Child if to == hypothesis_id => Some(json!(from)),
                _ => None,
            }
        })
        .collect::<Vec<_>>();
    ids.sort_by_key(|id| id.as_str().unwrap_or_default().to_string());
    ids.dedup();
    ids
}

fn refinement_depth_for(space: &AdvisorySpaceEnvelope, hypothesis_id: &str) -> u64 {
    let mut depth = 0_u64;
    let mut current = hypothesis_id.to_string();
    let mut seen = vec![current.clone()];
    while let Some(parent) = refinement_parent_ids_for(space, &current)
        .into_iter()
        .find_map(|id| id.as_str().map(str::to_string))
    {
        if seen.contains(&parent) {
            break;
        }
        depth += 1;
        current = parent.clone();
        seen.push(parent);
    }
    depth
}

fn refinement_status_for(
    space: &AdvisorySpaceEnvelope,
    hypothesis_id: &str,
    hypothesis: &Value,
) -> &'static str {
    if hypothesis
        .pointer("/metadata/hypothesis_refinement")
        .and_then(Value::as_bool)
        == Some(true)
        || refinement_depth_for(space, hypothesis_id) > 0
    {
        "refined"
    } else if !refinement_child_ids_for(space, hypothesis_id).is_empty() {
        "has_refinements"
    } else {
        "seed"
    }
}

fn is_refinement_relation(incidence: &Value) -> bool {
    matches!(
        incidence.get("relation_type").and_then(Value::as_str),
        Some("refines" | "refined_from" | "revises" | "revised_from")
    )
}

fn remaining_hypothesis_uncertainty(
    space: &AdvisorySpaceEnvelope,
    hypothesis: &Value,
) -> Vec<Value> {
    let mut items = Vec::new();
    let id = hypothesis
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("cell:unknown");
    if hypothesis
        .pointer("/metadata/expected_observations")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        items.push(json!("missing_expected_observations"));
    }
    if hypothesis
        .pointer("/metadata/falsifiers")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        items.push(json!("missing_falsifiers"));
    }
    let status = explicit_hypothesis_status(hypothesis);
    if matches!(
        status,
        "supported" | "strongly_supported" | "supported_needs_followup"
    ) && relation_ids_for(space, id, &["supports", "supported_by"]).is_empty()
    {
        items.push(json!("missing_support_incidence"));
    }
    if status == "falsified"
        && relation_ids_for(space, id, &["falsifies", "falsified_by"]).is_empty()
    {
        items.push(json!("missing_falsifying_incidence"));
    }
    if hypothesis
        .pointer("/metadata/refinement_required")
        .and_then(Value::as_bool)
        == Some(true)
        && refinement_parent_ids_for(space, id).is_empty()
        && refinement_child_ids_for(space, id).is_empty()
    {
        items.push(json!("missing_refinement_lineage"));
    }
    items
}

fn explicit_derived_hypothesis_ids(
    space: &AdvisorySpaceEnvelope,
    action_id: &str,
    action: &Value,
) -> Vec<String> {
    let mut ids = Vec::new();
    if let Some(id) = action
        .pointer("/metadata/derived_from_hypothesis")
        .and_then(Value::as_str)
    {
        ids.push(normalize_projection_cell_id(id));
    }
    if let Some(id) = action
        .pointer("/metadata/derived_from_hypothesis_id")
        .and_then(Value::as_str)
    {
        ids.push(normalize_projection_cell_id(id));
    }
    if let Some(values) = action
        .pointer("/metadata/derived_from_hypotheses")
        .and_then(Value::as_array)
    {
        ids.extend(
            values
                .iter()
                .filter_map(Value::as_str)
                .map(normalize_projection_cell_id),
        );
    }
    ids.extend(
        space
            .incidences
            .iter()
            .filter(|incidence| {
                incidence.get("relation_type").and_then(Value::as_str) == Some("derives_from")
                    && incidence.get("from_id").and_then(Value::as_str) == Some(action_id)
            })
            .filter_map(|incidence| incidence.get("to_id").and_then(Value::as_str))
            .map(normalize_projection_cell_id),
    );
    ids.sort();
    ids.dedup();
    ids
}

fn normalize_projection_cell_id(id: &str) -> String {
    if id.starts_with("record:") {
        format!("cell:{}", id.trim_start_matches("record:"))
    } else {
        id.to_string()
    }
}

fn proposal_quality_notes(space: &AdvisorySpaceEnvelope, action: &Value) -> Vec<Value> {
    let action_id = action
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("cell:unknown");
    let mut notes = Vec::new();
    let derived = explicit_derived_hypothesis_ids(space, action_id, action);
    if derived.is_empty() {
        notes.push(json!("missing_hypothesis_trace"));
    }
    if action
        .pointer("/metadata/required_verification")
        .and_then(Value::as_str)
        .is_none_or(|value| value.trim().is_empty())
    {
        notes.push(json!("missing_required_verification"));
    }
    if relation_ids_for(space, action_id, &["owns"]).is_empty() {
        notes.push(json!("missing_owner"));
    }
    notes
}

fn recommendation_trace(candidates: &[Value]) -> Value {
    let mut primary = Vec::new();
    let mut alternatives = Vec::new();
    let mut follow_up = Vec::new();
    let mut unsupported = 0_u64;

    for candidate in candidates {
        let item = recommendation_trace_item(candidate);
        match candidate
            .get("recommendation_role")
            .and_then(Value::as_str)
            .unwrap_or("follow_up_observation")
        {
            "primary" => primary.push(item),
            "alternative" => alternatives.push(item),
            _ => {
                unsupported += 1;
                follow_up.push(item);
            }
        }
    }

    json!({
        "primary_count": primary.len(),
        "alternative_count": alternatives.len(),
        "follow_up_observation_count": follow_up.len(),
        "unsupported_hypothesis_candidate_count": unsupported,
        "primary_recommendations": primary,
        "alternatives": alternatives,
        "follow_up_observations": follow_up,
        "rule": "Only candidates derived from supported or accepted hypotheses can be primary recommendations."
    })
}

fn observation_actions(recommendation_trace: &Value) -> Value {
    let actions = recommendation_trace
        .get("follow_up_observations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|item| {
            item.get("ranked_observation_tasks")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        })
        .map(observation_action_from_task)
        .collect::<Vec<_>>();
    json!({
        "count": actions.len(),
        "actions": actions,
        "rule": "Observation actions recommend bounded evidence-gathering steps; they do not execute observations or accept the investigated claim."
    })
}

fn observation_action_from_task(task: Value) -> Value {
    let task_id = task
        .get("task_id")
        .and_then(Value::as_str)
        .unwrap_or("observation:unknown");
    let source_ids = task
        .get("source_ids_to_inspect")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let review_required = task
        .pointer("/pass_fail_extraction/review_required")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let mut target_claims = Vec::new();
    if let Some(hypothesis_id) = task.get("hypothesis_id").and_then(Value::as_str) {
        target_claims.push(json!(hypothesis_id));
    } else if let Some(hypothesis_id) = task.get("hypothesis_id").filter(|value| !value.is_null()) {
        target_claims.push(hypothesis_id.clone());
    }
    json!({
        "id": format!("observation-action:{}", id_fragment(task_id)),
        "task_id": task_id,
        "target_claim_ids": target_claims,
        "candidate_id": task.get("candidate_id").cloned().unwrap_or(Value::Null),
        "expected_evidence_kind": expected_evidence_kind(task.get("observation_type").and_then(Value::as_str)),
        "estimated_cost": estimated_observation_cost(source_ids.len()),
        "expected_information_gain": expected_information_gain(task.get("observation_type").and_then(Value::as_str)),
        "policy_blockers": if review_required { json!(["review_required"]) } else { json!([]) },
        "source_ids_to_inspect": source_ids,
        "expected_observation": task.get("expected_observation").cloned().unwrap_or(Value::Null),
        "falsifier": task.get("falsifier").cloned().unwrap_or(Value::Null),
        "output_schema": task.get("output_schema").cloned().unwrap_or(Value::Null),
        "review_status": "unreviewed",
        "provenance": {
            "origin": "inferred",
            "actor": "advisorygraphen-projection",
            "confidence": 0.7,
            "review_status": "unreviewed"
        }
    })
}

fn expected_evidence_kind(observation_type: Option<&str>) -> &'static str {
    match observation_type {
        Some("hypothesis_support") => "support_or_falsification_witness",
        Some("proposal_structure_completion") => "structure_witness",
        Some("review_readiness") => "review_readiness_witness",
        _ => "bounded_observation_witness",
    }
}

fn expected_information_gain(observation_type: Option<&str>) -> &'static str {
    match observation_type {
        Some("hypothesis_support") => "high",
        Some("proposal_structure_completion") => "medium",
        Some("review_readiness") => "medium",
        _ => "unknown",
    }
}

fn estimated_observation_cost(source_count: usize) -> &'static str {
    match source_count {
        0 | 1 => "low",
        2 | 3 => "medium",
        _ => "high",
    }
}

fn recommendation_trace_item(candidate: &Value) -> Value {
    json!({
        "candidate_id": candidate.get("id").cloned().unwrap_or(Value::Null),
        "title": candidate.get("title").cloned().unwrap_or(Value::Null),
        "candidate_type": candidate.get("candidate_type").cloned().unwrap_or(Value::Null),
        "recommendation_role": candidate.get("recommendation_role").cloned().unwrap_or_else(|| json!("follow_up_observation")),
        "derived_hypothesis_id": candidate.pointer("/hypothesis_trace/derived_hypothesis_id").cloned().unwrap_or(Value::Null),
        "hypothesis_lifecycle_status": candidate.pointer("/hypothesis_trace/lifecycle_status").cloned().unwrap_or(Value::Null),
        "supported_hypothesis_ids": candidate.get("supported_hypothesis_ids").cloned().unwrap_or_else(|| json!([])),
        "unsupported_hypothesis_ids": candidate.get("unsupported_hypothesis_ids").cloned().unwrap_or_else(|| json!([])),
        "required_verification": candidate.pointer("/proposal_content/content_obstructions")
            .and_then(Value::as_array)
            .map(|items| {
                items.iter()
                    .filter_map(|item| item.get("required_resolution").and_then(Value::as_str))
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        "ranked_observation_tasks": ranked_observation_tasks(candidate)
    })
}

fn ranked_observation_tasks(candidate: &Value) -> Vec<Value> {
    let mut tasks = Vec::new();
    let candidate_id = candidate
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("candidate:unknown");
    let title = candidate
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("candidate");
    let candidate_type = candidate
        .get("candidate_type")
        .and_then(Value::as_str)
        .unwrap_or("completion_candidate");
    let source_ids = candidate
        .get("source_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let unsupported_hypothesis_ids = candidate
        .get("unsupported_hypothesis_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut rank = 1_u64;

    for hypothesis_id in unsupported_hypothesis_ids.iter().filter_map(Value::as_str) {
        tasks.push(json!({
            "rank": rank,
            "task_id": format!("observation:{}:support-{}", id_tail(candidate_id), rank),
            "observation_type": "hypothesis_support",
            "candidate_id": candidate_id,
            "hypothesis_id": hypothesis_id,
            "source_ids_to_inspect": source_ids,
            "command_template": observation_command_template(candidate_type),
            "required_inputs": required_observation_inputs(candidate_type),
            "output_schema": observation_output_schema(),
            "pass_fail_extraction": pass_fail_extraction(candidate_type),
            "expected_observation": expected_observation(candidate_type, title),
            "falsifier": falsifier_observation(candidate_type, title),
            "weakens_hypothesis_ids": competing_hypotheses(candidate, hypothesis_id),
            "promotion_effect": "If this observation supports the hypothesis, review-gated hypothesis support or acceptance can allow the candidate to be reconsidered as primary or alternative."
        }));
        rank += 1;
    }

    for obstruction in candidate
        .pointer("/proposal_content/content_obstructions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if obstruction.get("obstruction_type").and_then(Value::as_str)
            != Some("proposal_content_underspecified")
        {
            continue;
        }
        tasks.push(json!({
            "rank": rank,
            "task_id": format!("observation:{}:complete-structure", id_tail(candidate_id)),
            "observation_type": "proposal_structure_completion",
            "candidate_id": candidate_id,
            "hypothesis_id": candidate.pointer("/hypothesis_trace/derived_hypothesis_id").cloned().unwrap_or(Value::Null),
            "source_ids_to_inspect": source_ids,
            "command_template": "Inspect the candidate proposal_content and source snapshot, then draft the exact cell or incidence required to repair the obstruction.",
            "required_inputs": [
                "candidate_id",
                "proposal_content",
                "resolves_obstruction_ids",
                "bounded_source_snapshot"
            ],
            "output_schema": observation_output_schema(),
            "pass_fail_extraction": {
                "pass_when": "The output names concrete cells or incidences to add and maps them to the repaired obstruction.",
                "fail_when": "The output cannot name concrete structure without inventing facts outside the source boundary.",
                "review_required": true
            },
            "expected_observation": "Identify the concrete cell or incidence that would be added, plus the exact obstruction it repairs.",
            "falsifier": "No concrete structure can be named without inventing facts beyond the bounded source snapshot.",
            "weakens_hypothesis_ids": [],
            "promotion_effect": "A concrete proposed structure removes underspecification but still requires review before acceptance."
        }));
        rank += 1;
    }

    if tasks.is_empty()
        && candidate.get("recommendation_role").and_then(Value::as_str) != Some("primary")
    {
        tasks.push(json!({
            "rank": rank,
            "task_id": format!("observation:{}:review-readiness", id_tail(candidate_id)),
            "observation_type": "review_readiness",
            "candidate_id": candidate_id,
            "hypothesis_id": candidate.pointer("/hypothesis_trace/derived_hypothesis_id").cloned().unwrap_or(Value::Null),
            "source_ids_to_inspect": source_ids,
            "command_template": "Review candidate evidence, owners, verification fields, and proposal_content obstructions for promotion readiness.",
            "required_inputs": [
                "candidate_id",
                "supported_hypothesis_ids",
                "unsupported_hypothesis_ids",
                "proposal_content.content_obstructions"
            ],
            "output_schema": observation_output_schema(),
            "pass_fail_extraction": {
                "pass_when": "The candidate has supported or accepted hypotheses and no unresolved content obstructions.",
                "fail_when": "The candidate still depends on unreviewed hypotheses or underspecified proposal content.",
                "review_required": true
            },
            "expected_observation": "Confirm whether the candidate has enough accepted evidence, owner, and verification to enter review.",
            "falsifier": "The candidate still depends on inferred or unreviewed structure.",
            "weakens_hypothesis_ids": [],
            "promotion_effect": "A positive readiness observation identifies the next explicit review event; a negative one keeps the candidate as follow-up."
        }));
    }

    tasks
}

fn observation_command_template(candidate_type: &str) -> &'static str {
    match candidate_type {
        "owner_assignment" | "ownership_clarification" => {
            "Inspect source_ids_to_inspect for ownership evidence and return the owner claim, source id, and contradiction status."
        }
        "proposed_test" | "proposed_metric" => {
            "Inspect source_ids_to_inspect and define the smallest verification method, metric, or review path that can verify the requirement."
        }
        "proposed_interface" => {
            "Inspect boundary witnesses and source_ids_to_inspect, then identify the minimal interface contract and owner evidence."
        }
        "proposed_auth_guard" => {
            "Inspect route evidence and shared middleware evidence, then decide whether a route-specific auth guard is required."
        }
        _ => {
            "Inspect source_ids_to_inspect and candidate evidence, then return support, falsification, or insufficient-evidence status."
        }
    }
}

fn required_observation_inputs(candidate_type: &str) -> Vec<&'static str> {
    let mut inputs = vec![
        "candidate_id",
        "hypothesis_id",
        "source_ids_to_inspect",
        "expected_observation",
        "falsifier",
    ];
    match candidate_type {
        "owner_assignment" | "ownership_clarification" => {
            inputs.push("owner_cell_id");
            inputs.push("blocked_cell_id");
        }
        "proposed_interface" => {
            inputs.push("from_cell_id");
            inputs.push("to_cell_id");
        }
        _ => {}
    }
    inputs
}

fn observation_output_schema() -> Value {
    json!({
        "type": "object",
        "required": [
            "observation_status",
            "evidence_ids",
            "summary",
            "supports_hypothesis",
            "falsifies_hypothesis"
        ],
        "properties": {
            "observation_status": {
                "enum": [
                    "supports",
                    "falsifies",
                    "insufficient_evidence",
                    "requires_human_review"
                ]
            },
            "evidence_ids": {
                "type": "array",
                "items": { "type": "string" }
            },
            "summary": { "type": "string" },
            "supports_hypothesis": { "type": "boolean" },
            "falsifies_hypothesis": { "type": "boolean" },
            "review_note": { "type": "string" }
        }
    })
}

fn pass_fail_extraction(candidate_type: &str) -> Value {
    match candidate_type {
        "owner_assignment" | "ownership_clarification" => json!({
            "pass_when": "A source-backed owner or ownership incidence is identified for the blocked action.",
            "fail_when": "No owner evidence exists, a different accepted owner is found, or ownership is explicitly collective.",
            "review_required": true
        }),
        "proposed_test" | "proposed_metric" => json!({
            "pass_when": "A concrete verification method, metric, or review path can be named and linked to the requirement.",
            "fail_when": "The requirement is exploratory, already verified, or cannot be verified within the source boundary.",
            "review_required": true
        }),
        "proposed_interface" => json!({
            "pass_when": "The boundary need and minimal interface contract are both supported by source evidence.",
            "fail_when": "The direct dependency is absent, already mediated, or cannot be tied to a source-backed requirement.",
            "review_required": true
        }),
        "proposed_auth_guard" => json!({
            "pass_when": "The route touches protected data and lacks an effective shared or route-specific guard.",
            "fail_when": "Existing middleware protects the route or the data is intentionally public.",
            "review_required": true
        }),
        _ => json!({
            "pass_when": "Source-backed evidence supports the blocking hypothesis and weakens relevant alternatives.",
            "fail_when": "Evidence falsifies the hypothesis or remains insufficient after inspecting bounded sources.",
            "review_required": true
        }),
    }
}

fn expected_observation(candidate_type: &str, title: &str) -> String {
    match candidate_type {
        "owner_assignment" | "ownership_clarification" => format!(
            "Find source-backed evidence that the proposed owner is accountable for `{title}`."
        ),
        "proposed_test" | "proposed_metric" => format!(
            "Find or define a verification method that would demonstrate `{title}` without relying on agent inference."
        ),
        "proposed_interface" => format!(
            "Confirm the current cross-boundary need and the minimal interface shape required for `{title}`."
        ),
        "proposed_auth_guard" => format!(
            "Confirm the route lacks an effective guard and identify the guard required for `{title}`."
        ),
        _ => format!("Collect source-backed evidence that justifies `{title}`."),
    }
}

fn falsifier_observation(candidate_type: &str, title: &str) -> String {
    match candidate_type {
        "owner_assignment" | "ownership_clarification" => format!(
            "A different accepted owner is documented for `{title}`, or ownership is intentionally collective."
        ),
        "proposed_test" | "proposed_metric" => format!(
            "The requirement behind `{title}` is exploratory or already verified by an existing accepted relation."
        ),
        "proposed_interface" => format!(
            "The direct dependency behind `{title}` is not present, or an accepted interface already exists."
        ),
        "proposed_auth_guard" => format!(
            "The route behind `{title}` is already protected by middleware or does not touch protected data."
        ),
        _ => format!("Accepted evidence contradicts the need for `{title}`."),
    }
}

fn competing_hypotheses(candidate: &Value, current_hypothesis_id: &str) -> Vec<Value> {
    candidate
        .get("unsupported_hypothesis_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|id| *id != current_hypothesis_id)
        .map(|id| json!(id))
        .collect()
}

fn hypothesis_promotion_workflow(recommendation_trace: &Value) -> Value {
    let items: Vec<Value> = recommendation_trace
        .get("follow_up_observations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|item| {
            let observation_task_ids: Vec<Value> = item
                .get("ranked_observation_tasks")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|task| task.get("task_id").cloned())
                .collect();
            json!({
                "candidate_id": item.get("candidate_id").cloned().unwrap_or(Value::Null),
                "current_role": item.get("recommendation_role").cloned().unwrap_or_else(|| json!("follow_up_observation")),
                "blocking_hypothesis_ids": item.get("unsupported_hypothesis_ids").cloned().unwrap_or_else(|| json!([])),
                "observation_task_ids": observation_task_ids,
                "next_command_drafts": promotion_command_drafts(item),
                "promotion_steps": [
                    "Run the ranked observation tasks against the bounded source snapshot.",
                    "Record source-backed evidence that supports, weakens, or falsifies the blocking hypothesis.",
                    "Use a review-gated hypothesis support or accept command only after evidence exists.",
                    "Rerun completions propose and project ai_agent; promote only if the candidate no longer has unsupported hypotheses or content obstructions."
                ]
            })
        })
        .collect();

    json!({
        "workflow_rule": "Follow-up observations become primary recommendations only through source-backed hypothesis review and a fresh projection.",
        "review_gated_commands": [
            "hypothesis support",
            "hypothesis accept",
            "completions accept"
        ],
        "item_count": items.len(),
        "items": items
    })
}

fn promotion_command_drafts(item: &Value) -> Value {
    let task_ids: Vec<Value> = item
        .get("ranked_observation_tasks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|task| task.get("task_id").cloned())
        .collect();
    let hypothesis_id = item
        .get("unsupported_hypothesis_ids")
        .and_then(Value::as_array)
        .and_then(|ids| ids.first())
        .and_then(Value::as_str)
        .unwrap_or("<hypothesis-id>");
    json!({
        "record_observation": {
            "command": "advisorygraphen observation record --store STORE --space-id SPACE_ID --from-projection AI_AGENT.json --task-id TASK_ID --result OBSERVATION_RESULT.json --reviewer REVIEWER --reason REASON --base-revision REVISION --format json",
            "task_ids": task_ids
        },
        "support_hypothesis": {
            "command": format!("advisorygraphen hypothesis support --store STORE --from-report CHECK.json --hypothesis-id {hypothesis_id} --evidence EVIDENCE_CELL_ID --reviewer REVIEWER --reason REASON --base-revision REVISION --format json"),
            "requires": [
                "observation result with observation_status=supports",
                "evidence cell produced by observation record"
            ]
        },
        "falsify_hypothesis": {
            "command": format!("advisorygraphen hypothesis falsify --store STORE --from-report CHECK.json --hypothesis-id {hypothesis_id} --evidence EVIDENCE_CELL_ID --reviewer REVIEWER --reason REASON --base-revision REVISION --format json"),
            "requires": [
                "observation result with observation_status=falsifies",
                "evidence cell produced by observation record"
            ]
        }
    })
}

fn id_tail(id: &str) -> String {
    id.rsplit(':').next().unwrap_or(id).replace('_', "-")
}

fn id_fragment(id: &str) -> String {
    id.trim_start_matches("observation:")
        .replace([':', '_'], "-")
}

fn close_status_value(space: &AdvisorySpaceEnvelope, report: &Value) -> Value {
    let envelope = serde_json::from_value(report.clone()).unwrap_or_else(|_| {
        advisorygraphen_core::ReportEnvelope::new("check", None, json!({}), json!({}))
    });
    close_status(space, &envelope)
}
