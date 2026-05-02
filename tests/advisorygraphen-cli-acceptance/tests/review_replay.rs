#[allow(dead_code)]
mod support;
use support::*;

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
    assert_output_contains(&reject, "\"outcome_review_status\": \"rejected\"");

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
}
