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

When `--from-report` is supplied, the review event embeds a HigherGraphen
`CompletionReviewRecord` built from the preserved candidate snapshot. The source
candidate remains unmutated and unreviewed; the review event records the
accepted or rejected outcome. The report `input.space_id` must match the
candidate snapshot `higher_graphen.space_id`; mismatches are rejected as
validation errors before any review event is appended.

When the completion report identifies an imported `space_id` with a case-store
`HEAD`, `--base-revision` is required and checked against that `HEAD`. A missing
or stale value fails with exit code `5`.

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
