use advisorygraphen_runtime::{
    case_import_workflow, case_reason_workflow, check_workflow,
    completions_apply_accepted_workflow, completions_propose_workflow,
    hypothesis_apply_proposals_workflow, hypothesis_falsify_workflow, hypothesis_propose_workflow,
    lift_workflow, review_workflow, CaseImportOptions, CaseReasonOptions, CheckOptions,
    CompletionApplyAcceptedOptions, CompletionProposeOptions, HypothesisApplyProposalsOptions,
    HypothesisFalsifyOptions, HypothesisProposeOptions, LiftOptions, ReviewOptions,
};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/technical-advisory/direct-db-access")
        .join(path)
}

#[test]
fn falsifying_primary_hypothesis_propagates_to_candidates_and_obstructions() {
    let temp = TempDir::new().unwrap();
    let space_path = temp.path().join("space.json");
    let check_path = temp.path().join("check.json");
    let store_path = temp.path().join("store");

    lift_workflow(&LiftOptions {
        input: fixture("advisory.input.json"),
        package: "technical_advisory_mvp".to_string(),
        output: Some(space_path.clone()),
        command: None,
    })
    .unwrap();
    let check = check_workflow(&CheckOptions {
        space: space_path.clone(),
        ruleset: "technical_advisory_mvp".to_string(),
        output: Some(check_path.clone()),
        fail_on: None,
        command: None,
    })
    .unwrap();
    let space_id = check.input["space_id"].as_str().unwrap().to_string();

    case_import_workflow(&CaseImportOptions {
        store: store_path.clone(),
        space: space_path,
        revision_id: "revision:initial".to_string(),
    })
    .unwrap();

    hypothesis_falsify_workflow(&HypothesisFalsifyOptions {
        store: store_path.clone(),
        from_report: check_path,
        hypothesis_id: "hypothesis:order-service-direct-billing-db-access-implicit-interface"
            .to_string(),
        evidence_ids: vec!["cell:evidence-architecture-note".to_string()],
        reviewer: "reviewer:test".to_string(),
        reason: "ADR-0042 documents this as accepted exception".to_string(),
        base_revision: Some("revision:initial".to_string()),
    })
    .unwrap();

    let report = case_reason_workflow(&CaseReasonOptions {
        store: store_path,
        space_id,
    })
    .unwrap();
    let projection = &report["projection"];
    let candidates = projection["candidate_review_state"].as_array().unwrap();
    let blockers = report["result"]["blockers"].as_array().unwrap();

    let orphans: Vec<&serde_json::Value> = candidates
        .iter()
        .filter(|c| {
            c["metadata"]["parent_hypothesis_status"]
                .as_str()
                .map(|status| status == "falsified")
                .unwrap_or(false)
        })
        .collect();
    assert_eq!(orphans.len(), 2, "two candidates derived from falsified H1");
    assert!(
        orphans.iter().all(|c| c["review_status"] == "superseded"),
        "orphan candidates auto-superseded"
    );

    assert!(
        candidates
            .iter()
            .any(|c| c["candidate_type"] == "context_remap"),
        "candidate from supported context-misclassified hypothesis"
    );
    assert!(
        candidates
            .iter()
            .any(|c| c["candidate_type"] == "documented_exception_policy"),
        "candidate from supported undocumented-exception hypothesis"
    );

    let boundary = blockers
        .iter()
        .find(|b| b["obstruction_type"] == "boundary_violation")
        .unwrap();
    assert_eq!(boundary["severity"], "high");
    assert_eq!(
        boundary["metadata"]["effective_severity"], "medium",
        "obstruction reframed to medium effective_severity"
    );
    let reframe = &boundary["metadata"]["reframe"];
    assert!(reframe["primary_hypothesis_falsified"].as_bool().unwrap());
    let supporting_ids = reframe["supporting_hypothesis_ids"].as_array().unwrap();
    assert_eq!(supporting_ids.len(), 2);
}

