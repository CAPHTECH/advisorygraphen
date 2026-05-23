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
    assert_output_contains(&dashed, "0.1.2");

    let subcommand = run_cli(["version"]);
    assert_success(&subcommand);
    assert_output_contains(&subcommand, "0.1.2");
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
    let public_route_dir = repo.join("src/app/api/public-data");
    let test_dir = repo.join("__tests__");
    fs::create_dir_all(&route_dir).unwrap();
    fs::create_dir_all(&public_route_dir).unwrap();
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
        public_route_dir.join("route.ts"),
        r#"
export async function GET() {
  const rows = await prisma.publicData.findMany();
  return Response.json(rows);
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
    assert_file_contains(&snapshot, r#""auth_detected": false"#);
    assert_file_contains(&snapshot, "record:env-s3-bucket-src-app-api-upload-route-ts-");
    assert_file_contains(&snapshot, r#""api_route_files": 2"#);
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
    assert_file_contains(&check, "api_route_missing_auth");
    assert_file_contains(&check, "public-data");

    let hypothesis_id = first_hypothesis_id(&check);
    let mut space_json: serde_json::Value = serde_json::from_slice(&fs::read(&space).unwrap()).unwrap();
    space_json["cells"].as_array_mut().unwrap().push(serde_json::json!({
        "id": "cell:agent-route-observation",
        "cell_type": "evidence",
        "title": "Agent route observation",
        "summary": "Agent observed a lifecycle signal for the selected hypothesis.",
        "context_ids": [],
        "source_ids": [],
        "structure_refs": [],
        "provenance": {
            "origin": "inferred",
            "actor": "ai-agent:acceptance-test",
            "confidence": 0.62,
            "review_status": "unreviewed"
        },
        "metadata": {
            "supports_hypothesis_id": hypothesis_id
        }
    }));
    let observed_space = dir.join("code.observed.space.json");
    fs::write(&observed_space, serde_json::to_vec_pretty(&space_json).unwrap()).unwrap();
    let lifecycle = dir.join("code.hypothesis-lifecycle.json");
    let lifecycle_output = run_cli([
        "hypothesis",
        "propose",
        "--space",
        path_str(&observed_space),
        "--from-report",
        path_str(&check),
        "--output",
        path_str(&lifecycle),
        "--format",
        "json",
    ]);
    assert_success(&lifecycle_output);
    assert_file_contains(&lifecycle, "hypothesis_lifecycle_proposal");
    assert_file_contains(&lifecycle, r#""proposed_outcome": "supported""#);
    assert_file_contains(&lifecycle, r#""may_apply_events": false"#);
    let lifecycle_store = dir.join("hypothesis-store");
    let hypothesis_import = run_cli([
        "case",
        "import",
        "--store",
        path_str(&lifecycle_store),
        "--space",
        path_str(&observed_space),
        "--revision-id",
        "revision:hypothesis-apply-smoke",
        "--format",
        "json",
    ]);
    assert_success(&hypothesis_import);
    let apply_lifecycle = run_cli([
        "hypothesis",
        "apply-proposals",
        "--store",
        path_str(&lifecycle_store),
        "--from-report",
        path_str(&lifecycle),
        "--reviewer",
        "ai-agent:acceptance-test",
        "--reason",
        "Default conservative policy should skip inferred-only evidence.",
        "--base-revision",
        "revision:hypothesis-apply-smoke",
        "--dry-run",
        "--format",
        "json",
    ]);
    assert_success(&apply_lifecycle);
    assert_output_contains(&apply_lifecycle, r#""applied_count": 0"#);
    assert_output_contains(&apply_lifecycle, "below policy minimum");

    let completions = dir.join("code.completions.json");
    let exec = dir.join("code.executive.json");
    propose_completions(&space, &check, &completions);
    assert_file_contains(&completions, r#""specificity": "code_derived""#);
    let project = run_cli([
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
        path_str(&exec),
    ]);
    assert_success(&project);
    assert_file_contains(&exec, r#""code_derived": 1"#);
    assert_file_contains(&exec, r#""missing_precision_metadata": 0"#);
    assert_file_contains(&exec, "lexical_detection_caveat");
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
fn static_higher_graphen_example_surfaces_0_5_correspondence_and_gluing() {
    let dir = clean_case_dir("dogfood-higher-graphen-0-5-example");
    let space = dir.join("advisory.space.json");
    let check = dir.join("advisory.check.report.json");
    let completions = dir.join("advisory.completions.report.json");
    let dry_run = dir.join("advisory.dry-run.report.json");
    let ai_agent = dir.join("ai-agent.json");

    let validate = run_cli(["validate", "--input", DOGFOOD_FIXTURE, "--format", "json"]);
    assert_success(&validate);

    let lift = run_cli([
        "lift",
        "--input",
        DOGFOOD_FIXTURE,
        "--package",
        PACKAGE_NAME,
        "--output",
        path_str(&space),
        "--format",
        "json",
    ]);
    assert_success(&lift);
    assert_file_contains(&space, "source:hg-0-5-correspondence-adoption");

    check_space(&space, &check);
    assert_file_contains(&check, "obstruction:hg-0-5-correspondence-review-requirement-missing-verification");
    assert_file_contains(&check, "obstruction:hg-0-5-review-policy-action-missing-owner");

    propose_completions(&space, &check, &completions);
    assert_file_contains(
        &completions,
        "candidate:hg-0-5-correspondence-review-requirement-missing-verification-link-runtime-adoption-review-plan",
    );
    assert_file_contains(
        &completions,
        "candidate:hg-0-5-review-policy-action-missing-owner-owner",
    );

    let dry_run_output = run_cli([
        "completions",
        "dry-run",
        "--space",
        path_str(&space),
        "--from-report",
        path_str(&completions),
        "--output",
        path_str(&dry_run),
        "--format",
        "json",
    ]);
    assert_success(&dry_run_output);
    assert_file_contains(&dry_run, "highergraphen_0_5_correspondence_overlap_gluing");
    assert_file_contains(&dry_run, "higher_graphen_gluing_review");
    assert_file_contains(&dry_run, "gluing_failure_requires_explicit_review");
    assert_file_contains(&dry_run, "blocking_difference_requires_revision_or_override");

    let project_agent = run_cli([
        "project",
        "--space",
        path_str(&space),
        "--report",
        path_str(&check),
        "--completions-report",
        path_str(&completions),
        "--audience",
        "ai_agent",
        "--format",
        "json",
        "--output",
        path_str(&ai_agent),
    ]);
    assert_success(&project_agent);
    assert_file_contains(&ai_agent, "correspondence_analysis");
    assert_file_contains(&ai_agent, "highergraphen_0_5_correspondence_overlap_gluing");
    assert_file_contains(&ai_agent, "highergraphen.correspondence.projection.v1");
    assert_file_contains(
        &ai_agent,
        "inspect correspondence_analysis for shared evidence, conflicts, and gluing failures",
    );
}

#[test]
fn adversarial_dogfood_fixture_is_regression_oracle_for_hypothesis_gates() {
    let dir = clean_case_dir("dogfood-adversarial-hypothesis-gates");
    let input = dir.join("adversarial.input.json");
    let space = dir.join("adversarial.space.json");
    let check = dir.join("adversarial.check.json");
    let completions = dir.join("adversarial.completions.json");
    let ai_agent = dir.join("adversarial.ai-agent.json");
    let executive = dir.join("adversarial.executive.md");
    let observation_result = dir.join("observation-result.json");
    let store = dir.join("store");

    let generate = run_cli([
        "dogfood",
        "adversarial-fixture",
        "--output",
        path_str(&input),
        "--format",
        "json",
    ]);
    assert_success(&generate);
    assert_file_contains(&input, "adversarial_fixture:0.1.0");
    assert_file_contains(&input, "expected_recommendation_role");

    let validate = run_cli([
        "validate",
        "--input",
        path_str(&input),
        "--format",
        "json",
    ]);
    assert_success(&validate);

    let lift = run_cli([
        "lift",
        "--input",
        path_str(&input),
        "--package",
        PACKAGE_NAME,
        "--output",
        path_str(&space),
        "--format",
        "json",
    ]);
    assert_success(&lift);
    assert_file_contains(&space, "space:advisory:dogfood-adversarial-hypothesis-gates");
    assert_file_contains(&space, "schema-morphism:engagement-snapshot-to-advisory-space");
    assert_file_contains(&space, "compatible_with_declared_loss");

    check_space(&space, &check);
    assert_file_contains(&check, r#""obstruction_type": "boundary_violation""#);
    assert_file_contains(&check, r#""obstruction_type": "missing_owner""#);
    assert_file_contains(&check, r#""obstruction_type": "requirement_unverified""#);

    propose_completions(&space, &check, &completions);
    assert_file_contains(&completions, r#""recommendation_role": "follow_up_observation""#);
    assert_file_not_contains(&completions, r#""recommendation_role": "primary""#);
    assert_file_contains(&completions, "proposal_depends_on_unsupported_hypothesis");

    let project_agent = run_cli([
        "project",
        "--space",
        path_str(&space),
        "--report",
        path_str(&check),
        "--completions-report",
        path_str(&completions),
        "--audience",
        "ai_agent",
        "--format",
        "json",
        "--output",
        path_str(&ai_agent),
    ]);
    assert_success(&project_agent);
    assert_file_contains(&ai_agent, r#""primary_count": 0"#);
    assert_file_contains(&ai_agent, r#""follow_up_observation_count": 4"#);
    assert_file_contains(&ai_agent, r#""unsupported_hypothesis_candidate_count": 4"#);
    assert_file_contains(&ai_agent, "ranked_observation_tasks");
    assert_file_contains(&ai_agent, "observation_actions");
    assert_file_contains(&ai_agent, "projection_loss_metrics");
    assert_file_contains(&ai_agent, "schema_morphisms");
    assert_file_contains(&ai_agent, "inspect observation_actions before promoting unsupported hypotheses");
    assert_file_contains(&ai_agent, "inspect projection_loss_metrics and schema_morphisms before summarizing");
    assert_file_contains(&ai_agent, "command_template");
    assert_file_contains(&ai_agent, "output_schema");
    assert_file_contains(&ai_agent, "pass_fail_extraction");
    assert_file_contains(&ai_agent, "hypothesis_promotion_workflow");
    assert_file_contains(&ai_agent, "Run the ranked observation tasks");

    let project_executive = run_cli([
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
    assert_success(&project_executive);
    assert_file_contains(&executive, "Primary recommendations: 0");
    assert_file_contains(&executive, "Follow-up observations: 4");
    assert_file_contains(&executive, "Observation 1:");

    assert_success(&import_case(
        &store,
        &space,
        "revision:adversarial-observation-1",
    ));
    fs::write(
        &observation_result,
        serde_json::to_vec_pretty(&serde_json::json!({
            "observation_status": "supports",
            "evidence_ids": ["source:adversarial-governance-note"],
            "summary": "A concrete verification method can be defined for the fixture requirement.",
            "supports_hypothesis": true,
            "falsifies_hypothesis": false,
            "review_note": "Acceptance test fixture observation."
        }))
        .unwrap(),
    )
    .unwrap();
    let record = run_cli([
        "observation",
        "record",
        "--store",
        path_str(&store),
        "--space-id",
        "space:advisory:dogfood-adversarial-hypothesis-gates",
        "--from-projection",
        path_str(&ai_agent),
        "--task-id",
        "observation:agent-output-verification-requirement-missing-verification-verification:support-1",
        "--result",
        path_str(&observation_result),
        "--reviewer",
        "ai-agent:acceptance-test",
        "--reason",
        "Record source-backed observation result.",
        "--base-revision",
        "revision:adversarial-observation-1",
        "--format",
        "json",
    ]);
    assert_success(&record);
    assert_output_contains(&record, r#""report_type": "observation_record""#);
    assert_output_contains(&record, r#""recorded": true"#);
    assert_output_contains(&record, "supports_hypothesis_id");
    assert_output_contains(&record, "suggested_next_commands");

    let reason = run_cli([
        "case",
        "reason",
        "--store",
        path_str(&store),
        "--space-id",
        "space:advisory:dogfood-adversarial-hypothesis-gates",
        "--format",
        "json",
    ]);
    assert_success(&reason);
    assert_output_contains(&reason, r#""case_head_revision": "revision:observation-000001""#);
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

fn first_hypothesis_id(path: &Path) -> String {
    let report: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    report["result"]["hypotheses"][0]["id"]
        .as_str()
        .unwrap()
        .to_string()
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
