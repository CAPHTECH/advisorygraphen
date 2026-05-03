# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository purpose

AdvisoryGraphen is a Rust workspace + CLI that maps consulting work (technical advisory, architecture review, product decisions) onto HigherGraphen primitives. The CLI `advisorygraphen` lifts bounded source snapshots into an *advisory space*, runs invariant checks producing *obstructions*, proposes reviewable *completion candidates*, and emits audience-specific *projections* (`executive`, `developer_action`, `audit_trace`, `ai_agent`, `client_review`, `cli`). Engagements are persisted as an append-only case log; readiness/closeable status is derived by replay, never mutated in place.

The MVP scope is the `technical_advisory_mvp` interpretation package only. Hosted SaaS, UI, and MCP are explicitly deferred.

## Build and test commands

```sh
# Standard workspace checks (must all pass before merge — see .github/workflows/ci.yml)
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Acceptance tests live in a separate Cargo workspace (`tests/advisorygraphen-cli-acceptance/`)
# and must be invoked with --manifest-path:
cargo test --manifest-path tests/advisorygraphen-cli-acceptance/Cargo.toml

# Run a single library/integration test in a workspace crate:
cargo test -p advisorygraphen-reasoning invariants::test_name

# Run a single CLI acceptance test:
cargo test --manifest-path tests/advisorygraphen-cli-acceptance/Cargo.toml -- exact_test_name

# Quick end-to-end smoke (also run by CI as the fixture validation step):
cargo run -q -p advisorygraphen-cli -- validate \
  --input examples/technical-advisory/direct-db-access/advisory.input.json --format json
```

The CLI binary is `advisorygraphen` from `tools/advisorygraphen-cli`. The full golden flow (`lift → check → completions propose → project`) for the `direct-db-access` example is documented in `docs/13-testing-acceptance.md` and is the v0.1.0 acceptance gate.

## External path dependency

The workspace depends on the `higher-graphen` crates via **path** dependencies at `../higher-graphen/...` (see `Cargo.toml`). CI clones `https://github.com/CAPHTECH/higher-graphen` into `../higher-graphen` before building. Local development requires the same sibling checkout — the build will fail without it. Do not switch to crates.io or git deps unilaterally.

## Architecture

Crate dependency direction (lower never depends on higher):

```
advisorygraphen-cli  (tools/)
  → advisorygraphen-runtime    workflow coordinators + file I/O + case store
    → advisorygraphen-lift          snapshot → AdvisorySpaceEnvelope
    → advisorygraphen-interpretation packages (vocabulary, invariant templates, projection templates)
    → advisorygraphen-reasoning     invariants → obstructions → completion candidates → close-check
    → advisorygraphen-projection    audience-specific views (executive / developer_action / audit_trace / ai_agent / client_review / cli)
      → advisorygraphen-core        DTOs, IDs, validation, report envelope, error policy
        → higher-graphen-* (core, structure, evidence, reasoning, projection, interpretation)
```

Hard architectural rules (see `docs/02-rust-workspace.md`):

- `advisorygraphen-core` must not contain LLM calls, filesystem command logic, or provider SDKs.
- Projection crate emits views only; task-management integrations belong in separate adapters.
- Customer-specific interpretation packages live outside this public workspace (see `docs/15-commercial-boundary.md`).

## Domain invariants you must preserve

These are *load-bearing* for the product, not stylistic preferences:

1. **Confidence is not review.** Any AI- or rule-generated recommendation enters the system as `CompletionCandidate` with `review_status = unreviewed`. Promotion to accepted structure requires an explicit review event — never high confidence (ADR 0002).
2. **Accepted review ≠ structural application.** Accepting a candidate appends a review event. The obstruction is *not* resolved until the cells/incidences listed in `blocker_resolution_state.application_requirements` are applied and `check`/`case reason` rerun.
3. **Projection loss is required.** Every projection must include represented IDs, omitted IDs, and information-loss records. Markdown is rendered *from* projection JSON, not separately (ADR 0004).
4. **Append-only case log.** Materialized space, readiness, frontier, blockers, and `closeable` are *derived by replay* of the case log. Do not introduce mutate-in-place state (ADR 0003).
5. **Source boundary is explicit.** `bounded source snapshot` carries `included_source_ids`, `excluded_summary`, `extraction_loss`. Don't widen ingestion silently — bounded snapshots are the contract.
6. **Public/private boundary.** Customer engagement snapshots, real reports, case logs, prompt corpora, and proprietary interpretation packages do not belong in this repo. Examples must stay synthetic (`docs/15-commercial-boundary.md`, ADR 0005).

## Error and exit-code policy

Domain findings are not tool failures (see `docs/08-cli-contract.md`):

| Exit | Meaning |
| --- | --- |
| `0` | Success — *including* obstructions, missing evidence, unreviewed candidates, projection loss, uncovered requirements |
| `1` | Validation / schema error |
| `2` | CLI usage error |
| `3` | I/O error |
| `4` | Unsupported package, ruleset, or audience |
| `5` | Stale revision / optimistic concurrency failure |
| `6` | Explicit `--fail-on` threshold triggered |
| `101` | Internal panic, converted to structured error |

Never coerce a domain finding into a non-zero exit unless the user passes `--fail-on`.

## Determinism

JSON reports must be byte-deterministic across runs (sorted by stable IDs). Snapshot tests in `tests/` and the CLI acceptance crate depend on this. When adding output, use `IndexMap` + sorted iteration rather than `HashMap`.

## Schemas are the contract

`schemas/advisorygraphen/*.schema.json` are the stable wire contract for CLI ↔ agents ↔ external tools. Markdown output is *not* a contract. Schema-breaking changes require a versioned schema id (e.g. `advisorygraphen.space.v2`) and migration notes.

## Repo-specific conventions

- **400-line file cap.** CI fails any `.rs` file over 400 lines under `crates/`, `tools/`, or `tests/advisorygraphen-cli-acceptance/tests/`. Split modules rather than fighting this.
- **No customer data, ever.** Examples under `examples/` are synthetic. Dogfood fixtures (`examples/dogfood/*`) review AdvisoryGraphen against itself and must remain reproducible from `advisorygraphen dogfood repo-snapshot`.
- **Skill file mirrors CLI.** `skills/advisorygraphen/SKILL.md` is the agent-facing contract; when CLI commands or arguments change, update the skill file in the same change.
- **Read order for new contributors:** `docs/00` → `01` → `02` → `03` → `05` → `06` → `07` → `08` → `12` → `13` (see README).

## Repo-local rules (from user global instructions)

- Do not implement fallback handling unless explicitly instructed.
- Backward compatibility is not required unless explicitly instructed.
- Prefer "do nothing → delete → consolidate → make configurable → minimal implementation" — adding code is the last resort.
- No copy-paste duplication. Extract shared logic when the same pattern appears 2+ times.
- Do not add unrequested features (TTL, retry, custom error hierarchies, logging frameworks, etc.).
