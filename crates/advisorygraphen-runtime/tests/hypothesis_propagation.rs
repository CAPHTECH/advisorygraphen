use advisorygraphen_runtime::{
    case_import_workflow, case_reason_workflow, check_workflow,
    hypothesis_apply_proposals_workflow, hypothesis_falsify_workflow, hypothesis_propose_workflow,
    lift_workflow, CaseImportOptions, CaseReasonOptions, CheckOptions,
    HypothesisApplyProposalsOptions, HypothesisFalsifyOptions, HypothesisProposeOptions,
    LiftOptions,
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
