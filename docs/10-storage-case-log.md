# 10. Storage and Case Log

## Principle

AdvisoryGraphen should use file-based JSON for MVP. Durable case evolution should be append-only. Readiness, frontier, blockers, and closeable state are derived projections, not mutable task fields.

## MVP storage modes

| Mode | Description |
| --- | --- |
| `file` | Read one input JSON and write reports. No persistence. |
| `case-log` | Append review and morphism events to a local store. |
| `hosted` | Future service; not MVP. |

## Local store layout

```text
.advisorygraphen/store/
  spaces/
    space-advisory-technical-smoke/
      snapshots/
      materialized/
      logs/
        morphism-log.jsonl
      reports/
      projections/
      reviews/
```

## Case log entry

```json
{
  "schema": "advisorygraphen.case.log.entry.v1",
  "case_space_id": "space:advisory:technical-smoke",
  "sequence": 1,
  "entry_id": "log:000001",
  "morphism_id": "morphism:accept-candidate-billing-status-api",
  "source_revision_id": "revision:technical-advisory-smoke-1",
  "target_revision_id": "revision:technical-advisory-smoke-2",
  "actor": "reviewer:cto",
  "recorded_at": "2026-05-02T00:00:00Z",
  "previous_entry_hash": null,
  "entry_hash": "optional-content-hash",
  "payload": {}
}
```

## Morphism types

| Type | Description |
| --- | --- |
| `import` | Import initial advisory space |
| `review_accept` | Accept candidate, evidence, claim, or waiver |
| `review_reject` | Reject candidate or claim |
| `evidence_attach` | Attach source-backed evidence |
| `completion_accept` | Accept and optionally promote completion candidate |
| `completion_reject` | Reject completion candidate |
| `projection` | Record generated projection |
| `policy_update` | Update engagement policy |
| `schema_migration` | Migrate schema |
| `close` | Close engagement after close-check |
| `reopen` | Reopen engagement |

## Materialization

The materialized space is replay output.

```text
morphism-log.jsonl
  -> replay
  -> materialized advisory space
  -> derived readiness / blockers / close status
  -> projection cache
```

If cache and log disagree, log replay wins.

## Derived views

| View | Derivation |
| --- | --- |
| `ready_items` | hard dependencies satisfied, required evidence accepted/source-backed, required review accepted |
| `blocked_items` | hard obstruction exists |
| `frontier_items` | ready items with no completed downstream work |
| `waiting_items` | blocked only by external wait or missing owner |
| `close_status` | close invariants satisfied and medium-or-higher obstructions resolved |
| `evolution_report` | differences between revisions |

## Close check

An engagement can close only when:

1. no critical or high unresolved hard obstruction remains;
2. accepted recommendations have source-backed or review-promoted evidence;
3. completion candidates required for close are accepted, rejected, waived, or superseded;
4. projection loss for final reports is disclosed;
5. all required review events exist;
6. final audit projection can be generated.

## Concurrency

Commands that append to the log should accept `--base-revision`. If the store head changed, the command must fail with stale revision error.

## Storage future

After MVP:

- SQLite for local structured query
- PostgreSQL for hosted service
- object storage for source artifacts
- vector database only for retrieval, never as source of truth
- graph database optional, not required for core semantics
