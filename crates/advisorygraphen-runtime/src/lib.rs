use advisorygraphen_core::{
    json_id, validate_document, AdvisoryError, AdvisoryResult, AdvisorySpaceEnvelope,
    ReportEnvelope, HYPOTHESIS_EVENT_SCHEMA, REVIEW_EVENT_SCHEMA,
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
mod dry_run_gluing;
mod hypothesis_propagation;
mod hypothesis_review;
mod options;
mod projection_report;
mod review;
use case_review::apply_candidate_reviews;
pub use code_snapshot::{code_repo_snapshot_workflow, CodeRepoSnapshotOptions};
pub use dogfood::{
    dogfood_adversarial_fixture_workflow, dogfood_repo_snapshot_workflow,
    DogfoodAdversarialFixtureOptions, DogfoodRepoSnapshotOptions,
};
use hypothesis_propagation::{
    extend_candidates_from_supported_hypotheses, mark_orphaned_candidates, reframe_obstructions,
};
use hypothesis_review::apply_hypothesis_events;
pub use options::{
    CaseCloseCheckOptions, CaseImportOptions, CaseReasonOptions, CheckOptions,
    CompletionApplyAcceptedOptions, CompletionDryRunOptions, CompletionProposeOptions,
    HypothesisApplyProposalsOptions, HypothesisFalsifyOptions, HypothesisProposeOptions,
    LiftOptions, ObservationRecordOptions, ProjectOptions, ReviewOptions, ValidateOptions,
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

pub fn completions_dry_run_workflow(
    options: &CompletionDryRunOptions,
) -> AdvisoryResult<ReportEnvelope> {
    let space = read_space(&options.space)?;
    let completion_report: ReportEnvelope =
        serde_json::from_value(read_json(&options.from_report)?)?;
    if completion_report.report_type != "completion_proposal" {
        return Err(AdvisoryError::Validation(
            "from-report must be a completion_proposal report".to_string(),
        ));
    }
    let before_check = check_space(&space, "technical_advisory_mvp", None, None)?;
    let before_obstructions = before_check
        .result
        .get("obstructions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let candidates = completion_report
        .result
        .get("completion_candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let selected = candidates
        .iter()
        .filter(|candidate| {
            options.candidate_ids.is_empty()
                || options
                    .candidate_ids
                    .iter()
                    .any(|id| candidate.get("id").and_then(Value::as_str) == Some(id.as_str()))
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut dry_runs = Vec::new();

    for candidate in selected {
        let candidate_id = json_id(&candidate).to_string();
        let mut dry_space = space.clone();
        let materialization =
            materialize_candidate_dry_run(&dry_space, &before_obstructions, &candidate);
        match materialization {
            DryRunMaterialization::Applied {
                cells,
                incidences,
                removed_incidence_ids,
            } => {
                for incidence_id in &removed_incidence_ids {
                    dry_space
                        .incidences
                        .retain(|incidence| json_id(incidence) != incidence_id);
                }
                for cell in &cells {
                    upsert_by_id(&mut dry_space.cells, cell.clone());
                }
                for incidence in &incidences {
                    upsert_by_id(&mut dry_space.incidences, incidence.clone());
                }
                advisorygraphen_core::validate_space(&dry_space)?;
                let after_check = check_space(&dry_space, "technical_advisory_mvp", None, None)?;
                let before_ids = obstruction_ids(&before_check);
                let after_ids = obstruction_ids(&after_check);
                let resolved_ids = before_ids
                    .iter()
                    .filter(|id| !after_ids.contains(*id))
                    .cloned()
                    .collect::<Vec<_>>();
                let introduced_ids = after_ids
                    .iter()
                    .filter(|id| !before_ids.contains(*id))
                    .cloned()
                    .collect::<Vec<_>>();
                dry_runs.push(json!({
                    "candidate_id": candidate_id,
                    "candidate_type": candidate.get("candidate_type"),
                    "status": "applied_to_dry_run_space",
                    "application_plan": candidate.get("application_plan"),
                    "applied_structure": {
                        "cell_ids": ids_of(&cells),
                        "incidence_ids": ids_of(&incidences),
                        "removed_incidence_ids": removed_incidence_ids
                    },
                    "check_delta": {
                        "before_obstruction_ids": before_ids,
                        "after_obstruction_ids": after_ids,
                        "resolved_obstruction_ids": resolved_ids,
                        "introduced_obstruction_ids": introduced_ids
                    },
                    "after_close_status": close_status(&dry_space, &after_check),
                    "higher_graphen_gluing_review": dry_run_gluing::candidate_gluing_review(
                        &space,
                        &candidate,
                        &before_obstructions,
                        after_check
                            .result
                            .get("obstructions")
                            .and_then(Value::as_array)
                            .map(Vec::as_slice)
                            .unwrap_or(&[]),
                        &cells,
                        &incidences,
                        &removed_incidence_ids
                    )?
                }));
            }
            DryRunMaterialization::Skipped { reason } => {
                dry_runs.push(json!({
                    "candidate_id": candidate_id,
                    "candidate_type": candidate.get("candidate_type"),
                    "status": "skipped",
                    "reason": reason,
                    "application_plan": candidate.get("application_plan"),
                    "higher_graphen_gluing_review": dry_run_gluing::skipped_candidate_gluing_review(
                        &candidate,
                        &reason
                    )
                }));
            }
        }
    }

    let report = ReportEnvelope::new(
        "completion_dry_run",
        options.command.as_deref(),
        json!({
            "space_id": space.space_id,
            "from_report": file_name(&options.from_report),
            "candidate_ids": options.candidate_ids
        }),
        json!({
            "dry_runs": dry_runs,
            "candidate_count": candidates.len()
        }),
    );
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
    let gluing_policy = review_gluing_policy(
        &materialized_space,
        from_report,
        &options.candidate_id,
        &options.outcome,
    )?;
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
            "higher_graphen": higher_graphen_review,
            "higher_graphen_gluing_policy": gluing_policy
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

fn review_gluing_policy(
    space: &AdvisorySpaceEnvelope,
    from_report: &Path,
    candidate_id: &str,
    outcome: &str,
) -> AdvisoryResult<Value> {
    let report = read_json(from_report)?;
    let candidate = candidate_from_report(&report, candidate_id)?.clone();
    let before_check = check_space(space, "technical_advisory_mvp", None, None)?;
    let before_obstructions = before_check
        .result
        .get("obstructions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let review = match materialize_candidate_dry_run(space, &before_obstructions, &candidate) {
        DryRunMaterialization::Applied {
            cells,
            incidences,
            removed_incidence_ids,
        } => {
            let mut dry_space = space.clone();
            for incidence_id in &removed_incidence_ids {
                dry_space
                    .incidences
                    .retain(|incidence| json_id(incidence) != incidence_id);
            }
            for cell in &cells {
                upsert_by_id(&mut dry_space.cells, cell.clone());
            }
            for incidence in &incidences {
                upsert_by_id(&mut dry_space.incidences, incidence.clone());
            }
            advisorygraphen_core::validate_space(&dry_space)?;
            let after_check = check_space(&dry_space, "technical_advisory_mvp", None, None)?;
            dry_run_gluing::candidate_gluing_review(
                space,
                &candidate,
                &before_obstructions,
                after_check
                    .result
                    .get("obstructions")
                    .and_then(Value::as_array)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                &cells,
                &incidences,
                &removed_incidence_ids,
            )?
        }
        DryRunMaterialization::Skipped { reason } => {
            dry_run_gluing::skipped_candidate_gluing_review(&candidate, &reason)
        }
    };
    let policy_blockers = dry_run_gluing::policy_blockers(&review);
    Ok(json!({
        "schema": "advisorygraphen.completion_review.gluing_policy.v1",
        "outcome": outcome,
        "explicit_review_recorded": true,
        "policy_blockers": policy_blockers,
        "policy_override": if outcome == "accepted" && !policy_blockers.is_empty() {
            json!("explicit_completion_review")
        } else {
            json!(null)
        },
        "dry_run_gluing_review": review
    }))
}

fn application_gluing_review(
    space: &AdvisorySpaceEnvelope,
    candidate: &Value,
    before_obstructions: &[Value],
    cells: &[Value],
    incidences: &[Value],
) -> AdvisoryResult<Value> {
    let mut dry_space = space.clone();
    for cell in cells {
        upsert_by_id(&mut dry_space.cells, cell.clone());
    }
    for incidence in incidences {
        upsert_by_id(&mut dry_space.incidences, incidence.clone());
    }
    advisorygraphen_core::validate_space(&dry_space)?;
    let after_check = check_space(&dry_space, "technical_advisory_mvp", None, None)?;
    dry_run_gluing::candidate_gluing_review(
        space,
        candidate,
        before_obstructions,
        after_check
            .result
            .get("obstructions")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]),
        cells,
        incidences,
        &[],
    )
}

fn reviewed_policy_override(candidate: &Value, policy_blockers: &[Value]) -> AdvisoryResult<Value> {
    if policy_blockers.is_empty() {
        return Ok(json!(null));
    }
    let candidate_id = json_id(candidate);
    let Some(latest_review) = candidate.pointer("/metadata/latest_review") else {
        return Err(AdvisoryError::Validation(format!(
            "candidate {candidate_id} has gluing policy blockers but no review event summary"
        )));
    };
    if latest_review.get("outcome").and_then(Value::as_str) != Some("accepted") {
        return Err(AdvisoryError::Validation(format!(
            "candidate {candidate_id} has gluing policy blockers but was not accepted"
        )));
    }
    let override_value = latest_review
        .pointer("/higher_graphen_gluing_policy/policy_override")
        .cloned()
        .unwrap_or(Value::Null);
    if override_value != json!("explicit_completion_review") {
        return Err(AdvisoryError::Validation(format!(
            "candidate {candidate_id} has gluing policy blockers but review event did not record policy_override explicit_completion_review"
        )));
    }
    Ok(override_value)
}

fn candidate_from_report<'a>(report: &'a Value, candidate_id: &str) -> AdvisoryResult<&'a Value> {
    let candidates = report
        .pointer("/result/completion_candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AdvisoryError::Validation(
                "from-report must contain result.completion_candidates".to_string(),
            )
        })?;
    candidates
        .iter()
        .find(|candidate| candidate.get("id").and_then(Value::as_str) == Some(candidate_id))
        .ok_or_else(|| {
            AdvisoryError::Validation(format!("candidate {candidate_id} not found in from-report"))
        })
}

pub fn completions_apply_accepted_workflow(
    options: &CompletionApplyAcceptedOptions,
) -> AdvisoryResult<Value> {
    fs::create_dir_all(&options.store)?;
    let head = read_imported_space_head(&options.store, &options.space_id)?;
    ensure_base_revision(Some(&head), options.base_revision.as_deref())?;
    let mut space = read_materialized_space(&options.store, &options.space_id)?;
    let check = check_space(&space, "technical_advisory_mvp", None, None)?;
    let blockers = check
        .result
        .get("obstructions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut completions = propose_completions(&space, &check, "apply_accepted", None)?;
    let mut candidates = completions
        .result
        .get("completion_candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    apply_candidate_reviews(&options.store, &options.space_id, &mut candidates)?;
    completions.result["completion_candidates"] = json!(candidates.clone());
    let resolution_state = blocker_resolution_state(&blockers, &candidates);
    let mut applied_structures = Vec::new();
    let mut skipped_candidates = Vec::new();

    for item in &resolution_state {
        if item.get("resolution_status").and_then(Value::as_str)
            != Some("accepted_candidate_pending_application")
        {
            continue;
        }
        let obstruction_id = item
            .get("obstruction_id")
            .and_then(Value::as_str)
            .unwrap_or("obstruction:unknown");
        let Some(blocker) = blockers
            .iter()
            .find(|blocker| blocker.get("id").and_then(Value::as_str) == Some(obstruction_id))
        else {
            continue;
        };
        for candidate_id in item
            .get("accepted_candidate_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            let Some(candidate) = candidates.iter().find(|candidate| {
                candidate.get("id").and_then(Value::as_str) == Some(candidate_id)
            }) else {
                continue;
            };
            match materialize_candidate_structure(&space, blocker, candidate, &options.reviewer) {
                Materialization::Applied { cells, incidences } => {
                    let gluing_review = application_gluing_review(
                        &space,
                        candidate,
                        &blockers,
                        &cells,
                        &incidences,
                    )?;
                    let policy_blockers = dry_run_gluing::policy_blockers(&gluing_review);
                    let policy_override = reviewed_policy_override(candidate, &policy_blockers)?;
                    if options.dry_run {
                        applied_structures.push(json!({
                            "candidate_id": candidate_id,
                            "dry_run": true,
                            "cells": cells,
                            "incidences": incidences,
                            "higher_graphen_gluing_review": gluing_review,
                            "policy_blockers": policy_blockers,
                            "policy_override": policy_override
                        }));
                    } else {
                        for cell in &cells {
                            upsert_by_id(&mut space.cells, cell.clone());
                        }
                        for incidence in &incidences {
                            upsert_by_id(&mut space.incidences, incidence.clone());
                        }
                        applied_structures.push(json!({
                            "candidate_id": candidate_id,
                            "dry_run": false,
                            "cell_ids": ids_of(&cells),
                            "incidence_ids": ids_of(&incidences),
                            "higher_graphen_gluing_review": gluing_review,
                            "policy_blockers": policy_blockers,
                            "policy_override": policy_override
                        }));
                    }
                }
                Materialization::Skipped { reason } => {
                    skipped_candidates.push(json!({
                        "candidate_id": candidate_id,
                        "candidate_type": candidate.get("candidate_type"),
                        "reason": reason
                    }));
                }
            }
        }
    }

    let mut current_head = head.clone();
    if !options.dry_run && !applied_structures.is_empty() {
        advisorygraphen_core::validate_space(&space)?;
        let sequence = next_sequence(&options.store, &options.space_id);
        let target_revision = format!("revision:completion-apply-{sequence:06}");
        let event = json!({
            "schema": "advisorygraphen.completion.application.v1",
            "application_event_id": format!("completion-application:{sequence:06}"),
            "engagement_id": space.engagement_id,
            "reviewer_id": options.reviewer,
            "reviewed_at": Utc::now().to_rfc3339(),
            "reason": options.reason,
            "base_revision_id": options.base_revision,
            "applied_structures": applied_structures,
            "skipped_candidates": skipped_candidates
        });
        append_store_event(
            &options.store,
            &json!({
                "schema": "advisorygraphen.case.log.entry.v1",
                "case_space_id": options.space_id.clone(),
                "sequence": sequence,
                "entry_id": format!("log:{sequence:06}"),
                "morphism_id": format!("morphism:completion-apply-{sequence:06}"),
                "source_revision_id": head,
                "target_revision_id": target_revision.clone(),
                "actor": options.reviewer,
                "recorded_at": Utc::now().to_rfc3339(),
                "previous_entry_hash": null,
                "entry_hash": null,
                "payload": event
            }),
        )?;
        fs::write(
            space_dir(&options.store, &options.space_id).join("materialized/space.json"),
            serde_json::to_vec_pretty(&space)?,
        )?;
        fs::write(
            space_dir(&options.store, &options.space_id).join("HEAD"),
            &target_revision,
        )?;
        current_head = target_revision;
    }

    let post_apply_case_reason = if options.dry_run || applied_structures.is_empty() {
        json!(null)
    } else {
        let reasoned = case_reason_workflow(&CaseReasonOptions {
            store: options.store.clone(),
            space_id: options.space_id.clone(),
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
        "report_type": "completion_apply_accepted",
        "report_version": 1,
        "tool": advisorygraphen_core::tool_metadata(None),
        "input": {
            "space_id": options.space_id,
            "base_revision": options.base_revision,
            "dry_run": options.dry_run
        },
        "result": {
            "applied_count": applied_structures.len(),
            "skipped_count": skipped_candidates.len(),
            "applied_structures": applied_structures,
            "skipped_candidates": skipped_candidates,
            "initial_head_revision": head,
            "case_head_revision": current_head,
            "post_apply_case_reason": post_apply_case_reason,
            "supported_candidate_types": [
                "ownership_clarification",
                "proposed_test"
            ]
        },
        "projection": {},
        "warnings": []
    }))
}
pub fn hypothesis_falsify_workflow(options: &HypothesisFalsifyOptions) -> AdvisoryResult<Value> {
    hypothesis_lifecycle_event(options, "falsified")
}

pub fn observation_record_workflow(options: &ObservationRecordOptions) -> AdvisoryResult<Value> {
    let head = read_imported_space_head(&options.store, &options.space_id)?;
    ensure_base_revision(Some(&head), options.base_revision.as_deref())?;
    let mut space = read_materialized_space(&options.store, &options.space_id)?;
    let projection = read_json(&options.from_projection)?;
    let task = find_observation_task(&projection, &options.task_id).ok_or_else(|| {
        AdvisoryError::Validation(format!(
            "observation task {} not found in from-projection",
            options.task_id
        ))
    })?;
    let result = read_json(&options.result)?;
    validate_observation_result(&task, &result)?;

    let sequence = next_sequence(&options.store, &options.space_id);
    let target_revision = format!("revision:observation-{sequence:06}");
    let task_slug = advisorygraphen_core::slugify_id(&options.task_id);
    let evidence_id = format!("cell:observation-{task_slug}-{sequence:06}");
    let hypothesis_id = task
        .get("hypothesis_id")
        .and_then(Value::as_str)
        .unwrap_or("hypothesis:unknown");
    let supports = result
        .get("supports_hypothesis")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let falsifies = result
        .get("falsifies_hypothesis")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut metadata = json!({
        "observation_task_id": options.task_id,
        "candidate_id": task.get("candidate_id").cloned().unwrap_or(Value::Null),
        "hypothesis_id": hypothesis_id,
        "observation_status": result.get("observation_status").cloned().unwrap_or(Value::Null),
        "observation_result": result,
        "reviewer": options.reviewer,
        "reason": options.reason
    });
    if supports {
        metadata["supports_hypothesis_id"] = json!(hypothesis_id);
    }
    if falsifies {
        metadata["falsifies_hypothesis_id"] = json!(hypothesis_id);
    }
    let evidence_cell = json!({
        "id": evidence_id,
        "cell_type": "evidence",
        "title": format!("Observation result for {}", options.task_id),
        "summary": result.get("summary").and_then(Value::as_str).unwrap_or("Recorded observation result."),
        "context_ids": [],
        "source_ids": result.get("evidence_ids").cloned().unwrap_or_else(|| json!([])),
        "structure_refs": [options.task_id],
        "provenance": {
            "origin": "source_backed",
            "actor": options.reviewer,
            "confidence": 0.8,
            "review_status": "unreviewed"
        },
        "metadata": metadata
    });
    upsert_by_id(&mut space.cells, evidence_cell.clone());
    advisorygraphen_core::validate_space(&space)?;
    fs::write(
        space_dir(&options.store, &options.space_id).join("materialized/space.json"),
        serde_json::to_vec_pretty(&space)?,
    )?;
    append_store_event(
        &options.store,
        &json!({
            "schema": "advisorygraphen.case.log.entry.v1",
            "case_space_id": options.space_id,
            "sequence": sequence,
            "entry_id": format!("log:{sequence:06}"),
            "morphism_id": format!("morphism:observation-record-{task_slug}"),
            "source_revision_id": head,
            "target_revision_id": target_revision,
            "actor": options.reviewer,
            "recorded_at": Utc::now().to_rfc3339(),
            "previous_entry_hash": null,
            "entry_hash": null,
            "payload": {
                "schema": "advisorygraphen.observation.result.v1",
                "task_id": options.task_id,
                "evidence_cell_id": evidence_id,
                "hypothesis_id": hypothesis_id,
                "result": evidence_cell.pointer("/metadata/observation_result").cloned().unwrap_or(Value::Null)
            }
        }),
    )?;
    fs::write(
        space_dir(&options.store, &options.space_id).join("HEAD"),
        &target_revision,
    )?;
    Ok(json!({
        "schema": "advisorygraphen.report.v1",
        "report_type": "observation_record",
        "report_version": 1,
        "tool": advisorygraphen_core::tool_metadata(None),
        "input": {
            "space_id": options.space_id,
            "task_id": options.task_id,
            "base_revision": options.base_revision
        },
        "result": {
            "recorded": true,
            "case_head_revision": target_revision,
            "evidence_cell": evidence_cell,
            "promotion_gate": observation_promotion_gate(
                hypothesis_id,
                &evidence_id,
                &target_revision,
                supports,
                falsifies,
                options,
            ),
            "suggested_next_commands": observation_next_commands(
                hypothesis_id,
                &evidence_id,
                &target_revision,
                options,
            )
        },
        "projection": {},
        "warnings": []
    }))
}

fn find_observation_task(projection: &Value, task_id: &str) -> Option<Value> {
    projection
        .pointer("/recommendation_trace/follow_up_observations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|item| {
            item.get("ranked_observation_tasks")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .find(|task| task.get("task_id").and_then(Value::as_str) == Some(task_id))
        .cloned()
}

fn validate_observation_result(task: &Value, result: &Value) -> AdvisoryResult<()> {
    let missing = task
        .pointer("/output_schema/required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|field| result.get(*field).is_none())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(AdvisoryError::Validation(format!(
            "observation result missing required fields: {}",
            missing.join(", ")
        )));
    }
    let status = result
        .get("observation_status")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AdvisoryError::Validation("observation_status must be a string".to_string())
        })?;
    if ![
        "supports",
        "falsifies",
        "insufficient_evidence",
        "requires_human_review",
    ]
    .contains(&status)
    {
        return Err(AdvisoryError::Validation(format!(
            "invalid observation_status: {status}"
        )));
    }
    for field in ["supports_hypothesis", "falsifies_hypothesis"] {
        if result.get(field).and_then(Value::as_bool).is_none() {
            return Err(AdvisoryError::Validation(format!(
                "{field} must be a boolean"
            )));
        }
    }
    Ok(())
}

