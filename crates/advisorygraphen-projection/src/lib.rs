use advisorygraphen_core::{AdvisoryError, AdvisoryResult, AdvisorySpaceEnvelope};
use advisorygraphen_reasoning::close_status;
use serde_json::{json, Value};

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
        "executive" => Ok(executive_projection(space, report)),
        "developer_action" => Ok(developer_projection(space, report)),
        "audit_trace" => Ok(audit_projection(space, report)),
        "ai_agent" => Ok(ai_agent_projection(space, report)),
        "todoist_task_export" => Ok(todoist_projection(space, report)),
        "client_review" | "cli" => Ok(executive_projection(space, report)),
        other => Err(AdvisoryError::UnsupportedAudience(other.to_string())),
    }
}

pub fn todoist_projection(space: &AdvisorySpaceEnvelope, report: &Value) -> Value {
    let blocked_tasks = completion_candidates(report)
        .into_iter()
        .filter(|candidate| {
            candidate.get("candidate_type").and_then(Value::as_str) == Some("proposed_refactor_action")
                && candidate.get("review_status").and_then(Value::as_str) != Some("accepted")
        })
        .map(|candidate| {
            json!({
                "source_id": candidate["id"].clone(),
                "reason": "Completion candidate is unreviewed. Todoist export requires accepted action or draft export policy.",
                "export_status": "blocked_missing_review"
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema": "advisorygraphen.todoist.projection.v1",
        "projection_id": format!("projection:todoist:{}", space.space_id.trim_start_matches("space:advisory:")),
        "space_id": space.space_id,
        "tasks": [],
        "blocked_tasks": blocked_tasks,
        "projection_loss": projection_loss(space)
    })
}

fn executive_projection(space: &AdvisorySpaceEnvelope, report: &Value) -> Value {
    let obstructions = obstructions(report);
    json!({
        "schema": "advisorygraphen.projection.v1",
        "projection_id": format!("projection:executive:{}", space.space_id.trim_start_matches("space:advisory:")),
        "audience": "executive",
        "space_id": space.space_id,
        "represented_ids": represented_ids(report),
        "omitted_ids": source_ids(space),
        "summary": {
            "high_severity_obstructions": obstructions.iter().filter(|item| item["severity"] == "high").cloned().collect::<Vec<_>>(),
            "unreviewed_candidates_are_not_accepted": true
        },
        "projection_loss": projection_loss(space)
    })
}

fn developer_projection(space: &AdvisorySpaceEnvelope, report: &Value) -> Value {
    json!({
        "schema": "advisorygraphen.projection.v1",
        "projection_id": format!("projection:developer-action:{}", space.space_id.trim_start_matches("space:advisory:")),
        "audience": "developer_action",
        "space_id": space.space_id,
        "represented_ids": represented_ids(report),
        "omitted_ids": source_ids(space),
        "actions": completion_candidates(report),
        "projection_loss": projection_loss(space)
    })
}

fn audit_projection(space: &AdvisorySpaceEnvelope, report: &Value) -> Value {
    json!({
        "schema": "advisorygraphen.projection.v1",
        "projection_id": format!("projection:audit:{}", space.space_id.trim_start_matches("space:advisory:")),
        "audience": "audit_trace",
        "space_id": space.space_id,
        "represented_ids": represented_ids(report),
        "omitted_ids": [],
        "source_boundary": space.metadata.get("source_boundary").cloned().unwrap_or_else(|| json!({})),
        "report": report,
        "projection_loss": projection_loss(space)
    })
}

fn ai_agent_projection(space: &AdvisorySpaceEnvelope, report: &Value) -> Value {
    json!({
        "schema": "advisorygraphen.projection.v1",
        "projection_id": format!("projection:ai-agent:{}", space.space_id.trim_start_matches("space:advisory:")),
        "audience": "ai_agent",
        "space_id": space.space_id,
        "represented_ids": represented_ids(report),
        "omitted_ids": source_ids(space),
        "next_safe_operations": ["review_obstructions", "propose_or_review_candidates", "generate_audit_projection"],
        "close_status": close_status(space, &serde_json::from_value(report.clone()).unwrap_or_else(|_| advisorygraphen_core::ReportEnvelope::new("check", None, json!({}), json!({})))),
        "projection_loss": projection_loss(space)
    })
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
    report
        .pointer("/result/completion_candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}
