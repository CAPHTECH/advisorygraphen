use advisorygraphen_core::{AdvisorySpaceEnvelope, ReportEnvelope};
use advisorygraphen_projection::build_projection;
use serde_json::{from_value, json, Value};

#[test]
fn code_derived_candidates_count_into_their_own_bucket() {
    let space = empty_space();
    let report = check_report(
        vec![code_derived_obstruction()],
        vec![code_derived_candidate(), source_derived_candidate()],
    );

    let projection = build_projection(&space, &report, "executive").unwrap();

    let quality = projection
        .pointer("/summary/candidate_quality")
        .expect("candidate_quality summary present");
    assert_eq!(quality["code_derived"].as_u64(), Some(1));
    assert_eq!(quality["source_derived"].as_u64(), Some(1));
    assert_eq!(quality["missing_precision_metadata"].as_u64(), Some(0));
    assert_eq!(quality["total"].as_u64(), Some(2));
}

#[test]
fn code_derived_obstructions_emit_lexical_detection_loss_entry() {
    let space = empty_space();
    let report = check_report(vec![code_derived_obstruction()], vec![]);

    let projection = build_projection(&space, &report, "executive").unwrap();

    let losses = projection
        .pointer("/projection_loss")
        .and_then(Value::as_array)
        .expect("projection_loss array present");
    let lexical = losses
        .iter()
        .find(|entry| entry["loss_type"] == "lexical_detection_caveat")
        .expect("lexical_detection_caveat entry emitted");
    assert_eq!(lexical["severity"], json!("medium"));
    assert_eq!(
        lexical["omitted_ids"],
        json!(["obstruction:route-missing-auth-guard"])
    );
}

#[test]
fn projection_loss_omits_lexical_caveat_when_no_code_derived_finding() {
    let space = empty_space();
    let report = check_report(vec![], vec![source_derived_candidate()]);

    let projection = build_projection(&space, &report, "executive").unwrap();

    let losses = projection
        .pointer("/projection_loss")
        .and_then(Value::as_array)
        .expect("projection_loss array present");
    assert!(losses
        .iter()
        .all(|entry| entry["loss_type"] != "lexical_detection_caveat"));
}

#[test]
fn proposal_content_summary_counts_blocked_content_obstructions() {
    let space = empty_space();
    let report = check_report(
        vec![],
        vec![
            source_derived_candidate(),
            blocked_proposal_content_candidate(),
        ],
    );

    let projection = build_projection(&space, &report, "ai_agent").unwrap();

    let summary = projection
        .pointer("/proposal_content_summary")
        .expect("proposal_content_summary present");
    assert_eq!(summary["with_structured_content"], json!(1));
    assert_eq!(summary["blocked_content"], json!(1));
    assert_eq!(summary["content_obstruction_count"], json!(1));
    assert_eq!(
        summary["content_obstruction_types"]["proposal_content_underspecified"],
        json!(1)
    );
}

#[test]
fn recommendation_trace_separates_primary_from_follow_up_observations() {
    let space = empty_space();
    let report = check_report(
        vec![],
        vec![primary_candidate(), unsupported_follow_up_candidate()],
    );

    let projection = build_projection(&space, &report, "executive").unwrap();

    let trace = projection
        .pointer("/summary/recommendation_trace")
        .expect("recommendation_trace present");
    assert_eq!(trace["primary_count"], json!(1));
    assert_eq!(trace["follow_up_observation_count"], json!(1));
    assert_eq!(
        trace["primary_recommendations"][0]["candidate_id"],
        "candidate:supported-action"
    );
    assert_eq!(
        trace["follow_up_observations"][0]["unsupported_hypothesis_ids"],
        json!(["hypothesis:unreviewed"])
    );
    assert_eq!(
        trace["follow_up_observations"][0]["ranked_observation_tasks"][0]["observation_type"],
        json!("hypothesis_support")
    );
    assert_eq!(
        trace["follow_up_observations"][0]["ranked_observation_tasks"][0]["source_ids_to_inspect"],
        json!(["source:test"])
    );
    assert_eq!(
        trace["follow_up_observations"][0]["ranked_observation_tasks"][0]["output_schema"]
            ["required"][0],
        json!("observation_status")
    );
    assert!(
        trace["follow_up_observations"][0]["ranked_observation_tasks"][0]["command_template"]
            .as_str()
            .unwrap()
            .contains("verification method")
    );
    assert_eq!(
        trace["follow_up_observations"][0]["ranked_observation_tasks"][0]["pass_fail_extraction"]
            ["review_required"],
        json!(true)
    );
}

