use advisorygraphen_core::{AdvisoryError, AdvisoryResult, AdvisorySpaceEnvelope};
use higher_graphen_core::{Id, Severity};
use higher_graphen_projection::{
    InformationLoss, OutputSchema, Projection, ProjectionAudience, ProjectionEntry,
    ProjectionOutput, ProjectionPurpose, ProjectionResult, ProjectionSelector, RendererKind,
};
use serde_json::Value;
use std::collections::BTreeSet;

pub fn projection_result_json(
    space: &AdvisorySpaceEnvelope,
    report: &Value,
    audience: &str,
    represented_ids: Vec<String>,
    omitted_ids: Vec<String>,
) -> AdvisoryResult<Value> {
    let projection = Projection::new(
        id(&format!(
            "projection:higher:{}:{}",
            audience,
            space.space_id.trim_start_matches("space:")
        ))?,
        id(&space.space_id)?,
        format!("AdvisoryGraphen {audience} projection"),
        audience_for(audience)?,
        purpose_for(audience),
        selector(report)?,
        OutputSchema::key_value(["represented_ids", "omitted_ids", "report_type"])
            .map_err(hg_err)?,
        [information_loss(&omitted_ids, space)?],
    )
    .map_err(hg_err)?
    .with_renderer(renderer_for(audience)?);

    let output = ProjectionOutput::key_value([
        ProjectionEntry::new(
            "represented_ids",
            joined_value(&represented_ids),
            source_ids_for_result(&represented_ids, space)?,
        )
        .map_err(hg_err)?,
        ProjectionEntry::new(
            "omitted_ids",
            joined_value(&omitted_ids),
            source_ids_for_result(&omitted_ids, space)?,
        )
        .map_err(hg_err)?,
        ProjectionEntry::new(
            "report_type",
            report
                .get("report_type")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            [id(&space.space_id)?],
        )
        .map_err(hg_err)?,
    ])
    .map_err(hg_err)?;

    let result = ProjectionResult::from_projection(
        &projection,
        projection
            .renderer
            .clone()
            .unwrap_or(RendererKind::Structured),
        output,
        source_ids_for_result(&represented_ids, space)?,
        projection.information_loss.clone(),
    )
    .map_err(hg_err)?;
    serde_json::to_value(result).map_err(AdvisoryError::from)
}

fn selector(report: &Value) -> AdvisoryResult<ProjectionSelector> {
    let obstruction_ids = report
        .pointer("/result/obstructions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("id").and_then(Value::as_str))
        .map(id)
        .collect::<AdvisoryResult<Vec<_>>>()?;
    Ok(ProjectionSelector::all()
        .with_obstruction_ids(obstruction_ids)
        .with_min_severity(Severity::Low))
}

fn information_loss(
    omitted_ids: &[String],
    space: &AdvisorySpaceEnvelope,
) -> AdvisoryResult<InformationLoss> {
    let source_ids = source_ids_from_strings(omitted_ids)?;
    let source_ids = if source_ids.is_empty() {
        let space_source_ids = source_ids_from_space(space)?;
        if space_source_ids.is_empty() {
            vec![id(&space.space_id)?]
        } else {
            space_source_ids
        }
    } else {
        source_ids
    };
    InformationLoss::declared(
        "Projection omits or summarizes source material from the advisory space.",
        source_ids,
    )
    .map_err(hg_err)
}

fn source_ids_for_result(
    candidates: &[String],
    space: &AdvisorySpaceEnvelope,
) -> AdvisoryResult<Vec<Id>> {
    let ids = source_ids_from_strings(candidates)?;
    if ids.is_empty() {
        Ok(vec![id(&space.space_id)?])
    } else {
        Ok(ids)
    }
}

fn source_ids_from_strings(values: &[String]) -> AdvisoryResult<Vec<Id>> {
    values.iter().map(|value| id(value)).collect()
}

fn source_ids_from_space(space: &AdvisorySpaceEnvelope) -> AdvisoryResult<Vec<Id>> {
    space
        .cells
        .iter()
        .flat_map(|cell| {
            cell.get("source_ids")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(id)
        .collect()
}

fn joined_value(values: &[String]) -> String {
    if values.is_empty() {
        "(none)".to_string()
    } else {
        values.join(",")
    }
}

fn audience_for(value: &str) -> AdvisoryResult<ProjectionAudience> {
    match value {
        "executive" | "client_review" | "cli" => Ok(ProjectionAudience::Executive),
        "developer_action" => Ok(ProjectionAudience::Developer),
        "audit_trace" => Ok(ProjectionAudience::Audit),
        "ai_agent" => Ok(ProjectionAudience::AiAgent),
        other => Err(AdvisoryError::UnsupportedAudience(other.to_string())),
    }
}

fn purpose_for(value: &str) -> ProjectionPurpose {
    match value {
        "developer_action" | "ai_agent" => ProjectionPurpose::ActionPlan,
        "audit_trace" => ProjectionPurpose::Review,
        _ => ProjectionPurpose::Report,
    }
}

fn renderer_for(value: &str) -> AdvisoryResult<RendererKind> {
    match value {
        "executive" | "client_review" | "cli" => Ok(RendererKind::Markdown),
        "developer_action" | "audit_trace" | "ai_agent" => Ok(RendererKind::Structured),
        other => RendererKind::custom(format!("advisory-{other}")).map_err(hg_err),
    }
}

fn id(value: &str) -> AdvisoryResult<Id> {
    Id::new(value).map_err(hg_err)
}

fn hg_err(error: higher_graphen_core::CoreError) -> AdvisoryError {
    AdvisoryError::Validation(format!("higher-graphen projection: {error}"))
}