fn observation_promotion_gate(
    hypothesis_id: &str,
    evidence_id: &str,
    case_head_revision: &str,
    supports: bool,
    falsifies: bool,
    options: &ObservationRecordOptions,
) -> Value {
    let outcome = if supports {
        "support"
    } else if falsifies {
        "falsify"
    } else {
        "review"
    };
    json!({
        "outcome": outcome,
        "hypothesis_id": hypothesis_id,
        "evidence_cell_id": evidence_id,
        "case_head_revision": case_head_revision,
        "review_required": true,
        "next_command": match outcome {
            "support" => concrete_observation_next_command("support", hypothesis_id, evidence_id, case_head_revision, options),
            "falsify" => concrete_observation_next_command("falsify", hypothesis_id, evidence_id, case_head_revision, options),
            _ => "Review the observation result before supporting or falsifying the hypothesis.".to_string(),
        },
        "rerun_after_review": [
            "advisorygraphen case reason --store STORE --space-id SPACE_ID --format json",
            "advisorygraphen completions apply-accepted --store STORE --space-id SPACE_ID --reviewer REVIEWER --reason REASON --base-revision CASE_HEAD --format json"
        ]
    })
}

fn observation_next_commands(
    hypothesis_id: &str,
    evidence_id: &str,
    case_head_revision: &str,
    options: &ObservationRecordOptions,
) -> Value {
    json!({
        "support": concrete_observation_next_command("support", hypothesis_id, evidence_id, case_head_revision, options),
        "falsify": concrete_observation_next_command("falsify", hypothesis_id, evidence_id, case_head_revision, options)
    })
}

