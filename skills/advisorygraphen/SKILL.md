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

## Safety rules

- Do not treat AI-inferred structure as accepted fact.
- Do not accept a completion candidate without explicit review.
- Do not hide projection loss.
- Do not collapse context-specific terms into one meaning without a mapping.
- Do not present unsupported claims as evidence-backed conclusions.
- Do not treat accepted completion review as structural application; inspect `blocker_resolution_state.application_requirements` first.
- Do not autonomously apply hypothesis lifecycle proposals unless a policy allows the outcome and evidence trust level.

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
13. Inspect `proposal_content_summary`, `recommendation_trace`,
   `hypothesis_promotion_workflow`, `candidate_review_state`,
   `blocker_resolution_state`, `frontier_items`, `waiting_items`,
   `close_status`, and `projection_loss`.
14. Classify each candidate using its `proposal_content`.
15. Generate the requested human projection or `audit_trace`, including the
    hypothesis classification, proposal trace, falsified/secondary hypotheses,
    and remaining uncertainty.
16. When follow-up observation tasks are present, run the bounded observation,
    record it with `observation record`, then use `result.promotion_gate` to
    support or falsify the hypothesis before rerunning `case reason`.
17. Keep candidates unreviewed unless the user explicitly accepts or rejects
    them, or an explicit conservative policy allows an automated lifecycle
    event.

## Problem-driven hypothesis method

Use this method whenever the user asks for diagnosis, investigation, quality
assessment, root-cause analysis, or evidence-backed proposal generation.

1. State the problem as a single falsifiable question. Example: "Is the default
   unit-test lane a trustworthy quality gate?"
2. Generate multiple hypotheses that could explain the same problem. Avoid
   making the first plausible explanation the default conclusion.
3. For each hypothesis, record:
   - expected observations if true;
   - observations that would weaken or falsify it;
   - source IDs or commands needed to check it;
   - initial confidence as unreviewed or inferred unless source-backed.
   In JSON snapshots, prefer `record_type: "hypothesis_seed"` for these
   records. AdvisoryGraphen lifts them to `cell_type: "hypothesis"` and
   preserves `metadata.expected_observations`, `metadata.falsifiers`, and
   `metadata.candidate_structure_types`.
4. Run the cheapest discriminating observations first. A useful observation is
   one that separates at least two hypotheses, not merely one that adds detail.
   When an observation narrows or revises a hypothesis, record the next version
   as `record_type: "hypothesis_refinement"` and connect it to the earlier
   hypothesis with `relation_type: "refines"`. Prefer deriving proposals from
   the refined hypothesis, not the initial seed.
5. Classify hypotheses explicitly:
   - `strongly_supported`: direct observation supports it and major competing
     explanations are weakened.
   - `supported`: source-backed evidence supports it, but another plausible
     explanation remains.
   - `supported_needs_followup`: evidence supports it, but a blocker prevents a
     decisive measurement.
   - `plausible_secondary`: evidence suggests it may matter, but it is not the
     direct observed cause.
   - `falsified`: observed evidence contradicts the hypothesis.
   - `insufficient_evidence`: no discriminating observation was collected.
6. Build proposals from supported hypotheses only. Proposal priority should
   follow causal order: unblock observation first, fix false-positive/failure
   semantics next, then improve policy, ownership, or performance.
   In JSON snapshots, record these as `record_type: "structure_proposal"` and
   connect them to their source hypotheses with a `derives_from` relation.
   AdvisoryGraphen lifts them to proposal actions and checks whether the
   underlying hypothesis is supported before the action can be treated as a
   primary recommendation.
   For P0/P1 proposals, ensure the source hypothesis has refinement lineage.
   Otherwise AdvisoryGraphen emits
   `high_priority_proposal_missing_hypothesis_refinement`.
7. Report proposal trace in this shape:
   `problem -> hypothesis -> evidence -> classification -> proposal -> required
   verification/owner`.
