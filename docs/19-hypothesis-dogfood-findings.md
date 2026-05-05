# 19. Hypothesis Layer Dogfood Findings

Captured 2026-05-04 after running the new hypothesis layer (priorities 1–7) against real fixtures.

## Method

Lift + check on every available fixture; record obstructions emitted vs hypotheses generated.

```sh
for fixture in examples/dogfood/*/advisory.input.json examples/technical-advisory/*/advisory.input.json; do
  advisorygraphen lift   --input $fixture --package technical_advisory_mvp --output /tmp/space.json
  advisorygraphen check  --space /tmp/space.json --ruleset technical_advisory_mvp --output /tmp/check.json
done
```

## Result

| Fixture | Obstructions | Hypotheses emitted |
| --- | --- | --- |
| `dogfood/agent-operations` | `missing_owner`, `requirement_unverified` | **0** |
| `dogfood/commercial-boundary` | `missing_owner`, `requirement_unverified` | **0** |
| `dogfood/product-governance` | `missing_owner`, `requirement_unverified` | **0** |
| `dogfood/higher-graphen-integration` | (none) | 0 |
| `dogfood` (AG self repo-snapshot) | (none — adapter ignores Rust files) | 0 |
| `technical-advisory/direct-db-access` | `boundary_violation` | 3 |

## Headline

**Hypothesis coverage 2/7 was the wrong 2.**

The hypothesis layer covers `boundary_violation` and `api_route_missing_auth`. Real dogfood engagements emit *neither*. They emit `missing_owner` and `requirement_unverified` — both uncovered.

`api_route_missing_auth` requires a TypeScript / JavaScript codebase via `code repo-snapshot`. AdvisoryGraphen itself is Rust, and none of the synthetic dogfood fixtures wire api routes. So that branch contributes **zero** hypotheses across the entire dogfood corpus.

The only fixture that exercises the layer at all is the synthetic `direct-db-access` example written specifically to demonstrate boundary violations. It is not a representative engagement signal; it is a unit-test fixture in `examples/`.

## Implications

1. **Re-prioritise coverage**. `missing_owner` and `requirement_unverified` are the actual signal-bearing obstructions in current dogfood. They must get a hypothesis layer before any further breadth (other obstruction types) or polish (lifecycle CLI symmetry, HG vocabulary issue).

2. **Defer `api_route_missing_auth` polish**. The layer is structurally correct (unit-tested) but is dormant until the snapshot adapter sees TS/JS routes. Adding a Python/Go adapter to surface real signal is bigger scope than improving the hypothesis layer itself.

3. **The "2/7" claim was misleading**. Coverage by obstruction-type-count is not the right denominator; coverage by *occurrence frequency in real fixtures* is. By the latter measure, the hypothesis layer covered 0% of observed obstructions until this finding, and the dogfood corpus was the only way to detect it.

## Revised priority order

Original recommendation was `A → C → D → B → E → F`. After the dogfood finding, the order is:

1. **B-narrow**: hypothesis layer for `missing_owner` and `requirement_unverified` only. These two close the dogfood gap.
2. **C**: lifecycle CLI symmetry (`hypothesis support|accept|reject`).
3. **D**: HG issue draft for argumentation vocabulary.
4. **B-wide**: remaining `missing_evidence`, `circular_dependency`, `insufficient_evidence`.
5. **E**: projection rendering polish.
6. **F**: agent closed-loop. Defer.

## Hypothesis design notes for B-narrow

### `missing_owner`

| # | Hypothesis | Falsifier | Suggests completion |
| --- | --- | --- | --- |
| 1 (primary) | Owner is unassigned because no team currently holds the action | An `owns` incidence from any team or individual cell exists in another revision or external roster | `ownership_clarification`, `owner_assignment` |
| 2 | The most recent contributor is the de-facto owner; the explicit `owns` link is just missing | git history / reviewers / on-call rota assigns this domain to a specific team | `derive_owner_from_history`, `ownership_clarification` |
| 3 | The action belongs to a shared service whose ownership is collective, and a single owner placeholder is incorrect | Service-ownership map records the action as collectively owned with a documented escalation policy | `shared_ownership_policy`, `policy_review` |

### `requirement_unverified`

| # | Hypothesis | Falsifier | Suggests completion |
| --- | --- | --- | --- |
| 1 (primary) | Verification is genuinely missing | A `verifies` or `implements` incidence to any test/metric cell exists | `proposed_test`, `proposed_metric` |
| 2 | Verification exists but the `verifies`/`implements` link was not lifted | Manual review confirms the requirement maps to an existing test/metric cell that lacks the relation | `lift_verification_link`, `requirement_review` |
| 3 | The requirement is exploratory and should be marked as such | Reviewed metadata or stakeholder note confirms the requirement is exploratory | `mark_exploratory_requirement`, `requirement_review` |

## What this finding cost

Total time spent on hypothesis layer before this dogfood run: ~3 sessions.
Time to discover the misalignment: ~10 minutes once dogfood was run.

The lesson is to dogfood *during* implementation, not after. Coverage decisions should follow real obstruction frequency, not invariant ID alphabetical order.

## Resolution status

Updated 2026-05-05 after finishing priorities A-E.

- `missing_owner` and `requirement_unverified` now emit competing hypotheses and falsifiers.
- The three signal-bearing dogfood fixtures now produce 6 hypotheses each: 3 for the ownership obstruction and 3 for the verification obstruction.
- `hypothesis support|falsify|accept|reject` remains review-event driven. Deterministic propagation can reframe candidates and obstructions after those events, but agents do not autonomously propose lifecycle transitions.
- F, the LLM agent closed-loop for proposing hypothesis lifecycle changes, remains deferred until operational evidence from 1-2 real customer engagements clarifies authority boundaries, rollback behavior, and the human-review split.

Updated 2026-05-05 after the first F implementation.

- `hypothesis propose` now implements the closed-loop proposal half without auto-promotion.
- AI/agent observations can be registered as generic structure, for example with `metadata.supports_hypothesis_id` or `metadata.falsifies_hypothesis_id`.
- `hypothesis propose` reads the advisory space plus check report and emits unreviewed lifecycle proposals (`supported`, `falsified`, or `review_conflict`).
- The authority boundary is explicit in the report: proposals cannot apply events. The existing review-gated `hypothesis support|falsify|accept|reject` commands remain the only mutation path.

Updated 2026-05-05 after adding the first autonomy gate.

- `hypothesis apply-proposals` can append `supported` / `falsified` events when a lifecycle proposal passes a conservative autonomy policy.
- The default policy requires candidate hypotheses, confidence >= 0.7, and at least one reviewed/source-backed, test-passed, or runtime-observed signal.
- `review_conflict`, `accept`, and `reject` remain outside autonomous application.
- `--dry-run` reports policy decisions without writing events or moving case `HEAD`.
