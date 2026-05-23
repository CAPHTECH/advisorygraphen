# PR Snapshot Modeling

Use this reference when creating an AdvisoryGraphen snapshot for human review focus. The key distinction is that a review target is a verification requirement, not a task to execute.

## Minimal Shape

```json
{
  "schema_version": "advisorygraphen.input.v1",
  "case_id": "pr-review-example",
  "records": [
    {
      "id": "src-runtime-diff",
      "record_type": "source",
      "title": "Runtime diff",
      "body": "Changed completion apply semantics and gluing policy output.",
      "source_ref": "git diff -- crates/advisorygraphen-runtime/src/lib.rs"
    },
    {
      "id": "req-runtime-policy-review",
      "record_type": "requirement",
      "title": "Review runtime gluing policy semantics",
      "body": "A human should verify that policy overrides are emitted only when gluing blockers exist, and that apply output does not imply review was required when no blockers were present.",
      "require_verification": true,
      "links": [
        {
          "type": "evidence",
          "target": "src-runtime-diff"
        }
      ]
    }
  ]
}
```

## Modeling Rules

- Use `requirement` for each human review focus.
- Use `source` for each changed file group or command output.
- Use `observation` for tests, static checks, or manual inspection already performed.
- Use `decision` for explicit design choices such as "break compatibility to adopt HigherGraphen 0.5.0".
- Use `risk` for plausible regressions that are not yet demonstrated.
- Use `action` only when someone must perform a concrete follow-up task and the graph can represent that ownership.

## Corrected Pattern

Good:

```json
{
  "id": "req-projection-review",
  "record_type": "requirement",
  "title": "Review AI-agent projection correspondence output",
  "body": "A human should verify that correspondence candidates are useful review guidance and do not overwhelm the projection.",
  "require_verification": true
}
```

Bad:

```json
{
  "id": "act-review-projection",
  "record_type": "action",
  "title": "Review AI-agent projection correspondence output"
}
```

The bad shape creates a workflow problem rather than a review signal because unowned actions are expected to have an owner.

## Useful Review Buckets

Create one requirement per review bucket:

- runtime behavior and policy semantics
- projection shape and AI-agent output volume
- schema and CLI contract changes
- examples and dogfood outputs
- documentation and skill contract
- dependency or lockfile compatibility
- tests and acceptance coverage

For AI-generated code, also create boundary/contract requirements:

- AI authority boundary
- persistence boundary
- evidence-to-fact promotion boundary
- public output or schema boundary
- dependency/version boundary

## Interpreting Results

Expected findings:

- `requirement_unverified`: the human review focus remains open
- follow-up completion proposals: suggested evidence to reduce uncertainty
- correspondence `review_candidate`: potentially useful structural link for reviewer attention

Method defects:

- `missing_owner` on review focus records
- no open findings despite unresolved review requirements
- generic proposals that do not point to changed files or changed contracts
- projection loss hiding the records that justify the priorities
