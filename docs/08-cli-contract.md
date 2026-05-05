# 08. CLI Contract

## Binary

`advisorygraphen`

## General rules

1. All report-producing commands support `--format json`.
2. Human-readable output may be added, but JSON remains the contract.
3. Domain findings return exit `0`.
4. Tool failures return non-zero exit.
5. Output ordering is deterministic.
6. Commands that mutate case log require base revision when stale-write protection is enabled.

## Commands

### `version`

```sh
advisorygraphen version
advisorygraphen --version
```

### `validate`

Validate input snapshot, advisory space, report, review event, or projection request.

```sh
advisorygraphen validate \
  --input path/to/file.json \
  --schema advisorygraphen.engagement.snapshot.v1 \
  --format json
```

### `lift`

Lift bounded source snapshot into advisory space.

```sh
advisorygraphen lift \
  --input advisory.input.json \
  --package technical_advisory \
  --output advisory.space.json \
  --format json
```

### `check`

Evaluate invariants and emit obstructions.

```sh
advisorygraphen check \
  --space advisory.space.json \
  --ruleset technical_advisory_mvp \
  --format json \
  --output advisory.check.report.json
```

Optional CI behavior:

```sh
advisorygraphen check --space advisory.space.json --fail-on high
```

Without `--fail-on`, high-severity findings still exit `0`.

### `completions propose`

Generate reviewable completion candidates from obstructions.

```sh
advisorygraphen completions propose \
  --space advisory.space.json \
  --from-report advisory.check.report.json \
  --format json \
  --output advisory.completions.report.json
```

### `hypothesis propose`

Generate reviewable hypothesis lifecycle proposals from a check report and the
current advisory space. This command is read-only: it does not write a
hypothesis event and does not change case state.

```sh
advisorygraphen hypothesis propose \
  --space advisory.space.json \
  --from-report advisory.check.report.json \
  --format json \
  --output advisory.hypothesis-lifecycle.report.json
```

The report may propose `supported` or `falsified` when it finds explicit
agent-observed lifecycle signals, such as cells with
`metadata.supports_hypothesis_id`, cells with
`metadata.falsifies_hypothesis_id`, or matching support/falsify incidences.
Conflicting support and falsify signals are emitted as `review_conflict`.

All proposals are `review_status: unreviewed`. Applying a lifecycle transition
still requires the review-gated commands below.

### `hypothesis support|falsify|accept|reject`

Append a hypothesis lifecycle review event. These commands require an imported
case store and a current `--base-revision`; they are the only CLI path that
mutates hypothesis lifecycle state.

```sh
advisorygraphen hypothesis support \
  --store .advisorygraphen/store \
  --from-report advisory.check.report.json \
  --hypothesis-id hypothesis:billing-route-shared-middleware-auth \
  --evidence cell:agent-auth-observation \
  --reviewer reviewer:tech-lead \
  --reason "Agent observation reviewed; shared middleware covers route" \
  --base-revision revision:technical-advisory-smoke-1 \
  --format json
```

### `completions accept`

Append an acceptance review event. This command does not promote candidate
structure into accepted cells or incidences; promotion is a separate
review-gated transformation.

```sh
advisorygraphen completions accept \
  --store .advisorygraphen/store \
  --candidate-id candidate:billing-status-api \
  --from-report advisory.completions.report.json \
  --reviewer reviewer:cto \
  --reason "Accepted target direction" \
  --base-revision revision:technical-advisory-smoke-1 \
  --format json
```

### `completions reject`

```sh
advisorygraphen completions reject \
  --store .advisorygraphen/store \
  --candidate-id candidate:billing-status-api \
  --from-report advisory.completions.report.json \
  --reviewer reviewer:cto \
  --reason "Deferred until ownership redesign" \
  --format json
```

`--from-report` is required. The review event embeds a HigherGraphen
`CompletionReviewRecord` built from the preserved candidate snapshot. The source
candidate remains unmutated and unreviewed; the review event records the
accepted or rejected outcome. The report `input.space_id` must
match the candidate snapshot `higher_graphen.space_id`; missing values or
mismatches are rejected as validation errors before any review event is
appended.

The completion report `space_id` must already be imported into the case store.
`--base-revision` is required and checked against that space's case-store
`HEAD`. A missing or stale value fails with exit code `5`.

### `project`

Create audience-specific projection.

```sh
advisorygraphen project \
  --space advisory.space.json \
  --report advisory.check.report.json \
  --audience executive \
  --format markdown \
  --output executive-review.md
```

