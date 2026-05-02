#[allow(dead_code)]
mod support;

use std::fs;
use support::repo_root;

#[test]
fn repository_skill_documents_agent_resume_protocol() {
    let skill = fs::read_to_string(repo_root().join("skills/advisorygraphen/SKILL.md"))
        .expect("skill file should be readable");

    for required in [
        "project --audience ai_agent",
        "--completions-report",
        "agent_operation_contract",
        "blocker_resolution_state",
        "application_requirements",
        "case reason",
        "case close-check",
        "review_gated_commands",
    ] {
        assert!(
            skill.contains(required),
            "skill should document {required:?}\n{skill}"
        );
    }
}
