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

Validate input snapshot, advisory space, report, review event, projection
request, or micro review request. The schema is auto-detected from the `schema`
field; `--schema` may pin an expected schema (e.g. `micro_review_request`).

```sh
advisorygraphen validate \
  --input path/to/file.json \
  --schema advisorygraphen.engagement.snapshot.v1 \
  --format json
```

### `propose`

Run the task-oriented hypothesis/proposal facade. This command accepts a
bounded source snapshot JSON, creates a case directory, runs the standard
workflow, writes a case manifest, imports the generated space into the local
case store, and returns a `facade_propose` report.

```sh
advisorygraphen propose \
  --input advisory.input.json \
  --case target/tmp/advisory-case \
  --package technical_advisory \
  --ruleset technical_advisory_mvp \
  --audience ai_agent \
  --format json
```

The case directory must be empty or nonexistent. The command writes
`advisorygraphen.case-manifest.json` with artifact paths for input, space,
check report, completion report, hypothesis report, `ai_agent` projection, and
case store. Arbitrary Markdown/text ingestion is intentionally out of scope;
small text uses `micro review`.

### `status`

Read a facade case manifest, replay the case store, and return current blockers,
waiting items, frontier items, close status, and head revision.

```sh
advisorygraphen status --case target/tmp/advisory-case --format json
advisorygraphen status --case target/tmp/advisory-case --brief --format json
```

The full `result` includes a compact decision surface for agents and humans:

- `summary`: `status_label`, `closeable`, blocker/waiting/frontier counts, and
  current case head.
- `top_blockers`: up to three highest-severity blockers with stable IDs,
  messages, blocked IDs, and recommended completion types.
- `next_best_action`: the next command class to run, such as
  `review_pending_candidate`, `advance_frontier`, or `report_or_close`.

The full `close_status`, `blockers`, `frontier_items`, and `waiting_items`
remain present for detailed inspection and backward compatibility. With
`--brief`, `result` contains only `summary`, `top_blockers`,
`next_best_action`, `case_head_revision`, and `next_commands`.

### `report`

Render an audience projection from a facade case manifest. For `ai_agent`, the
projection is built from current `case reason` state so recorded reviews are
visible.

```sh
advisorygraphen report \
  --case target/tmp/advisory-case \
  --audience ai_agent \
  --format json \
  --output target/tmp/advisory-case/ai-agent.json
```

### `review`

Review facade completion or hypothesis targets using manifest-derived artifact
paths and the current case head as the base revision.

```sh
advisorygraphen review completion reject \
  --case target/tmp/advisory-case \
  --candidate-id candidate:inventory-status-api \
  --reviewer reviewer:tech-lead \
  --reason "Need a smaller interface proposal first" \
  --format json

advisorygraphen review hypothesis support \
  --case target/tmp/advisory-case \
  --hypothesis-id hypothesis:example \
  --evidence cell:reviewed-observation \
  --reviewer reviewer:tech-lead \
  --reason "Reviewed bounded observation supports the hypothesis" \
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

### `micro review`

Review a small AI answer, note, issue, or PR comment without first lifting it
into a full advisory space. `micro review` does **not** classify prose itself.
Deciding whether a sentence is overconfident, an assumption, or evidence-backed
is a semantic judgement that belongs to the calling agent; pattern-matching
keywords is brittle and was removed. The input is the agent's self-classified
claims, and the command enforces *structural* honesty deterministically — the
same discipline as the full workflow's `supported_hypothesis_missing_support`
invariant, at small scope.

```sh
advisorygraphen micro review \
  --input micro-review.request.json \
  --format json \
  --output micro-review.report.json
```

The input is a `advisorygraphen.micro_review.request.v1` document. Each claim
carries a `classification` (`test_backed`, `source_backed`, `assumption`,
`unsupported_strong_claim`, or `unsupported`) and optional `evidence_refs`,
`risk_surface`, `alternative_hypotheses`, and `missing_checks`. An unknown
classification is a validation error (exit `1`).

The report type is `micro_review`. The command enforces:

- `obstructions`: structural findings the agent cannot self-certify away —
  `claim_marked_supported_without_evidence` (a claim classified evidence-backed
  but citing no witness), `unsupported_strong_claim` (a declared strong claim
  with no evidence), and `high_blast_radius_claim_without_evidence`.
- `claims`: each claim echoed with its `structural_status`.
- `assumptions`: claims classified as assumptions, pending confirmation.
- `missing_checks`: tool-required checks (cite a witness, add a falsifier) plus
  agent-declared checks.
- `alternative_hypotheses`: competing explanations supplied by the agent.
- `scale_signals`: deterministic counts by structural status.
- `mode`: `micro_review` or `full_advisory_workflow_recommended`. Escalation is a
  deterministic rule over the agent's classifications (many claims, two or more
  unsupported strong claims, an unsupported high-blast-radius claim, or more than
  five claims without cited evidence).

### `completions propose`

Generate reviewable completion candidates from obstructions. Each candidate is
still unreviewed, but its proposed content is also shaped as HigherGraphen
structure so reviewers can inspect the proposal itself, not only the fact that a
candidate exists.

Candidate `proposal_content` includes:

- `scenario`: the planned world created by the candidate, changed structures,
  affected invariants, expected obstructions, and required witnesses.
- `morphism`: the As-Is to proposed structure mapping, preserved invariants,
  distortion, and composition constraints.
- `invariant_checks`: candidate-level repair or review-gate checks.
- `derivation`: why the candidate follows from obstructions, witnesses, and
  source material.
- `witnesses`: known source or structure references supporting the proposal.
- `valuation`: criteria, values, trade-offs, and confidence for comparing the
  proposal with alternatives.
- `policy`: the review gate that prevents proposal content from becoming
  accepted structure without explicit review and materialization.
- `content_obstructions`: missing proposal details, such as absent concrete
  structures or source-backed witnesses.

For `missing_owner` and `requirement_unverified`, the generator first searches
the current advisory space for related `owner`, `test_or_verification`, or
`metric` cells using shared context and source IDs. When it finds one, it emits
a concrete relation candidate with `proposed_incidence_ids`
(`owner_assignment` or `lift_verification_link`) instead of a generic
placeholder. If no related structure is found, the candidate remains
underspecified and records that in `content_obstructions`.

Every completion candidate includes `application_plan`, a review-gated operation
preview. It names proposed cell and incidence upserts and, for boundary repair
candidates, direct-access incidence removals. Agents should treat the plan as a
candidate-local hypothesis until review or dry-run evidence exists.

AI-agent and executive projections include `proposal_content_summary` so agents
and reviewers can see how many candidates have structured proposal content, how
many remain blocked, and which content obstruction types remain.

```sh
advisorygraphen completions propose \
  --space advisory.space.json \
  --from-report advisory.check.report.json \
  --format json \
  --output advisory.completions.report.json
