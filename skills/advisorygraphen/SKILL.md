---
name: advisorygraphen
description: Use when an agent needs to run AdvisoryGraphen for evidence-backed technical advisory, architecture review, AI-governed completion review, or case reasoning workflows.
---

# AdvisoryGraphen Skill

Use this skill when a task asks for evidence-backed consulting, technical
advisory, architecture review, product decision analysis, AI transformation
governance, delivery risk analysis, or projection of advisory findings into
reports or tasks.

This skill is not just a CLI runbook. Use AdvisoryGraphen to structure
obstructions, hypotheses, reviewable completion candidates, proposal content,
and audience-specific projections. The primary agent loop is:

```text
bounded source -> lift -> check -> hypotheses/completions -> ai_agent projection
-> inspect proposal content -> request or apply reviewed structure -> rerun check
```

For advisory work about a problem, default to a problem-driven hypothesis
workflow:

```text
one bounded problem -> multiple competing hypotheses -> observations/falsifiers
-> classify hypothesis support -> derive proposals only from supported hypotheses
-> project proposal trace and remaining uncertainty
```

## Phase references

Read the relevant reference before starting each phase:

| Phase | When | Reference |
| --- | --- | --- |
| Requirements definition | Task starts from existing documents (interviews, requirements, research) | `skills/advisorygraphen/references/requirements-definition.md` |
| Hypothesis diagnosis | Diagnosis, investigation, root-cause analysis, evidence-backed proposals | `skills/advisorygraphen/references/hypothesis-diagnosis.md` |
| Proposal review | Evaluating completion candidates, hypothesis lifecycle, dry-run | `skills/advisorygraphen/references/proposal-review.md` |
| Projection / output | Reading ai_agent projection, interpreting output fields | `skills/advisorygraphen/references/projection.md` |

## Safety rules

- Do not treat AI-inferred structure as accepted fact.
- Do not accept a completion candidate without explicit review.
- Do not hide projection loss.
- Do not collapse context-specific terms into one meaning without a mapping.
- Do not present unsupported claims as evidence-backed conclusions.
- Do not treat accepted completion review as structural application; inspect `blocker_resolution_state.application_requirements` first.
- Do not autonomously apply hypothesis lifecycle proposals unless a policy allows the outcome and evidence trust level.
- Do not ignore HigherGraphen gluing blockers. Treat
  `higher_graphen_gluing_review.policy_blockers` and
  `higher_graphen_gluing_policy.policy_blockers` as evidence requiring
  candidate revision or explicit completion review.

## Workflow

1. Define one bounded problem statement before collecting proposals. If the
   user gives several concerns, split them or explicitly choose the current
   problem.
2. Create multiple competing hypotheses for that problem. Include at least one
   alternative cause and one falsifiable condition for each hypothesis.
3. Define a bounded source snapshot that records the problem, hypotheses,
   observation sources, known extraction loss, and trust notes.
4. Collect observations that can support, weaken, or falsify the hypotheses.
   Prefer direct command output, repository files, tests, metrics, or reviewed
   source material over agent inference.
5. Validate input JSON.
6. Run `advisorygraphen lift`.
7. Run `advisorygraphen check`.
8. Inspect `obstructions`, `hypotheses`, `falsifiers`, and
   `argumentation_incidences`.
9. Classify each hypothesis as `strongly_supported`, `supported`,
   `supported_needs_followup`, `plausible_secondary`, `falsified`, or
   `insufficient_evidence`. Do not collapse this classification into a single
   narrative before recording it.
10. Derive recommendations only from hypotheses with support. If a proposal
    depends on a weak or untested hypothesis, mark it as follow-up observation
    rather than primary action.
11. Run `advisorygraphen completions propose` when missing structure or
   corrective action is needed.
12. Generate `advisorygraphen project --audience ai_agent` with
   `--completions-report`.
13. Inspect projection fields (see `references/projection.md`).
14. Classify each candidate using its `proposal_content`
    (see `references/proposal-review.md`).
15. For candidates that may be accepted, run `advisorygraphen completions
    dry-run` and inspect `higher_graphen_gluing_review` before asking for
    review or recording an acceptance.
16. Generate the requested human projection or `audit_trace`, including the
    hypothesis classification, proposal trace, falsified/secondary hypotheses,
    and remaining uncertainty.
17. When follow-up observation tasks are present, run the bounded observation,
    record it with `observation record`, then use `result.promotion_gate` to
    support or falsify the hypothesis before rerunning `case reason`.
18. Keep candidates unreviewed unless the user explicitly accepts or rejects
    them, or an explicit conservative policy allows an automated lifecycle
    event.

## Agent operating model

HigherGraphen is operated primarily by AI agents through AdvisoryGraphen.
Humans set goals, constraints, and explicit accept/reject decisions; they do
not need to hand-edit HG structure.

Treat `ai_agent` projection and `case reason` output as the resume protocol.
If a candidate is accepted, do not mark the obstruction resolved until the
required cells and incidences in `blocker_resolution_state.application_requirements`
have been applied and `check`/`case reason` have been rerun.

In the AI-agent projection, inspect `agent_operation_contract` before taking
action. Treat `review_gated_commands` as commands that require explicit review,
inspect `correspondence_analysis` for HigherGraphen overlap, difference, and
gluing failures, and prefer concrete `ranked_observation_tasks` from the
`hypothesis_promotion_workflow` over broad follow-up questions.

For completion work, treat HigherGraphen gluing output as part of the review
contract:

- `completions dry-run` exposes `higher_graphen_gluing_review` for each
  candidate-specific application attempt.
