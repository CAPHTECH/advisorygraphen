use advisorygraphen_core::AdvisorySpaceEnvelope;
use advisorygraphen_reasoning::check_space;
use serde_json::{json, Value};

#[test]
fn boundary_violation_emits_three_competing_hypotheses_and_falsifiers() {
    let space = boundary_space();

    let report = check_space(&space, "technical_advisory_mvp", None, None).unwrap();
    let hypotheses = report.result["hypotheses"].as_array().unwrap();
    let falsifiers = report.result["falsifiers"].as_array().unwrap();
    let incidences = report.result["argumentation_incidences"]
        .as_array()
        .unwrap();

    assert_eq!(
        hypotheses.len(),
        3,
        "primary + 2 alternatives expected, got {hypotheses:#?}"
    );
    assert!(
        hypotheses
            .iter()
            .all(|hypothesis| hypothesis["lifecycle_status"] == "candidate"
                && hypothesis["cell_type"] == "hypothesis"),
        "all hypotheses begin as candidates"
    );
    assert_eq!(falsifiers.len(), 2, "primary + misclassified falsifiers");
    assert!(falsifiers
        .iter()
        .all(|falsifier| falsifier["cell_type"] == "falsifier"),);
    let primary = hypotheses
        .iter()
        .find(|hypothesis| {
            hypothesis["id"]
                .as_str()
                .unwrap_or("")
                .ends_with("-implicit-interface")
        })
        .expect("primary implicit-interface hypothesis");
    let competes_with = primary["metadata"]["competes_with"].as_array().unwrap();
    assert_eq!(
        competes_with.len(),
        2,
        "primary competes with 2 alternatives"
    );
    assert!(
        incidences
            .iter()
            .any(|incidence| incidence["relation_type"] == "competes_with"),
        "competes_with argumentation incidence emitted"
    );
    assert!(
        incidences
            .iter()
            .any(|incidence| incidence["relation_type"] == "explains"),
        "explains argumentation incidence emitted"
    );
    assert!(
        incidences
            .iter()
            .any(|incidence| incidence["relation_type"] == "falsified_by"),
        "falsified_by argumentation incidence emitted"
    );
}

#[test]
fn api_route_missing_auth_emits_three_competing_hypotheses() {
    let route = json!({
        "id": "cell:api-route-test",
        "cell_type": "component",
        "title": "API route /api/widget",
        "summary": null,
        "context_ids": ["context:application"],
        "source_ids": ["source:route"],
        "structure_refs": [],
        "provenance": provenance(),
        "metadata": {
            "component_type": "api_endpoint",
            "route_path": "/api/widget",
            "http_methods": ["GET"],
            "db_access_detected": true,
            "auth_detected": false,
            "public_endpoint": false
        }
    });
    let space = AdvisorySpaceEnvelope {
        schema: "advisorygraphen.space.v1".to_string(),
        space_id: "space:test-auth".to_string(),
        engagement_id: "engagement:test".to_string(),
        snapshot_id: "snapshot:test".to_string(),
        package_id: "package:technical_advisory_mvp".to_string(),
        cells: vec![route],
        contexts: vec![],
        incidences: vec![],
        morphisms: vec![],
        invariants: vec![],
        policies: vec![],
        metadata: serde_json::Map::new(),
    };

    let report = check_space(&space, "technical_advisory_mvp", None, None).unwrap();
    let hypotheses = report.result["hypotheses"].as_array().unwrap();
    let falsifiers = report.result["falsifiers"].as_array().unwrap();

    assert_eq!(hypotheses.len(), 3);
    assert_eq!(falsifiers.len(), 3);
    assert!(hypotheses
        .iter()
        .any(|h| h["id"].as_str().unwrap().ends_with("-truly-unprotected")));
    assert!(hypotheses.iter().any(|h| h["id"]
        .as_str()
        .unwrap()
        .ends_with("-shared-middleware-auth")));
    assert!(hypotheses
        .iter()
        .any(|h| h["id"].as_str().unwrap().ends_with("-intentionally-public")));
}

