use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PACKAGE: &str = "advisorygraphen-cli";
pub const BINARY: &str = "advisorygraphen";

pub fn run_cli<const N: usize>(args: [&str; N]) -> Output {
    let repo = repo_root();
    let mut command = Command::new("cargo");
    command
        .current_dir(&repo)
        .env("CARGO_NET_OFFLINE", "true")
        .args([
            "run",
            "--quiet",
            "--package",
            PACKAGE,
            "--bin",
            BINARY,
            "--",
        ])
        .args(args);

    command.output().unwrap_or_else(|error| {
        panic!("failed to execute cargo for {BINARY}: {error}");
    })
}

pub fn assert_success(output: &Output) {
    if !output.status.success() {
        panic!(
            "command failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

pub fn assert_output_contains(output: &Output, needle: &str) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stdout.contains(needle) && !stderr.contains(needle) {
        panic!("expected command output to contain {needle:?}\nstdout:\n{stdout}\nstderr:\n{stderr}");
    }
}

pub fn assert_output_contains_any(output: &Output, needles: &[&str]) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !needles
        .iter()
        .any(|needle| stdout.contains(needle) || stderr.contains(needle))
    {
        panic!(
            "expected command output to contain one of {needles:?}\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
    }
}

pub fn assert_file_contains(path: &Path, needle: &str) {
    let contents = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });
    if !contents.contains(needle) {
        panic!(
            "expected {} to contain {needle:?}\ncontents:\n{contents}",
            path.display()
        );
    }
}

pub fn assert_file_not_contains(path: &Path, needle: &str) {
    let contents = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });
    if contents.contains(needle) {
        panic!(
            "expected {} not to contain {needle:?}\ncontents:\n{contents}",
            path.display()
        );
    }
}

pub fn clean_case_dir(name: &str) -> PathBuf {
    let dir = repo_root()
        .join("target")
        .join("tmp")
        .join("cli-acceptance")
        .join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap_or_else(|error| {
            panic!("failed to remove {}: {error}", dir.display());
        });
    }
    fs::create_dir_all(&dir).unwrap_or_else(|error| {
        panic!("failed to create {}: {error}", dir.display());
    });
    dir
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("test crate should live under tests/advisorygraphen-cli-acceptance")
        .to_path_buf()
}

pub fn path_str(path: &Path) -> &str {
    path.to_str().expect("test paths should be valid UTF-8")
}

pub struct AdvisoryFixtureFlow<'a> {
    pub case_name: &'a str,
    pub fixture: &'a str,
    pub package: &'a str,
    pub ruleset: &'a str,
    pub space_id: &'a str,
    pub revision_id: &'a str,
    pub expected_obstructions: &'a [&'a str],
    pub unexpected_obstructions: &'a [&'a str],
    pub expected_audit_text: &'a str,
    pub expected_candidate_text: &'a str,
}