`--report` is the primary report for the projection. For `ai_agent`, callers can
also pass a completion proposal report so the projection includes current
candidate review state:

```sh
advisorygraphen project \
  --space advisory.space.json \
  --report advisory.check.report.json \
  --completions-report advisory.completions.report.json \
  --audience ai_agent \
  --format json \
  --output ai-agent.json
```

Supported audiences:

- `executive`
- `developer_action`
- `audit_trace`
- `ai_agent`
- `client_review`
- `cli`

### `dogfood repo-snapshot`

Generate a bounded engagement snapshot from this repository's own docs and
workspace manifest.

```sh
advisorygraphen dogfood repo-snapshot \
  --repo . \
  --output advisorygraphen-dogfood.input.json \
  --format json
```

The generated snapshot can be passed through `lift`, `check`, and `project`.
It is intentionally bounded: git history, issue tracker state, PR comments, and
the full HigherGraphen workspace source body are outside this ingestion path.

### `code repo-snapshot`

Generate a bounded code-derived engagement snapshot from a local repository.
The initial adapter targets deterministic TypeScript/JavaScript/Next.js signals:
`package.json`, `tsconfig.json`, `app/api/**/route.*`,
`src/app/api/**/route.*`, test/spec files, lexical database access, and
`process.env.*` usage.

```sh
advisorygraphen code repo-snapshot \
  --repo . \
  --output advisorygraphen-code.input.json \
  --format json
```

The generated snapshot includes `metadata.coverage_summary` with parsed files,
skipped files, unsupported extensions, route/test/db/env counts, and the
confidence model. The adapter is intentionally lexical and path-based; it does
not resolve TypeScript types or runtime control flow.

After lift, `advisorygraphen check` uses these code-derived route signals for
security-oriented design review. A database-touching API route with
`auth_detected = false` emits `api_route_missing_auth` unless reviewed metadata
explicitly marks the endpoint public or anonymous.

AI agents should not compensate for framework diversity by extending lexical
detectors for every local convention. When an agent reads source and observes
auth coverage, shared middleware, or an intentionally public route, it should
register that observation as an inferred, unreviewed snapshot record plus a
`supports` relation to the relevant route record. `check` keeps the obstruction
open, but attaches the observation to the competing route-auth hypothesis via
`metadata.supported_by` and a soft `supported_by` argumentation incidence.

Reviewed/source-backed `public_endpoint` or `anonymous_allowed` metadata on the
route structure can suppress the obstruction. Inferred/unreviewed metadata
cannot.

### `case import`

Import an advisory space into an append-only case store.

```sh
advisorygraphen case import \
  --store .advisorygraphen/store \
  --space advisory.space.json \
  --revision-id revision:technical-advisory-smoke-1 \
  --format json
```

### `case reason`

Replay case log and derive readiness, blockers, frontier, and close status.
The report includes `case_head_revision`; agents should use that value as the
`--base-revision` for a following `case close-check`.
`frontier_items` lists agent-actionable next work, while `waiting_items` lists
blockers waiting on review or new bounded source structure.

```sh
advisorygraphen case reason \
  --store .advisorygraphen/store \
  --space-id space:advisory:technical-smoke \
  --format json
```

### `case close-check`

```sh
advisorygraphen case close-check \
  --store .advisorygraphen/store \
  --space-id space:advisory:technical-smoke \
  --base-revision revision:technical-advisory-smoke-1 \
  --format json
```

## Exit code policy

| Exit code | Meaning |
| --- | --- |
| `0` | Command succeeded, including domain findings |
| `1` | Validation or schema error |
| `2` | CLI usage error |
| `3` | I/O error |
| `4` | Unsupported package, ruleset, or audience |
| `5` | Stale revision / optimistic concurrency failure |
| `6` | Explicit `--fail-on` threshold triggered |
| `101` | Internal error that should be reported as bug |

## Report metadata

Every report must include:

```json
{
  "tool": {
    "name": "advisorygraphen",
    "version": "0.1.0",
    "command": "advisorygraphen check --space ...",
    "git_revision": "optional"
  }
}
```

## CLI implementation guidance

Use `clap` derive and keep command handlers thin.

```rust
#[derive(clap::Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {
    Version,
    Validate(ValidateArgs),
    Lift(LiftArgs),
    Check(CheckArgs),
    Project(ProjectArgs),
    Completions(CompletionsCommand),
    Case(CaseCommand),
}
```
