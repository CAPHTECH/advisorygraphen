# AdvisoryGraphen Docs Manifest

## Root

- `Cargo.toml`: Rust workspace definition for crates and CLI.
- `Cargo.lock`: locked dependency set for reproducible CLI builds.
- `README.md`: overview, scope, first commands, design principles.
- `MANIFEST.md`: this file.
- `.github/workflows/ci.yml`: fmt, clippy, tests, acceptance tests, fixture validation.

## Implementation

- `crates/advisorygraphen-core`: shared DTOs, IDs, validation, report envelope, error policy.
- `crates/advisorygraphen-interpretation`: `technical_advisory_mvp` package metadata.
- `crates/advisorygraphen-lift`: JSON snapshot to advisory space lift workflow.
- `crates/advisorygraphen-reasoning`: invariant checks, obstructions, completion candidates, close status.
- `crates/advisorygraphen-projection`: executive, developer, audit, AI, and Todoist projections.
- `crates/advisorygraphen-runtime`: file workflows and local append-only case store.
- `tools/advisorygraphen-cli`: `advisorygraphen` command-line binary.
- `tests/advisorygraphen-cli-acceptance`: black-box CLI acceptance tests.

## Docs

- `docs/00-product-charter.md`: product purpose, non-goals, MVP success criteria.
- `docs/01-domain-model.md`: AdvisoryGraphen concepts mapped to HigherGraphen primitives.
- `docs/02-rust-workspace.md`: Rust workspace layout, crate boundaries, dependency direction.
- `docs/03-data-contracts.md`: schema contracts and report envelope.
- `docs/04-source-adapters.md`: bounded source snapshot and adapter contracts.
- `docs/05-reasoning-invariants.md`: invariant set, obstruction policy, reasoning API.
- `docs/06-completion-and-review-workflow.md`: completion lifecycle and review event workflow.
- `docs/07-projections.md`: executive, developer, audit, AI, Todoist projection contracts.
- `docs/08-cli-contract.md`: implementable `advisorygraphen` CLI contract.
- `docs/09-agent-integration-and-skill.md`: agent-facing workflow and future MCP map.
- `docs/10-storage-case-log.md`: append-only case log and derived readiness.
- `docs/11-security-governance.md`: customer-data, AI, projection, and Todoist governance.
- `docs/12-implementation-roadmap.md`: phase-by-phase implementation plan.
- `docs/13-testing-acceptance.md`: tests, golden scenario, acceptance criteria.
- `docs/14-todoist-projection.md`: Todoist task-export projection.
- `docs/15-commercial-boundary.md`: open/public vs private/commercial boundary.
- `docs/16-rust-api-sketches.md`: Rust type and trait sketches.
- `docs/99-source-alignment.md`: traceability to HigherGraphen source documents.

## ADRs

- `adrs/0001-rust-cli-first.md`
- `adrs/0002-reviewable-completions.md`
- `adrs/0003-append-only-case-log.md`
- `adrs/0004-projection-loss-required.md`
- `adrs/0005-public-private-boundary.md`

## Schemas

- `schemas/advisorygraphen/engagement.snapshot.schema.json`
- `schemas/advisorygraphen/advisory.space.schema.json`
- `schemas/advisorygraphen/advisory.report.schema.json`
- `schemas/advisorygraphen/projection.request.schema.json`
- `schemas/advisorygraphen/review.event.schema.json`
- `schemas/advisorygraphen/todoist.projection.schema.json`

## Examples

- `examples/technical-advisory/direct-db-access/advisory.input.json`
- `examples/technical-advisory/direct-db-access/expected.check.report.json`
- `examples/technical-advisory/direct-db-access/expected.completion.report.json`
- `examples/technical-advisory/direct-db-access/expected.todoist.projection.json`

## Skill

- `skills/advisorygraphen/SKILL.md`