fn concrete_observation_next_command(
    action: &str,
    hypothesis_id: &str,
    evidence_id: &str,
    case_head_revision: &str,
    options: &ObservationRecordOptions,
) -> String {
    format!(
        "advisorygraphen hypothesis {action} --store {} --from-report CHECK.json --hypothesis-id {} --evidence {} --reviewer {} --reason '{}' --base-revision {} --format json",
        options.store.display(),
        hypothesis_id,
        evidence_id,
        options.reviewer,
        options.reason.replace('\'', " "),
        case_head_revision
    )
}

enum Materialization {
    Applied {
        cells: Vec<Value>,
        incidences: Vec<Value>,
    },
    Skipped {
        reason: String,
    },
}

enum DryRunMaterialization {
    Applied {
        cells: Vec<Value>,
        incidences: Vec<Value>,
        removed_incidence_ids: Vec<String>,
    },
    Skipped {
        reason: String,
    },
}

fn materialize_candidate_dry_run(
    space: &AdvisorySpaceEnvelope,
    blockers: &[Value],
    candidate: &Value,
) -> DryRunMaterialization {
    let candidate_type = candidate
        .get("candidate_type")
        .and_then(Value::as_str)
        .unwrap_or("");
    match candidate_type {
        "owner_assignment" => relation_candidate_dry_run(space, candidate, "owner_cell_id", "owns"),
        "lift_verification_link" => {
            relation_candidate_dry_run(space, candidate, "verification_cell_id", "verifies")
        }
        "ownership_clarification" | "proposed_test" => {
            let Some(blocker) = candidate_blocker(blockers, candidate) else {
                return DryRunMaterialization::Skipped {
                    reason: "candidate does not resolve a known obstruction".to_string(),
                };
            };
            let mut reviewed = candidate.clone();
            reviewed["review_status"] = json!("accepted");
            match materialize_candidate_structure(space, blocker, &reviewed, "dry-run") {
                Materialization::Applied { cells, incidences } => DryRunMaterialization::Applied {
                    cells,
                    incidences,
                    removed_incidence_ids: Vec::new(),
                },
                Materialization::Skipped { reason } => DryRunMaterialization::Skipped { reason },
            }
        }
        "proposed_interface" => interface_candidate_dry_run(space, candidate),
        "proposed_refactor_action" => refactor_candidate_dry_run(space, candidate),
        other => DryRunMaterialization::Skipped {
            reason: format!("candidate_type {other} is not supported for dry-run application"),
        },
    }
}

