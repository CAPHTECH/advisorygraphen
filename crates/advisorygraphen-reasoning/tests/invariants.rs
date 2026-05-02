use advisorygraphen_core::AdvisorySpaceEnvelope;
use advisorygraphen_reasoning::{
    blocker_resolution_state, check_space, frontier_items, waiting_items,
};
use serde_json::{json, Value};

#[test]
fn action_without_owner_emits_missing_owner_obstruction() {
    let space = base_space(vec![action_cell("cell:ship-action")], vec![]);

    let report = check_space(&space, "technical_advisory_mvp", None, None).unwrap();

    assert_obstruction(&report.result, "missing_owner");
}

#[test]
fn requirement_marked_verification_required_emits_obstruction() {
    let requirement = json!({
        "id": "cell:requirement",
        "cell_type": "requirement",
        "title": "Requirement",
        "summary": null,
        "context_ids": [],
        "source_ids": ["source:test"],
        "structure_refs": [],
        "provenance": provenance("source_backed", "accepted"),
        "metadata": { "require_verification": true }
    });
    let space = base_space(vec![requirement], vec![]);

    let report = check_space(&space, "technical_advisory_mvp", None, None).unwrap();

    assert_obstruction(&report.result, "requirement_unverified");
}

#[test]
fn accepted_inferred_action_emits_insufficient_evidence() {
    let action = json!({
        "id": "cell:inferred-action",
        "cell_type": "action",
        "title": "Inferred action",
        "summary": null,
        "context_ids": [],
        "source_ids": [],
        "structure_refs": [],
        "provenance": provenance("inferred", "accepted"),
        "metadata": {}
    });
    let space = base_space(vec![action], vec![]);

    let report = check_space(&space, "technical_advisory_mvp", None, None).unwrap();

    assert_obstruction(&report.result, "insufficient_evidence");
}

#[test]
fn blocker_resolution_excludes_rejected_candidates_from_application_requirements() {
    let blockers = vec![json!({
        "id": "obstruction:missing-owner",
        "blocked_ids": ["cell:ship-action"]
    })];
    let candidates = vec![json!({
        "id": "candidate:missing-owner-owner",
        "candidate_type": "ownership_clarification",
        "review_status": "rejected",
        "resolves_obstruction_ids": ["obstruction:missing-owner"]
    })];

    let state = blocker_resolution_state(&blockers, &candidates);

    assert_eq!(state[0]["resolution_status"], "all_candidates_rejected");
    assert!(state[0]["application_requirements"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn blocker_resolution_describes_accepted_candidate_application_contract() {
    let blockers = vec![json!({
        "id": "obstruction:missing-verification",
        "blocked_ids": ["cell:requirement"]
    })];
    let candidates = vec![json!({
        "id": "candidate:missing-verification-test",
        "candidate_type": "proposed_test",
        "review_status": "accepted",
        "resolves_obstruction_ids": ["obstruction:missing-verification"]
    })];

    let state = blocker_resolution_state(&blockers, &candidates);
    let requirement = &state[0]["application_requirements"][0];

    assert_eq!(
        state[0]["resolution_status"],
        "accepted_candidate_pending_application"
    );
    assert_eq!(
        requirement["required_cell_types"],
        json!(["test_or_verification"])
    );
    assert_eq!(requirement["required_relation_types"], json!(["verifies"]));
}

#[test]
fn no_candidate_frontier_preserves_obstruction_completion_hints() {
    let blockers = vec![json!({
        "id": "obstruction:missing-owner",
        "obstruction_type": "missing_owner",
        "severity": "medium",
        "blocked_ids": ["cell:ship-action"],
        "recommended_completion_types": ["ownership_clarification"]
    })];
    let state = blocker_resolution_state(&blockers, &[]);
    let frontier = frontier_items(&state);

    assert_eq!(state[0]["resolution_status"], "no_candidate");
    assert_eq!(frontier[0]["item_type"], "propose_completion_candidate");
    assert_eq!(frontier[0]["blocked_ids"], json!(["cell:ship-action"]));
    assert_eq!(
        frontier[0]["recommended_completion_types"],
        json!(["ownership_clarification"])
    );
}

#[test]
fn waiting_items_preserve_rejected_candidate_completion_hints() {
    let blockers = vec![json!({
        "id": "obstruction:missing-owner",
        "obstruction_type": "missing_owner",
        "severity": "medium",
        "blocked_ids": ["cell:ship-action"],
        "recommended_completion_types": ["ownership_clarification"]
    })];
    let candidates = vec![json!({
        "id": "candidate:missing-owner-owner",
        "candidate_type": "ownership_clarification",
        "review_status": "rejected",
        "resolves_obstruction_ids": ["obstruction:missing-owner"]
    })];
    let state = blocker_resolution_state(&blockers, &candidates);
    let waiting = waiting_items(&state);

    assert_eq!(state[0]["resolution_status"], "all_candidates_rejected");
    assert_eq!(waiting[0]["item_type"], "all_candidates_rejected");
    assert_eq!(
        waiting[0]["candidate_ids"],
        json!(["candidate:missing-owner-owner"])
    );
    assert_eq!(
        waiting[0]["recommended_completion_types"],
        json!(["ownership_clarification"])
    );
}

fn assert_obstruction(result: &Value, obstruction_type: &str) {
    let obstructions = result["obstructions"].as_array().unwrap();
    assert!(
        obstructions
            .iter()
            .any(|item| item["obstruction_type"] == obstruction_type),
        "expected obstruction_type {obstruction_type}, got {obstructions:#?}"
    );
}

fn base_space(cells: Vec<Value>, incidences: Vec<Value>) -> AdvisorySpaceEnvelope {
    AdvisorySpaceEnvelope {
        schema: "advisorygraphen.space.v1".to_string(),
        space_id: "space:test".to_string(),
        engagement_id: "engagement:test".to_string(),
        snapshot_id: "snapshot:test".to_string(),
        package_id: "package:technical_advisory_mvp".to_string(),
        cells,
        contexts: vec![],
        incidences,
        morphisms: vec![],
        invariants: vec![],
        policies: vec![],
        metadata: serde_json::Map::new(),
    }
}

fn action_cell(id: &str) -> Value {
    json!({
        "id": id,
        "cell_type": "action",
        "title": "Ship action",
        "summary": null,
        "context_ids": [],
        "source_ids": ["source:test"],
        "structure_refs": [],
        "provenance": provenance("source_backed", "accepted"),
        "metadata": {}
    })
}

fn provenance(origin: &str, review_status: &str) -> Value {
    json!({
        "origin": origin,
        "actor": "tester",
        "confidence": 1.0,
        "review_status": review_status
    })
}
