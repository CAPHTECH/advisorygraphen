# 09. Agent Integration and Skill

## Principle

AI agent integration must teach agents when and how to use AdvisoryGraphen without violating evidence, review, or projection boundaries.

## MVP integration surface

MVP uses:

```text
advisorygraphen CLI
  -> stable JSON report schema
  -> repository-owned skill
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
- A projection is needed for executive, developer, audit, AI, or Todoist use.

## Agent must not

- Treat AI-generated cells as accepted facts.
- Accept completion candidates without explicit review policy.
- Hide projection loss.
- Collapse context-specific terms without mapping.
- Report invariant preservation unless a check ran.
- Export unreviewed actions to Todoist unless draft export policy allows it.
- Rewrite customer source material outside the bounded snapshot.

## Agent workflow

```text
1. Identify the bounded source snapshot.
2. Create or validate engagement snapshot JSON.
3. Run advisorygraphen lift.
4. Run advisorygraphen check.
5. Inspect obstructions and evidence gaps.
6. Run advisorygraphen completions propose.
7. Generate requested projection.
8. Keep candidates unreviewed unless the user explicitly reviews them.
9. For Todoist, export projection JSON instead of mutating tasks directly unless an explicit integration action is requested.
```

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

## Skill testing

Each agent skill example must include:

- input snapshot
- exact CLI commands
- expected output shape
- expected safety behavior
- projection with loss disclosure
- candidate that remains unreviewed
