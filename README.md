# AdvisoryGraphen

AdvisoryGraphen is a Rust CLI for evidence-backed consulting and advisory workflows on HigherGraphen.

It turns bounded source material such as strategy notes, operating constraints, architecture notes, ADRs, issue summaries, interview notes, requirements, and verification records into a structured advisory space. From that structure it can detect obstructions, propose reviewable completions, keep case history append-only, and generate audience-specific projections.

The released CLI is intentionally file-based and deterministic. It is not a hosted SaaS, a general task manager, or an AI system that finalizes consulting decisions without review.

The first released interpretation package is `technical_advisory_mvp`, which focuses on technical advisory, architecture review, and product/development decision support. The underlying model is broader: other consulting domains can be supported by adding domain-specific interpretation packages, invariants, and projection policies.

## Install

The crates.io package is `advisorygraphen-cli`. The installed command is `advisorygraphen`.

```sh
cargo install advisorygraphen-cli
advisorygraphen version
```

Current release: `0.1.3`.

For local development from this repository:

```sh
cargo run -q -p advisorygraphen-cli -- version
```

## Quick Start

For a small AI answer, note, issue, or PR comment, use `micro review` first. It
does not require a snapshot or advisory space:

```sh
advisorygraphen micro review \
  --input ai-answer.txt \
  --output /tmp/micro-review.report.json \
  --format json
```

The report flags unsupported strong claims, assumptions, structure error risk,
falsification checks, missing checks, alternative hypotheses, and whether the
input should escalate to the full workflow.

Run the released workflow against the included `technical_advisory_mvp` fixture.

```sh
advisorygraphen validate \
  --input examples/technical-advisory/direct-db-access/advisory.input.json \
  --format json

advisorygraphen lift \
  --input examples/technical-advisory/direct-db-access/advisory.input.json \
  --package technical_advisory \
  --output /tmp/advisory.space.json \
  --format json

advisorygraphen check \
  --space /tmp/advisory.space.json \
  --ruleset technical_advisory_mvp \
  --output /tmp/advisory.check.report.json \
  --format json

advisorygraphen completions propose \
  --space /tmp/advisory.space.json \
  --from-report /tmp/advisory.check.report.json \
  --output /tmp/advisory.completions.report.json \
  --format json

advisorygraphen project \
  --space /tmp/advisory.space.json \
  --report /tmp/advisory.check.report.json \
  --completions-report /tmp/advisory.completions.report.json \
  --audience executive \
  --format markdown \
  --output /tmp/executive-review.md
```

For agent operations, generate the `ai_agent` projection as the resume protocol:

```sh
advisorygraphen project \
  --space /tmp/advisory.space.json \
  --report /tmp/advisory.check.report.json \
  --completions-report /tmp/advisory.completions.report.json \
  --audience ai_agent \
  --format json \
  --output /tmp/ai-agent.json
```

## Core Workflow

AdvisoryGraphen uses this model:

```text
bounded source snapshot
  -> advisory space
  -> invariant check report
  -> reviewable completion candidates
  -> audience-specific projections
  -> append-only case log and case reasoning
```

Key commands:

| Command | Purpose |
| --- | --- |
| `validate` | Validate a snapshot, advisory space, report, projection request, or review event. |
| `lift` | Convert a bounded source snapshot into an advisory space. |
| `check` | Evaluate advisory invariants and emit obstructions. |
| `micro review` | Review small text inputs for claims, assumptions, evidence gaps, structure error risk, falsification checks, missing checks, and escalation need. |
| `completions propose` | Generate reviewable completion candidates from obstructions. |
| `completions dry-run` | Apply candidates in memory and rerun checks without changing a case store. |
| `project` | Render a projection for a specific audience. |
| `case import` | Import a space into an append-only local case store. |
| `case reason` | Replay case state and derive readiness, blockers, frontier, and close status. |
| `case close-check` | Check whether a case can be closed at a specific revision. |
| `hypothesis propose` | Propose reviewable hypothesis lifecycle transitions from source-backed signals. |
| `observation record` | Record bounded observation results before supporting or falsifying hypotheses. |
| `dogfood repo-snapshot` | Generate a bounded snapshot of this repository for self-review. |
| `code repo-snapshot` | Generate a bounded lexical code snapshot for code-derived advisory signals. |

Run `advisorygraphen --help` or `advisorygraphen <command> --help` for the current CLI surface.

## Core Ideas

AdvisoryGraphen is built around a few conceptual commitments:

- Reports are projections, not the source of truth. The source of truth is the structured advisory space plus the append-only case log.
- Source material is bounded input, not automatic truth. Claims, evidence, AI inferences, and reviewed conclusions stay separate.
- Advisory work should be hypothesis-first, not conclusion-first. Proposals are derived from hypotheses and observations, and unsupported hypotheses remain visible.
- AI-generated structure is review-gated. A candidate can be useful, concrete, and source-backed without being accepted.
- Obstructions are first-class consulting objects. They represent what prevents a decision, recommendation, architecture, plan, or case from being safely closed.
- Projection loss must be explicit. Audience-specific summaries are useful only when they disclose what they omit, collapse, or cannot prove.
- Domain knowledge belongs in interpretation packages. `technical_advisory_mvp` is the first package, not the limit of the model.