8. Preserve non-winning hypotheses in the projection. They are useful because
   they show why the chosen proposal is not just a single-agent guess.

## Agent operating model

HigherGraphen is operated primarily by AI agents through AdvisoryGraphen. Humans set goals, constraints, and explicit accept/reject decisions; they do not need to hand-edit HG structure.

Treat `ai_agent` projection and `case reason` output as the resume protocol. If a candidate is accepted, do not mark the obstruction resolved until the required cells and incidences in `blocker_resolution_state.application_requirements` have been applied and `check`/`case reason` have been rerun.

## Proposal content evaluation

Completion candidates include `proposal_content`. Use it to evaluate the
substance of a proposal, not only its existence.

Inspect these fields for every candidate:

- `proposal_content.scenario.status`
- `proposal_content.scenario.changed_structures`
- `proposal_content.morphism`
- `proposal_content.invariant_checks`
- `proposal_content.derivation`
- `proposal_content.witnesses`
- `proposal_content.valuation`
- `proposal_content.policy`
- `proposal_content.content_obstructions`
- `hypothesis_trace`
- `supported_hypothesis_ids`
- `unsupported_hypothesis_ids`
- `recommendation_role`
- `recommendation_trace.follow_up_observations[].ranked_observation_tasks`
- `hypothesis_promotion_workflow`
- `application_plan`
- `proposed_cell_ids`
- `proposed_incidence_ids`

Classify candidates as:

- `ready_for_review`: scenario status is `candidate`, content obstructions are
  empty, the candidate proposes concrete cells or incidences, and
  `application_plan.dry_run_supported` is true.
- `needs_structure`: content obstruction
  `proposal_content_underspecified` is present or both `proposed_cell_ids` and
  `proposed_incidence_ids` are empty.
- `needs_source_witness`: content obstruction
  `proposal_content_missing_source_witness` is present or `source_ids` is empty.
- `needs_derivation_review`: derivation has `failure_mode` other than `none`,
  or `verification_status` remains `unverified` for a high-impact decision.
- `needs_hypothesis_support`: `recommendation_role` is
  `follow_up_observation`, `supported_hypothesis_ids` is empty, or content
  obstruction `proposal_depends_on_unsupported_hypothesis` is present.
- `review_gated`: policy rules or `review_status: unreviewed` require human or
  policy-approved review before promotion.

Do not treat `ready_for_review` as accepted. It means the candidate is concrete
enough to dry-run or submit for review.

Only treat a candidate as a primary recommendation when `recommendation_role` is
`primary`. Candidates with `recommendation_role: follow_up_observation` identify
the next observation or review needed before recommendation, even when they
propose concrete cells or incidences.

For follow-up observations, use
`recommendation_trace.follow_up_observations[].ranked_observation_tasks` before
asking broad questions. Each task should identify the hypothesis being tested,
source IDs to inspect, command template, required inputs, output schema,
pass/fail extraction rule, expected observation, falsifier, and promotion
effect.
Use `hypothesis_promotion_workflow` to sequence observation -> evidence ->
review-gated hypothesis support/acceptance -> rerun projection.
After `observation record`, prefer `result.promotion_gate.next_command` over
hand-building a `hypothesis support` or `hypothesis falsify` command. It carries
the concrete evidence cell id and case head revision needed for the next
review-gated step.

## Candidate-specific actions

Use candidate type and proposed structure to decide the next action.

| Candidate type | Meaning | Agent action |
| --- | --- | --- |
| `owner_assignment` | Existing owner cell can be linked to the blocked action | Present the proposed `owns` incidence for review; do not silently apply it. |
| `ownership_clarification` | Owner is still unknown | Ask for or add bounded owner evidence; do not invent a team. |
| `lift_verification_link` | Existing test, metric, or verification cell can be linked | Present the proposed `verifies` incidence for review. |
| `proposed_test` | Verification structure is missing | Ask for or create a concrete test/metric cell with source-backed rationale. |
| `proposed_interface` | Boundary-safe interface cell is proposed | Review interface owner, contract, compatibility, and verification witnesses. |
| `proposed_refactor_action` | Refactor action cell is proposed | Review migration plan, rollback, and regression evidence. |
| `proposed_auth_guard` | Auth control is proposed | Check shared middleware, intentional-public policy, and route-specific evidence. |

