# 05. Reasoning, Invariants, and Obstructions

## Reasoning engine responsibilities

The reasoning engine evaluates AdvisoryGraphen structures under an interpretation package. It emits domain findings as successful report data.

It must produce:

- invariant results
- obstructions
- evidence sufficiency results
- review requirement results
- completion candidate triggers
- close-check results

## MVP invariant set

### `recommendation_requires_evidence`

A recommendation, decision, or action that appears in an executive or developer projection must either:

1. be supported by source-backed or review-promoted evidence; or
2. be explicitly marked as unreviewed hypothesis or completion candidate.

Violation creates:

- `obstruction_type = missing_evidence`
- recommended completion: `missing_evidence` or `review_required`

### `accepted_candidate_requires_review`

A completion candidate cannot become accepted structure without an explicit review event or review morphism.

Violation creates:

- `obstruction_type = review_required`
- recommended completion: `missing_review`

### `blocking_obstruction_prevents_close`

An engagement cannot close while high-severity hard obstructions are unresolved, unless a policy-specific waiver exists.

Violation creates:

- `obstruction_type = impossible_closure`
- recommended completion: `resolve_obstruction` or `waiver_review`

### `action_requires_owner`

A developer action candidate must have an owner cell or owner placeholder.

Violation creates:

- `obstruction_type = missing_owner`
- recommended completion: `ownership_clarification`

### `action_requires_success_metric`

A strategic or technical action above medium severity must have at least one success metric or verification method.

Violation creates:

- `obstruction_type = missing_verification`
- recommended completion: `define_metric` or `define_test`

### `requirement_requires_verification`

A requirement must map to at least one implementation/design element and one verification method, unless the requirement is explicitly exploratory.

Violation creates:

- `obstruction_type = requirement_unverified`
- recommended completion: `proposed_test`, `proposed_metric`, or `requirement_review`

### `architecture_no_cross_context_direct_database_access`

A component must not directly access a data store owned by another technical context unless an explicit exception policy exists.

Violation creates:

- `obstruction_type = boundary_violation`
- recommended completion: `proposed_interface`, `proposed_refactor_action`, or `exception_review`
- obstruction IDs and completion candidates are derived from the violating
  `from` cell, `to` data store, incidence evidence, and source IDs. The
  fallback `decision:approve-current-architecture` blocked ID is only used when
  the source incidence does not provide explicit `metadata.blocked_ids`.

### `projection_loss_declared`

Every projection must declare omitted source IDs, compressed structures, hidden internal policy, and summarized evidence when applicable.

Violation creates:

- `obstruction_type = projection_loss`
- recommended completion: `projection_disclosure`

### `context_mapping_required_for_equivalence`

Terms from different contexts cannot be treated as identical without an equivalence claim that states criteria, scope, evidence, and loss.

Violation creates:

- `obstruction_type = correspondence_mismatch`
- recommended completion: `equivalence_review`

### `ai_inference_cannot_satisfy_hard_evidence`

Inferred evidence cannot satisfy hard evidence requirements unless review-promoted.

Violation creates:

- `obstruction_type = insufficient_evidence`
- recommended completion: `review_promote_evidence` or `source_backed_evidence`

## Obstruction shape

An obstruction must answer:

1. Which invariant or policy failed?
2. Which cells, contexts, incidences, or morphisms are involved?
3. Which evidence or witness supports the finding?
4. Is the severity informational, low, medium, high, or critical?
5. Is it blocking close, blocking projection, blocking action export, or merely diagnostic?
6. Which completion candidate types could resolve it?
7. What review status does the obstruction have?

## Severity policy

| Severity | Meaning |
| --- | --- |
| `info` | Diagnostic only |
| `low` | Should be visible but does not block MVP workflows |
| `medium` | Blocks acceptance of related action unless reviewed |
| `high` | Blocks engagement close and accepted recommendation |
| `critical` | Blocks projection/export unless explicitly waived |

## Reasoning API sketch

```rust
pub trait AdvisoryInvariantEvaluator {
    fn invariant_id(&self) -> &'static str;

    fn evaluate(
        &self,
        space: &AdvisorySpaceEnvelope,
        policy: &AdvisoryPolicy,
    ) -> AdvisoryResult<Vec<InvariantCheckResult>>;
}

pub trait ObstructionEmitter {
    fn emit(
        &self,
        result: &InvariantCheckResult,
        space: &AdvisorySpaceEnvelope,
    ) -> AdvisoryResult<Vec<AdvisoryObstruction>>;
}
```

## Check workflow

```text
load advisory.space.v1
  -> validate schema
  -> load interpretation package
  -> load policy defaults
  -> evaluate invariants
  -> emit obstructions
  -> emit evidence sufficiency results
  -> create check report envelope
  -> optionally project
```

## Domain findings are successful results

`advisorygraphen check` exits `0` when it finds obstructions. The tool succeeded because it detected domain facts. CI users may request a non-zero exit only with an explicit flag such as `--fail-on high`.

## Determinism

The reasoning engine must sort output by stable IDs and rule order. Reports must be deterministic to support snapshot tests.
