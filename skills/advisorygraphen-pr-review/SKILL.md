---
name: advisorygraphen-pr-review
description: Use when reviewing a pull request, local diff, or agent-made change with AdvisoryGraphen to propose where a human reviewer should focus. This skill builds an evidence-backed advisory snapshot from code/docs/test changes, models review focus as verification requirements, runs AdvisoryGraphen validation/check/projection, and translates obstructions and HigherGraphen correspondence signals into review priorities.
---

# AdvisoryGraphen PR Review

## Overview

Use AdvisoryGraphen as a review-orientation tool, not as a substitute for human judgment. The goal is to turn a PR or working-tree diff into a small advisory graph that identifies which changed contracts, behaviors, examples, or documentation claims still need human review.

This skill depends on the repository's AdvisoryGraphen CLI and pairs naturally with the `advisorygraphen` skill when you need exact command syntax or schema details.

## Workflow

### 0. Micro triage for small PRs or AI summaries

When the PR is small, or when the input is primarily an AI work summary, PR
description, review note, or compact local diff summary, run `micro review`
before building a full advisory snapshot:

Classify each claim in the PR summary yourself — `micro review` does not
pattern-match prose — and write an `advisorygraphen.micro_review.request.v1`
document. Mark each claim `test_backed`, `source_backed`, `assumption`,
`unsupported_strong_claim`, or `unsupported`, and give every evidence-backed
claim concrete `evidence_refs` (changed files, test commands, source ids). Add
`risk_surface` tags for high-blast-radius areas (auth, database, billing, etc.).

```sh
advisorygraphen micro review \
  --input target/tmp/pr-review-<slug>/micro-review.request.json \
  --output target/tmp/pr-review-<slug>/micro-review.json \
  --format json
```

Use the result as a cheap review-orientation gate:

- Treat each `obstructions` entry as an initial **Must Review** candidate, by
  `obstruction_type`: `claim_marked_supported_without_evidence` (you asserted a
  fact you cannot cite), `unsupported_strong_claim`, and
  `high_blast_radius_claim_without_evidence`.
- Use the obstruction `message` as the reason the claim may be wrong, and
  `required_resolution` plus `missing_checks` as concrete reviewer checks.
- A claim marked `source_backed`/`test_backed` without `evidence_refs` is a
  structural failure, not a judgement call: cite the witness or downgrade it.
- Escalate to the full AdvisoryGraphen PR-review workflow when
  `mode.recommended` is `full_advisory_workflow_recommended`, when multiple
  obstructions exist, or when the reviewer needs durable graph artifacts.

Skip this step only when the review surface is already large enough that the
full snapshot workflow is clearly required.

### 0b. Medium / large review mode

For medium or large PRs, the value is not finding every issue. The value is
turning a broad diff into a bounded, evidence-backed review priority map.

Use the full snapshot workflow when any of these are true:

- the diff touches multiple ownership or module boundaries;
- runtime behavior, schemas, projections, docs, and tests change together;
- review cannot be completed by reading every changed line closely;
- the PR contains generated or AI-authored structure;
- compatibility, persistence, authorization, or public output contracts may
  have changed.

Model medium/large reviews by splitting the review surface into file-group or
contract-group requirements. Each group should have a review question, evidence
sources, and an explicit verification expectation. Avoid one broad requirement
for the whole PR; it hides the review focus and makes the graph output generic.

The output must help a human decide:

- **Must Review**: changed contracts, high/medium obstructions, blocked
  proposal content, gluing failures, unresolved verification requirements, or
  micro review `obstructions`.
- **Should Review**: medium-risk areas with partial evidence, projection loss,
  unreviewed candidate clusters, or uncertain correspondence differences.
- **Can Skim**: areas with accepted evidence, no open requirement, no
  high/medium obstruction, and no correspondence/gluing warning.

Treat a medium/large PR review as failed if it cannot produce this prioritised
map, if it labels everything as Must Review, or if it labels everything as Can
Skim.

### 0c. Medium/large PR review evaluation smoke

