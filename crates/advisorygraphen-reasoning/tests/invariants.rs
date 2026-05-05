use advisorygraphen_core::AdvisorySpaceEnvelope;
use advisorygraphen_reasoning::{
    blocker_resolution_state, check_space, frontier_items, propose_completions,
    propose_hypothesis_lifecycle, waiting_items,
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
fn boundary_completion_candidates_are_derived_from_witness_cells() {
    let mut space = base_space(
        vec![
            component_cell(
                "cell:inventory-service",
                "Inventory Service",
                "context:inventory",
            ),
            data_store_cell("cell:pricing-db", "Pricing DB", "context:pricing"),
        ],
        vec![json!({
            "id": "incidence:inventory-service-accesses-pricing-db",
            "relation_type": "accesses",
            "from_id": "cell:inventory-service",
            "to_id": "cell:pricing-db",
            "source_ids": ["source:pricing-note"],
            "evidence_ids": ["source:pricing-note"],
            "provenance": provenance("source_backed", "accepted"),
            "metadata": { "access_type": "direct_database_read" }
        })],
    );
    space.contexts = vec![
        context("context:inventory", "Inventory"),
        context("context:pricing", "Pricing"),
    ];

    let check_report = check_space(&space, "technical_advisory_mvp", None, None).unwrap();
    let completion_report =
        propose_completions(&space, &check_report, "check-report.json", None).unwrap();
    let candidates = completion_report.result["completion_candidates"]
        .as_array()
        .unwrap();

    assert!(check_report.result["obstructions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"] == "obstruction:inventory-service-direct-pricing-db-access"));
    assert!(candidates
        .iter()
        .any(|item| item["id"] == "candidate:pricing-status-api"));
    assert!(candidates.iter().any(|item| {
        item["metadata"]["specificity"] == "source_derived"
            && item["source_ids"] == json!(["source:pricing-note"])
    }));
    let status_api = candidates
        .iter()
        .find(|item| item["id"] == "candidate:pricing-status-api")
        .expect("pricing status API candidate");
    assert_eq!(
        status_api["proposal_content"]["scenario"]["scenario_kind"],
        "planned"
    );
    assert_eq!(
        status_api["proposal_content"]["morphism"]["morphism_type"],
        "as_is_to_to_be"
    );
    assert_eq!(
        status_api["proposal_content"]["scenario"]["affected_invariants"],
        json!(["invariant:architecture_no_cross_context_direct_database_access"])
    );
    assert_eq!(
        status_api["proposal_content"]["derivation"]["verification_status"],
        "unverified"
    );
    assert_eq!(
        status_api["proposal_content"]["valuation"]["order_type"],
        "partial_order"
    );
    assert_eq!(
        status_api["proposal_content"]["policy"]["policy_type"],
        "completion_review_gate"
    );
    assert!(status_api["proposal_content"]["content_obstructions"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!candidates
        .iter()
        .any(|item| item["id"] == "candidate:billing-status-api"));
}

#[test]
fn database_touching_api_route_without_auth_emits_obstruction_and_completion() {
    let route = api_route_cell(
        "cell:api-route-src-app-api-public-data-route-ts-abc123",
        "/api/public-data",
        true,
        false,
        false,
    );
    let space = base_space(vec![route], vec![]);

    let check_report = check_space(&space, "technical_advisory_mvp", None, None).unwrap();
    let completion_report =
        propose_completions(&space, &check_report, "check-report.json", None).unwrap();
    let obstructions = check_report.result["obstructions"].as_array().unwrap();
    let candidates = completion_report.result["completion_candidates"]
        .as_array()
        .unwrap();

    assert!(obstructions.iter().any(|item| {
        item["obstruction_type"] == "api_route_missing_auth"
            && item["severity"] == "high"
            && item["metadata"]["specificity"] == "code_derived"
            && item["metadata"]["route_path"] == "/api/public-data"
            && item["evidence_ids"] == json!(["source:route"])
            && item["metadata"].get("confidence").is_none()
    }));
    assert!(candidates.iter().any(|item| {
        item["candidate_type"] == "proposed_auth_guard"
            && item["metadata"]["specificity"] == "code_derived"
            && item["source_ids"] == json!(["source:route"])
    }));
}

#[test]
fn generic_completion_content_records_missing_structure_obstructions() {
    let space = base_space(vec![action_cell("cell:ship-action")], vec![]);
    let check_report = check_space(&space, "technical_advisory_mvp", None, None).unwrap();
    let completion_report =
        propose_completions(&space, &check_report, "check-report.json", None).unwrap();
    let candidate = completion_report.result["completion_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["candidate_type"] == "ownership_clarification")
        .expect("ownership clarification candidate");

    assert_eq!(
        candidate["proposal_content"]["scenario"]["status"],
        "blocked"
    );
    assert!(candidate["proposal_content"]["content_obstructions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["obstruction_type"] == "proposal_content_underspecified"));
    assert!(!candidate["proposal_content"]["content_obstructions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["obstruction_type"] == "proposal_content_missing_source_witness"));
    assert_eq!(candidate["source_ids"], json!(["source:test"]));
    assert!(candidate["proposal_content"]["policy"]["rules"]
        .as_array()
        .unwrap()
        .iter()
        .any(|rule| rule
            .as_str()
            .unwrap()
            .contains("must not accept it as current state")));
}

#[test]
fn owner_completion_uses_related_owner_cell_when_available() {
    let mut action = action_cell("cell:ship-action");
    action["context_ids"] = json!(["context:release"]);
    let owner = owner_cell("cell:release-team", "Release Team", "context:release");
    let space = base_space(vec![action, owner], vec![]);

    let check_report = check_space(&space, "technical_advisory_mvp", None, None).unwrap();
    let completion_report =
        propose_completions(&space, &check_report, "check-report.json", None).unwrap();
    let candidate = completion_report.result["completion_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["candidate_type"] == "owner_assignment")
        .expect("owner assignment candidate");

    assert_eq!(candidate["metadata"]["owner_cell_id"], "cell:release-team");
    assert_eq!(
        candidate["proposal_content"]["scenario"]["status"],
        "candidate"
    );
    assert!(candidate["proposed_incidence_ids"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id.as_str().unwrap().contains("-owns-")));
    assert!(candidate["proposal_content"]["content_obstructions"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn verification_completion_links_related_test_when_available() {
    let mut requirement = json!({
        "id": "cell:requirement",
        "cell_type": "requirement",
        "title": "Requirement",
        "summary": null,
        "context_ids": ["context:release"],
        "source_ids": ["source:test"],
        "structure_refs": [],
        "provenance": provenance("source_backed", "accepted"),
        "metadata": { "require_verification": true }
    });
    requirement["context_ids"] = json!(["context:release"]);
    let verification = verification_cell(
        "cell:release-smoke-test",
        "Release smoke test",
        "context:release",
    );
    let space = base_space(vec![requirement, verification], vec![]);

    let check_report = check_space(&space, "technical_advisory_mvp", None, None).unwrap();
    let completion_report =
        propose_completions(&space, &check_report, "check-report.json", None).unwrap();
    let candidate = completion_report.result["completion_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["candidate_type"] == "lift_verification_link")
        .expect("verification link candidate");

    assert_eq!(
        candidate["metadata"]["verification_cell_id"],
        "cell:release-smoke-test"
    );
    assert_eq!(
        candidate["proposal_content"]["scenario"]["status"],
        "candidate"
    );
    assert!(candidate["proposed_incidence_ids"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id.as_str().unwrap().contains("-verifies-")));
    assert!(candidate["proposal_content"]["content_obstructions"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn explicitly_public_database_route_does_not_emit_auth_obstruction() {
    let route = api_route_cell(
        "cell:api-route-src-app-api-public-feed-route-ts-abc123",
        "/api/public-feed",
        true,
        false,
        true,
    );
    let space = base_space(vec![route], vec![]);

    let report = check_space(&space, "technical_advisory_mvp", None, None).unwrap();

    assert!(!report.result["obstructions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["obstruction_type"] == "api_route_missing_auth"));
}

#[test]
fn inferred_public_database_route_still_requires_reviewed_resolution() {
    let mut route = api_route_cell(
        "cell:api-route-src-app-api-public-feed-route-ts-abc123",
        "/api/public-feed",
        true,
        false,
        true,
    );
    route["provenance"] = provenance("inferred", "unreviewed");
    let space = base_space(vec![route], vec![]);

    let report = check_space(&space, "technical_advisory_mvp", None, None).unwrap();

    assert!(report.result["obstructions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["obstruction_type"] == "api_route_missing_auth"));
}

#[test]
fn directed_dependency_cycle_emits_circular_dependency_obstruction() {
    let cells = vec![
        component_cell("cell:service-a", "Service A", "context:platform"),
        component_cell("cell:service-b", "Service B", "context:platform"),
        component_cell("cell:service-c", "Service C", "context:platform"),
    ];
    let incidences = vec![
        depends_on_incidence("incidence:a-b", "cell:service-a", "cell:service-b"),
        depends_on_incidence("incidence:b-c", "cell:service-b", "cell:service-c"),
        depends_on_incidence("incidence:c-a", "cell:service-c", "cell:service-a"),
    ];
    let space = base_space(cells, incidences);

    let report = check_space(&space, "technical_advisory_mvp", None, None).unwrap();
    let obstructions = report.result["obstructions"].as_array().unwrap();
    let cycle = obstructions
        .iter()
        .find(|item| item["obstruction_type"] == "circular_dependency")
        .expect("circular_dependency obstruction emitted");

    assert_eq!(cycle["severity"], "medium");
    assert_eq!(cycle["metadata"]["specificity"], "topology_derived");
    let participants = cycle["metadata"]["cycle_cell_ids"].as_array().unwrap();
    assert_eq!(participants.len(), 3);
}

#[test]
fn dag_dependencies_do_not_emit_cycle_obstruction() {
    let cells = vec![
        component_cell("cell:service-a", "Service A", "context:platform"),
        component_cell("cell:service-b", "Service B", "context:platform"),
    ];
    let incidences = vec![depends_on_incidence(
        "incidence:a-b",
        "cell:service-a",
        "cell:service-b",
    )];
    let space = base_space(cells, incidences);

    let report = check_space(&space, "technical_advisory_mvp", None, None).unwrap();

    assert!(report.result["obstructions"]
        .as_array()
        .unwrap()
        .iter()
        .all(|item| item["obstruction_type"] != "circular_dependency"));
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

#[test]
fn hypothesis_lifecycle_proposal_uses_agent_observation_without_applying_event() {
    let action = action_cell("cell:ship-action");
    let check_space_without_observation = base_space(vec![action.clone()], vec![]);
    let check_report = check_space(
        &check_space_without_observation,
        "technical_advisory_mvp",
        None,
        None,
    )
    .unwrap();
    let hypothesis_id = "hypothesis:ship-action-missing-owner-no-team-holds-action";
    let observation = json!({
        "id": "cell:owner-observation",
        "cell_type": "evidence",
        "title": "Agent observed no owner metadata",
        "summary": "An AI agent inspected the action record and found no owner cell or owns incidence.",
        "context_ids": [],
        "source_ids": [],
        "structure_refs": [],
        "provenance": provenance("inferred", "unreviewed"),
        "metadata": {
            "supports_hypothesis_id": hypothesis_id
        }
    });
    let space_with_observation = base_space(vec![action, observation], vec![]);

    let proposal = propose_hypothesis_lifecycle(
        &space_with_observation,
        &check_report,
        "check-report.json",
        None,
    )
    .unwrap();
    let proposals = proposal.result["lifecycle_proposals"].as_array().unwrap();

    assert_eq!(proposal.report_type, "hypothesis_lifecycle_proposal");
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0]["target_hypothesis_id"], hypothesis_id);
    assert_eq!(proposals[0]["proposed_outcome"], "supported");
    assert_eq!(proposals[0]["review_status"], "unreviewed");
    assert_eq!(
        proposal.result["authority_boundary"]["may_apply_events"],
        json!(false)
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

fn depends_on_incidence(id: &str, from: &str, to: &str) -> Value {
    json!({
        "id": id,
        "relation_type": "depends_on",
        "from_id": from,
        "to_id": to,
        "context_ids": [],
        "evidence_ids": [],
        "strength": "hard",
        "provenance": provenance("source_backed", "accepted"),
        "metadata": {}
    })
}

fn component_cell(id: &str, title: &str, context_id: &str) -> Value {
    json!({
        "id": id,
        "cell_type": "component",
        "title": title,
        "summary": null,
        "context_ids": [context_id],
        "source_ids": ["source:test"],
        "structure_refs": [],
        "provenance": provenance("source_backed", "accepted"),
        "metadata": {}
    })
}

fn owner_cell(id: &str, title: &str, context_id: &str) -> Value {
    json!({
        "id": id,
        "cell_type": "owner",
        "title": title,
        "summary": null,
        "context_ids": [context_id],
        "source_ids": ["source:test"],
        "structure_refs": [],
        "provenance": provenance("source_backed", "accepted"),
        "metadata": {}
    })
}

fn verification_cell(id: &str, title: &str, context_id: &str) -> Value {
    json!({
        "id": id,
        "cell_type": "test_or_verification",
        "title": title,
        "summary": null,
        "context_ids": [context_id],
        "source_ids": ["source:test"],
        "structure_refs": [],
        "provenance": provenance("source_backed", "accepted"),
        "metadata": {}
    })
}

fn api_route_cell(
    id: &str,
    route_path: &str,
    db_access_detected: bool,
    auth_detected: bool,
    public_endpoint: bool,
) -> Value {
    json!({
        "id": id,
        "cell_type": "component",
        "title": format!("API route {route_path}"),
        "summary": null,
        "context_ids": ["context:application"],
        "source_ids": ["source:route"],
        "structure_refs": [],
        "provenance": provenance("source_backed", "accepted"),
        "metadata": {
            "component_type": "api_endpoint",
            "route_path": route_path,
            "http_methods": ["GET"],
            "db_access_detected": db_access_detected,
            "auth_detected": auth_detected,
            "public_endpoint": public_endpoint
        }
    })
}

fn data_store_cell(id: &str, title: &str, context_id: &str) -> Value {
    json!({
        "id": id,
        "cell_type": "data_store",
        "title": title,
        "summary": null,
        "context_ids": [context_id],
        "source_ids": ["source:test"],
        "structure_refs": [],
        "provenance": provenance("source_backed", "accepted"),
        "metadata": {}
    })
}

fn context(id: &str, title: &str) -> Value {
    json!({
        "id": id,
        "context_type": "bounded_context",
        "title": title,
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
