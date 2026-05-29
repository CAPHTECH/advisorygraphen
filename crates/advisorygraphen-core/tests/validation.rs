use advisorygraphen_core::{
    validate_document, MICRO_REVIEW_REQUEST_SCHEMA, REVIEW_EVENT_SCHEMA, SNAPSHOT_SCHEMA,
};
use serde_json::{json, Value};

#[test]
fn accepts_valid_micro_review_request() {
    let request = json!({
        "schema": MICRO_REVIEW_REQUEST_SCHEMA,
        "claims": [
            { "text": "The guard test passes.", "classification": "test_backed", "evidence_refs": ["cargo test"] },
            { "text": "It probably regressed.", "classification": "assumption" }
        ]
    });

    let report = validate_document(&request, Some(MICRO_REVIEW_REQUEST_SCHEMA)).unwrap();

    assert!(report.valid);
    assert_eq!(report.document_type, "micro_review_request");
}

#[test]
fn rejects_micro_review_unknown_classification() {
    let request = json!({
        "schema": MICRO_REVIEW_REQUEST_SCHEMA,
        "claims": [ { "text": "x", "classification": "totally_fine" } ]
    });

    let error = validate_document(&request, Some(MICRO_REVIEW_REQUEST_SCHEMA)).unwrap_err();

    assert!(error.to_string().contains("unknown classification"));
}

#[test]
fn rejects_micro_review_empty_claims() {
    let request = json!({ "schema": MICRO_REVIEW_REQUEST_SCHEMA, "claims": [] });

    let error = validate_document(&request, Some(MICRO_REVIEW_REQUEST_SCHEMA)).unwrap_err();

    assert!(error.to_string().contains("`claims` must not be empty"));
}

#[test]
fn rejects_duplicate_snapshot_record_ids() {
    let mut snapshot = minimal_snapshot();
    snapshot["records"] = json!([
        minimal_record("record:duplicate"),
        minimal_record("record:duplicate")
    ]);

    let error = validate_document(&snapshot, Some(SNAPSHOT_SCHEMA)).unwrap_err();

    assert!(error.to_string().contains("duplicate id: record:duplicate"));
}

#[test]
fn rejects_secret_source_ingestion() {
    let mut snapshot = minimal_snapshot();
    snapshot["sources"][0]["classification"] = json!("secret");

    let error = validate_document(&snapshot, Some(SNAPSHOT_SCHEMA)).unwrap_err();

    assert!(error
        .to_string()
        .contains("secret source is not ingestible"));
}

#[test]
fn rejects_inferred_record_promoted_without_review() {
    let mut snapshot = minimal_snapshot();
    snapshot["records"][0]["provenance"]["origin"] = json!("inferred");
    snapshot["records"][0]["provenance"]["review_status"] = json!("accepted");

    let error = validate_document(&snapshot, Some(SNAPSHOT_SCHEMA)).unwrap_err();

    assert!(error
        .to_string()
        .contains("cannot accept inferred provenance without review"));
}

#[test]
fn review_event_requires_reason_and_target() {
    let review = json!({
        "schema": REVIEW_EVENT_SCHEMA,
        "review_event_id": "review:empty",
        "engagement_id": "engagement:test",
        "target_ids": [],
        "outcome": "accepted",
        "reviewer_id": "reviewer:test",
        "reviewed_at": "2026-05-02T00:00:00Z",
        "reason": "",
        "evidence_ids": [],
        "metadata": {}
    });

    let error = validate_document(&review, Some(REVIEW_EVENT_SCHEMA)).unwrap_err();

    assert!(error.to_string().contains("reason is required"));
    assert!(error.to_string().contains("target at least one id"));
}

fn minimal_snapshot() -> Value {
    json!({
        "schema": SNAPSHOT_SCHEMA,
        "snapshot_id": "snapshot:test",
        "engagement_id": "engagement:test",
        "captured_at": "2026-05-02T00:00:00Z",
        "source_boundary": {
            "included_source_ids": ["source:test"],
            "excluded_summary": [],
            "extraction_loss": []
        },
        "sources": [{
            "id": "source:test",
            "source_type": "note",
            "title": "Test source",
            "classification": "public",
            "metadata": {}
        }],
        "records": [minimal_record("record:test")],
        "metadata": {}
    })
}

fn minimal_record(id: &str) -> Value {
    json!({
        "id": id,
        "record_type": "claim",
        "title": "Test record",
        "source_ids": ["source:test"],
        "provenance": {
            "origin": "source_backed",
            "actor": "tester",
            "confidence": 1.0,
            "review_status": "accepted"
        },
        "metadata": {}
    })
}
