#[allow(dead_code)]
mod support;

use std::fs;
use support::repo_root;

#[test]
fn repository_skill_documents_agent_resume_protocol() {
    let skill = fs::read_to_string(repo_root().join("skills/advisorygraphen/SKILL.md"))
        .expect("skill file should be readable");

    assert!(
        skill.starts_with("---\nname: advisorygraphen\ndescription:"),
        "skill should start with YAML frontmatter\n{skill}"
    );

    for required in [
        "project --audience ai_agent",
        "--completions-report",
        "agent_operation_contract",
        "blocker_resolution_state",
        "application_requirements",
        "case reason",
        "case close-check",
        "review_gated_commands",
        "ranked_observation_tasks",
        "hypothesis_promotion_workflow",
        "dogfood adversarial-fixture",
    ] {
        assert!(
            skill.contains(required),
            "skill should document {required:?}\n{skill}"
        );
    }
}

#[test]
fn pr_review_skill_documents_micro_review_structure_risk_triage() {
    let skill = fs::read_to_string(repo_root().join("skills/advisorygraphen-pr-review/SKILL.md"))
        .expect("PR review skill file should be readable");

    assert!(
        skill.starts_with("---\nname: advisorygraphen-pr-review\ndescription:"),
        "PR review skill should start with YAML frontmatter\n{skill}"
    );

    for required in [
        "Micro triage for small PRs or AI summaries",
        "advisorygraphen micro review",
        "micro-review.json",
        "structure_error_risks",
        "risk_factors",
        "falsification_checks",
        "relative_error_risk_not_probability",
        "full_advisory_workflow_recommended",
        "Structure Error Risk",
    ] {
        assert!(
            skill.contains(required),
            "PR review skill should document {required:?}\n{skill}"
        );
    }
}
