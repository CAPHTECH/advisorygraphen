# Projection and output interpretation reference

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
- `projection_loss_metrics` to quantify what the projection collapses, omits,
  or leaves without source trace.
- `observation_actions` to choose bounded next evidence-gathering steps before
  promoting weak hypotheses.
- `schema_morphisms` to understand the lift or contract mapping and declared
  compatibility/loss.

For human-facing output:

- Use `executive` for concise status, risks, candidate quality, and projection
  loss.
- Use `developer_action` for candidate/action details.
- Use `audit_trace` when the user needs the full machine-readable trail.

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
- `blocker_resolution_state` describes whether a blocker has no candidate,
  pending review, all candidates rejected, or an accepted candidate pending
  structural application.
- `frontier_items` lists agent-actionable next work; `waiting_items` lists
  review or source-structure waits.
- `application_requirements` names the cells and incidences an AI agent must
  create before treating a blocker as resolved.
- `case_head_revision` from `case reason` is the base revision for the next
  `case close-check`.
- Run `case close-check` before reporting a case as closeable.
- `review_gated_commands` require explicit human review before accept/reject
  events.
- `hypothesis apply-proposals` can apply only policy-allowed `supported` /
  `falsified` proposal events; it must skip inferred-only evidence under the
  default conservative policy.
- For imported case stores, `completions accept` and `completions reject`
  require `--base-revision`; missing or stale base revision is a stale-write
  error.
- `projection_loss` must be disclosed when summarizing the projection.
- `evidence_origin: inferred` cannot satisfy hard evidence requirements by
  default.