#[test]
fn ai_agent_projection_exposes_hypothesis_promotion_workflow() {
    let space = empty_space();
    let report = check_report(vec![], vec![unsupported_follow_up_candidate()]);

    let projection = build_projection(&space, &report, "ai_agent").unwrap();

    let workflow = projection
        .pointer("/hypothesis_promotion_workflow")
        .expect("hypothesis_promotion_workflow present");
    assert_eq!(workflow["item_count"], json!(1));
    assert_eq!(
        workflow["items"][0]["blocking_hypothesis_ids"],
        json!(["hypothesis:unreviewed"])
    );
    assert_eq!(
        workflow["items"][0]["promotion_steps"][0],
        json!("Run the ranked observation tasks against the bounded source snapshot.")
    );
}

#[test]
fn projections_expose_explicit_hypothesis_matrix_and_proposal_trace() {
    let space = explicit_hypothesis_space();
    let report = check_report(vec![], vec![]);

    let executive = build_projection(&space, &report, "executive").unwrap();
    let matrix = executive
        .pointer("/summary/explicit_hypothesis_matrix")
        .expect("explicit_hypothesis_matrix present");
    assert_eq!(matrix["count"], json!(2));
    assert_eq!(matrix["status_counts"]["strongly_supported"], json!(1));
    assert_eq!(matrix["status_counts"]["falsified"], json!(1));
    assert_eq!(
        matrix["hypotheses"][0]["expected_observations"],
        json!(["unit test import resolution fails when node_modules is absent"])
    );
    assert_eq!(
        matrix["hypotheses"][0]["falsifiers"],
        json!(["clean install still cannot resolve elkjs"])
    );
    assert_eq!(
        matrix["hypotheses"][0]["supporting_incidence_ids"],
        json!(["incidence:evidence-supports-local-install"])
    );
    assert_eq!(
        matrix["hypotheses"][0]["competing_hypothesis_ids"],
        json!(["cell:hypothesis-lockfile"])
    );

    let trace = executive
        .pointer("/summary/explicit_proposal_trace")
        .expect("explicit_proposal_trace present");
    assert_eq!(trace["count"], json!(1));
    assert_eq!(
        trace["proposals"][0]["derived_hypothesis_ids"],
        json!(["cell:hypothesis-local-install"])
    );
    assert_eq!(
        trace["proposals"][0]["derived_hypothesis_statuses"][0]["status"],
        json!("strongly_supported")
    );
    assert_eq!(
        trace["proposals"][0]["required_verification"],
        json!("Run unit tests after reinstalling dependencies.")
    );
    assert_eq!(trace["proposals"][0]["owner_state"], json!("present"));
    assert_eq!(trace["proposals"][0]["proposal_quality_notes"], json!([]));

    let ai_agent = build_projection(&space, &report, "ai_agent").unwrap();
    assert_eq!(ai_agent["explicit_hypothesis_matrix"]["count"], json!(2));
    assert_eq!(ai_agent["explicit_proposal_trace"]["count"], json!(1));
    assert_eq!(ai_agent["hypothesis_summary"]["total"], json!(2));
    assert_eq!(
        ai_agent["hypothesis_summary"]["strongly_supported"],
        json!(1)
    );
    assert_eq!(ai_agent["hypothesis_summary"]["falsified"], json!(1));
    assert_eq!(
        ai_agent["hypotheses"][0]["source"],
        json!("explicit_advisory_space")
    );
    assert_eq!(
        ai_agent["hypotheses"][0]["refinement_status"],
        json!("seed")
    );
}

fn empty_space() -> AdvisorySpaceEnvelope {
    from_value(json!({
        "schema": "advisorygraphen.space.v1",
        "space_id": "space:advisory:precision-test",
        "engagement_id": "engagement:precision-test",
        "snapshot_id": "snapshot:precision-test",
        "package_id": "technical_advisory_mvp",
        "cells": [],
        "contexts": [],
        "incidences": [],
        "morphisms": [],
        "invariants": [],
        "policies": [],
        "metadata": {}
    }))
    .unwrap()
}