In short, AdvisoryGraphen treats consulting work as structured evidence, hypotheses, constraints, obstructions, proposals, review events, and projections rather than as a single report document.

## Projection Audiences

`project` supports these audiences:

| Audience | Use |
| --- | --- |
| `executive` | Decision-focused summary for leadership review. |
| `developer_action` | Implementation-oriented actions and blockers. |
| `audit_trace` | Evidence, provenance, review status, and projection loss. |
| `ai_agent` | Agent resume protocol, allowed commands, forbidden operations, review gates, observation actions, and remaining blockers. |
| `client_review` | Client-facing executive-style review. |
| `cli` | CLI-oriented executive-style view. |

Every projection is intentionally lossy. Use `audit_trace` or `ai_agent` when you need to inspect represented IDs, omitted information, projection loss metrics, schema morphisms, or review state.

## Review-Gated Advisory

AdvisoryGraphen separates facts, claims, AI inferences, hypotheses, obstructions, completion candidates, review events, and projections.

Completion candidates are proposals. They are not accepted structure until explicit review events and supported application steps exist. Accepted completion review is also separate from materializing the candidate into the advisory space.

Important review commands:

```sh
advisorygraphen completions accept \
  --store .advisorygraphen/store \
  --candidate-id candidate:example \
  --from-report /tmp/advisory.completions.report.json \
  --reviewer reviewer:cto \
  --reason "Accepted target direction" \
  --base-revision revision:technical-advisory-smoke-1 \
  --format json

advisorygraphen completions apply-accepted \
  --store .advisorygraphen/store \
  --space-id space:advisory:technical-advisory-direct-db-access \
  --reviewer ai-agent:codex \
  --reason "Apply reviewed accepted completion candidates" \
  --base-revision revision:review-000002 \
  --format json
```

The `ai_agent` projection and `case reason` output should be treated as the operational contract for agents. They expose review gates, observation actions, candidate review state, blocker resolution requirements, projection loss metrics, and safe next commands.

## Code-Derived Snapshot

The `code repo-snapshot` adapter creates a bounded lexical snapshot from a local repository. The initial adapter targets deterministic TypeScript, JavaScript, and Next.js signals such as `package.json`, `tsconfig.json`, API route files, test files, database access patterns, and `process.env.*` usage.

```sh
advisorygraphen code repo-snapshot \
  --repo . \
  --output /tmp/advisory-code.input.json \
  --format json
```

This adapter is intentionally conservative. It does not resolve TypeScript types or runtime control flow. Use observations and review events to record evidence that the lexical adapter cannot prove.

## Public and Private Boundary

The public release contains:

- Rust core types and CLI workflows.
- Stable JSON schemas.
- Generic `technical_advisory_mvp` package behavior for the first technical advisory use case.
- Synthetic examples and dogfood fixtures.
- Public documentation and the agent skill.

Keep these outside the public repository and package:

- Customer source material and real engagement case logs.
- Customer-specific invariants or interpretation packages.
- Commercial templates, private benchmarks, pricing, sales, and support playbooks.
- Production infrastructure or hosted-service secrets.

Do not publish real customer data through examples, fixtures, projections, or case logs.

## Repository Map

| Path | Purpose |
| --- | --- |
| `tools/advisorygraphen-cli` | `advisorygraphen` command-line binary. |
| `crates/advisorygraphen-core` | Shared DTOs, IDs, validation, report envelopes, and error policy. |
| `crates/advisorygraphen-lift` | Snapshot-to-space lift workflow. |
| `crates/advisorygraphen-reasoning` | Invariant checks, obstructions, completions, and close status. |
| `crates/advisorygraphen-projection` | Executive, developer, audit, and AI projections. |
| `crates/advisorygraphen-runtime` | File workflows and local append-only case store. |
| `schemas/advisorygraphen` | JSON schema contracts. |
| `examples` | Synthetic advisory and dogfood fixtures. |
| `skills/advisorygraphen` | Agent-facing operating guidance. |
| `docs` | Product, domain, CLI, storage, security, and testing documentation. |
| `adrs` | Architecture decision records. |

See `MANIFEST.md` for the full document map.

## Development

Run the main local checks:

```sh
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace
cargo test --manifest-path tests/advisorygraphen-cli-acceptance/Cargo.toml
```

Release packaging check:

```sh
cargo package --workspace
```

## More Documentation

- `CHANGELOG.md`: release history.
- `docs/00-product-charter.md`: product purpose, users, boundaries, and non-goals.
- `docs/03-data-contracts.md`: JSON schema and report contracts.
- `docs/05-reasoning-invariants.md`: invariant and obstruction policy.
- `docs/06-completion-and-review-workflow.md`: completion candidate lifecycle.
- `docs/07-projections.md`: projection contracts and projection loss.
- `docs/08-cli-contract.md`: full CLI contract and exit code policy.
- `docs/09-agent-integration-and-skill.md`: agent operation model.
- `docs/10-storage-case-log.md`: append-only case log design.
- `docs/11-security-governance.md`: data and projection governance.