```

### `completions dry-run`

Apply one or more completion candidates to a cloned in-memory space and rerun
the technical advisory check. This command is read-only: it does not write
review events, accept candidates, or mutate a case store.

```sh
advisorygraphen completions dry-run \
  --space advisory.space.json \
  --from-report advisory.completions.report.json \
  --candidate-id candidate:billing-status-api \
  --format json \
  --output advisory.completion-dry-run.report.json
```

If `--candidate-id` is omitted, the command dry-runs all candidates in the
completion report. The output report type is `completion_dry_run`; each entry
contains `applied_structure`, `check_delta`, `after_close_status`, and
`higher_graphen_gluing_review`. The gluing review is generated with
HigherGraphen correspondence / overlap / gluing primitives and should be read
as review evidence: failures and blocking differences mean the candidate cannot
be silently promoted without explicit review or revision.

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

### `hypothesis apply-proposals`

Apply policy-allowed lifecycle proposals as append-only hypothesis events. This
is the first autonomy step: it can apply `supported` / `falsified` only when the
proposal passes the autonomy policy. It cannot apply `accept` / `reject`, cannot
apply `review_conflict`, and cannot change source material.

```sh
advisorygraphen hypothesis apply-proposals \
  --store .advisorygraphen/store \
  --from-report advisory.hypothesis-lifecycle.report.json \
  --reviewer ai-agent:codex \
  --reason "Source-backed observation matched conservative autonomy policy" \
  --base-revision revision:technical-advisory-smoke-1 \
  --format json
```

Default conservative policy:

```json
{
  "allowed_outcomes": ["supported", "falsified"],
  "min_confidence": 0.7,
  "allowed_trust_levels": [
    "reviewed_or_source_backed",
    "test_passed",
    "runtime_observed"
  ],
  "max_events": 3,
  "require_candidate_status": true,
  "allow_review_conflict": false
}
```

Callers may pass `--policy policy.json` to override these values. Use
`--dry-run` to report which proposals would be applied without writing case-log
events or moving `HEAD`.

When events are written, the result includes `post_apply_case_reason` so agents
can inspect the updated case head, close status, frontier items, and waiting
items before deciding whether another loop is allowed by policy.

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
appended. The review event also embeds `higher_graphen_gluing_policy`, which
contains the candidate dry-run gluing review and `policy_blockers`; accepting a
candidate records an explicit review override rather than silently clearing
those blockers.

The completion report `space_id` must already be imported into the case store.
`--base-revision` is required and checked against that space's case-store
`HEAD`. A missing or stale value fails with exit code `5`.

### `completions apply-accepted`

Apply reviewed accepted completion candidates into the materialized advisory
space. The command reruns check/propose, overlays review events, inspects
`blocker_resolution_state.application_requirements`, and only materializes
candidate types with an explicit generic mapping.

```sh
advisorygraphen completions apply-accepted \
  --store .advisorygraphen/store \
  --space-id space:technical-advisory-smoke \
  --reviewer ai-agent:codex \
  --reason "Apply reviewed accepted completion candidates" \
  --base-revision revision:review-000002 \
  --format json
```

Supported automatic applications:

| Candidate type | Materialized structure |
| --- | --- |
| `ownership_clarification` | placeholder `owner` cell plus `owns` incidence to the blocked action |
| `proposed_test` | placeholder `test_or_verification` cell plus `verifies` incidence to the blocked requirement |

Unsupported accepted candidate types are reported in `skipped_candidates` and
remain review-visible. `--dry-run` returns the cells and incidences that would
be written without changing `materialized/space.json` or `HEAD`. Successful
application appends a `advisorygraphen.completion.application.v1` case-log
event, advances `HEAD`, and includes `post_apply_case_reason` so an agent can
verify whether the blocker actually disappeared. Each applied structure carries
`higher_graphen_gluing_review`, `policy_blockers`, and `policy_override` so
operators can audit which gluing failures were overridden by explicit
completion review. When current gluing blockers exist, `policy_override` is
copied from the recorded completion review event; apply must not synthesize an
override that was not recorded at accept time.

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
    "version": "0.2.1",
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