fn explicit_hypothesis_space() -> AdvisorySpaceEnvelope {
    from_value(json!({
        "schema": "advisorygraphen.space.v1",
        "space_id": "space:advisory:precision-test",
        "engagement_id": "engagement:precision-test",
        "snapshot_id": "snapshot:precision-test",
        "package_id": "technical_advisory_mvp",
        "cells": [
            {
                "id": "cell:hypothesis-local-install",
                "cell_type": "hypothesis",
                "title": "Local install state is missing elkjs",
                "summary": null,
                "context_ids": [],
                "source_ids": ["source:test"],
                "structure_refs": [],
                "provenance": { "derivation": "source_backed", "review_status": "accepted" },
                "metadata": {
                    "hypothesis_status": "strongly_supported",
                    "expected_observations": ["unit test import resolution fails when node_modules is absent"],
                    "falsifiers": ["clean install still cannot resolve elkjs"]
                }
            },
            {
                "id": "cell:hypothesis-lockfile",
                "cell_type": "hypothesis",
                "title": "Lockfile pins an incompatible elkjs version",
                "summary": null,
                "context_ids": [],
                "source_ids": ["source:test"],
                "structure_refs": [],
                "provenance": { "derivation": "source_backed", "review_status": "accepted" },
                "metadata": {
                    "hypothesis_status": "falsified",
                    "expected_observations": ["lockfile contains a broken elkjs resolution"],
                    "falsifiers": ["lockfile resolution is internally consistent"]
                }
            },
            {
                "id": "cell:evidence-install",
                "cell_type": "evidence",
                "title": "node_modules lacks elkjs",
                "summary": null,
                "context_ids": [],
                "source_ids": ["source:test"],
                "structure_refs": [],
                "provenance": { "derivation": "source_backed", "review_status": "accepted" },
                "metadata": {}
            },
            {
                "id": "cell:proposal-install-deps",
                "cell_type": "action",
                "title": "Reinstall dependencies before judging test failures",
                "summary": null,
                "context_ids": [],
                "source_ids": ["source:test"],
                "structure_refs": [],
                "provenance": { "derivation": "source_backed", "review_status": "accepted" },
                "metadata": {
                    "priority": "P0",
                    "required_verification": "Run unit tests after reinstalling dependencies."
                }
            },
            {
                "id": "cell:owner-test",
                "cell_type": "owner",
                "title": "Test owner",
                "summary": null,
                "context_ids": [],
                "source_ids": ["source:test"],
                "structure_refs": [],
                "provenance": { "derivation": "source_backed", "review_status": "accepted" },
                "metadata": {}
            }
        ],
        "contexts": [],
        "incidences": [
            {
                "id": "incidence:evidence-supports-local-install",
                "relation_type": "supports",
                "from_id": "cell:evidence-install",
                "to_id": "cell:hypothesis-local-install",
                "source_ids": ["source:test"],
                "evidence_ids": ["source:test"],
                "provenance": { "derivation": "source_backed", "review_status": "accepted" },
                "metadata": {}
            },
            {
                "id": "incidence:evidence-falsifies-lockfile",
                "relation_type": "falsifies",
                "from_id": "cell:evidence-install",
                "to_id": "cell:hypothesis-lockfile",
                "source_ids": ["source:test"],
                "evidence_ids": ["source:test"],
                "provenance": { "derivation": "source_backed", "review_status": "accepted" },
                "metadata": {}
            },
            {
                "id": "incidence:hypotheses-compete",
                "relation_type": "competes_with",
                "from_id": "cell:hypothesis-local-install",
                "to_id": "cell:hypothesis-lockfile",
                "source_ids": ["source:test"],
                "evidence_ids": ["source:test"],
                "provenance": { "derivation": "source_backed", "review_status": "accepted" },
                "metadata": {}
            },
            {
                "id": "incidence:proposal-derives-from-hypothesis",
                "relation_type": "derives_from",
                "from_id": "cell:proposal-install-deps",
                "to_id": "cell:hypothesis-local-install",
                "source_ids": ["source:test"],
                "evidence_ids": ["source:test"],
                "provenance": { "derivation": "source_backed", "review_status": "accepted" },
                "metadata": {}
            },
            {
                "id": "incidence:owner-owns-proposal",
                "relation_type": "owns",
                "from_id": "cell:owner-test",
                "to_id": "cell:proposal-install-deps",
                "source_ids": ["source:test"],
                "evidence_ids": ["source:test"],
                "provenance": { "derivation": "source_backed", "review_status": "accepted" },
                "metadata": {}
            }
        ],
        "morphisms": [],
        "invariants": [],
        "policies": [],
        "metadata": { "method": "one-problem-multiple-hypotheses-observe-classify-propose" }
    }))
    .unwrap()
}

