#[allow(dead_code)]
mod support;
use support::*;
use std::fs;

const FIXTURE: &str = "examples/dogfood/product-governance/advisory.input.json";
const PACKAGE_NAME: &str = "technical_advisory";
const RULESET: &str = "technical_advisory_mvp";
const SPACE_ID: &str = "space:advisory:dogfood-product-governance";
const REVISION_ID: &str = "revision:dogfood-product-governance-reject";
const OWNER_CANDIDATE: &str = "candidate:enterprise-packaging-action-missing-owner-owner";

#[test]
fn rejected_candidate_survives_case_replay() {
    let dir = clean_case_dir("reject-review-replay");
    let space = dir.join("advisory.space.json");
    let check = dir.join("advisory.check.report.json");
    let completions = dir.join("advisory.completions.report.json");
    let store = dir.join("store");

    assert_success(&run_cli(["validate", "--input", FIXTURE, "--format", "json"]));
    assert_success(&run_cli([
        "lift",
        "--input",
        FIXTURE,
        "--package",
        PACKAGE_NAME,
        "--output",
        path_str(&space),
        "--format",
        "json",
    ]));
    assert_success(&run_cli([
        "check",
        "--space",
        path_str(&space),
        "--ruleset",
        RULESET,
        "--output",
        path_str(&check),
        "--format",
        "json",
    ]));
    assert_success(&run_cli([
        "completions",
        "propose",
        "--space",
        path_str(&space),
        "--from-report",
        path_str(&check),
        "--output",
        path_str(&completions),
        "--format",
        "json",
    ]));

    let unimported_store = dir.join("unimported-store");
    let unimported_reject = run_cli([
        "completions",
        "reject",
        "--store",
        path_str(&unimported_store),
        "--candidate-id",
        OWNER_CANDIDATE,
        "--from-report",
        path_str(&completions),
        "--reviewer",
        "reviewer:dogfood-agent",
        "--reason",
        "Reject before import should fail.",
        "--base-revision",
        REVISION_ID,
        "--format",
        "json",
    ]);
    assert_failure_code(&unimported_reject, 1);
    assert_output_contains(&unimported_reject, "must be imported before review");

    assert_success(&run_cli([
        "case",
        "import",
        "--store",
        path_str(&store),
        "--space",
        path_str(&space),
        "--revision-id",
        REVISION_ID,
        "--format",
        "json",
    ]));

    let missing_report_reject = run_cli([
        "completions",
        "reject",
        "--store",
        path_str(&store),
        "--candidate-id",
        OWNER_CANDIDATE,
        "--reviewer",
        "reviewer:dogfood-agent",
        "--reason",
        "Reject without from-report should fail.",
        "--base-revision",
        REVISION_ID,
        "--format",
        "json",
    ]);
    assert_failure_code(&missing_report_reject, 1);
    assert_output_contains(&missing_report_reject, "from-report is required");

    let missing_base_reject = run_cli([
        "completions",
        "reject",
        "--store",
        path_str(&store),
        "--candidate-id",
        OWNER_CANDIDATE,
        "--from-report",
        path_str(&completions),
        "--reviewer",
        "reviewer:dogfood-agent",
        "--reason",
        "Reject without base revision should fail.",
        "--format",
        "json",
    ]);
    assert_failure_code(&missing_base_reject, 5);
    assert_output_contains(&missing_base_reject, "stale revision");
    assert_output_contains(&missing_base_reject, "<missing>");

    let tampered_completions = dir.join("tampered.completions.report.json");
    let mut tampered: serde_json::Value =
        serde_json::from_slice(&fs::read(&completions).expect("completion report should exist"))
            .expect("completion report should be valid json");
    tampered["input"]["space_id"] = serde_json::json!("space:advisory:wrong-space");
    fs::write(
        &tampered_completions,
        serde_json::to_vec_pretty(&tampered).expect("tampered report should serialize"),
    )
    .expect("tampered report should be writable");
    let mismatched_space_reject = run_cli([
        "completions",
        "reject",
        "--store",
        path_str(&store),
        "--candidate-id",
        OWNER_CANDIDATE,
        "--from-report",
        path_str(&tampered_completions),
        "--reviewer",
        "reviewer:dogfood-agent",
        "--reason",
        "Reject with mismatched report space should fail.",
        "--base-revision",
        REVISION_ID,
        "--format",
        "json",
    ]);
    assert_failure_code(&mismatched_space_reject, 1);
    assert_output_contains(&mismatched_space_reject, "does not match");
    assert_output_contains(&mismatched_space_reject, "higher_graphen.space_id");

    let missing_space_completions = dir.join("missing-space.completions.report.json");
    let mut missing_space: serde_json::Value =
        serde_json::from_slice(&fs::read(&completions).expect("completion report should exist"))
            .expect("completion report should be valid json");
    missing_space["input"]
        .as_object_mut()
        .expect("completion report input should be an object")
        .remove("space_id");
    fs::write(
        &missing_space_completions,
        serde_json::to_vec_pretty(&missing_space)
            .expect("missing-space report should serialize"),
    )
    .expect("missing-space report should be writable");
    let missing_space_reject = run_cli([
        "completions",
        "reject",
        "--store",
        path_str(&store),
        "--candidate-id",
        OWNER_CANDIDATE,
        "--from-report",
        path_str(&missing_space_completions),
        "--reviewer",
        "reviewer:dogfood-agent",
        "--reason",
        "Reject with missing report space should fail.",
        "--base-revision",
        REVISION_ID,
        "--format",
        "json",
    ]);
    assert_failure_code(&missing_space_reject, 1);
    assert_output_contains(&missing_space_reject, "input.space_id");

    let reject = run_cli([
        "completions",
        "reject",
        "--store",
        path_str(&store),
        "--candidate-id",
        OWNER_CANDIDATE,
        "--from-report",
        path_str(&completions),
        "--reviewer",
        "reviewer:dogfood-agent",
        "--reason",
        "Reject owner candidate during replay test.",
        "--base-revision",
        REVISION_ID,
        "--format",
        "json",
    ]);
    assert_success(&reject);
    assert_output_contains(&reject, r#""engagement_id": "engagement:advisorygraphen-self-review""#);
    assert_output_contains(&reject, "\"outcome_review_status\": \"rejected\"");
    assert_output_contains(
        &reject,
        "review:rejected:enterprise-packaging-action-missing-owner-owner-000001",
    );

    let reason = run_cli([
        "case",
        "reason",
        "--store",
        path_str(&store),
        "--space-id",
        SPACE_ID,
        "--format",
        "json",
    ]);
    assert_success(&reason);
    assert_output_contains(&reason, OWNER_CANDIDATE);
    assert_output_contains(&reason, r#""review_status": "rejected""#);
    assert_output_contains(&reason, "all_candidates_rejected");
    assert_output_contains(&reason, r#""application_requirements": []"#);

    let accept = run_cli([
        "completions",
        "accept",
        "--store",
        path_str(&store),
        "--candidate-id",
        OWNER_CANDIDATE,
        "--from-report",
        path_str(&completions),
        "--reviewer",
        "reviewer:dogfood-agent",
        "--reason",
        "Accept owner candidate after rejection.",
        "--base-revision",
        "revision:review-000001",
        "--format",
        "json",
    ]);
    assert_success(&accept);
    assert_output_contains(&accept, r#""engagement_id": "engagement:advisorygraphen-self-review""#);
    assert_output_contains(&accept, "\"outcome_review_status\": \"accepted\"");
    assert_output_contains(
        &accept,
        "review:accepted:enterprise-packaging-action-missing-owner-owner-000002",
    );
    let space_head = store
        .join("spaces")
        .join(SPACE_ID.replace([':', '/'], "-"))
        .join("HEAD");
    assert_file_contains(&space_head, "revision:review-000002");

    let reason_after_accept = run_cli([
        "case",
        "reason",
        "--store",
        path_str(&store),
        "--space-id",
        SPACE_ID,
        "--format",
        "json",
    ]);
    assert_success(&reason_after_accept);
    assert_output_contains(&reason_after_accept, r#""review_status": "accepted""#);
    assert_output_contains(&reason_after_accept, "accepted_candidate_pending_application");
    assert_output_contains(&reason_after_accept, "add owner cell and owns incidence");
}
