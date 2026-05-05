use advisorygraphen_core::{
    validate_document, AdvisoryError, AdvisoryResult, AdvisorySpaceEnvelope, ReportEnvelope,
    HYPOTHESIS_EVENT_SCHEMA, REVIEW_EVENT_SCHEMA,
};
use advisorygraphen_interpretation::InterpretationPackage;
use advisorygraphen_lift::lift_snapshot;
use advisorygraphen_projection::{build_projection, project};
use advisorygraphen_reasoning::{
    blocker_resolution_state, check_space, close_status, frontier_items, propose_completions,
    propose_hypothesis_lifecycle, waiting_items,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

mod case_review;
mod code_snapshot;
mod dogfood;
mod hypothesis_propagation;
mod hypothesis_review;
mod options;
mod projection_report;
mod review;
use case_review::apply_candidate_reviews;
pub use code_snapshot::{code_repo_snapshot_workflow, CodeRepoSnapshotOptions};
pub use dogfood::{dogfood_repo_snapshot_workflow, DogfoodRepoSnapshotOptions};
use hypothesis_propagation::{
    extend_candidates_from_supported_hypotheses, mark_orphaned_candidates, reframe_obstructions,
};
use hypothesis_review::apply_hypothesis_events;
pub use options::{
    CaseCloseCheckOptions, CaseImportOptions, CaseReasonOptions, CheckOptions,
    CompletionProposeOptions, HypothesisApplyProposalsOptions, HypothesisFalsifyOptions,
    HypothesisProposeOptions, LiftOptions, ProjectOptions, ReviewOptions, ValidateOptions,
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

pub fn hypothesis_propose_workflow(
    options: &HypothesisProposeOptions,
) -> AdvisoryResult<ReportEnvelope> {
    let space = read_space(&options.space)?;
    let check_report: ReportEnvelope = serde_json::from_value(read_json(&options.from_report)?)?;
    let report = propose_hypothesis_lifecycle(
        &space,
        &check_report,
        file_name(&options.from_report),
        options.command.as_deref(),
    )?;
    write_json_if_requested(&options.output, &report)?;
    Ok(report)
}

pub fn hypothesis_apply_proposals_workflow(
    options: &HypothesisApplyProposalsOptions,
) -> AdvisoryResult<Value> {
    fs::create_dir_all(&options.store)?;
    let proposal_report = read_json(&options.from_report)?;
    if proposal_report.get("report_type").and_then(Value::as_str)
        != Some("hypothesis_lifecycle_proposal")
    {
        return Err(AdvisoryError::Validation(
            "from-report must be a hypothesis_lifecycle_proposal report".to_string(),
        ));
    }
    let space_id = proposal_report
        .pointer("/input/space_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AdvisoryError::Validation(
                "from-report must contain input.space_id for hypothesis proposal application"
                    .to_string(),
            )
        })?
        .to_string();
    let policy = read_autonomy_policy(options.policy.as_deref())?;
    let initial_head = read_imported_space_head(&options.store, &space_id)?;
    let materialized_space = read_materialized_space(&options.store, &space_id)?;
    ensure_base_revision(Some(&initial_head), options.base_revision.as_deref())?;

    let proposals = proposal_report
        .pointer("/result/lifecycle_proposals")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut applied = Vec::new();
    let mut skipped = Vec::new();
    let mut current_head = initial_head.clone();

    for proposal in proposals {
        let decision = autonomy_decision(&proposal, &policy);
        if !decision.allowed {
            skipped.push(application_skip(&proposal, decision.reason));
            continue;
        }
        if applied.len() >= policy.max_events {
            skipped.push(application_skip(
                &proposal,
                format!("policy max_events {} reached", policy.max_events),
            ));
            continue;
        }
        let event = hypothesis_event_from_proposal(
            &materialized_space.engagement_id,
            &proposal,
            &options.reviewer,
            &options.reason,
            &options.from_report,
            Some(&current_head),
            applied.len() + 1,
        )?;
        if !options.dry_run {
            let sequence = next_sequence(&options.store, &space_id);
            let target_revision = format!("revision:hypothesis-auto-{sequence:06}");
            let hypothesis_slug = event["target_hypothesis_id"]
                .as_str()
                .unwrap_or("hypothesis:unknown")
                .trim_start_matches("hypothesis:")
                .to_string();
            append_store_event(
                &options.store,
                &json!({
                    "schema": "advisorygraphen.case.log.entry.v1",
                    "case_space_id": space_id.clone(),
                    "sequence": sequence,
                    "entry_id": format!("log:{sequence:06}"),
                    "morphism_id": format!("morphism:hypothesis-auto-{}-{hypothesis_slug}", event["outcome"].as_str().unwrap_or("unknown")),
                    "source_revision_id": current_head,
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
            current_head = target_revision;
        }
        applied.push(event);
    }
    let post_apply_case_reason = if options.dry_run || applied.is_empty() {
        json!(null)
    } else {
        let reasoned = case_reason_workflow(&CaseReasonOptions {
            store: options.store.clone(),
            space_id: space_id.clone(),
        })?;
        json!({
            "case_head_revision": reasoned.pointer("/result/case_head_revision"),
            "close_status": reasoned.pointer("/result/close_status"),
            "frontier_items": reasoned.pointer("/result/frontier_items"),
            "waiting_items": reasoned.pointer("/result/waiting_items")
        })
    };

    Ok(json!({
        "schema": "advisorygraphen.report.v1",
        "report_type": "hypothesis_lifecycle_apply_proposals",
        "report_version": 1,
        "tool": advisorygraphen_core::tool_metadata(None),
        "input": {
            "space_id": space_id,
            "from_report": options.from_report,
            "policy": options.policy,
            "base_revision": options.base_revision,
            "dry_run": options.dry_run
        },
        "result": {
            "applied_count": applied.len(),
            "skipped_count": skipped.len(),
            "applied_events": applied,
            "skipped_proposals": skipped,
            "initial_head_revision": initial_head,
            "case_head_revision": current_head,
            "policy": policy.as_json(),
            "post_apply_case_reason": post_apply_case_reason
        },
        "projection": {},
        "warnings": []
    }))
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
pub fn hypothesis_falsify_workflow(options: &HypothesisFalsifyOptions) -> AdvisoryResult<Value> {
    hypothesis_lifecycle_event(options, "falsified")
}

pub fn hypothesis_support_workflow(options: &HypothesisFalsifyOptions) -> AdvisoryResult<Value> {
    hypothesis_lifecycle_event(options, "supported")
}

pub fn hypothesis_accept_workflow(options: &HypothesisFalsifyOptions) -> AdvisoryResult<Value> {
    hypothesis_lifecycle_event(options, "accepted")
}

pub fn hypothesis_reject_workflow(options: &HypothesisFalsifyOptions) -> AdvisoryResult<Value> {
    hypothesis_lifecycle_event(options, "rejected")
}

#[derive(Debug, Clone)]
struct HypothesisAutonomyPolicy {
    allowed_outcomes: Vec<String>,
    min_confidence: f64,
    allowed_trust_levels: Vec<String>,
    max_events: usize,
    require_candidate_status: bool,
    allow_review_conflict: bool,
}

impl HypothesisAutonomyPolicy {
    fn default_conservative() -> Self {
        Self {
            allowed_outcomes: vec!["supported".to_string(), "falsified".to_string()],
            min_confidence: 0.7,
            allowed_trust_levels: vec![
                "reviewed_or_source_backed".to_string(),
                "test_passed".to_string(),
                "runtime_observed".to_string(),
            ],
            max_events: 3,
            require_candidate_status: true,
            allow_review_conflict: false,
        }
    }

    fn as_json(&self) -> Value {
        json!({
            "allowed_outcomes": self.allowed_outcomes,
            "min_confidence": self.min_confidence,
            "allowed_trust_levels": self.allowed_trust_levels,
            "max_events": self.max_events,
            "require_candidate_status": self.require_candidate_status,
            "allow_review_conflict": self.allow_review_conflict
        })
    }
}

struct AutonomyDecision {
    allowed: bool,
    reason: String,
}

fn read_autonomy_policy(path: Option<&Path>) -> AdvisoryResult<HypothesisAutonomyPolicy> {
    let Some(path) = path else {
        return Ok(HypothesisAutonomyPolicy::default_conservative());
    };
    let value = read_json(path)?;
    let default = HypothesisAutonomyPolicy::default_conservative();
    Ok(HypothesisAutonomyPolicy {
        allowed_outcomes: optional_string_vec(&value, "allowed_outcomes")
            .unwrap_or(default.allowed_outcomes),
        min_confidence: value
            .get("min_confidence")
            .and_then(Value::as_f64)
            .unwrap_or(default.min_confidence),
        allowed_trust_levels: optional_string_vec(&value, "allowed_trust_levels")
            .unwrap_or(default.allowed_trust_levels),
        max_events: value
            .get("max_events")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(default.max_events),
        require_candidate_status: value
            .get("require_candidate_status")
            .and_then(Value::as_bool)
            .unwrap_or(default.require_candidate_status),
        allow_review_conflict: value
            .get("allow_review_conflict")
            .and_then(Value::as_bool)
            .unwrap_or(default.allow_review_conflict),
    })
}

fn optional_string_vec(value: &Value, key: &str) -> Option<Vec<String>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
}

fn autonomy_decision(proposal: &Value, policy: &HypothesisAutonomyPolicy) -> AutonomyDecision {
    let outcome = proposal
        .get("proposed_outcome")
        .and_then(Value::as_str)
        .unwrap_or("");
    if outcome == "review_conflict" && !policy.allow_review_conflict {
        return denied("review_conflict proposals require human review");
    }
    if !policy
        .allowed_outcomes
        .iter()
        .any(|allowed| allowed == outcome)
    {
        return denied(format!("outcome {outcome} is not policy-allowed"));
    }
    if policy.require_candidate_status
        && proposal
            .get("target_hypothesis_status")
            .and_then(Value::as_str)
            != Some("candidate")
    {
        return denied("target hypothesis is not in candidate lifecycle status");
    }
    let confidence = proposal
        .get("confidence")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    if confidence < policy.min_confidence {
        return denied(format!(
            "confidence {confidence} is below policy minimum {}",
            policy.min_confidence
        ));
    }
    let signal_pointer = match outcome {
        "supported" => "/supporting_signals",
        "falsified" => "/falsifying_signals",
        _ => "/supporting_signals",
    };
    let signals = proposal
        .pointer(signal_pointer)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if signals.is_empty() {
        return denied("proposal has no outcome-specific evidence signals");
    }
    let has_allowed_trust = signals.iter().any(|signal| {
        signal
            .get("trust_level")
            .and_then(Value::as_str)
            .is_some_and(|trust| {
                policy
                    .allowed_trust_levels
                    .iter()
                    .any(|allowed| allowed == trust)
            })
    });
    if !has_allowed_trust {
        return denied("proposal has no evidence signal with policy-allowed trust level");
    }
    AutonomyDecision {
        allowed: true,
        reason: "policy allowed".to_string(),
    }
}

fn denied(reason: impl Into<String>) -> AutonomyDecision {
    AutonomyDecision {
        allowed: false,
        reason: reason.into(),
    }
}

fn application_skip(proposal: &Value, reason: impl Into<String>) -> Value {
    json!({
        "proposal_id": proposal.get("id"),
        "target_hypothesis_id": proposal.get("target_hypothesis_id"),
        "proposed_outcome": proposal.get("proposed_outcome"),
        "reason": reason.into()
    })
}

fn hypothesis_event_from_proposal(
    engagement_id: &str,
    proposal: &Value,
    reviewer: &str,
    reason: &str,
    from_report: &Path,
    base_revision: Option<&str>,
    ordinal: usize,
) -> AdvisoryResult<Value> {
    let hypothesis_id = proposal
        .get("target_hypothesis_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AdvisoryError::Validation("lifecycle proposal missing target_hypothesis_id".to_string())
        })?;
    let outcome = proposal
        .get("proposed_outcome")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AdvisoryError::Validation("lifecycle proposal missing proposed_outcome".to_string())
        })?;
    let event_outcome = match outcome {
        "supported" | "falsified" => outcome,
        other => {
            return Err(AdvisoryError::Validation(format!(
                "cannot apply lifecycle proposal outcome {other}"
            )))
        }
    };
    let evidence_ids = proposal
        .get("evidence_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let hypothesis_slug = hypothesis_id.trim_start_matches("hypothesis:");
    let event = json!({
        "schema": HYPOTHESIS_EVENT_SCHEMA,
        "hypothesis_event_id": format!("hypothesis-event:auto-{event_outcome}:{hypothesis_slug}-{ordinal:06}"),
        "engagement_id": engagement_id,
        "target_hypothesis_id": hypothesis_id,
        "outcome": event_outcome,
        "reviewer_id": reviewer,
        "reviewed_at": Utc::now().to_rfc3339(),
        "reason": reason,
        "evidence_ids": evidence_ids,
        "base_revision_id": base_revision,
        "metadata": {
            "from_report": from_report.display().to_string(),
            "proposal_id": proposal.get("id"),
            "autonomy": {
                "applied_from_proposal": true,
                "proposal_confidence": proposal.get("confidence"),
                "supporting_signals": proposal.get("supporting_signals"),
                "falsifying_signals": proposal.get("falsifying_signals")
            }
        }
    });
    validate_document(&event, Some(HYPOTHESIS_EVENT_SCHEMA))?;
    Ok(event)
}

fn hypothesis_lifecycle_event(
    options: &HypothesisFalsifyOptions,
    outcome: &str,
) -> AdvisoryResult<Value> {
    fs::create_dir_all(&options.store)?;
    let report = read_json(&options.from_report)?;
    let space_id = report
        .pointer("/input/space_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AdvisoryError::Validation(
                "from-report must contain input.space_id for hypothesis events".to_string(),
            )
        })?
        .to_string();
    let hypothesis = report
        .pointer("/result/hypotheses")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|item| item.get("id").and_then(Value::as_str) == Some(&options.hypothesis_id))
        .ok_or_else(|| {
            AdvisoryError::Validation(format!(
                "hypothesis {} not found in from-report",
                options.hypothesis_id
            ))
        })?
        .clone();
    let head = read_imported_space_head(&options.store, &space_id)?;
    let materialized_space = read_materialized_space(&options.store, &space_id)?;
    ensure_base_revision(Some(&head), options.base_revision.as_deref())?;
    let sequence = next_sequence(&options.store, &space_id);
    let target_revision = format!("revision:hypothesis-{sequence:06}");
    let hypothesis_slug = options.hypothesis_id.trim_start_matches("hypothesis:");
    let hypothesis_event_id = format!("hypothesis-event:{outcome}:{hypothesis_slug}-{sequence:06}");
    let evidence_ids: Vec<Value> = options
        .evidence_ids
        .iter()
        .map(|id| Value::String(id.clone()))
        .collect();
    let event = json!({
        "schema": HYPOTHESIS_EVENT_SCHEMA,
        "hypothesis_event_id": hypothesis_event_id,
        "engagement_id": materialized_space.engagement_id,
        "target_hypothesis_id": options.hypothesis_id,
        "outcome": outcome,
        "reviewer_id": options.reviewer,
        "reviewed_at": Utc::now().to_rfc3339(),
        "reason": options.reason,
        "evidence_ids": evidence_ids,
        "base_revision_id": options.base_revision,
        "metadata": {
            "from_report": options.from_report.display().to_string(),
            "competes_with": hypothesis
                .pointer("/metadata/competes_with")
                .cloned()
                .unwrap_or_else(|| json!([])),
            "falsified_by": hypothesis
                .pointer("/metadata/falsified_by")
                .cloned()
                .unwrap_or_else(|| json!([]))
        }
    });
    validate_document(&event, Some(HYPOTHESIS_EVENT_SCHEMA))?;
    append_store_event(
        &options.store,
        &json!({
            "schema": "advisorygraphen.case.log.entry.v1",
            "case_space_id": space_id.clone(),
            "sequence": sequence,
            "entry_id": format!("log:{sequence:06}"),
            "morphism_id": format!("morphism:hypothesis-{outcome}-{hypothesis_slug}"),
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
    let mut check = check_space(&space, "technical_advisory_mvp", None, None)?;
    let mut hypotheses = check
        .result
        .get("hypotheses")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    apply_hypothesis_events(&options.store, &options.space_id, &mut hypotheses)?;
    check.result["hypotheses"] = json!(hypotheses.clone());
    let mut obstructions = check
        .result
        .get("obstructions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    reframe_obstructions(&mut obstructions, &hypotheses);
    check.result["obstructions"] = json!(obstructions.clone());
    let mut completions = propose_completions(&space, &check, "case_reason", None)?;
    let mut candidates = completions
        .result
        .get("completion_candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    extend_candidates_from_supported_hypotheses(&mut candidates, &hypotheses, &obstructions);
    mark_orphaned_candidates(&mut candidates, &hypotheses);
    apply_candidate_reviews(&options.store, &options.space_id, &mut candidates)?;
    completions.result["completion_candidates"] = json!(candidates.clone());
    let blockers = obstructions.clone();
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