When `proposed_incidence_ids` is non-empty, the proposal is usually more
specific than a placeholder because it reuses existing structure. Still require
review before adding the incidence to the case space.

## Hypothesis lifecycle

Hypotheses explain obstructions; they are not accepted facts.

Use `hypothesis propose` when the space includes agent observations or
source-backed signals such as `metadata.supports_hypothesis_id` or
`metadata.falsifies_hypothesis_id`.

Interpret lifecycle proposals as follows:

- `supported`: evidence supports a candidate hypothesis; review is still
  required for acceptance.
- `falsified`: evidence refutes a candidate hypothesis; downstream candidates
  may need reframing.
- `review_conflict`: support and falsification signals conflict; ask for human
  review.

Only use `hypothesis apply-proposals` when an explicit autonomy policy allows
the outcome and evidence trust level. The default conservative policy should
skip inferred-only evidence and should not apply `accept` or `reject`.

## Projection use

The `ai_agent` projection is the operational contract. Read it before deciding
the next command.

Use:

- `recommendation_trace` to separate primary recommendations, alternatives,
  and follow-up observations. Primary recommendations must derive from
  supported or accepted hypotheses. Follow-up observations include ranked
  observation tasks.
- `hypothesis_promotion_workflow` to see which unsupported hypotheses block
  each candidate and which review-gated steps are needed before promotion.
- `proposal_content_summary` to see how many candidates are structurally
  concrete versus blocked by content obstructions.
- `candidate_quality` to distinguish source-derived, code-derived,
  requirement-derived, and generic candidates.
- `blocker_resolution_state.application_requirements` to know which cells and
  incidences must exist after a candidate is accepted.
- `frontier_items` for agent-actionable work.
- `waiting_items` for human review, source evidence, or blocked states.
- `projection_loss` to disclose omitted source text, lexical caveats, and
  compression loss.

For human-facing output:

- Use `executive` for concise status, risks, candidate quality, and projection
  loss.
- Use `developer_action` for candidate/action details.
- Use `audit_trace` when the user needs the full machine-readable trail.

## External source boundary

Before running the workflow on external material:

