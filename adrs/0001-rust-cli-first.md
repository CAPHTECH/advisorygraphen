# ADR 0001: Rust CLI and JSON Schema First

## Status

Accepted for MVP.

## Decision

AdvisoryGraphen will begin as a Rust workspace with `advisorygraphen` CLI, deterministic JSON reports, and JSON schemas. Hosted service, UI, MCP, and marketplace packaging are postponed.

## Rationale

The core risk is not UI. The core risk is whether consulting material can be lifted into stable structure, checked by invariants, produce obstructions, propose candidates, and generate projections without losing evidence and review boundaries.

CLI + JSON makes this testable, scriptable, and agent-usable.

## Consequences

- File-based examples become the first contract.
- CI can validate report determinism.
- External integrations consume projection JSON.
- UI can be added later without changing the model.