fn relation_candidate_dry_run(
    space: &AdvisorySpaceEnvelope,
    candidate: &Value,
    from_metadata_key: &str,
    relation_type: &str,
) -> DryRunMaterialization {
    let Some(from_id) = candidate
        .pointer(&format!("/metadata/{from_metadata_key}"))
        .and_then(Value::as_str)
    else {
        return DryRunMaterialization::Skipped {
            reason: format!("metadata.{from_metadata_key} is missing"),
        };
    };
    let Some(to_id) = candidate
        .pointer("/metadata/blocked_cell_id")
        .and_then(Value::as_str)
    else {
        return DryRunMaterialization::Skipped {
            reason: "metadata.blocked_cell_id is missing".to_string(),
        };
    };
    if !space.cells.iter().any(|cell| json_id(cell) == from_id)
        || !space.cells.iter().any(|cell| json_id(cell) == to_id)
    {
        return DryRunMaterialization::Skipped {
            reason: "proposed relation endpoint is not present in the space".to_string(),
        };
    }
    let incidence_id = candidate
        .get("proposed_incidence_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .next()
        .map(str::to_string)
        .unwrap_or_else(|| {
            format!(
                "incidence:{}-{relation_type}-{}",
                id_suffix(from_id),
                id_suffix(to_id)
            )
        });
    DryRunMaterialization::Applied {
        cells: Vec::new(),
        incidences: vec![json!({
            "id": incidence_id,
            "relation_type": relation_type,
            "from_id": from_id,
            "to_id": to_id,
            "context_ids": [],
            "evidence_ids": evidence_cell_ids_for_candidate(space, candidate),
            "strength": "soft",
            "provenance": dry_run_provenance(),
            "metadata": {
                "materialized_from_candidate_id": json_id(candidate),
                "materialization_kind": "completion_dry_run"
            }
        })],
        removed_incidence_ids: Vec::new(),
    }
}

fn interface_candidate_dry_run(
    space: &AdvisorySpaceEnvelope,
    candidate: &Value,
) -> DryRunMaterialization {
    let Some(interface_id) = candidate
        .get("proposed_cell_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .next()
    else {
        return DryRunMaterialization::Skipped {
            reason: "proposed interface candidate has no proposed_cell_ids".to_string(),
        };
    };
    let Some(from_id) = candidate
        .pointer("/metadata/from_cell_id")
        .and_then(Value::as_str)
    else {
        return DryRunMaterialization::Skipped {
            reason: "metadata.from_cell_id is missing".to_string(),
        };
    };
    let removed_incidence_ids = candidate
        .pointer("/metadata/incidence_id")
        .and_then(Value::as_str)
        .map(|id| vec![id.to_string()])
        .unwrap_or_default();
    let context_ids = space
        .cells
        .iter()
        .find(|cell| json_id(cell) == from_id)
        .and_then(|cell| cell.get("context_ids").and_then(Value::as_array).cloned())
        .unwrap_or_default();
    let title = candidate
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Proposed interface");
    DryRunMaterialization::Applied {
        cells: vec![json!({
            "id": interface_id,
            "cell_type": "interface",
            "title": title,
            "summary": candidate.get("rationale").and_then(Value::as_str),
            "context_ids": context_ids,
            "source_ids": candidate.get("source_ids").cloned().unwrap_or_else(|| json!([])),
            "structure_refs": [],
            "provenance": dry_run_provenance(),
            "metadata": {
                "materialized_from_candidate_id": json_id(candidate),
                "materialization_kind": "completion_dry_run"
            }
        })],
        incidences: vec![json!({
            "id": format!("incidence:{}-uses-{}", id_suffix(from_id), id_suffix(interface_id)),
            "relation_type": "uses",
            "from_id": from_id,
            "to_id": interface_id,
            "context_ids": [],
            "evidence_ids": evidence_cell_ids_for_candidate(space, candidate),
            "strength": "soft",
            "provenance": dry_run_provenance(),
            "metadata": {
                "materialized_from_candidate_id": json_id(candidate),
                "materialization_kind": "completion_dry_run"
            }
        })],
        removed_incidence_ids,
    }
}

fn refactor_candidate_dry_run(
    space: &AdvisorySpaceEnvelope,
    candidate: &Value,
) -> DryRunMaterialization {
    let Some(action_id) = candidate
        .get("proposed_cell_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .next()
    else {
        return DryRunMaterialization::Skipped {
            reason: "refactor candidate has no proposed_cell_ids".to_string(),
        };
    };
    let removed_incidence_ids = candidate
        .pointer("/metadata/incidence_id")
        .and_then(Value::as_str)
        .map(|id| vec![id.to_string()])
        .unwrap_or_default();
    let context_ids = candidate
        .pointer("/metadata/from_cell_id")
        .and_then(Value::as_str)
        .and_then(|from_id| {
            space
                .cells
                .iter()
                .find(|cell| json_id(cell) == from_id)
                .and_then(|cell| cell.get("context_ids").and_then(Value::as_array).cloned())
        })
        .unwrap_or_default();
    DryRunMaterialization::Applied {
        cells: vec![json!({
            "id": action_id,
            "cell_type": "action",
            "title": candidate.get("title").and_then(Value::as_str).unwrap_or("Proposed refactor action"),
            "summary": candidate.get("rationale").and_then(Value::as_str),
            "context_ids": context_ids,
            "source_ids": candidate.get("source_ids").cloned().unwrap_or_else(|| json!([])),
            "structure_refs": [],
            "provenance": dry_run_provenance(),
            "metadata": {
                "materialized_from_candidate_id": json_id(candidate),
                "materialization_kind": "completion_dry_run"
            }
        })],
        incidences: Vec::new(),
        removed_incidence_ids,
    }
}

fn candidate_blocker<'a>(blockers: &'a [Value], candidate: &Value) -> Option<&'a Value> {
    let resolved_ids = candidate
        .get("resolves_obstruction_ids")
        .and_then(Value::as_array)?;
    blockers.iter().find(|blocker| {
        let blocker_id = json_id(blocker);
        resolved_ids
            .iter()
            .any(|id| id.as_str() == Some(blocker_id))
    })
}

fn dry_run_provenance() -> Value {
    json!({
        "origin": "inferred",
        "actor": "advisorygraphen:completion-dry-run",
        "confidence": 0.6,
        "review_status": "unreviewed"
    })
}

fn evidence_cell_ids_for_candidate(space: &AdvisorySpaceEnvelope, candidate: &Value) -> Vec<Value> {
    candidate
        .get("source_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter_map(|source_id| {
            let evidence_id = format!("cell:evidence-{}", source_id.trim_start_matches("source:"));
            space
                .cells
                .iter()
                .any(|cell| json_id(cell) == evidence_id)
                .then_some(json!(evidence_id))
        })
        .collect()
}

