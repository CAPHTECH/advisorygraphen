# 09. Agent Integration and Skill

## Principle

AI agent integration must teach agents when and how to operate HigherGraphen through AdvisoryGraphen without violating evidence, review, or projection boundaries. HG is not assumed to be a hand-edited human workspace; it is the structural substrate an AI agent reads, writes, checks, and projects under explicit review gates.

## MVP integration surface

MVP uses:

```text
advisorygraphen CLI
  -> propose/status/report/review facade for normal agent operation
  -> stable JSON report schema
  -> repository-owned skill
  -> ai_agent projection operation contract
```

MCP, provider plugin bundles, and marketplace metadata are future work.

## Agent-facing name

Skill name: `advisorygraphen`

CLI command: `advisorygraphen`

## Agent should use AdvisoryGraphen when

- A consulting or advisory task has multiple claims, evidence items, risks, and recommendations.
- The user needs evidence-backed recommendations.
- The user needs to distinguish accepted facts, AI inference, and reviewable candidates.
- The task involves technical advisory, architecture review, product decision support, AI transformation governance, or delivery risk analysis.
- A projection is needed for executive, developer, audit, or AI use.

## Agent must not

- Treat AI-generated cells as accepted facts.
- Accept completion candidates without explicit review policy.
- Hide projection loss.
- Collapse context-specific terms without mapping.
- Report invariant preservation unless a check ran.
- Rewrite customer source material outside the bounded snapshot.

## Agent workflow

```text
1. Identify the bounded source snapshot.
2. Run `advisorygraphen propose --input <snapshot> --case <case-dir>`.
3. Inspect the returned manifest path, recommendation trace, blockers, and
   waiting items.
4. Run `advisorygraphen status --case <case-dir> --brief` before resuming
   later work.
5. Read `status.result.summary`, `status.result.top_blockers`, and
   `status.result.next_best_action` before expanding the full blocker details.
6. Run `advisorygraphen report --case <case-dir> --audience ai_agent` before
   choosing review or observation commands.
7. Use `advisorygraphen review ... --case <case-dir>` for explicit
   completion or hypothesis review decisions.
8. Generate requested human projection or audit_trace.
9. Keep candidates unreviewed unless the user explicitly reviews them.

The low-level `validate`, `lift`, `check`, `completions propose`, `project`,
`case`, `hypothesis`, and `observation` commands remain the stable primitive
surface for CI, debugging, and custom orchestration.
```

The agent should treat `ai_agent` projection as its resume protocol. It should use `open_obstructions`, `candidate_review_state`, `blocker_resolution_state`, `observation_actions`, `projection_loss_metrics`, `schema_morphisms`, `review_gated_commands`, and `forbidden_operations` before deciding the next command. `candidate_review_state` and `blocker_resolution_state` are populated when the agent supplies the completion proposal report to `project`; `case reason` derives the same state for the current case log while overlaying recorded review events. `observation_actions` are bounded evidence-gathering steps for weak or blocked hypotheses, `projection_loss_metrics` quantify omitted/collapsed/source-gap risk, and `schema_morphisms` disclose the lift or contract mapping with declared loss. When a candidate is accepted, the agent must inspect `blocker_resolution_state.application_requirements` and create the required cells/incidences before treating the blocker as resolved. The human does not need to edit HG directly; the human reviews projections and explicit accept/reject/waive events.

## Minimal skill file

A concrete skill file is included at:

`skills/advisorygraphen/SKILL.md`

## Future MCP capability map

MCP should expose structural intent, not low-level storage only.

| Capability | Purpose |
| --- | --- |
| `create_engagement_space` | Create structural universe for a consulting engagement |
| `add_advisory_cells` | Add typed cells with provenance |
| `add_advisory_incidences` | Add support, contradiction, dependency, ownership, verification relations |
| `define_advisory_context` | Define local scope |
| `define_advisory_morphism` | Define source→structure, as-is→to-be, requirement→verification mapping |
| `check_advisory_invariants` | Evaluate consulting invariants |
| `detect_advisory_obstructions` | Find structured blockers |
| `propose_advisory_completions` | Generate reviewable candidates |
| `accept_advisory_completion` | Record explicit review and promote if policy allows |
| `reject_advisory_completion` | Record rejection |
| `project_advisory_view` | Produce audience-specific view |
| `explain_advisory_obstruction` | Explain finding through selected projection |

## Agent output interpretation

Agents should interpret command output as follows:

| Output | Meaning |
| --- | --- |
| `obstructions` non-empty | The tool found domain blockers; command succeeded |
| `completion_candidates` non-empty | Proposals exist; not accepted yet |
| `projection_loss` non-empty | View omitted or compressed information; disclose to user |
| `review_status = unreviewed` | Do not present as accepted fact |
| `evidence_origin = inferred` | Cannot satisfy hard evidence requirement by default |
| `hg_operation_model.primary_operator = ai_agent` | Agent may operate HG only within the returned operation contract |
| `review_gated_commands` non-empty | Require explicit human review before promotion or rejection |

## Skill testing

Each agent skill example must include:

- input snapshot
- exact CLI commands
- expected output shape
- expected safety behavior
- projection with loss disclosure
- ai_agent projection with operation contract
- candidate that remains unreviewed
