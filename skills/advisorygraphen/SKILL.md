# AdvisoryGraphen Skill

Use this skill when a task asks for evidence-backed consulting, technical advisory, architecture review, product decision analysis, AI transformation governance, delivery risk analysis, or projection of advisory findings into reports or tasks.

## Safety rules

- Do not treat AI-inferred structure as accepted fact.
- Do not accept a completion candidate without explicit review.
- Do not hide projection loss.
- Do not collapse context-specific terms into one meaning without a mapping.
- Do not present unsupported claims as evidence-backed conclusions.
- Do not treat accepted completion review as structural application; inspect `blocker_resolution_state.application_requirements` first.

## Workflow

1. Define a bounded source snapshot.
2. Validate input JSON.
3. Run `advisorygraphen lift`.
4. Run `advisorygraphen check`.
5. Review obstructions.
6. Run `advisorygraphen completions propose` when missing structure or corrective action is needed.
7. Generate `advisorygraphen project --audience ai_agent` with `--completions-report`.
8. Follow `agent_operation_contract`, `open_obstructions`, `candidate_review_state`, `blocker_resolution_state`, `close_status`, and `projection_loss`.
9. Generate the requested human projection or `audit_trace`.
10. Keep candidates unreviewed unless the user explicitly accepts or rejects them.

## Agent operating model

HigherGraphen is operated primarily by AI agents through AdvisoryGraphen. Humans set goals, constraints, and explicit accept/reject decisions; they do not need to hand-edit HG structure.

Treat `ai_agent` projection and `case reason` output as the resume protocol. If a candidate is accepted, do not mark the obstruction resolved until the required cells and incidences in `blocker_resolution_state.application_requirements` have been applied and `check`/`case reason` have been rerun.

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
```

## Output interpretation

- `obstructions` means the tool found structured blockers; it is not a tool failure.
- `completion_candidates` are proposals, not accepted changes.
- `review_status: unreviewed` means do not present as accepted.
- `agent_operation_contract` lists safe next commands and review-gated commands.
- `blocker_resolution_state` describes whether a blocker has no candidate, pending review, all candidates rejected, or an accepted candidate pending structural application.
- `application_requirements` names the cells and incidences an AI agent must create before treating a blocker as resolved.
- Run `case close-check` before reporting a case as closeable.
- `review_gated_commands` require explicit human review before accept/reject events.
- For imported case stores, `completions accept` and `completions reject` require `--base-revision`; missing or stale base revision is a stale-write error.
- `projection_loss` must be disclosed when summarizing the projection.
- `evidence_origin: inferred` cannot satisfy hard evidence requirements by default.