#[test]
fn api_route_hypotheses_attach_unreviewed_agent_observation_support() {
    let route = json!({
        "id": "cell:api-route-test",
        "cell_type": "component",
        "title": "API route /api/widget",
        "summary": null,
        "context_ids": ["context:application"],
        "source_ids": ["source:route"],
        "structure_refs": [],
        "provenance": provenance(),
        "metadata": {
            "component_type": "api_endpoint",
            "route_path": "/api/widget",
            "http_methods": ["GET"],
            "db_access_detected": true,
            "auth_detected": false,
            "public_endpoint": false
        }
    });
    let observation = json!({
        "id": "cell:agent-observation-route-auth",
        "cell_type": "claim",
        "title": "Agent observed auth context before database access",
        "summary": "An AI agent read the route and observed a framework auth context check before database access.",
        "context_ids": ["context:application"],
        "source_ids": ["source:route"],
        "structure_refs": [],
        "provenance": {
            "origin": "inferred",
            "actor": "ai-agent",
            "confidence": 0.72,
            "review_status": "unreviewed"
        },
        "metadata": {
            "supports_hypothesis_type": "shared_middleware_auth"
        }
    });
    let support = json!({
        "id": "incidence:agent-observation-supports-route",
        "relation_type": "supports",
        "from_id": "cell:agent-observation-route-auth",
        "to_id": "cell:api-route-test",
        "context_ids": ["context:application"],
        "evidence_ids": ["source:route"],
        "strength": "soft",
        "provenance": {
            "origin": "inferred",
            "actor": "ai-agent",
            "confidence": 0.72,
            "review_status": "unreviewed"
        },
        "metadata": {}
    });
    let space = AdvisorySpaceEnvelope {
        schema: "advisorygraphen.space.v1".to_string(),
        space_id: "space:test-auth".to_string(),
        engagement_id: "engagement:test".to_string(),
        snapshot_id: "snapshot:test".to_string(),
        package_id: "package:technical_advisory_mvp".to_string(),
        cells: vec![route, observation],
        contexts: vec![],
        incidences: vec![support],
        morphisms: vec![],
        invariants: vec![],
        policies: vec![],
        metadata: serde_json::Map::new(),
    };

    let report = check_space(&space, "technical_advisory_mvp", None, None).unwrap();
    let hypotheses = report.result["hypotheses"].as_array().unwrap();
    let shared_middleware = hypotheses
        .iter()
        .find(|h| {
            h["id"]
                .as_str()
                .unwrap()
                .ends_with("-shared-middleware-auth")
        })
        .expect("shared middleware hypothesis");
    assert_eq!(
        shared_middleware["metadata"]["evidence_strength"],
        "agent_observation_unreviewed"
    );
    assert_eq!(
        shared_middleware["metadata"]["supported_by"],
        json!(["cell:agent-observation-route-auth"])
    );
    assert!(report.result["argumentation_incidences"]
        .as_array()
        .unwrap()
        .iter()
        .any(|incidence| incidence["relation_type"] == "supported_by"
            && incidence["to_id"] == "cell:agent-observation-route-auth"));
}

#[test]
fn missing_owner_emits_three_competing_hypotheses() {
    let action = json!({
        "id": "cell:ship-action",
        "cell_type": "action",
        "title": "Ship dashboard widget",
        "summary": null,
        "context_ids": [],
        "source_ids": ["source:test"],
        "structure_refs": [],
        "provenance": provenance(),
        "metadata": {}
    });
    let space = AdvisorySpaceEnvelope {
        schema: "advisorygraphen.space.v1".to_string(),
        space_id: "space:test-owner".to_string(),
        engagement_id: "engagement:test".to_string(),
        snapshot_id: "snapshot:test".to_string(),
        package_id: "package:technical_advisory_mvp".to_string(),
        cells: vec![action],
        contexts: vec![],
        incidences: vec![],
        morphisms: vec![],
        invariants: vec![],
        policies: vec![],
        metadata: serde_json::Map::new(),
    };

    let report = check_space(&space, "technical_advisory_mvp", None, None).unwrap();
    let hypotheses = report.result["hypotheses"].as_array().unwrap();

    assert_eq!(hypotheses.len(), 3);
    assert!(hypotheses
        .iter()
        .any(|h| h["id"].as_str().unwrap().ends_with("-no-team-holds-action")));
    assert!(hypotheses.iter().any(|h| h["id"]
        .as_str()
        .unwrap()
        .ends_with("-de-facto-owner-link-missing")));
    assert!(hypotheses
        .iter()
        .any(|h| h["id"].as_str().unwrap().ends_with("-collective-ownership")));
}