- `completions accept` records `higher_graphen_gluing_policy` in review-event
  metadata. If blockers remain and the reviewer still accepts, the event must
  carry `policy_override: "explicit_completion_review"`.
- `completions apply-accepted` carries `higher_graphen_gluing_review`,
  `policy_blockers`, and `policy_override` into applied-structure output.

Do not interpret gluing success as acceptance. Do not interpret gluing failure
as automatic rejection. It is review evidence that must be resolved by revising
the candidate or by an explicit completion review decision.

## External source boundary

Before running the workflow on external material:

1. Ensure the snapshot is bounded and contains no secrets.
2. Keep customer-specific spaces, reports, and case logs out of public repos.
3. Prefer synthetic or public fixtures for examples.
4. Preserve source IDs so proposal content can carry witnesses.
5. Disclose `source_boundary.extraction_loss`, `projection_loss`, and
   `projection_loss_metrics` in summaries.

If the source snapshot lacks enough structure for a concrete proposal, report
the missing structure rather than fabricating facts.

## Commands

```sh
advisorygraphen validate --input INPUT.json --format json
advisorygraphen dogfood adversarial-fixture --output ADVERSARIAL_INPUT.json --format json
advisorygraphen lift --input INPUT.json --package technical_advisory --output SPACE.json --format json
advisorygraphen check --space SPACE.json --ruleset technical_advisory_mvp --output CHECK.json --format json
advisorygraphen completions propose --space SPACE.json --from-report CHECK.json --output COMPLETIONS.json --format json
advisorygraphen completions dry-run --space SPACE.json --from-report COMPLETIONS.json --candidate-id CANDIDATE --output DRY_RUN.json --format json
advisorygraphen project --space SPACE.json --report CHECK.json --completions-report COMPLETIONS.json --audience ai_agent --format json --output AI_AGENT.json
advisorygraphen project --space SPACE.json --report CHECK.json --audience executive --format markdown --output REPORT.md
advisorygraphen project --space SPACE.json --report CHECK.json --audience audit_trace --format json --output AUDIT.json
advisorygraphen case import --store STORE --space SPACE.json --revision-id REVISION --format json
advisorygraphen case reason --store STORE --space-id SPACE_ID --format json
advisorygraphen case close-check --store STORE --space-id SPACE_ID --base-revision REVISION --format json
advisorygraphen completions accept --store STORE --candidate-id CANDIDATE --from-report COMPLETIONS.json --reviewer REVIEWER --reason REASON --base-revision REVISION --format json
advisorygraphen completions reject --store STORE --candidate-id CANDIDATE --from-report COMPLETIONS.json --reviewer REVIEWER --reason REASON --base-revision REVISION --format json
advisorygraphen completions apply-accepted --store STORE --space-id SPACE_ID --reviewer REVIEWER --reason REASON --base-revision REVISION --format json
advisorygraphen hypothesis propose --space SPACE.json --from-report CHECK.json --output HYPOTHESIS_PROPOSALS.json --format json
advisorygraphen observation record --store STORE --space-id SPACE_ID --from-projection AI_AGENT.json --task-id TASK_ID --result OBSERVATION_RESULT.json --reviewer REVIEWER --reason REASON --base-revision REVISION --format json
advisorygraphen hypothesis apply-proposals --store STORE --from-report HYPOTHESIS_PROPOSALS.json --reviewer ai-agent:codex --reason REASON --base-revision REVISION --format json
advisorygraphen hypothesis falsify --store STORE --from-report CHECK.json --hypothesis-id HYPOTHESIS --evidence EVIDENCE_ID --reviewer REVIEWER --reason REASON --base-revision REVISION --format json
advisorygraphen hypothesis support --store STORE --from-report CHECK.json --hypothesis-id HYPOTHESIS --evidence EVIDENCE_ID --reviewer REVIEWER --reason REASON --base-revision REVISION --format json
advisorygraphen hypothesis accept  --store STORE --from-report CHECK.json --hypothesis-id HYPOTHESIS --evidence EVIDENCE_ID --reviewer REVIEWER --reason REASON --base-revision REVISION --format json
advisorygraphen hypothesis reject  --store STORE --from-report CHECK.json --hypothesis-id HYPOTHESIS --evidence EVIDENCE_ID --reviewer REVIEWER --reason REASON --base-revision REVISION --format json
```

## Minimum external smoke test

For a new external installation or agent bundle, run:

```sh
advisorygraphen validate --input examples/dogfood/agent-operations/advisory.input.json --format json
advisorygraphen lift --input examples/dogfood/agent-operations/advisory.input.json --package technical_advisory_mvp --output /tmp/advisory.space.json --format json
advisorygraphen check --space /tmp/advisory.space.json --ruleset technical_advisory_mvp --output /tmp/advisory.check.json --format json
advisorygraphen completions propose --space /tmp/advisory.space.json --from-report /tmp/advisory.check.json --output /tmp/advisory.completions.json --format json
advisorygraphen completions dry-run --space /tmp/advisory.space.json --from-report /tmp/advisory.completions.json --output /tmp/advisory.dry-run.json --format json
advisorygraphen project --space /tmp/advisory.space.json --report /tmp/advisory.check.json --completions-report /tmp/advisory.completions.json --audience ai_agent --output /tmp/advisory.ai-agent.json --format json
```

Expected smoke result:

- commands exit successfully;
- obstructions may be present and are domain findings, not CLI failures;
- completion candidates remain `review_status: unreviewed`;
- `proposal_content_summary` is present in the AI-agent projection;
- `correspondence_analysis` is present in the AI-agent projection;
- dry-run entries include `higher_graphen_gluing_review`;
- projection loss is present and must be disclosed;
- no candidate is treated as accepted structure.
