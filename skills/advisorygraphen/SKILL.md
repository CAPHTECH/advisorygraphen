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

## Safety rules

- Do not treat AI-inferred structure as accepted fact.
- Do not accept a completion candidate without explicit review.
- Do not hide projection loss.
- Do not collapse context-specific terms into one meaning without a mapping.
- Do not present unsupported claims as evidence-backed conclusions.
- Do not treat accepted completion review as structural application; inspect `blocker_resolution_state.application_requirements` first.
- Do not autonomously apply hypothesis lifecycle proposals unless a policy allows the outcome and evidence trust level.

## Workflow

1. Define a bounded source snapshot.
2. Validate input JSON.
3. Run `advisorygraphen lift`.
4. Run `advisorygraphen check`.
5. Inspect `obstructions`, `hypotheses`, `falsifiers`, and
   `argumentation_incidences`.
6. Run `advisorygraphen completions propose` when missing structure or
   corrective action is needed.
7. Generate `advisorygraphen project --audience ai_agent` with
   `--completions-report`.
8. Inspect `proposal_content_summary`, `candidate_review_state`,
   `blocker_resolution_state`, `frontier_items`, `waiting_items`,
   `close_status`, and `projection_loss`.
9. Classify each candidate using its `proposal_content`.
10. Generate the requested human projection or `audit_trace`.
11. Keep candidates unreviewed unless the user explicitly accepts or rejects
    them, or an explicit conservative policy allows an automated lifecycle
    event.

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
- `proposed_cell_ids`
- `proposed_incidence_ids`

Classify candidates as:

- `ready_for_review`: scenario status is `candidate`, content obstructions are
  empty, and the candidate proposes concrete cells or incidences.
- `needs_structure`: content obstruction
  `proposal_content_underspecified` is present or both `proposed_cell_ids` and
  `proposed_incidence_ids` are empty.
- `needs_source_witness`: content obstruction
  `proposal_content_missing_source_witness` is present or `source_ids` is empty.
- `needs_derivation_review`: derivation has `failure_mode` other than `none`,
  or `verification_status` remains `unverified` for a high-impact decision.
- `review_gated`: policy rules or `review_status: unreviewed` require human or
  policy-approved review before promotion.

Do not treat `ready_for_review` as accepted. It means the candidate is concrete
enough for explicit review.

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
advisorygraphen lift --input INPUT.json --package technical_advisory --output SPACE.json --format json
advisorygraphen check --space SPACE.json --ruleset technical_advisory_mvp --output CHECK.json --format json
advisorygraphen completions propose --space SPACE.json --from-report CHECK.json --output COMPLETIONS.json --format json
advisorygraphen project --space SPACE.json --report CHECK.json --completions-report COMPLETIONS.json --audience ai_agent --format json --output AI_AGENT.json
advisorygraphen project --space SPACE.json --report CHECK.json --audience executive --format markdown --output REPORT.md
advisorygraphen project --space SPACE.json --report CHECK.json --audience audit_trace --format json --output AUDIT.json
advisorygraphen case import --store STORE --space SPACE.json --revision-id REVISION --format json
advisorygraphen case reason --store STORE --space-id SPACE_ID --format json
advisorygraphen case close-check --store STORE --space-id SPACE_ID --base-revision REVISION --format json
advisorygraphen completions accept --store STORE --candidate-id CANDIDATE --from-report COMPLETIONS.json --reviewer REVIEWER --reason REASON --base-revision REVISION --format json
advisorygraphen completions reject --store STORE --candidate-id CANDIDATE --from-report COMPLETIONS.json --reviewer REVIEWER --reason REASON --base-revision REVISION --format json
advisorygraphen hypothesis propose --space SPACE.json --from-report CHECK.json --output HYPOTHESIS_PROPOSALS.json --format json
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
- `proposal_content.scenario.status: candidate` means proposal content is
  concrete enough for review, not accepted.
- `proposal_content.scenario.status: blocked` means the proposal itself has
  unresolved content obstructions.
- `proposal_content_summary.blocked_content > 0` means some candidates need
  more source or structure before review.
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
