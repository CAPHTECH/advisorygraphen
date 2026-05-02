use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PACKAGE: &str = "advisorygraphen-cli";
const BINARY: &str = "advisorygraphen";
const FIXTURE: &str = "examples/technical-advisory/direct-db-access/advisory.input.json";
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
fn direct_fixture_lift_check_completions_and_executive_projection() {
    let dir = clean_case_dir("direct-flow");
    let space = dir.join("advisory.space.json");
    let check = dir.join("advisory.check.report.json");
    let completions = dir.join("advisory.completions.report.json");
    let executive = dir.join("executive-review.md");
    let executive_json = dir.join("executive-review.json");

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
    assert_file_contains(&completions, "unreviewed");

    let output = run_cli([
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
    assert_success(&output);
    assert_file_contains(&executive, "Billing DB");
    assert_file_contains(&executive, "boundary");
    assert_file_contains(&executive, "projection");

    let output = run_cli([
        "project",
        "--space",
        path_str(&space),
        "--report",
        path_str(&check),
        "--audience",
        "executive",
        "--format",
        "json",
        "--output",
        path_str(&executive_json),
    ]);
    assert_success(&output);
    assert_file_contains(&executive_json, "higher_graphen");
    assert_file_contains(&executive_json, "projection:higher:executive");
}

#[test]
fn case_import_reason_and_close_check_report_unresolved_obstruction() {
    let dir = clean_case_dir("case-basics");
    let space = dir.join("advisory.space.json");
    let check = dir.join("advisory.check.report.json");
    let store = dir.join("store");

    lift_fixture(&space);
    check_space(&space, &check);

    let import = run_cli([
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
    ]);
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
    assert_output_contains(&reason, "blockers");

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

fn run_cli<const N: usize>(args: [&str; N]) -> Output {
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

fn assert_success(output: &Output) {
    if !output.status.success() {
        panic!(
            "command failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn assert_output_contains(output: &Output, needle: &str) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stdout.contains(needle) && !stderr.contains(needle) {
        panic!(
            "expected command output to contain {needle:?}\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
    }
}

fn assert_output_contains_any(output: &Output, needles: &[&str]) {
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

fn assert_file_contains(path: &Path, needle: &str) {
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

fn clean_case_dir(name: &str) -> PathBuf {
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

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("test crate should live under tests/advisorygraphen-cli-acceptance")
        .to_path_buf()
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("test paths should be valid UTF-8")
}