1. Ensure the snapshot is bounded and contains no secrets.
2. Keep customer-specific spaces, reports, and case logs out of public repos.
3. Prefer synthetic or public fixtures for examples.
4. Preserve source IDs so proposal content can carry witnesses.
5. Disclose `source_boundary.extraction_loss` and `projection_loss` in
   summaries.

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
advisorygraphen hypothesis propose --space SPACE.json --from-report CHECK.json --output HYPOTHESIS_PROPOSALS.json --format json
advisorygraphen observation record --store STORE --space-id SPACE_ID --from-projection AI_AGENT.json --task-id TASK_ID --result OBSERVATION_RESULT.json --reviewer REVIEWER --reason REASON --base-revision REVISION --format json
advisorygraphen hypothesis apply-proposals --store STORE --from-report HYPOTHESIS_PROPOSALS.json --reviewer ai-agent:codex --reason REASON --base-revision REVISION --format json
advisorygraphen hypothesis falsify --store STORE --from-report CHECK.json --hypothesis-id HYPOTHESIS --evidence EVIDENCE_ID --reviewer REVIEWER --reason REASON --base-revision REVISION --format json
advisorygraphen hypothesis support --store STORE --from-report CHECK.json --hypothesis-id HYPOTHESIS --evidence EVIDENCE_ID --reviewer REVIEWER --reason REASON --base-revision REVISION --format json
advisorygraphen hypothesis accept  --store STORE --from-report CHECK.json --hypothesis-id HYPOTHESIS --evidence EVIDENCE_ID --reviewer REVIEWER --reason REASON --base-revision REVISION --format json
advisorygraphen hypothesis reject  --store STORE --from-report CHECK.json --hypothesis-id HYPOTHESIS --evidence EVIDENCE_ID --reviewer REVIEWER --reason REASON --base-revision REVISION --format json
```

## Output interpretation

- `obstructions` means the tool found structured blockers; it is not a tool failure.
- `completion_candidates` are proposals, not accepted changes.
- `review_status: unreviewed` means do not present as accepted.
- `recommendation_trace.follow_up_observations[].ranked_observation_tasks`
  lists the cheapest next observations before a follow-up can be promoted. Each
  task should include `command_template`, `required_inputs`, `output_schema`,
  and `pass_fail_extraction`.
- `hypothesis_promotion_workflow.items[]` is the AI-agent sequence for turning
  a follow-up observation into a reviewable recommendation.
- `observation record` validates an observation result against the selected
  task's `output_schema`, materializes it as an evidence cell in the imported
  case, and returns suggested `hypothesis support` / `hypothesis falsify`
  command drafts.
- `proposal_content.scenario.status: candidate` means proposal content is
  concrete enough for review, not accepted.
- `proposal_content.scenario.status: blocked` means the proposal itself has
  unresolved content obstructions.
- `proposal_content_summary.blocked_content > 0` means some candidates need
  more source or structure before review.
- `application_plan` is the candidate's unreviewed operation preview; it does
  not prove the blocker will resolve.
- `completion_dry_run.result.dry_runs[].check_delta` is the evidence for what a
  candidate resolves or introduces on a cloned space.
- `proposed_incidence_ids` means the candidate proposes a concrete relation,
  usually `owns` or `verifies`, based on existing related structure.
- `agent_operation_contract` lists safe next commands and review-gated commands.
- `blocker_resolution_state` describes whether a blocker has no candidate, pending review, all candidates rejected, or an accepted candidate pending structural application.
- `frontier_items` lists agent-actionable next work; `waiting_items` lists review or source-structure waits.
- `application_requirements` names the cells and incidences an AI agent must create before treating a blocker as resolved.
- `case_head_revision` from `case reason` is the base revision for the next `case close-check`.
- Run `case close-check` before reporting a case as closeable.
- `review_gated_commands` require explicit human review before accept/reject events.
- `hypothesis apply-proposals` can apply only policy-allowed `supported` / `falsified` proposal events; it must skip inferred-only evidence under the default conservative policy.
- For imported case stores, `completions accept` and `completions reject` require `--base-revision`; missing or stale base revision is a stale-write error.
- `projection_loss` must be disclosed when summarizing the projection.
- `evidence_origin: inferred` cannot satisfy hard evidence requirements by default.

## Minimum external smoke test

For a new external installation or agent bundle, run:

```sh
advisorygraphen validate --input examples/dogfood/agent-operations/advisory.input.json --format json
advisorygraphen lift --input examples/dogfood/agent-operations/advisory.input.json --package technical_advisory_mvp --output /tmp/advisory.space.json --format json
advisorygraphen check --space /tmp/advisory.space.json --ruleset technical_advisory_mvp --output /tmp/advisory.check.json --format json
advisorygraphen completions propose --space /tmp/advisory.space.json --from-report /tmp/advisory.check.json --output /tmp/advisory.completions.json --format json
advisorygraphen project --space /tmp/advisory.space.json --report /tmp/advisory.check.json --completions-report /tmp/advisory.completions.json --audience ai_agent --output /tmp/advisory.ai-agent.json --format json
```

Expected smoke result:

- commands exit successfully;
- obstructions may be present and are domain findings, not CLI failures;
- completion candidates remain `review_status: unreviewed`;
- `proposal_content_summary` is present in the AI-agent projection;
- projection loss is present and must be disclosed;
- no candidate is treated as accepted structure.
