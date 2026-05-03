use std::fs;
use std::path::Path;

mod support;
use support::*;

const FIXTURE: &str = "examples/technical-advisory/direct-db-access/advisory.input.json";
const DOGFOOD_FIXTURE: &str = "examples/dogfood/higher-graphen-integration/advisory.input.json";
const PACKAGE_NAME: &str = "technical_advisory";
const RULESET: &str = "technical_advisory_mvp";
const SPACE_ID: &str = "space:advisory:technical-advisory-direct-db-access";
const REVISION_ID: &str = "revision:technical-advisory-smoke-1";

#[test]
fn version_command_reports_planned_cli_version() {
    let dashed = run_cli(["--version"]);
    assert_success(&dashed);
    assert_output_contains(&dashed, BINARY);
    assert_output_contains(&dashed, "0.1.0");

    let subcommand = run_cli(["version"]);
    assert_success(&subcommand);
    assert_output_contains(&subcommand, "0.1.0");
}

#[test]
fn validate_accepts_direct_db_access_fixture() {
    let output = run_cli(["validate", "--input", FIXTURE, "--format", "json"]);
    assert_success(&output);
    assert_output_contains(&output, "advisorygraphen");
}

#[test]
fn code_repo_snapshot_extracts_nextjs_route_signals() {
    let dir = clean_case_dir("code-repo-snapshot");
    let repo = dir.join("repo");
    let route_dir = repo.join("src/app/api/upload");
    let test_dir = repo.join("__tests__");
    fs::create_dir_all(&route_dir).unwrap();
    fs::create_dir_all(&test_dir).unwrap();
    fs::write(
        repo.join("package.json"),
        r#"{"scripts":{"test":"vitest"},"dependencies":{"next":"latest"}}"#,
    )
    .unwrap();
    fs::write(
        route_dir.join("route.ts"),
        r#"
export async function POST(req: Request) {
  const user = await auth();
  await prisma.upload.create({ data: { ownerId: user.id } });
  return Response.json({ bucket: process.env.S3_BUCKET });
}
"#,
    )
    .unwrap();
    fs::write(
        test_dir.join("upload.test.ts"),
        "it('uploads', async () => expect(true).toBe(true));",
    )
    .unwrap();

    let snapshot = dir.join("code.snapshot.json");
    let space = dir.join("code.space.json");
    let check = dir.join("code.check.json");
    let output = run_cli([
        "code",
        "repo-snapshot",
        "--repo",
        path_str(&repo),
        "--output",
        path_str(&snapshot),
        "--format",
        "json",
    ]);
    assert_success(&output);
    assert_file_contains(&snapshot, "code_repo_snapshot:0.1.0");
    assert_file_contains(&snapshot, "record:api-route-src-app-api-upload-route-ts-");
    assert_file_contains(&snapshot, r#""auth_detected": true"#);
    assert_file_contains(&snapshot, r#""db_access_detected": true"#);
    assert_file_contains(&snapshot, "record:env-s3-bucket-src-app-api-upload-route-ts-");
    assert_file_contains(&snapshot, r#""api_route_files": 1"#);
    assert_file_contains(&snapshot, r#""test_files": 1"#);

    let validate = run_cli([
        "validate",
        "--input",
        path_str(&snapshot),
        "--format",
        "json",
    ]);
    assert_success(&validate);

    let lift = run_cli([
        "lift",
        "--input",
        path_str(&snapshot),
        "--package",
        PACKAGE_NAME,
        "--output",
        path_str(&space),
        "--format",
        "json",
    ]);
    assert_success(&lift);
    assert_file_contains(&space, "cell:api-route-src-app-api-upload-route-ts-");
    assert_file_contains(&space, "accesses-application-database");

    check_space(&space, &check);
    assert_file_contains(
        &check,
        "s3-bucket-src-app-api-upload-route-ts",
    );
}

#[test]
fn dogfood_fixture_surfaces_higher_graphen_runtime_followups() {
    let dir = clean_case_dir("dogfood-higher-graphen");
    let generated = dir.join("generated.advisory.input.json");
    let space = dir.join("advisory.space.json");
    let check = dir.join("advisory.check.report.json");
    let audit = dir.join("audit-trace.json");
    let executive = dir.join("executive-review.md");
    let store = dir.join("store");

    let validate = run_cli(["validate", "--input", DOGFOOD_FIXTURE, "--format", "json"]);
    assert_success(&validate);

    let generate = run_cli([
        "dogfood",
        "repo-snapshot",
        "--repo",
        ".",
        "--output",
        path_str(&generated),
        "--format",
        "json",
    ]);
    assert_success(&generate);
    assert_file_contains(&generated, "repo_snapshot:0.1.0");
    assert_file_contains(&generated, "source:workspace-manifest");
    assert_file_contains(&generated, "source:cli-contract");
    assert_file_contains(&generated, "source:reviewable-completions-adr");

    let validate_generated = run_cli([
        "validate",
        "--input",
        path_str(&generated),
        "--format",
        "json",
    ]);
    assert_success(&validate_generated);

    let lift = run_cli([
        "lift",
        "--input",
        path_str(&generated),
        "--package",
        PACKAGE_NAME,
        "--output",
        path_str(&space),
        "--format",
        "json",
    ]);
    assert_success(&lift);
    assert_file_contains(&space, "space:advisory:dogfood-higher-graphen-integration");
    assert_file_contains(&space, "higher_graphen_interpretation");
    assert_file_contains(&space, "morphism:source-to-advisory-space");
    assert_file_contains(&space, "\"morphism_type\": \"lift\"");

    check_space(&space, &check);
    assert_file_contains(&check, "higher_graphen");
    assert_file_not_contains(&check, "obstruction:runtime-adoption-action-missing-owner");
    assert_file_not_contains(
        &check,
        "obstruction:runtime-adoption-requirement-missing-verification",
    );
    assert_file_not_contains(
        &check,
        "obstruction:hg-boundary-requirement-missing-verification",
    );

    assert_success(&import_case(&store, &space, "revision:dogfood-hg-1"));

    let reason = run_cli([
        "case",
        "reason",
        "--store",
        path_str(&store),
        "--space-id",
        "space:advisory:dogfood-higher-graphen-integration",
        "--format",
        "json",
    ]);
    assert_success(&reason);
    assert_output_not_contains(&reason, "obstruction:runtime-adoption-action-missing-owner");
    assert_output_contains_any(&reason, &[r#""closeable": true"#, r#""closeable":true"#]);
    assert_output_contains(&reason, r#""blocking_threshold": "medium""#);

    let project = run_cli([
        "project",
        "--space",
        path_str(&space),
        "--report",
        path_str(&check),
        "--audience",
        "audit_trace",
        "--format",
        "json",
        "--output",
        path_str(&audit),
    ]);
    assert_success(&project);
    assert_file_contains(&audit, "projection:higher:audit_trace");
    assert_file_contains(&audit, r#""obstructions": []"#);
    assert_file_contains(
        &audit,
        "Git history, issue tracker, pull request comments, CI run history, and the HigherGraphen workspace source body were not ingested.",
    );

    let executive_project = run_cli([
        "project",
        "--space",
        path_str(&space),
        "--report",
        path_str(&check),
        "--audience",
        "executive",
        "--format",
        "markdown",
        "--output",
        path_str(&executive),
    ]);
    assert_success(&executive_project);
    assert_file_contains(&executive, "Closeable: `true`");
    assert_file_contains(&executive, "medium: 0");
    assert_file_contains(&executive, "Included sources: 9");
}

#[test]
fn advanced_dogfood_fixtures_cover_multiple_self_review_domains() {
    assert_advanced_dogfood_fixture_flows(PACKAGE_NAME, RULESET);
}

#[test]
fn direct_fixture_lift_check_completions_and_executive_projection() {
    let dir = clean_case_dir("direct-flow");
    let space = dir.join("advisory.space.json");
    let check = dir.join("advisory.check.report.json");
    let completions = dir.join("advisory.completions.report.json");
    let executive = dir.join("executive-review.md");
    let executive_json = dir.join("executive-review.json");
    let review_store = dir.join("review-store");

    lift_fixture(&space);
    assert_file_contains(&space, SPACE_ID);
    assert_file_contains(&space, "Order Service");
    assert_file_contains(&space, "Billing DB");
    assert_file_contains(&space, "context:orders");
    assert_file_contains(&space, "context:billing");
    assert_file_contains(&space, "incidence:order-service-accesses-billing-db");

    check_space(&space, &check);
    assert_file_contains(
        &check,
        "architecture_no_cross_context_direct_database_access",
    );
    assert_file_contains(&check, "violated");
    assert_file_contains(&check, "obstruction:order-service-direct-billing-db-access");
    assert_file_contains(&check, "higher_graphen");
    assert_file_contains(&check, r#""materialized": true"#);

    propose_completions(&space, &check, &completions);
    assert_file_contains(&completions, "candidate:billing-status-api");
    assert_file_contains(&completions, r#""specificity": "source_derived""#);
    assert_file_contains(&completions, r#""precision_note""#);
    assert_file_contains(&completions, "source:architecture-note");
    assert_file_contains(&completions, "unreviewed");
    assert_file_contains(&completions, "higher_graphen");
    assert_file_contains(&completions, "\"missing_type\": \"cell\"");

    assert_success(&import_case(&review_store, &space, REVISION_ID));

    let accept = run_cli([
        "completions",
        "accept",
        "--store",
        path_str(&review_store),
        "--candidate-id",
        "candidate:billing-status-api",
        "--from-report",
        path_str(&completions),
        "--reviewer",
        "reviewer:cto",
        "--reason",
        "Reviewed dogfood completion path.",
        "--base-revision",
        REVISION_ID,
        "--format",
        "json",
    ]);
    assert_success(&accept);
    assert_output_contains(&accept, "accepted_completion");
    assert_output_contains(&accept, "\"outcome_review_status\": \"accepted\"");

    let output = run_cli([
        "project",
        "--space",
        path_str(&space),
        "--report",
        path_str(&check),
        "--completions-report",
        path_str(&completions),
        "--audience",
        "executive",
        "--format",
        "markdown",
        "--output",
        path_str(&executive),
    ]);
    assert_success(&output);
    assert_file_contains(&executive, "Billing DB");
    assert_file_contains(&executive, "Candidate quality");
    assert_file_contains(&executive, "Source-derived: 2");
    assert_file_contains(&executive, "boundary");
    assert_file_contains(&executive, "projection");

    let output = run_cli([
        "project",
        "--space",
        path_str(&space),
        "--report",
        path_str(&check),
        "--completions-report",
        path_str(&completions),
        "--audience",
        "executive",
        "--format",
        "json",
        "--output",
        path_str(&executive_json),
    ]);
    assert_success(&output);
    assert_file_contains(&executive_json, "candidate_quality");
    assert_file_contains(&executive_json, r#""source_derived": 2"#);
    assert_file_contains(&executive_json, r#""source_backed": 2"#);
    assert_file_contains(&executive_json, "higher_graphen");
    assert_file_contains(&executive_json, "projection:higher:executive");
}

#[test]
fn case_import_reason_and_close_check_report_unresolved_obstruction() {
    let dir = clean_case_dir("case-basics");
    let space = dir.join("advisory.space.json");
    let check = dir.join("advisory.check.report.json");
    let completions = dir.join("advisory.completions.report.json");
    let store = dir.join("store");

    lift_fixture(&space);
    check_space(&space, &check);
    propose_completions(&space, &check, &completions);

    let import = import_case(&store, &space, REVISION_ID);
    assert_success(&import);
    assert_output_contains(&import, REVISION_ID);

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
    assert_output_contains(&reason, SPACE_ID);
    assert_output_contains(&reason, "case_head_revision");
    assert_output_contains(&reason, REVISION_ID);
    assert_output_contains(&reason, "blockers");
    assert_output_contains(&reason, "waiting_items");
    assert_output_contains(&reason, "candidate_review_pending");

    let stale_accept = review_billing_candidate(&store, &completions, "stale base should fail", "revision:stale");
    assert_failure_code(&stale_accept, 5);
    assert_output_contains(&stale_accept, "stale revision");

    let close_check = run_cli([
        "case",
        "close-check",
        "--store",
        path_str(&store),
        "--space-id",
        SPACE_ID,
        "--base-revision",
        REVISION_ID,
        "--format",
        "json",
    ]);
    assert_success(&close_check);
    assert_output_contains_any(
        &close_check,
        &[r#""closeable": false"#, r#""closeable":false"#],
    );
    assert_output_contains(
        &close_check,
        "obstruction:order-service-direct-billing-db-access",
    );
    assert_output_contains(&close_check, "incidence:order-service-accesses-billing-db");
    assert_output_contains(&close_check, "source:architecture-note");
    assert_output_not_contains(&close_check, "source:unknown");

    let accept = review_billing_candidate(&store, &completions, "advance case head", REVISION_ID);
    assert_success(&accept);
    let space_head = store
        .join("spaces")
        .join(SPACE_ID.replace([':', '/'], "-"))
        .join("HEAD");
    assert_file_contains(&space_head, "revision:review-000001");

    let stale_second_accept = review_billing_candidate(&store, &completions, "same base should now be stale", REVISION_ID);
    assert_failure_code(&stale_second_accept, 5);
}

fn lift_fixture(output_path: &Path) {
    let output = run_cli([
        "lift",
        "--input",
        FIXTURE,
        "--package",
        PACKAGE_NAME,
        "--output",
        path_str(output_path),
        "--format",
        "json",
    ]);
    assert_success(&output);
}

fn check_space(space_path: &Path, output_path: &Path) {
    let output = run_cli([
        "check",
        "--space",
        path_str(space_path),
        "--ruleset",
        RULESET,
        "--output",
        path_str(output_path),
        "--format",
        "json",
    ]);
    assert_success(&output);
}

fn propose_completions(space_path: &Path, check_path: &Path, output_path: &Path) {
    let output = run_cli([
        "completions",
        "propose",
        "--space",
        path_str(space_path),
        "--from-report",
        path_str(check_path),
        "--output",
        path_str(output_path),
        "--format",
        "json",
    ]);
    assert_success(&output);
}

fn review_billing_candidate(
    store: &Path,
    completions: &Path,
    reason: &str,
    base_revision: &str,
) -> std::process::Output {
    run_cli([
        "completions",
        "accept",
        "--store",
        path_str(store),
        "--candidate-id",
        "candidate:billing-status-api",
        "--from-report",
        path_str(completions),
        "--reviewer",
        "reviewer:cto",
        "--reason",
        reason,
        "--base-revision",
        base_revision,
        "--format",
        "json",
    ])
}