Use this repository fixture to verify that the method still provides value for
medium/large review orientation:

```sh
advisorygraphen validate --input examples/evaluation/medium-pr-review/advisory.input.json --format json
advisorygraphen lift --input examples/evaluation/medium-pr-review/advisory.input.json --package technical_advisory --output target/tmp/medium-pr-review.space.json --format json
advisorygraphen check --space target/tmp/medium-pr-review.space.json --ruleset technical_advisory_mvp --output target/tmp/medium-pr-review.check.json --format json
advisorygraphen completions propose --space target/tmp/medium-pr-review.space.json --from-report target/tmp/medium-pr-review.check.json --output target/tmp/medium-pr-review.completions.json --format json
advisorygraphen project --space target/tmp/medium-pr-review.space.json --report target/tmp/medium-pr-review.check.json --completions-report target/tmp/medium-pr-review.completions.json --audience ai_agent --output target/tmp/medium-pr-review.ai-agent.json --format json
```

Expected proof of usefulness:

- the fixture declares `medium_large_review_priority_map`;
- auth tenant isolation, billing migration rollback safety, and public API
  compatibility remain `requirement_unverified` and become **Must Review**:
  `req-authz-tenant-isolation`, `req-migration-rollback-safety`, and
  `req-public-api-compatibility`;
- docs changelog and UI copy snapshot are covered by `verifies` relations and
  do not produce missing-verification obstructions, so they are **Can Skim**:
  `req-docs-changelog-updated` and `req-ui-copy-snapshot-stable`;
- completion candidates are generated only for unresolved review targets;
- the `ai_agent` projection includes `ranked_observation_tasks`,
  `correspondence_analysis`, and `projection_loss_metrics`.

### 1. Bound the review surface

Collect only enough evidence to model the review:

- `git status --short`
- `git diff --name-status`
- `git diff --stat`
- focused diffs for changed runtime, projection, schema, docs, examples, and tests
- test or CI output already available in the workspace

Write scratch artifacts under `target/tmp/pr-review-<slug>/` so they do not pollute source directories.

### 2. Build an advisory snapshot

Create `target/tmp/pr-review-<slug>/advisory.input.json` from the diff. Model the review surface with these record types:

- `source`: changed file, command output, example output, or PR note used as evidence
- `requirement`: a specific claim that a human should verify
- `observation`: a command result or static inspection result
- `decision`: an explicit design choice, policy, or compatibility stance
- `risk`: a possible regression or blind spot
- `action`: only a real implementation task with an accountable owner

Critical modeling rule: represent "human should review this" as a `requirement` with `require_verification: true`, not as an `action`. AdvisoryGraphen treats unowned actions as workflow defects, so using `action` for review focus creates artificial `missing_owner` obstructions.

For AI-generated code, add explicit boundary and contract review requirements even when the diff already has runtime or projection review buckets:

- AI authority boundary: what the agent may infer, propose, accept, reject, or apply
- persistence boundary: what may mutate case state, logs, stores, schemas, or generated files
- evidence-to-fact boundary: what remains evidence, candidate, hypothesis, or accepted fact
- public output/schema boundary: what fields, counts, omissions, and compatibility promises downstream users may rely on
- dependency/version boundary: which external crate or service contracts the change assumes

For snapshot examples and the corrected PR-review modeling pattern, load `references/pr-snapshot-modeling.md`.

### 3. Run AdvisoryGraphen

Use the repository CLI or local binary. A typical sequence is:

```sh
advisorygraphen validate --input target/tmp/pr-review-<slug>/advisory.input.json
advisorygraphen lift --input target/tmp/pr-review-<slug>/advisory.input.json --package technical_advisory_mvp --output target/tmp/pr-review-<slug>/space.json
advisorygraphen check --space target/tmp/pr-review-<slug>/space.json --ruleset technical_advisory_mvp --output target/tmp/pr-review-<slug>/check.json
advisorygraphen completions propose --space target/tmp/pr-review-<slug>/space.json --from-report target/tmp/pr-review-<slug>/check.json --output target/tmp/pr-review-<slug>/completions.json
advisorygraphen project --space target/tmp/pr-review-<slug>/space.json --report target/tmp/pr-review-<slug>/check.json --completions-report target/tmp/pr-review-<slug>/completions.json --audience ai_agent --output target/tmp/pr-review-<slug>/projection.json
```