pub fn assert_advisory_fixture_flow(flow: AdvisoryFixtureFlow<'_>) {
    let dir = clean_case_dir(flow.case_name);
    let space = dir.join("advisory.space.json");
    let check = dir.join("advisory.check.report.json");
    let completions = dir.join("advisory.completions.report.json");
    let audit = dir.join("audit-trace.json");
    let ai_agent = dir.join("ai-agent.json");
    let store = dir.join("store");

    let validate = run_cli(["validate", "--input", flow.fixture, "--format", "json"]);
    assert_success(&validate);

    let lift = run_cli([
        "lift",
        "--input",
        flow.fixture,
        "--package",
        flow.package,
        "--output",
        path_str(&space),
        "--format",
        "json",
    ]);
    assert_success(&lift);
    assert_file_contains(&space, flow.space_id);
    assert_file_contains(&space, "higher_graphen_interpretation");
    assert_file_contains(&space, "morphism:source-to-advisory-space");

    let check_output = run_cli([
        "check",
        "--space",
        path_str(&space),
        "--ruleset",
        flow.ruleset,
        "--output",
        path_str(&check),
        "--format",
        "json",
    ]);
    assert_success(&check_output);
    assert_file_contains(&check, "higher_graphen");
    for obstruction_id in flow.expected_obstructions {
        assert_file_contains(&check, obstruction_id);
    }
    for obstruction_id in flow.unexpected_obstructions {
        assert_file_not_contains(&check, obstruction_id);
    }

    let propose = run_cli([
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
    ]);
    assert_success(&propose);
    assert_file_contains(&completions, "higher_graphen");
    assert_file_contains(&completions, "\"missing_type\": \"cell\"");

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
    assert_file_contains(&audit, flow.expected_audit_text);

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
    assert_file_contains(&ai_agent, "projection:higher:ai_agent");
    assert_file_contains(&ai_agent, r#""primary_operator": "ai_agent""#);
    assert_file_contains(&ai_agent, "agent_operation_contract");
    assert_file_contains(&ai_agent, "promote unreviewed candidate structure");
    assert_file_contains(&ai_agent, "open_obstructions");
    assert_file_contains(&ai_agent, "candidate_review_state");
    assert_file_contains(&ai_agent, flow.expected_candidate_text);
    assert_file_contains(&ai_agent, r#""closeable": false"#);

    let import = run_cli([
        "case",
        "import",
        "--store",
        path_str(&store),
        "--space",
        path_str(&space),
        "--revision-id",
        flow.revision_id,
        "--format",
        "json",
    ]);
    assert_success(&import);

    let accept = run_cli([
        "completions",
        "accept",
        "--store",
        path_str(&store),
        "--candidate-id",
        flow.expected_candidate_text,
        "--from-report",
        path_str(&completions),
        "--reviewer",
        "reviewer:dogfood-agent",
        "--reason",
        "Accepted during dogfood case resume.",
        "--format",
        "json",
    ]);
    assert_success(&accept);

    let reason = run_cli([
        "case",
        "reason",
        "--store",
        path_str(&store),
        "--space-id",
        flow.space_id,
        "--format",
        "json",
    ]);
    assert_success(&reason);
    assert_output_contains_any(&reason, &[r#""closeable": false"#, r#""closeable":false"#]);
    assert_output_contains(&reason, r#""blocking_threshold": "medium""#);
    assert_output_contains(&reason, "candidate_review_state");
    assert_output_contains(&reason, flow.expected_candidate_text);
    assert_output_contains(&reason, r#""review_status": "accepted""#);
    assert_output_contains(&reason, "blocker_resolution_state");
    assert_output_contains(&reason, "accepted_candidate_pending_application");
    assert_output_contains(
        &reason,
        "does_not_clear_obstruction_until_structure_changes",
    );
    assert_output_contains(&reason, "projection:ai-agent");
    for obstruction_id in flow.expected_obstructions {
        assert_output_contains(&reason, obstruction_id);
    }
}

pub fn assert_advanced_dogfood_fixture_flows(package: &str, ruleset: &str) {
    assert_advisory_fixture_flow(AdvisoryFixtureFlow {
        case_name: "dogfood-product-governance",
        fixture: "examples/dogfood/product-governance/advisory.input.json",
        package,
        ruleset,
        space_id: "space:advisory:dogfood-product-governance",
        revision_id: "revision:dogfood-product-governance-1",
        expected_obstructions: &[
            "obstruction:enterprise-packaging-action-missing-owner",
            "obstruction:hosted-rollout-requirement-missing-verification",
        ],
        unexpected_obstructions: &[
            "obstruction:mvp-release-gate-missing-verification",
            "obstruction:private-boundary-checklist-action-missing-owner",
        ],
        expected_audit_text: "Define enterprise packaging owner and launch gate",
        expected_candidate_text: "candidate:enterprise-packaging-action-missing-owner-owner",
    });

    assert_advisory_fixture_flow(AdvisoryFixtureFlow {
        case_name: "dogfood-agent-operations",
        fixture: "examples/dogfood/agent-operations/advisory.input.json",
        package,
        ruleset,
        space_id: "space:advisory:dogfood-agent-operations",
        revision_id: "revision:dogfood-agent-operations-1",
        expected_obstructions: &[
            "obstruction:agent-recovery-runbook-action-missing-owner",
            "obstruction:memory-feedback-audit-requirement-missing-verification",
        ],
        unexpected_obstructions: &[
            "obstruction:handoff-preserves-review-state-missing-verification",
            "obstruction:prompt-injection-boundary-action-missing-owner",
        ],
        expected_audit_text: "Create agent recovery runbook",
        expected_candidate_text: "candidate:agent-recovery-runbook-action-missing-owner-owner",
    });

    assert_advisory_fixture_flow(AdvisoryFixtureFlow {
        case_name: "dogfood-commercial-boundary",
        fixture: "examples/dogfood/commercial-boundary/advisory.input.json",
        package,
        ruleset,
        space_id: "space:advisory:dogfood-commercial-boundary",
        revision_id: "revision:dogfood-commercial-boundary-1",
        expected_obstructions: &[
            "obstruction:commercial-packaging-review-board-action-missing-owner",
            "obstruction:commercial-rules-export-policy-check-missing-verification",
        ],
        unexpected_obstructions: &[
            "obstruction:publication-scrub-checklist-action-missing-owner",
            "obstruction:public-examples-no-customer-data-missing-verification",
        ],
        expected_audit_text: "Create commercial packaging review board",
        expected_candidate_text: "candidate:commercial-packaging-review-board-action-missing-owner-owner",
    });
}
