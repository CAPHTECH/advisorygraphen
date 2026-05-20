# Proposal review reference

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
- `needs_structure`: content obstruction `proposal_content_underspecified` is
  present or both `proposed_cell_ids` and `proposed_incidence_ids` are empty.
- `needs_source_witness`: content obstruction
  `proposal_content_missing_source_witness` is present or `source_ids` is empty.
- `needs_derivation_review`: derivation has `failure_mode` other than `none`,
  or `verification_status` remains `unverified` for a high-impact decision.
- `needs_hypothesis_support`: `recommendation_role` is `follow_up_observation`,
  `supported_hypothesis_ids` is empty, or content obstruction
  `proposal_depends_on_unsupported_hypothesis` is present.
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
pass/fail extraction rule, expected observation, falsifier, and promotion effect.
Use `hypothesis_promotion_workflow` to sequence observation -> evidence ->
review-gated hypothesis support/acceptance -> rerun projection.
After `observation record`, prefer `result.promotion_gate.next_command` over
hand-building a `hypothesis support` or `hypothesis falsify` command.

## Candidate-specific actions

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
