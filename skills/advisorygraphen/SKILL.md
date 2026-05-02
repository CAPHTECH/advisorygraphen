# AdvisoryGraphen Skill

Use this skill when a task asks for evidence-backed consulting, technical advisory, architecture review, product decision analysis, AI transformation governance, delivery risk analysis, or projection of advisory findings into reports or tasks.

## Safety rules

- Do not treat AI-inferred structure as accepted fact.
- Do not accept a completion candidate without explicit review.
- Do not hide projection loss.
- Do not collapse context-specific terms into one meaning without a mapping.
- Do not present unsupported claims as evidence-backed conclusions.

## Workflow

1. Define a bounded source snapshot.
2. Validate input JSON.
3. Run `advisorygraphen lift`.
4. Run `advisorygraphen check`.
5. Review obstructions.
6. Run `advisorygraphen completions propose` when missing structure or corrective action is needed.
7. Generate the requested projection with `advisorygraphen project`.
8. Keep candidates unreviewed unless the user explicitly accepts or rejects them.

## Commands

```sh
advisorygraphen validate --input INPUT.json --format json
advisorygraphen lift --input INPUT.json --package technical_advisory --output SPACE.json --format json
advisorygraphen check --space SPACE.json --ruleset technical_advisory_mvp --output CHECK.json --format json
advisorygraphen completions propose --space SPACE.json --from-report CHECK.json --output COMPLETIONS.json --format json
advisorygraphen project --space SPACE.json --report CHECK.json --audience executive --format markdown --output REPORT.md
advisorygraphen project --space SPACE.json --report CHECK.json --audience audit_trace --format json --output AUDIT.json
```

## Output interpretation

- `obstructions` means the tool found structured blockers; it is not a tool failure.
- `completion_candidates` are proposals, not accepted changes.
- `review_status: unreviewed` means do not present as accepted.
- `projection_loss` must be disclosed when summarizing the projection.
- `evidence_origin: inferred` cannot satisfy hard evidence requirements by default.
