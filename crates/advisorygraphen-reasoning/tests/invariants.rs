use advisorygraphen_core::AdvisorySpaceEnvelope;
use advisorygraphen_reasoning::check_space;
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
