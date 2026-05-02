# ADR 0002: Completion Candidates Are Reviewable, Not Accepted

## Status

Accepted.

## Decision

Any AI-generated or rule-generated recommendation, action, interface, test, evidence request, or task proposal starts as `CompletionCandidate` with `review_status = unreviewed`.

## Rationale

Consulting recommendations can affect architecture, cost, staffing, and business decisions. Confidence scores are not sufficient to make them accepted decisions.

## Consequences

- Candidate promotion requires explicit review event.
- Todoist export excludes unreviewed candidates unless draft export policy allows it.
- Audit projection must show candidate lifecycle.
- Executive projection must not present unreviewed candidate as final recommendation.