fn code_derived_obstruction() -> Value {
    json!({
        "id": "obstruction:route-missing-auth-guard",
        "obstruction_type": "api_route_missing_auth",
        "severity": "high",
        "review_status": "unreviewed",
        "message": "Route touches the database without an authentication guard.",
        "witness_ids": ["cell:route"],
        "blocked_ids": ["cell:route"],
        "evidence_ids": ["source:route"],
        "recommended_completion_types": ["proposed_auth_guard"],
        "metadata": { "specificity": "code_derived" }
    })
}

fn code_derived_candidate() -> Value {
    json!({
        "id": "candidate:route-auth-guard",
        "candidate_type": "proposed_auth_guard",
        "confidence": 0.72,
        "source_ids": ["source:route"],
        "metadata": { "specificity": "code_derived" }
    })
}

fn source_derived_candidate() -> Value {
    json!({
        "id": "candidate:billing-status-api",
        "candidate_type": "proposed_interface",
        "confidence": 0.82,
        "source_ids": ["source:architecture"],
        "metadata": { "specificity": "source_derived" }
    })
}

fn blocked_proposal_content_candidate() -> Value {
    json!({
        "id": "candidate:missing-owner-owner",
        "candidate_type": "ownership_clarification",
        "confidence": 0.7,
        "source_ids": ["source:runbook"],
        "metadata": { "specificity": "generic" },
        "proposal_content": {
            "scenario": { "status": "blocked" },
            "content_obstructions": [
                { "obstruction_type": "proposal_content_underspecified" }
            ]
        }
    })
}

fn primary_candidate() -> Value {
    json!({
        "id": "candidate:supported-action",
        "title": "Supported action",
        "candidate_type": "proposed_test",
        "confidence": 0.8,
        "source_ids": ["source:test"],
        "recommendation_role": "primary",
        "supported_hypothesis_ids": ["hypothesis:supported"],
        "unsupported_hypothesis_ids": [],
        "hypothesis_trace": {
            "derived_hypothesis_id": "hypothesis:supported",
            "lifecycle_status": "supported",
            "supported": true
        },
        "metadata": { "specificity": "requirement_derived" }
    })
}

fn unsupported_follow_up_candidate() -> Value {
    json!({
        "id": "candidate:follow-up",
        "title": "Follow-up observation",
        "candidate_type": "proposed_test",
        "confidence": 0.6,
        "source_ids": ["source:test"],
        "recommendation_role": "follow_up_observation",
        "supported_hypothesis_ids": [],
        "unsupported_hypothesis_ids": ["hypothesis:unreviewed"],
        "hypothesis_trace": {
            "derived_hypothesis_id": "hypothesis:unreviewed",
            "lifecycle_status": "candidate",
            "supported": false
        },
        "metadata": { "specificity": "requirement_derived" },
        "proposal_content": {
            "content_obstructions": [
                {
                    "obstruction_type": "proposal_depends_on_unsupported_hypothesis",
                    "required_resolution": "Collect supporting observations."
                }
            ]
        }
    })
}

fn check_report(obstructions: Vec<Value>, candidates: Vec<Value>) -> Value {
    let envelope = ReportEnvelope::new(
        "check",
        Some("test"),
        json!({"space_id": "space:advisory:precision-test"}),
        json!({
            "obstructions": obstructions,
            "completion_candidates": candidates
        }),
    );
    serde_json::to_value(envelope).unwrap()
}
