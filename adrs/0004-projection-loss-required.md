# ADR 0004: Projection Loss Is Required

## Status

Accepted.

## Decision

Every projection must include represented IDs, omitted IDs, and information loss records.

## Rationale

Executive summaries, developer tasks, and AI views necessarily omit information. Without explicit loss records, users may over-trust simplified views.

## Consequences

- Projection generation fails or emits obstruction if loss is omitted.
- Markdown output must be generated from projection JSON.
- Audit projection must preserve loss records.
