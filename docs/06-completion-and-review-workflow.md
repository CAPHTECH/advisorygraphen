# 06. Completion and Review Workflow

## Principle

Completion candidates are proposals for missing or corrective structure. They are not accepted changes, approved tasks, or facts.

## Candidate types

| Candidate type | Description |
| --- | --- |
| `missing_evidence` | Add source-backed evidence for a claim, recommendation, or decision |
| `missing_review` | Ask a human or policy-approved reviewer to accept/reject a candidate |
| `ownership_clarification` | Identify owner or responsible team |
| `proposed_interface` | Add API, event, contract, or boundary interface |
| `proposed_refactor_action` | Change architecture or implementation |
| `proposed_test` | Add verification test |
| `define_metric` | Add success metric or verification metric |
| `projection_disclosure` | Add projection loss disclosure |
| `equivalence_review` | Review whether two context-specific meanings can be treated as equivalent |
| `waiver_review` | Explicitly waive a hard requirement under policy |
| `replacement_morphism` | Replace an unsafe or invalid morphism |

## Candidate lifecycle

```text
proposed
  -> needs_review
  -> accepted | rejected | waived | superseded
  -> reopened
```

A generated candidate starts as:

```json
{
  "review_status": "unreviewed",
  "lifecycle": "proposed"
}
```

## Accept workflow

Accepting a candidate must append a review event. In case-log mode, it must append a review morphism. It must not mutate the candidate in place without history.

```sh
advisorygraphen completions accept \
  --store .advisorygraphen/store \
  --candidate-id candidate:billing-status-api \
  --from-report advisory.completions.report.json \
  --reviewer reviewer:cto \
  --reason "Accepted as target architecture direction" \
  --base-revision revision:2026-05-02T00-00-00Z \
  --format json
```

## Reject workflow

Rejecting a candidate records reason and leaves the original candidate visible in audit projection.

```sh
advisorygraphen completions reject \
  --store .advisorygraphen/store \
  --candidate-id candidate:billing-status-api \
  --from-report advisory.completions.report.json \
  --reviewer reviewer:cto \
  --reason "Billing service ownership will be redesigned first" \
  --format json
```

The source completion report is required. The event metadata includes the
HigherGraphen `CompletionReviewRecord`, preserving the candidate snapshot and
creating a separate accepted or rejected result without mutating the candidate
itself.
The event `engagement_id` is copied from the imported case space so replayed
review decisions remain tied to the original advisory engagement.

## Review event fields

```json
{
  "schema": "advisorygraphen.review.event.v1",
  "review_event_id": "review:accept-candidate-billing-status-api",
  "engagement_id": "engagement:acme-technical-advisory",
  "target_ids": ["candidate:billing-status-api"],
  "outcome": "accepted",
  "reviewer_id": "reviewer:cto",
  "reviewed_at": "2026-05-02T00:00:00Z",
  "reason": "Accepted as target architecture direction.",
  "evidence_ids": ["cell:evidence-cto-review"],
  "base_revision_id": "revision:technical-advisory-smoke-1",
  "metadata": {}
}
```

## Candidate-to-structure promotion

When a candidate is accepted, promotion is a separate transformation.

```text
candidate:billing-status-api
  --review morphism accepted-->
review:accept-candidate-billing-status-api
  --promotion morphism-->
cell:billing-status-api + incidence:order-service-uses-billing-api
```

This separation prevents silent promotion.

The generic case-store command for promotion is:

```sh
advisorygraphen completions apply-accepted \
  --store .advisorygraphen/store \
  --space-id space:technical-advisory-smoke \
  --reviewer ai-agent:codex \
  --reason "Apply reviewed accepted completion candidates" \
  --base-revision revision:review-000002 \
  --format json
```

Automatic promotion is intentionally narrow. It currently supports:

| Candidate type | Required structure applied |
| --- | --- |
| `ownership_clarification` | placeholder `owner` cell and `owns` incidence |
| `proposed_test` | placeholder `test_or_verification` cell and `verifies` incidence |

Other accepted candidates remain pending structural application until a specific
generic materialization rule exists. After application, the agent must rerun
case reasoning or inspect `post_apply_case_reason` before treating the blocker
as resolved.

## Review policy

| Target | Required review |
| --- | --- |
| Executive recommendation | human reviewer |
| Developer action export | human reviewer or explicit draft export policy |
| High-severity waiver | named accountable reviewer |
| AI-inferred evidence promotion | reviewer with evidence authority |
| Projection loss waiver | reviewer with publication authority |

## Completion generation API sketch

```rust
pub trait CompletionRule {
    fn rule_id(&self) -> &'static str;

    fn propose(
        &self,
        space: &AdvisorySpaceEnvelope,
        obstructions: &[AdvisoryObstruction],
        policy: &AdvisoryPolicy,
    ) -> AdvisoryResult<Vec<AdvisoryCompletionCandidate>>;
}
```

## Audit requirements

The audit projection must show:

- candidate creation source
- generated or human-authored origin
- confidence
- review status
- accept/reject/waive reason
- reviewer ID
- evidence IDs
- promoted structure IDs
- projection loss

## Safety rule

Never let `confidence >= threshold` automatically accept a candidate. Confidence can rank candidate review priority, but cannot replace review.
