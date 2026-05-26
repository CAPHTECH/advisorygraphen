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
        "Hypothesis-to-proposal evaluation smoke",
        "examples/evaluation/medium-hypothesis-proposal/advisory.input.json",
        "proposal_derived_from_unsupported_hypothesis",
        "high_priority_proposal_missing_hypothesis_refinement",
        "follow_up_observation",
        "primary_count",
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
        "Medium / large review mode",
        "bounded, evidence-backed review priority map",
        "Must Review",
        "Should Review",
        "Can Skim",
        "changed contracts",
        "gluing failures",
        "unresolved verification requirements",
        "Medium/large PR review evaluation smoke",
        "examples/evaluation/medium-pr-review/advisory.input.json",
        "medium_large_review_priority_map",
        "req-authz-tenant-isolation",
        "req-docs-changelog-updated",
        "ranked_observation_tasks",
        "projection_loss_metrics",
    ] {
        assert!(
            skill.contains(required),
            "PR review skill should document {required:?}\n{skill}"
        );
    }
}
