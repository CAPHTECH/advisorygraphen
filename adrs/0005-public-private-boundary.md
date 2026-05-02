# ADR 0005: Public Core, Private Interpretation Packages

## Status

Accepted.

## Decision

Generic AdvisoryGraphen crates, schemas, docs, and examples may be public. Customer-specific data, production interpretation packages, hosted infrastructure, and commercial evaluation assets remain private unless intentionally open-sourced.

## Rationale

The reusable model benefits from openness, but consulting value often lives in domain rules, customer context, and operational workflows.

## Consequences

- Synthetic fixtures only in public examples.
- Private package repositories are expected.
- Commercial differentiation accumulates in interpretation packages and hosted workflows.
