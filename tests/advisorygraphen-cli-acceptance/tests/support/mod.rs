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