If the repository uses `cargo run`, `./target/debug/advisorygraphen`, or another wrapper, keep the same command sequence and substitute the binary invocation.

### 4. Interpret the output

Read the graph output as review guidance:

- `micro-review.json.result.obstructions` identifies risky claims before
  snapshot modeling; carry each obstruction into review priorities or into
  explicit `requirement` records in the full snapshot
- open `requirement_unverified` obstructions are expected review targets
- `missing_owner` on review-focus records usually means the snapshot modeled a review target as an `action`; fix the snapshot and rerun
- `higher_graphen_gluing_review.policy_blockers` identify completion proposals that should not be silently applied
- `correspondence_analysis.review_focus_candidates` is the first place to inspect HigherGraphen correspondence output
- `correspondence_analysis.gluing_summary.review_candidate` and `failure` identify structural areas where the projection may have lost or distorted relationships
- `correspondence_analysis.omitted_candidate_count` is expected when many low-signal success-only correspondences exist; report the count, but do not read or summarize every omitted candidate
- `generic_candidate_similarity_deprioritized` means the signal is a generated candidate cluster, not a primary human review target
- `projection_loss` and `blocked_content` must be disclosed as limitations
- completion proposals are evidence-gathering or follow-up tasks, not proof that the PR is correct

Treat the output as invalid if it is mostly generic, if every changed area maps to the same requirement, or if the snapshot omits major changed files.
For medium/large reviews, also treat the output as invalid if it cannot
distinguish Must Review, Should Review, and Can Skim areas from the graph
findings.

### 5. Report review priorities

Lead with where the human should spend attention. Use this shape:

```md
**Method Integrity**
- Snapshot modeled review targets as requirements: yes/no
- Artificial workflow defects found: yes/no
- Micro triage used or skipped: reason
- Projection loss or blocked content: summary

**Structure Error Risk**
- Risky claim: claim/file/area from an `obstructions` entry
- Why it may be wrong: obstruction `message` and `obstruction_type`
- Reviewer checks: `required_resolution` and `missing_checks`
- Note: micro review enforces structure (uncited support, declared strong
  claims, unsupported high-blast-radius claims); claim classification is the
  agent's judgement, not a tool heuristic

**Must Review**
- Area: why it is risky, which evidence points to it, what to check

**Boundary / Contract Review**
- Boundary: what the AI-generated change may cross, what must remain gated, and how to verify it

**Should Review**
- Area: why it matters, what would reduce uncertainty

**Can Skim**
- Area: why it has lower apparent risk

**Evidence**
- Snapshot path
- Check/projection/completions paths
- Commands or tests run

**Limitations**
- Missing evidence, unrun tests, stale PR context, or assumptions
```

Keep priorities grounded in file paths, changed contracts, and AdvisoryGraphen findings. Do not claim the graph proves correctness.
For medium/large reviews, explicitly state whether the graph produced a useful
priority map. If it did not, repair the snapshot before reporting findings.

## Method Repair

If the first run produces suspicious findings, repair the method before reporting:

- If micro triage flags `obstructions` that are absent from the full snapshot,
  add matching `requirement` records or explain why they are out of scope.
- If unowned actions dominate, remodel review targets as `requirement`.
- If the output cannot distinguish changed areas, split broad requirements by behavior or file group.
- If a medium/large review marks every area Must Review or Can Skim, split
  requirements more precisely and rerun.
- If the graph has no open review targets, check whether requirements were accidentally marked verified.
- If HigherGraphen correspondence volume is noisy, use `review_focus_candidates`; only escalate omitted candidates when failures, review candidates, or blocking/major differences do not explain the review risk.
- If examples are included to demonstrate the method, include at least one intentionally unresolved requirement so the user can see how review focus appears.
