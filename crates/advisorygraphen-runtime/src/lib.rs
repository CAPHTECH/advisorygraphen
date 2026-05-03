use advisorygraphen_core::{
    validate_document, AdvisoryError, AdvisoryResult, AdvisorySpaceEnvelope, ReportEnvelope,
    REVIEW_EVENT_SCHEMA,
};
use advisorygraphen_interpretation::InterpretationPackage;
use advisorygraphen_lift::lift_snapshot;
use advisorygraphen_projection::{build_projection, project};
use advisorygraphen_reasoning::{
    blocker_resolution_state, check_space, close_status, frontier_items, propose_completions,
    waiting_items,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

mod case_review;
mod code_snapshot;
mod dogfood;
mod options;
mod projection_report;
mod review;
use case_review::apply_candidate_reviews;
pub use code_snapshot::{code_repo_snapshot_workflow, CodeRepoSnapshotOptions};
pub use dogfood::{dogfood_repo_snapshot_workflow, DogfoodRepoSnapshotOptions};
pub use options::{
    CaseCloseCheckOptions, CaseImportOptions, CaseReasonOptions, CheckOptions,
    CompletionProposeOptions, LiftOptions, ProjectOptions, ReviewOptions, ValidateOptions,
};
use projection_report::{attach_completion_report, read_projection_report};
use review::{higher_graphen_completion_review, review_report_path, review_space_id};
pub fn validate_workflow(options: &ValidateOptions) -> AdvisoryResult<Value> {
    let value = read_json(&options.input)?;
    let schema = options.schema.as_deref().map(canonical_schema_name);
    let report = validate_document(&value, schema.as_deref())?;
    Ok(serde_json::to_value(report)?)
}
pub fn lift_workflow(options: &LiftOptions) -> AdvisoryResult<AdvisorySpaceEnvelope> {
    let snapshot = read_json(&options.input)?;
    let package = InterpretationPackage::load(&options.package)?;
    let space = lift_snapshot(&snapshot, &package)?;
    write_json_if_requested(&options.output, &space)?;
    let _ = &options.command;
    Ok(space)
}
pub fn check_workflow(options: &CheckOptions) -> AdvisoryResult<ReportEnvelope> {
    let space = read_space(&options.space)?;
    let report = check_space(
        &space,
        &options.ruleset,
        options.fail_on,
        options.command.as_deref(),
    )?;
    write_json_if_requested(&options.output, &report)?;
    Ok(report)
}
pub fn completions_propose_workflow(
    options: &CompletionProposeOptions,
) -> AdvisoryResult<ReportEnvelope> {
    let space = read_space(&options.space)?;
    let check_report: ReportEnvelope = serde_json::from_value(read_json(&options.from_report)?)?;
    let report = propose_completions(
        &space,
        &check_report,
        file_name(&options.from_report),
        options.command.as_deref(),
    )?;
    write_json_if_requested(&options.output, &report)?;
    Ok(report)
}
pub fn review_workflow(options: &ReviewOptions) -> AdvisoryResult<Value> {
    fs::create_dir_all(&options.store)?;
    let from_report = review_report_path(options)?;
    let space_id = review_space_id(from_report)?;
    let reviewed_at = Utc::now().to_rfc3339();
    let higher_graphen_review =
        higher_graphen_completion_review(options, from_report, &reviewed_at)?;
    let head = read_imported_space_head(&options.store, &space_id)?;
    let materialized_space = read_materialized_space(&options.store, &space_id)?;
    ensure_base_revision(Some(&head), options.base_revision.as_deref())?;
    let sequence = next_sequence(&options.store, &space_id);
    let target_revision = format!("revision:review-{sequence:06}");
    let candidate_slug = options.candidate_id.trim_start_matches("candidate:");
    let review_event_id = format!("review:{}:{candidate_slug}-{sequence:06}", options.outcome);
    let event = json!({
        "schema": REVIEW_EVENT_SCHEMA,
        "review_event_id": review_event_id,
        "engagement_id": materialized_space.engagement_id,
        "target_ids": [options.candidate_id],
        "outcome": options.outcome,
        "reviewer_id": options.reviewer,
        "reviewed_at": reviewed_at,
        "reason": options.reason,
        "evidence_ids": [],
        "base_revision_id": options.base_revision,
        "metadata": {
            "from_report": options.from_report.as_ref().map(|path| path.display().to_string()),
            "higher_graphen": higher_graphen_review
        }
    });
    validate_document(&event, Some(REVIEW_EVENT_SCHEMA))?;
    append_store_event(
        &options.store,
        &json!({
            "schema": "advisorygraphen.case.log.entry.v1",
            "case_space_id": space_id.clone(),
            "sequence": sequence,
            "entry_id": format!("log:{sequence:06}"),
            "morphism_id": format!("morphism:{}-{candidate_slug}", options.outcome),
            "source_revision_id": head,
            "target_revision_id": target_revision.clone(),
            "actor": event["reviewer_id"],
            "recorded_at": Utc::now().to_rfc3339(),
            "previous_entry_hash": null,
            "entry_hash": null,
            "payload": event
        }),
    )?;
    fs::write(
        space_dir(&options.store, &space_id).join("HEAD"),
        &target_revision,
    )?;
    Ok(event)
}
pub fn project_workflow(options: &ProjectOptions) -> AdvisoryResult<String> {
    let space = read_space(&options.space)?;
    let report = read_projection_report(&options.report, options.completions_report.as_deref())?;
    let rendered = project(&space, &report, &options.audience, options.format)?;
    write_string_if_requested(&options.output, &rendered)?;
    Ok(rendered)
}
pub fn case_import_workflow(options: &CaseImportOptions) -> AdvisoryResult<Value> {
    let space = read_space(&options.space)?;
    let dir = space_dir(&options.store, &space.space_id);
    fs::create_dir_all(dir.join("materialized"))?;
    fs::create_dir_all(dir.join("logs"))?;
    fs::write(
        dir.join("materialized/space.json"),
        serde_json::to_vec_pretty(&space)?,
    )?;
    fs::write(dir.join("HEAD"), &options.revision_id)?;
    let log_entry = json!({
        "schema": "advisorygraphen.case.log.entry.v1",
        "case_space_id": space.space_id,
        "sequence": 1,
        "entry_id": "log:000001",
        "morphism_id": "morphism:import",
        "source_revision_id": null,
        "target_revision_id": options.revision_id,
        "actor": "advisorygraphen",
        "recorded_at": Utc::now().to_rfc3339(),
        "previous_entry_hash": null,
        "entry_hash": null,
        "payload": { "space_id": space.space_id }
    });
    append_log_line(&dir.join("logs/morphism-log.jsonl"), &log_entry)?;
    Ok(json!({
        "schema": "advisorygraphen.report.v1",
        "report_type": "case_import",
        "report_version": 1,
        "tool": advisorygraphen_core::tool_metadata(None),
        "input": {
            "store": options.store,
            "space_id": space.space_id,
            "revision_id": options.revision_id
        },
        "result": {
            "imported": true,
            "revision_id": options.revision_id,
            "log_entry_id": "log:000001"
        },
        "projection": {},
        "warnings": []
    }))
}

pub fn case_reason_workflow(options: &CaseReasonOptions) -> AdvisoryResult<Value> {
    let head = read_space_head_revision(&options.store, &options.space_id)?;
    let space = read_materialized_space(&options.store, &options.space_id)?;
    let check = check_space(&space, "technical_advisory_mvp", None, None)?;
    let mut completions = propose_completions(&space, &check, "case_reason", None)?;
    let blockers = check
        .result
        .get("obstructions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut candidates = completions
        .result
        .get("completion_candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    apply_candidate_reviews(&options.store, &options.space_id, &mut candidates)?;
    completions.result["completion_candidates"] = json!(candidates.clone());
    let resolution_state = blocker_resolution_state(&blockers, &candidates);
    let frontier = frontier_items(&resolution_state);
    let waiting = waiting_items(&resolution_state);
    let agent_report = attach_completion_report(
        serde_json::to_value(&check)?,
        serde_json::to_value(&completions)?,
    )?;
    let mut projection = build_projection(&space, &agent_report, "ai_agent")?;
    projection["case_head_revision"] = json!(head.clone());
    Ok(json!({
        "schema": "advisorygraphen.report.v1",
        "report_type": "case_reason",
        "report_version": 1,
        "tool": advisorygraphen_core::tool_metadata(None),
        "input": {
            "space_id": options.space_id,
            "case_head_revision": head
        },
        "result": {
            "space_id": options.space_id,
            "case_head_revision": head,
            "blockers": blockers,
            "candidate_review_state": candidates,
            "blocker_resolution_state": resolution_state,
            "close_status": close_status(&space, &check),
            "frontier_items": frontier,
            "waiting_items": waiting
        },
        "projection": projection,
        "warnings": []
    }))
}

pub fn case_close_check_workflow(options: &CaseCloseCheckOptions) -> AdvisoryResult<Value> {
    let head = read_space_head_revision(&options.store, &options.space_id)?;
    ensure_base_revision(Some(&head), options.base_revision.as_deref())?;
    let space = read_materialized_space(&options.store, &options.space_id)?;
    let check = check_space(&space, "technical_advisory_mvp", None, None)?;
    let status = close_status(&space, &check);
    Ok(json!({
        "schema": "advisorygraphen.report.v1",
        "report_type": "case_close_check",
        "report_version": 1,
        "tool": advisorygraphen_core::tool_metadata(None),
        "input": {
            "space_id": options.space_id,
            "base_revision": options.base_revision
        },
        "result": status,
        "projection": build_projection(&space, &serde_json::to_value(&check)?, "audit_trace")?,
        "warnings": []
    }))
}

pub fn read_json(path: &Path) -> AdvisoryResult<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

pub fn write_json_if_requested<T: serde::Serialize>(
    path: &Option<PathBuf>,
    value: &T,
) -> AdvisoryResult<()> {
    if let Some(path) = path {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_vec_pretty(value)?)?;
    }
    Ok(())
}

pub fn write_string_if_requested(path: &Option<PathBuf>, value: &str) -> AdvisoryResult<()> {
    if let Some(path) = path {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, value)?;
    }
    Ok(())
}

fn read_space(path: &Path) -> AdvisoryResult<AdvisorySpaceEnvelope> {
    let space: AdvisorySpaceEnvelope = serde_json::from_value(read_json(path)?)?;
    advisorygraphen_core::validate_space(&space)?;
    Ok(space)
}

fn read_materialized_space(store: &Path, space_id: &str) -> AdvisoryResult<AdvisorySpaceEnvelope> {
    read_space(&space_dir(store, space_id).join("materialized/space.json"))
}

fn read_space_head_revision(store: &Path, space_id: &str) -> AdvisoryResult<String> {
    Ok(fs::read_to_string(space_dir(store, space_id).join("HEAD"))?)
}

fn read_imported_space_head(store: &Path, space_id: &str) -> AdvisoryResult<String> {
    read_space_head_revision(store, space_id).map_err(|error| match error {
        AdvisoryError::Io(_) => AdvisoryError::Validation(format!(
            "case space {space_id} must be imported before review"
        )),
        other => other,
    })
}

fn ensure_base_revision(head: Option<&str>, base: Option<&str>) -> AdvisoryResult<()> {
    let Some(head) = head.map(str::trim) else {
        return Ok(());
    };
    let Some(base) = base else {
        return Err(AdvisoryError::StaleRevision {
            expected: head.to_string(),
            actual: "<missing>".to_string(),
        });
    };
    if head != base {
        return Err(AdvisoryError::StaleRevision {
            expected: head.to_string(),
            actual: base.to_string(),
        });
    }
    Ok(())
}

fn space_dir(store: &Path, space_id: &str) -> PathBuf {
    store.join("spaces").join(space_id.replace([':', '/'], "-"))
}

fn canonical_schema_name(schema: &str) -> String {
    match schema {
        "engagement_snapshot" | "snapshot" => advisorygraphen_core::SNAPSHOT_SCHEMA,
        "space" | "advisory_space" => advisorygraphen_core::SPACE_SCHEMA,
        "report" => advisorygraphen_core::REPORT_SCHEMA,
        "projection_request" => advisorygraphen_core::PROJECTION_REQUEST_SCHEMA,
        "review_event" => REVIEW_EVENT_SCHEMA,
        other => other,
    }
    .to_string()
}

fn file_name(path: &Path) -> &str {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("report.json")
}

fn append_store_event(store: &Path, value: &Value) -> AdvisoryResult<()> {
    fs::create_dir_all(store.join("logs"))?;
    append_log_line(&store.join("logs/morphism-log.jsonl"), value)
}

fn append_log_line(path: &Path, value: &Value) -> AdvisoryResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", serde_json::to_string(value)?)?;
    Ok(())
}

fn next_sequence(store: &Path, space_id: &str) -> u64 {
    let path = store.join("logs/morphism-log.jsonl");
    fs::read_to_string(path)
        .ok()
        .map(|contents| {
            contents
                .lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .filter(|entry| {
                    entry.get("case_space_id").and_then(Value::as_str) == Some(space_id)
                })
                .count() as u64
                + 1
        })
        .unwrap_or(1)
}
