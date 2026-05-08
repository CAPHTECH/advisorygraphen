use advisorygraphen_projection::OutputFormat;
use advisorygraphen_runtime::{
    case_import_workflow, case_reason_workflow, check_workflow,
    completions_apply_accepted_workflow, completions_dry_run_workflow,
    completions_propose_workflow, hypothesis_apply_proposals_workflow, hypothesis_falsify_workflow,
    hypothesis_propose_workflow, hypothesis_support_workflow, lift_workflow,
    observation_record_workflow, project_workflow, review_workflow, CaseImportOptions,
    CaseReasonOptions, CheckOptions, CompletionApplyAcceptedOptions, CompletionDryRunOptions,
    CompletionProposeOptions, HypothesisApplyProposalsOptions, HypothesisFalsifyOptions,
    HypothesisProposeOptions, LiftOptions, ObservationRecordOptions, ProjectOptions, ReviewOptions,
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
fn hypothesis_seed_lift_drives_hypothesis_first_proposal_trace() {
    let temp = TempDir::new().unwrap();
    let snapshot_path = temp.path().join("hypothesis-first.snapshot.json");
    let space_path = temp.path().join("space.json");
    let check_path = temp.path().join("check.json");
    let projection_path = temp.path().join("ai-agent.json");

    fs::write(
        &snapshot_path,
        serde_json::to_vec_pretty(&json!({
            "schema": "advisorygraphen.engagement.snapshot.v1",
            "snapshot_id": "snapshot:hypothesis-first-structuring",
            "engagement_id": "engagement:advisorygraphen-self-review",
            "captured_at": "2026-05-08T00:00:00Z",
            "source_boundary": {
                "included_source_ids": ["source:runtime-design-note", "source:case-store-observation"],
                "excluded_summary": [],
                "extraction_loss": ["Synthetic regression fixture records only the discriminating design signal."],
                "trust_notes": ["AI-inferred hypothesis remains unreviewed until observation support is recorded."],
                "adapter_version": "json_snapshot:0.1.0"
            },
            "metadata": {},
            "sources": [
                {
                    "id": "source:runtime-design-note",
                    "source_type": "repository_note",
                    "title": "Runtime design note",
                    "uri": null,
                    "captured_at": "2026-05-08T00:00:00Z",
                    "classification": "internal",
                    "metadata": {}
                },
                {
                    "id": "source:case-store-observation",
                    "source_type": "code_observation",
                    "title": "Case store observation",
                    "uri": null,
                    "captured_at": "2026-05-08T00:00:00Z",
                    "classification": "internal",
                    "metadata": {}
                }
            ],
            "records": [
                {
                    "id": "record:case-log-source-of-truth-drift",
                    "record_type": "hypothesis_seed",
                    "title": "Case log source-of-truth drift",
                    "summary": "Case reasoning may rely on materialized space state instead of replaying the append-only event log.",
                    "source_ids": ["source:runtime-design-note", "source:case-store-observation"],
                    "context_hints": ["case-reasoning", "storage"],
                    "relation": null,
                    "provenance": {
                        "origin": "inferred",
                        "actor": "ai-agent:design-scan",
                        "confidence": 0.64,
                        "review_status": "unreviewed"
                    },
                    "metadata": {
                        "expected_observations": [
                            "case reason reads materialized/space.json without replaying morphism-log.jsonl"
                        ],
                        "falsifiers": [
                            "case reason reconstructs the advisory space by replaying the append-only log before projection"
                        ],
                        "candidate_structure_types": ["obstruction", "invariant", "refactor_action"]
                    }
                },
                {
                    "id": "record:case-log-replay-gap-refined",
                    "record_type": "hypothesis_refinement",
                    "title": "Case log replay gap after import",
                    "summary": "The likely design issue is narrower: imported materialized state and later log events can diverge unless all case reasoning uses one replay path.",
                    "source_ids": ["source:runtime-design-note", "source:case-store-observation"],
                    "context_hints": ["case-reasoning", "storage"],
                    "relation": null,
                    "provenance": {
                        "origin": "inferred",
                        "actor": "ai-agent:design-scan",
                        "confidence": 0.68,
                        "review_status": "unreviewed"
                    },
                    "metadata": {
                        "refinement_iteration": 2,
                        "expected_observations": [
                            "case import writes materialized state and later review events mutate the same case through a separate log path"
                        ],
                        "falsifiers": [
                            "all case reasoning reconstructs state exclusively from the append-only log"
                        ],
                        "candidate_structure_types": ["invariant", "refactor_action"]
                    }
                },
                {
                    "id": "record:unify-case-store-replay-action",
                    "record_type": "structure_proposal",
                    "title": "Unify case store replay path",
                    "summary": "Make case reason and close checks rebuild state through the same append-only replay path before projecting recommendations.",
                    "source_ids": ["source:runtime-design-note"],
                    "context_hints": ["case-reasoning", "storage"],
                    "relation": null,
                    "provenance": {
                        "origin": "inferred",
                        "actor": "ai-agent:design-scan",
                        "confidence": 0.58,
                        "review_status": "unreviewed"
                    },
                    "metadata": {
                        "priority": "p1",
                        "derived_from_hypothesis_id": "cell:case-log-replay-gap-refined",
                        "required_verification": "case reason and close check produce identical state from materialized import plus replayed log entries"
                    }
                },
                {
                    "id": "record:architecture-maintainer",
                    "record_type": "owner",
                    "title": "Architecture maintainer",
                    "summary": "Maintains AdvisoryGraphen runtime architecture.",
                    "source_ids": ["source:runtime-design-note"],
                    "context_hints": ["case-reasoning"],
                    "relation": null,
                    "provenance": {
                        "origin": "source_backed",
                        "actor": "source-adapter:json",
                        "confidence": 1.0,
                        "review_status": "accepted"
                    },
                    "metadata": {}
                },
                {
                    "id": "record:refined-hypothesis-narrows-seed",
                    "record_type": "refinement_relation",
                    "title": "Replay-gap hypothesis refines source-of-truth drift",
                    "summary": "The refined replay-gap hypothesis narrows the broader source-of-truth drift hypothesis.",
                    "source_ids": ["source:case-store-observation"],
                    "context_hints": ["case-reasoning", "storage"],
                    "relation": {
                        "relation_type": "refines",
                        "from_record_id": "record:case-log-replay-gap-refined",
                        "to_record_id": "record:case-log-source-of-truth-drift"
                    },
                    "provenance": {
                        "origin": "inferred",
                        "actor": "ai-agent:design-scan",
                        "confidence": 0.68,
                        "review_status": "unreviewed"
                    },
                    "metadata": {}
                },
                {
                    "id": "record:proposal-derived-from-hypothesis",
                    "record_type": "derivation_relation",
                    "title": "Proposal derives from source-of-truth hypothesis",
                    "summary": "The replay-path action is only justified if the refined replay-gap hypothesis remains plausible.",
                    "source_ids": ["source:runtime-design-note"],
                    "context_hints": ["case-reasoning", "storage"],
                    "relation": {
                        "relation_type": "derives_from",
                        "from_record_id": "record:unify-case-store-replay-action",
                        "to_record_id": "record:case-log-replay-gap-refined"
                    },
                    "provenance": {
                        "origin": "inferred",
                        "actor": "ai-agent:design-scan",
                        "confidence": 0.58,
                        "review_status": "unreviewed"
                    },
                    "metadata": {}
                },
                {
                    "id": "record:architecture-maintainer-owns-proposal",
                    "record_type": "ownership_relation",
                    "title": "Architecture maintainer owns replay proposal",
                    "summary": "The maintainer owns verification of the replay-path proposal.",
                    "source_ids": ["source:runtime-design-note"],
                    "context_hints": ["case-reasoning"],
                    "relation": {
                        "relation_type": "owns",
                        "from_record_id": "record:architecture-maintainer",
                        "to_record_id": "record:unify-case-store-replay-action"
                    },
                    "provenance": {
                        "origin": "source_backed",
                        "actor": "source-adapter:json",
                        "confidence": 1.0,
                        "review_status": "accepted"
                    },
                    "metadata": {}
                }
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    lift_workflow(&LiftOptions {
        input: snapshot_path,
        package: "technical_advisory_mvp".to_string(),
        output: Some(space_path.clone()),
        command: None,
    })
    .unwrap();

    let space: serde_json::Value = serde_json::from_slice(&fs::read(&space_path).unwrap()).unwrap();
    let hypothesis = space["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|cell| cell["id"] == "cell:case-log-source-of-truth-drift")
        .unwrap();
    assert_eq!(hypothesis["cell_type"], "hypothesis");
    assert_eq!(hypothesis["metadata"]["hypothesis"], true);
    assert_eq!(hypothesis["metadata"]["hypothesis_status"], "candidate");
    assert_eq!(
        hypothesis["metadata"]["structuring_phase"],
        "hypothesis_first"
    );
    let refined = space["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|cell| cell["id"] == "cell:case-log-replay-gap-refined")
        .unwrap();
    assert_eq!(refined["cell_type"], "hypothesis");
    assert_eq!(refined["metadata"]["hypothesis_refinement"], true);
    assert_eq!(
        refined["metadata"]["structuring_phase"],
        "hypothesis_refinement"
    );

    let proposal = space["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|cell| cell["id"] == "cell:unify-case-store-replay-action")
        .unwrap();
    assert_eq!(proposal["cell_type"], "action");
    assert_eq!(proposal["metadata"]["structure_proposal"], true);
    assert_eq!(
        proposal["metadata"]["structuring_phase"],
        "derived_from_hypothesis"
    );
    assert!(space["incidences"]
        .as_array()
        .unwrap()
        .iter()
        .any(|incidence| {
            incidence["relation_type"] == "derives_from"
                && incidence["from_id"] == "cell:unify-case-store-replay-action"
                && incidence["to_id"] == "cell:case-log-replay-gap-refined"
        }));
    assert!(space["incidences"]
        .as_array()
        .unwrap()
        .iter()
        .any(|incidence| {
            incidence["relation_type"] == "refines"
                && incidence["from_id"] == "cell:case-log-replay-gap-refined"
                && incidence["to_id"] == "cell:case-log-source-of-truth-drift"
        }));

    check_workflow(&CheckOptions {
        space: space_path.clone(),
        ruleset: "technical_advisory_mvp".to_string(),
        output: Some(check_path.clone()),
        fail_on: None,
        command: None,
    })
    .unwrap();
    let check: serde_json::Value = serde_json::from_slice(&fs::read(&check_path).unwrap()).unwrap();
    assert!(check["result"]["obstructions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|obstruction| {
            obstruction["obstruction_type"] == "proposal_derived_from_unsupported_hypothesis"
                && obstruction["metadata"]["action_id"] == "cell:unify-case-store-replay-action"
                && obstruction["metadata"]["hypothesis_id"] == "cell:case-log-replay-gap-refined"
        }));

    project_workflow(&ProjectOptions {
        space: space_path,
        report: check_path,
        completions_report: None,
        audience: "ai_agent".to_string(),
        format: OutputFormat::Json,
        output: Some(projection_path.clone()),
    })
    .unwrap();
    let projection: serde_json::Value =
        serde_json::from_slice(&fs::read(&projection_path).unwrap()).unwrap();
    assert!(projection["explicit_hypothesis_matrix"]["hypotheses"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["hypothesis_id"] == "cell:case-log-source-of-truth-drift"));
    let refined_projection = projection["explicit_hypothesis_matrix"]["hypotheses"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["hypothesis_id"] == "cell:case-log-replay-gap-refined")
        .unwrap();
    assert_eq!(refined_projection["refinement_status"], "refined");
    assert_eq!(refined_projection["refinement_depth"], 1);
    assert!(refined_projection["refinement_parent_ids"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == "cell:case-log-source-of-truth-drift"));
    let seed_projection = projection["explicit_hypothesis_matrix"]["hypotheses"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["hypothesis_id"] == "cell:case-log-source-of-truth-drift")
        .unwrap();
    assert_eq!(seed_projection["refinement_status"], "has_refinements");
    assert!(seed_projection["refinement_child_ids"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == "cell:case-log-replay-gap-refined"));
    assert!(projection["explicit_proposal_trace"]["proposals"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| {
            entry["action_id"] == "cell:unify-case-store-replay-action"
                && entry["derived_hypothesis_ids"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|id| id == "cell:case-log-replay-gap-refined")
        }));
}

#[test]
fn supported_high_priority_proposal_requires_hypothesis_refinement_lineage() {
    let temp = TempDir::new().unwrap();
    let space_path = temp.path().join("space.json");
    let check_path = temp.path().join("check.json");
    let space = json!({
        "schema": "advisorygraphen.space.v1",
        "space_id": "space:hypothesis-refinement-gate",
        "engagement_id": "engagement:hypothesis-refinement-gate",
        "snapshot_id": "snapshot:hypothesis-refinement-gate",
        "package_id": "package:technical_advisory_mvp",
        "cells": [
            {
                "id": "cell:evidence-runtime-observation",
                "cell_type": "evidence",
                "title": "Runtime observation",
                "summary": "Source-backed observation supports the hypothesis.",
                "context_ids": [],
                "source_ids": ["source:runtime-observation"],
                "structure_refs": [],
                "provenance": {
                    "origin": "source_backed",
                    "actor": "test",
                    "confidence": 1.0,
                    "review_status": "accepted"
                },
                "metadata": {}
            },
            {
                "id": "cell:case-store-drift",
                "cell_type": "hypothesis",
                "title": "Case store drift",
                "summary": "A supported but still unrefined hypothesis.",
                "context_ids": [],
                "source_ids": ["source:runtime-observation"],
                "structure_refs": [],
                "provenance": {
                    "origin": "source_backed",
                    "actor": "test",
                    "confidence": 1.0,
                    "review_status": "accepted"
                },
                "metadata": {
                    "hypothesis": true,
                    "hypothesis_status": "supported",
                    "expected_observations": ["case reason uses materialized state"],
                    "falsifiers": ["case reason always replays append-only log"]
                }
            },
            {
                "id": "cell:architecture-owner",
                "cell_type": "owner",
                "title": "Architecture owner",
                "summary": "Owns runtime design.",
                "context_ids": [],
                "source_ids": ["source:runtime-observation"],
                "structure_refs": [],
                "provenance": {
                    "origin": "source_backed",
                    "actor": "test",
                    "confidence": 1.0,
                    "review_status": "accepted"
                },
                "metadata": {}
            },
            {
                "id": "cell:rewrite-case-store",
                "cell_type": "action",
                "title": "Rewrite case store",
                "summary": "High-priority action should not be promoted from an unrefined hypothesis.",
                "context_ids": [],
                "source_ids": ["source:runtime-observation"],
                "structure_refs": [],
                "provenance": {
                    "origin": "source_backed",
                    "actor": "test",
                    "confidence": 1.0,
                    "review_status": "accepted"
                },
                "metadata": {
                    "priority": "p1",
                    "derived_from_hypothesis_id": "cell:case-store-drift",
                    "required_verification": "case replay parity test passes"
                }
            }
        ],
        "contexts": [],
        "incidences": [
            {
                "id": "incidence:evidence-supports-case-store-drift",
                "relation_type": "supports",
                "from_id": "cell:evidence-runtime-observation",
                "to_id": "cell:case-store-drift",
                "context_ids": [],
                "evidence_ids": ["cell:evidence-runtime-observation"],
                "strength": "hard",
                "provenance": {
                    "origin": "source_backed",
                    "actor": "test",
                    "confidence": 1.0,
                    "review_status": "accepted"
                },
                "metadata": {}
            },
            {
                "id": "incidence:rewrite-derived-from-case-store-drift",
                "relation_type": "derives_from",
                "from_id": "cell:rewrite-case-store",
                "to_id": "cell:case-store-drift",
                "context_ids": [],
                "evidence_ids": ["cell:evidence-runtime-observation"],
                "strength": "hard",
                "provenance": {
                    "origin": "source_backed",
                    "actor": "test",
                    "confidence": 1.0,
                    "review_status": "accepted"
                },
                "metadata": {}
            },
            {
                "id": "incidence:owner-owns-rewrite",
                "relation_type": "owns",
                "from_id": "cell:architecture-owner",
                "to_id": "cell:rewrite-case-store",
                "context_ids": [],
                "evidence_ids": ["cell:evidence-runtime-observation"],
                "strength": "hard",
                "provenance": {
                    "origin": "source_backed",
                    "actor": "test",
                    "confidence": 1.0,
                    "review_status": "accepted"
                },
                "metadata": {}
            }
        ],
        "morphisms": [],
        "invariants": [],
        "policies": [],
        "metadata": {}
    });
    fs::write(&space_path, serde_json::to_vec_pretty(&space).unwrap()).unwrap();

    check_workflow(&CheckOptions {
        space: space_path,
        ruleset: "technical_advisory_mvp".to_string(),
        output: Some(check_path.clone()),
        fail_on: None,
        command: None,
    })
    .unwrap();
    let check: serde_json::Value = serde_json::from_slice(&fs::read(&check_path).unwrap()).unwrap();
    assert!(check["result"]["obstructions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|obstruction| {
            obstruction["obstruction_type"]
                == "high_priority_proposal_missing_hypothesis_refinement"
                && obstruction["metadata"]["action_id"] == "cell:rewrite-case-store"
        }));
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
fn observation_support_promotes_follow_up_candidate_to_primary_after_case_reason() {
    let temp = TempDir::new().unwrap();
    let space_path = temp.path().join("space.json");
    let check_path = temp.path().join("check.json");
    let completions_path = temp.path().join("completions.json");
    let projection_path = temp.path().join("ai-agent.json");
    let observation_result_path = temp.path().join("observation-result.json");
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
    completions_propose_workflow(&CompletionProposeOptions {
        space: space_path.clone(),
        from_report: check_path.clone(),
        output: Some(completions_path.clone()),
        command: None,
    })
    .unwrap();
    project_workflow(&ProjectOptions {
        space: space_path.clone(),
        report: check_path.clone(),
        completions_report: Some(completions_path),
        audience: "ai_agent".to_string(),
        format: OutputFormat::Json,
        output: Some(projection_path.clone()),
    })
    .unwrap();
    let projection: serde_json::Value =
        serde_json::from_slice(&fs::read(&projection_path).unwrap()).unwrap();
    assert_eq!(
        projection["recommendation_trace"]["primary_count"], 0,
        "unsupported hypotheses should keep candidates out of primary recommendations"
    );
    let task = projection["recommendation_trace"]["follow_up_observations"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|item| item["ranked_observation_tasks"].as_array().unwrap().iter())
        .find(|task| {
            task["hypothesis_id"]
                == "hypothesis:order-service-direct-billing-db-access-implicit-interface"
        })
        .unwrap();
    let task_id = task["task_id"].as_str().unwrap().to_string();

    case_import_workflow(&CaseImportOptions {
        store: store_path.clone(),
        space: space_path,
        revision_id: "revision:initial".to_string(),
    })
    .unwrap();
    fs::write(
        &observation_result_path,
        serde_json::to_vec_pretty(&json!({
            "observation_status": "supports",
            "evidence_ids": ["source:architecture-note"],
            "summary": "Reviewed architecture evidence supports the implicit interface hypothesis.",
            "supports_hypothesis": true,
            "falsifies_hypothesis": false
        }))
        .unwrap(),
    )
    .unwrap();
    let observation = observation_record_workflow(&ObservationRecordOptions {
        store: store_path.clone(),
        space_id: space_id.clone(),
        from_projection: projection_path,
        task_id,
        result: observation_result_path,
        reviewer: "reviewer:test".to_string(),
        reason: "Observed source-backed support for the hypothesis.".to_string(),
        base_revision: Some("revision:initial".to_string()),
    })
    .unwrap();
    let evidence_id = observation["result"]["evidence_cell"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let observation_head = observation["result"]["case_head_revision"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        observation["result"]["promotion_gate"]["evidence_cell_id"],
        evidence_id
    );
    let support_command = observation["result"]["suggested_next_commands"]["support"]
        .as_str()
        .unwrap();
    assert!(support_command.contains(&evidence_id));
    assert!(support_command.contains(&observation_head));
    assert!(!support_command.contains("<evidence_cell_id>"));

    hypothesis_support_workflow(&HypothesisFalsifyOptions {
        store: store_path.clone(),
        from_report: check_path,
        hypothesis_id: "hypothesis:order-service-direct-billing-db-access-implicit-interface"
            .to_string(),
        evidence_ids: vec![evidence_id],
        reviewer: "reviewer:test".to_string(),
        reason: "Observation supports the implicit interface hypothesis.".to_string(),
        base_revision: Some(observation_head),
    })
    .unwrap();

    let report = case_reason_workflow(&CaseReasonOptions {
        store: store_path,
        space_id,
    })
    .unwrap();
    let trace = &report["projection"]["recommendation_trace"];
    assert!(
        trace["primary_count"].as_u64().unwrap() > 0,
        "supporting the hypothesis should allow derived candidates to become primary"
    );
    assert!(trace["primary_recommendations"]
        .as_array()
        .unwrap()
        .iter()
        .all(|candidate| candidate["supported_hypothesis_ids"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |id| id == "hypothesis:order-service-direct-billing-db-access-implicit-interface"
            )));
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
    let dry_run = completions_dry_run_workflow(&CompletionDryRunOptions {
        space: space_path.clone(),
        from_report: completions_path.clone(),
        candidate_ids: vec![candidate_id.clone()],
        output: None,
        command: None,
    })
    .unwrap();
    assert_eq!(dry_run.report_type, "completion_dry_run");
    let run = &dry_run.result["dry_runs"][0];
    assert_eq!(run["status"], "applied_to_dry_run_space");
    assert_eq!(
        run["applied_structure"]["cell_ids"][0],
        "cell:auto-owner-ship-release-action"
    );
    assert!(run["check_delta"]["resolved_obstruction_ids"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == "obstruction:ship-release-action-missing-owner"));
    assert_eq!(run["after_close_status"]["closeable"], true);

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
