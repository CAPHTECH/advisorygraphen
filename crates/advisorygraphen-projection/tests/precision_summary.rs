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