#[test]
fn policy_allowed_hypothesis_lifecycle_proposals_apply_as_events() {
    let temp = TempDir::new().unwrap();
    let space_path = temp.path().join("space.json");
    let check_path = temp.path().join("check.json");
    let proposals_path = temp.path().join("hypothesis-proposals.json");
    let store_path = temp.path().join("store");

    lift_workflow(&LiftOptions {
        input: fixture("advisory.input.json"),
        package: "technical_advisory_mvp".to_string(),
        output: Some(space_path.clone()),
        command: None,
    })
    .unwrap();
    let check = check_workflow(&CheckOptions {
        space: space_path.clone(),
        ruleset: "technical_advisory_mvp".to_string(),
        output: Some(check_path.clone()),
        fail_on: None,
        command: None,
    })
    .unwrap();
    let space_id = check.input["space_id"].as_str().unwrap().to_string();

    let mut observed_space: serde_json::Value =
        serde_json::from_slice(&fs::read(&space_path).unwrap()).unwrap();
    observed_space["cells"].as_array_mut().unwrap().push(json!({
        "id": "cell:reviewed-direct-db-observation",
        "cell_type": "evidence",
        "title": "Reviewed direct DB observation",
        "summary": "Reviewed evidence supports the implicit interface hypothesis.",
        "context_ids": [],
        "source_ids": ["source:architecture-note"],
        "structure_refs": [],
        "provenance": {
            "origin": "source_backed",
            "actor": "reviewer:test",
            "confidence": 0.9,
            "review_status": "accepted"
        },
        "metadata": {
            "supports_hypothesis_id": "hypothesis:order-service-direct-billing-db-access-implicit-interface"
        }
    }));
    fs::write(
        &space_path,
        serde_json::to_vec_pretty(&observed_space).unwrap(),
    )
    .unwrap();

    let proposals = hypothesis_propose_workflow(&HypothesisProposeOptions {
        space: space_path.clone(),
        from_report: check_path,
        output: Some(proposals_path.clone()),
        command: None,
    })
    .unwrap();
    assert_eq!(proposals.result["proposal_count"], 1);
    assert_eq!(
        proposals.result["lifecycle_proposals"][0]["proposed_outcome"],
        "supported"
    );

    case_import_workflow(&CaseImportOptions {
        store: store_path.clone(),
        space: space_path,
        revision_id: "revision:initial".to_string(),
    })
    .unwrap();

    let apply = hypothesis_apply_proposals_workflow(&HypothesisApplyProposalsOptions {
        store: store_path.clone(),
        from_report: proposals_path,
        policy: None,
        reviewer: "ai-agent:test".to_string(),
        reason: "Default conservative policy allowed source-backed support.".to_string(),
        base_revision: Some("revision:initial".to_string()),
        dry_run: false,
    })
    .unwrap();
    assert_eq!(apply["result"]["applied_count"], 1);
    assert_eq!(apply["result"]["skipped_count"], 0);
    assert_eq!(
        apply["result"]["post_apply_case_reason"]["case_head_revision"],
        apply["result"]["case_head_revision"]
    );

    let report = case_reason_workflow(&CaseReasonOptions {
        store: store_path,
        space_id,
    })
    .unwrap();
    let hypotheses = report["projection"]["hypotheses"].as_array().unwrap();
    let supported = hypotheses
        .iter()
        .find(|hypothesis| {
            hypothesis["id"]
                == "hypothesis:order-service-direct-billing-db-access-implicit-interface"
        })
        .unwrap();
    assert_eq!(supported["lifecycle_status"], "supported");
}

#[test]
fn accepted_completion_candidate_materializes_required_owner_structure() {
    let temp = TempDir::new().unwrap();
    let space_path = temp.path().join("space.json");
    let check_path = temp.path().join("check.json");
    let completions_path = temp.path().join("completions.json");
    let store_path = temp.path().join("store");
    let space = json!({
        "schema": "advisorygraphen.space.v1",
        "space_id": "space:completion-application-test",
        "engagement_id": "engagement:completion-application-test",
        "snapshot_id": "snapshot:completion-application-test",
        "package_id": "package:technical_advisory_mvp",
        "cells": [
            {
                "id": "cell:ship-release-action",
                "cell_type": "action",
                "title": "Ship release action",
                "summary": "Release action needs an owner before the case can close.",
                "context_ids": [],
                "source_ids": ["source:test-plan"],
                "structure_refs": [],
                "provenance": {
                    "origin": "source_backed",
                    "actor": "test",
                    "confidence": 1.0,
                    "review_status": "accepted"
                },
                "metadata": {}
            }
        ],
        "contexts": [],
        "incidences": [],
        "morphisms": [],
        "invariants": [],
        "policies": [],
        "metadata": {}
    });
    fs::write(&space_path, serde_json::to_vec_pretty(&space).unwrap()).unwrap();

    let check = check_workflow(&CheckOptions {
        space: space_path.clone(),
        ruleset: "technical_advisory_mvp".to_string(),
        output: Some(check_path.clone()),
        fail_on: None,
        command: None,
    })
    .unwrap();
    assert_eq!(
        check.result["obstructions"][0]["id"],
        "obstruction:ship-release-action-missing-owner"
    );

    let completions = completions_propose_workflow(&CompletionProposeOptions {
        space: space_path.clone(),
        from_report: check_path.clone(),
        output: Some(completions_path.clone()),
        command: None,
    })
    .unwrap();
    let candidate_id = completions.result["completion_candidates"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        candidate_id,
        "candidate:ship-release-action-missing-owner-owner"
    );

    case_import_workflow(&CaseImportOptions {
        store: store_path.clone(),
        space: space_path,
        revision_id: "revision:initial".to_string(),
    })
    .unwrap();
    review_workflow(&ReviewOptions {
        store: store_path.clone(),
        candidate_id,
        from_report: Some(completions_path),
        reviewer: "reviewer:test".to_string(),
        reason: "Owner placeholder is acceptable for materialization test.".to_string(),
        outcome: "accepted".to_string(),
        base_revision: Some("revision:initial".to_string()),
    })
    .unwrap();
    let review_head = fs::read_to_string(
        store_path
            .join("spaces")
            .join("space-completion-application-test")
            .join("HEAD"),
    )
    .unwrap();

    let apply = completions_apply_accepted_workflow(&CompletionApplyAcceptedOptions {
        store: store_path.clone(),
        space_id: "space:completion-application-test".to_string(),
        reviewer: "ai-agent:test".to_string(),
        reason: "Apply reviewed accepted completion.".to_string(),
        base_revision: Some(review_head),
        dry_run: false,
    })
    .unwrap();
    assert_eq!(apply["result"]["applied_count"], 1);
    assert_eq!(
        apply["result"]["applied_structures"][0]["cell_ids"][0],
        "cell:auto-owner-ship-release-action"
    );
    assert_eq!(
        apply["result"]["post_apply_case_reason"]["close_status"]["closeable"],
        true
    );

    let reason = case_reason_workflow(&CaseReasonOptions {
        store: store_path,
        space_id: "space:completion-application-test".to_string(),
    })
    .unwrap();
    assert!(reason["result"]["blockers"].as_array().unwrap().is_empty());
}
