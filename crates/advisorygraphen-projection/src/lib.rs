use advisorygraphen_core::{AdvisoryError, AdvisoryResult, AdvisorySpaceEnvelope};
use advisorygraphen_reasoning::close_status;
use serde_json::{json, Value};

mod higher;

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
            "high_severity_obstructions": obstructions.iter().filter(|item| item["severity"] == "high").cloned().collect::<Vec<_>>(),
            "unreviewed_candidates_are_not_accepted": true
        },
        "projection_loss": projection_loss(space),
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
        "projection_loss": projection_loss(space),
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
        "projection_loss": projection_loss(space),
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
    let higher_graphen = higher::projection_result_json(
        space,
        report,
        audience,
        represented_ids.clone(),
        omitted_ids.clone(),
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
                "project ai_agent",
                "project audit_trace",
                "case import",
                "case reason"
            ],
            "review_gated_commands": [
                "completions accept",
                "completions reject"
            ],
            "forbidden_operations": [
                "promote unreviewed candidate structure",
                "hide projection_loss",
                "treat inferred evidence as accepted fact",
                "rewrite source material outside the bounded snapshot"
            ],
            "resume_protocol": [
                "read close_status",
                "inspect open_obstructions",
                "inspect candidate_review_state",
                "inspect blocker_resolution_state.application_requirements when present",
                "propose missing owner or verification structure",
                "generate audit_trace before reporting final state"
            ]
        },
        "open_obstructions": obstructions(report),
        "candidate_review_state": completion_candidates(report),
        "next_safe_operations": [
            "review_obstructions",
            "inspect_application_requirements",
            "propose_or_review_candidates",
            "generate_audit_projection"
        ],
        "close_status": close_status(space, &serde_json::from_value(report.clone()).unwrap_or_else(|_| advisorygraphen_core::ReportEnvelope::new("check", None, json!({}), json!({})))),
        "projection_loss": projection_loss(space),
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

fn projection_loss(space: &AdvisorySpaceEnvelope) -> Vec<Value> {
    vec![json!({
        "loss_type": "omitted_source_text",
        "description": "Source material is represented by structured records and summarized for this projection.",
        "omitted_ids": source_ids(space),
        "severity": "low"
    })]
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
