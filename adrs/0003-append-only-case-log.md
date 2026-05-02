# ADR 0003: Append-only Case Log for Engagement Memory

## Status

Accepted for case-log mode.

## Decision

Engagement evolution is persisted as an append-only log of morphisms and review events. Materialized space, readiness, blockers, and close status are replay-derived.

## Rationale

Consulting engagements require traceability. Mutating task state in place loses why a decision changed, who accepted a candidate, and which evidence was available at the time.

## Consequences

- Accept/reject records are durable.
- Stale revisions can be detected.
- Audit projection can explain history.
- Store implementation is more complex than simple CRUD but safer for advisory work.