#[test]
fn requirement_unverified_emits_three_competing_hypotheses() {
    let requirement = json!({
        "id": "cell:requirement",
        "cell_type": "requirement",
        "title": "Audit logs must capture user actions",
        "summary": null,
        "context_ids": [],
        "source_ids": ["source:test"],
        "structure_refs": [],
        "provenance": provenance(),
        "metadata": { "require_verification": true }
    });
    let space = AdvisorySpaceEnvelope {
        schema: "advisorygraphen.space.v1".to_string(),
        space_id: "space:test-requirement".to_string(),
        engagement_id: "engagement:test".to_string(),
        snapshot_id: "snapshot:test".to_string(),
        package_id: "package:technical_advisory_mvp".to_string(),
        cells: vec![requirement],
        contexts: vec![],
        incidences: vec![],
        morphisms: vec![],
        invariants: vec![],
        policies: vec![],
        metadata: serde_json::Map::new(),
    };

    let report = check_space(&space, "technical_advisory_mvp", None, None).unwrap();
    let hypotheses = report.result["hypotheses"].as_array().unwrap();

    assert_eq!(hypotheses.len(), 3);
    assert!(hypotheses.iter().any(|h| h["id"]
        .as_str()
        .unwrap()
        .ends_with("-verification-genuinely-missing")));
    assert!(hypotheses.iter().any(|h| h["id"]
        .as_str()
        .unwrap()
        .ends_with("-test-exists-link-missing")));
    assert!(hypotheses.iter().any(|h| h["id"]
        .as_str()
        .unwrap()
        .ends_with("-requirement-is-exploratory")));
}

#[test]
fn insufficient_evidence_emits_three_competing_hypotheses() {
    let action = json!({
        "id": "cell:inferred-action",
        "cell_type": "action",
        "title": "Inferred recommendation",
        "summary": null,
        "context_ids": [],
        "source_ids": [],
        "structure_refs": [],
        "provenance": {
            "origin": "inferred",
            "actor": "tester",
            "confidence": 1.0,
            "review_status": "accepted"
        },
        "metadata": {}
    });
    let space = AdvisorySpaceEnvelope {
        schema: "advisorygraphen.space.v1".to_string(),
        space_id: "space:test-evidence".to_string(),
        engagement_id: "engagement:test".to_string(),
        snapshot_id: "snapshot:test".to_string(),
        package_id: "package:technical_advisory_mvp".to_string(),
        cells: vec![action],
        contexts: vec![],
        incidences: vec![],
        morphisms: vec![],
        invariants: vec![],
        policies: vec![],
        metadata: serde_json::Map::new(),
    };

    let report = check_space(&space, "technical_advisory_mvp", None, None).unwrap();
    let hypotheses = report.result["hypotheses"].as_array().unwrap();
    let evidence_hypotheses: Vec<&Value> = hypotheses
        .iter()
        .filter(|h| {
            h["id"]
                .as_str()
                .unwrap_or("")
                .contains("insufficient-evidence")
        })
        .collect();

    assert_eq!(evidence_hypotheses.len(), 3);
    assert!(evidence_hypotheses.iter().any(|h| h["id"]
        .as_str()
        .unwrap()
        .ends_with("-source-evidence-genuinely-missing")));
    assert!(evidence_hypotheses.iter().any(|h| h["id"]
        .as_str()
        .unwrap()
        .ends_with("-evidence-exists-link-missing")));
    assert!(evidence_hypotheses.iter().any(|h| h["id"]
        .as_str()
        .unwrap()
        .ends_with("-accepted-as-judgment-call")));
}