fn obstruction_ids(report: &ReportEnvelope) -> Vec<String> {
    let mut ids = report
        .result
        .get("obstructions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|obstruction| obstruction.get("id").and_then(Value::as_str))
        .map(str::to_string)
        .collect::<Vec<_>>();
    ids.sort();
    ids
}

fn materialize_candidate_structure(
    space: &AdvisorySpaceEnvelope,
    blocker: &Value,
    candidate: &Value,
    reviewer: &str,
) -> Materialization {
    if candidate.get("review_status").and_then(Value::as_str) != Some("accepted") {
        return Materialization::Skipped {
            reason: "candidate is not accepted".to_string(),
        };
    }
    let Some(blocked_id) = blocker
        .get("blocked_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .find(|id| space.cells.iter().any(|cell| json_id(cell) == *id))
    else {
        return Materialization::Skipped {
            reason: "blocker has no materializable blocked cell".to_string(),
        };
    };
    let Some(blocked_cell) = space.cells.iter().find(|cell| json_id(cell) == blocked_id) else {
        return Materialization::Skipped {
            reason: "blocked cell not found in materialized space".to_string(),
        };
    };
    let candidate_id = json_id(candidate);
    let blocked_slug = id_suffix(blocked_id);
    let provenance = reviewed_materialization_provenance(reviewer);
    let source_ids = materialization_source_ids(candidate, blocker);
    let context_ids = blocked_cell
        .get("context_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    match candidate.get("candidate_type").and_then(Value::as_str) {
        Some("ownership_clarification") => {
            let owner_id = format!("cell:auto-owner-{blocked_slug}");
            let incidence_id = format!("incidence:auto-owner-{blocked_slug}-owns-{blocked_slug}");
            Materialization::Applied {
                cells: vec![json!({
                    "id": owner_id,
                    "cell_type": "owner",
                    "title": format!("Owner for {}", title(blocked_cell)),
                    "summary": format!(
                        "Placeholder owner materialized from accepted completion candidate {candidate_id}."
                    ),
                    "context_ids": context_ids,
                    "source_ids": source_ids,
                    "structure_refs": [],
                    "provenance": provenance.clone(),
                    "metadata": {
                        "materialized_from_candidate_id": candidate_id,
                        "materialization_kind": "accepted_completion",
                        "placeholder": true,
                        "requires_human_named_owner": true
                    }
                })],
                incidences: vec![json!({
                    "id": incidence_id,
                    "relation_type": "owns",
                    "from_id": owner_id,
                    "to_id": blocked_id,
                    "context_ids": [],
                    "evidence_ids": [],
                    "strength": "soft",
                    "provenance": provenance,
                    "metadata": {
                        "materialized_from_candidate_id": candidate_id,
                        "materialization_kind": "accepted_completion"
                    }
                })],
            }
        }
        Some("proposed_test") => {
            let verification_id = format!("cell:auto-verification-{blocked_slug}");
            let incidence_id =
                format!("incidence:auto-verification-{blocked_slug}-verifies-{blocked_slug}");
            Materialization::Applied {
                cells: vec![json!({
                    "id": verification_id,
                    "cell_type": "test_or_verification",
                    "title": format!("Verification for {}", title(blocked_cell)),
                    "summary": candidate
                        .get("rationale")
                        .and_then(Value::as_str)
                        .unwrap_or("Verification method materialized from an accepted completion candidate."),
                    "context_ids": context_ids,
                    "source_ids": source_ids,
                    "structure_refs": [],
                    "provenance": provenance.clone(),
                    "metadata": {
                        "materialized_from_candidate_id": candidate_id,
                        "materialization_kind": "accepted_completion",
                        "placeholder": true,
                        "requires_concrete_test_details": true
                    }
                })],
                incidences: vec![json!({
                    "id": incidence_id,
                    "relation_type": "verifies",
                    "from_id": verification_id,
                    "to_id": blocked_id,
                    "context_ids": [],
                    "evidence_ids": [],
                    "strength": "soft",
                    "provenance": provenance,
                    "metadata": {
                        "materialized_from_candidate_id": candidate_id,
                        "materialization_kind": "accepted_completion"
                    }
                })],
            }
        }
        Some(other) => Materialization::Skipped {
            reason: format!("candidate_type {other} is not supported for automatic application"),
        },
        None => Materialization::Skipped {
            reason: "candidate_type is missing".to_string(),
        },
    }
}

fn reviewed_materialization_provenance(reviewer: &str) -> Value {
    json!({
        "origin": "reviewed",
        "actor": reviewer,
        "confidence": 0.7,
        "review_status": "accepted"
    })
}

fn materialization_source_ids(candidate: &Value, blocker: &Value) -> Vec<Value> {
    let mut ids = candidate
        .get("source_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if ids.is_empty() {
        ids = blocker
            .get("evidence_ids")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
    }
    ids.sort_by(|left, right| {
        left.as_str()
            .unwrap_or("")
            .cmp(right.as_str().unwrap_or(""))
    });
    ids.dedup();
    ids
}

fn upsert_by_id(items: &mut Vec<Value>, value: Value) {
    let id = json_id(&value);
    if let Some(existing) = items.iter_mut().find(|item| json_id(item) == id) {
        *existing = value;
    } else {
        items.push(value);
    }
}

fn ids_of(items: &[Value]) -> Vec<String> {
    items.iter().map(json_id).map(str::to_string).collect()
}

fn title(value: &Value) -> &str {
    value
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_else(|| json_id(value))
}

fn id_suffix(id: &str) -> String {
    let raw = id.split_once(':').map(|(_, suffix)| suffix).unwrap_or(id);
    advisorygraphen_core::slugify_id(raw)
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