#[test]
fn circular_dependency_emits_three_competing_hypotheses() {
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
    let space = AdvisorySpaceEnvelope {
        schema: "advisorygraphen.space.v1".to_string(),
        space_id: "space:test-cycle".to_string(),
        engagement_id: "engagement:test".to_string(),
        snapshot_id: "snapshot:test".to_string(),
        package_id: "package:technical_advisory_mvp".to_string(),
        cells,
        contexts: vec![context("context:platform", "Platform")],
        incidences,
        morphisms: vec![],
        invariants: vec![],
        policies: vec![],
        metadata: serde_json::Map::new(),
    };

    let report = check_space(&space, "technical_advisory_mvp", None, None).unwrap();
    let hypotheses = report.result["hypotheses"].as_array().unwrap();

    assert_eq!(hypotheses.len(), 3);
    assert!(hypotheses
        .iter()
        .any(|h| h["id"].as_str().unwrap().ends_with("-true-runtime-cycle")));
    assert!(hypotheses
        .iter()
        .any(|h| h["id"].as_str().unwrap().ends_with("-edge-misclassified")));
    assert!(hypotheses.iter().any(|h| h["id"]
        .as_str()
        .unwrap()
        .ends_with("-cycle-broken-by-runtime-mechanism")));
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
        "provenance": provenance(),
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
        "provenance": provenance(),
        "metadata": {}
    })
}

#[test]
fn space_without_boundary_obstruction_emits_no_hypotheses() {
    let space = AdvisorySpaceEnvelope {
        schema: "advisorygraphen.space.v1".to_string(),
        space_id: "space:test-empty".to_string(),
        engagement_id: "engagement:test".to_string(),
        snapshot_id: "snapshot:test".to_string(),
        package_id: "package:technical_advisory_mvp".to_string(),
        cells: vec![],
        contexts: vec![],
        incidences: vec![],
        morphisms: vec![],
        invariants: vec![],
        policies: vec![],
        metadata: serde_json::Map::new(),
    };

    let report = check_space(&space, "technical_advisory_mvp", None, None).unwrap();

    assert!(report.result["hypotheses"].as_array().unwrap().is_empty());
    assert!(report.result["falsifiers"].as_array().unwrap().is_empty());
}

fn boundary_space() -> AdvisorySpaceEnvelope {
    AdvisorySpaceEnvelope {
        schema: "advisorygraphen.space.v1".to_string(),
        space_id: "space:test-boundary".to_string(),
        engagement_id: "engagement:test".to_string(),
        snapshot_id: "snapshot:test".to_string(),
        package_id: "package:technical_advisory_mvp".to_string(),
        cells: vec![
            cell(
                "cell:inventory-service",
                "component",
                "Inventory Service",
                "context:inventory",
            ),
            cell(
                "cell:pricing-db",
                "data_store",
                "Pricing DB",
                "context:pricing",
            ),
        ],
        contexts: vec![
            context("context:inventory", "Inventory"),
            context("context:pricing", "Pricing"),
        ],
        incidences: vec![json!({
            "id": "incidence:inventory-service-accesses-pricing-db",
            "relation_type": "accesses",
            "from_id": "cell:inventory-service",
            "to_id": "cell:pricing-db",
            "source_ids": ["source:pricing-note"],
            "evidence_ids": ["source:pricing-note"],
            "provenance": provenance(),
            "metadata": { "access_type": "direct_database_read" }
        })],
        morphisms: vec![],
        invariants: vec![],
        policies: vec![],
        metadata: serde_json::Map::new(),
    }
}

fn cell(id: &str, cell_type: &str, title: &str, context_id: &str) -> Value {
    json!({
        "id": id,
        "cell_type": cell_type,
        "title": title,
        "summary": null,
        "context_ids": [context_id],
        "source_ids": ["source:test"],
        "structure_refs": [],
        "provenance": provenance(),
        "metadata": {}
    })
}

fn context(id: &str, title: &str) -> Value {
    json!({
        "id": id,
        "context_type": "bounded_context",
        "title": title,
        "provenance": provenance(),
        "metadata": {}
    })
}

fn provenance() -> Value {
    json!({
        "origin": "source_backed",
        "actor": "tester",
        "confidence": 1.0,
        "review_status": "accepted"
    })
}
