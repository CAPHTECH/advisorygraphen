---
name: advisorygraphen-pr-review
description: Use when reviewing a pull request, local diff, or agent-made change with AdvisoryGraphen to propose where a human reviewer should focus. This skill builds an evidence-backed advisory snapshot from code/docs/test changes, models review focus as verification requirements, runs AdvisoryGraphen validation/check/projection, and translates obstructions and HigherGraphen correspondence signals into review priorities.
---

# AdvisoryGraphen PR Review

## Overview

Use AdvisoryGraphen as a review-orientation tool, not as a substitute for human judgment. The goal is to turn a PR or working-tree diff into a small advisory graph that identifies which changed contracts, behaviors, examples, or documentation claims still need human review.

This skill depends on the repository's AdvisoryGraphen CLI and pairs naturally with the `advisorygraphen` skill when you need exact command syntax or schema details.

## Workflow

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

### 5. Report review priorities

Lead with where the human should spend attention. Use this shape:

```md
**Method Integrity**
- Snapshot modeled review targets as requirements: yes/no
- Artificial workflow defects found: yes/no
- Projection loss or blocked content: summary

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

## Method Repair

If the first run produces suspicious findings, repair the method before reporting:

- If unowned actions dominate, remodel review targets as `requirement`.
- If the output cannot distinguish changed areas, split broad requirements by behavior or file group.
- If the graph has no open review targets, check whether requirements were accidentally marked verified.
- If HigherGraphen correspondence volume is noisy, use `review_focus_candidates`; only escalate omitted candidates when failures, review candidates, or blocking/major differences do not explain the review risk.
- If examples are included to demonstrate the method, include at least one intentionally unresolved requirement so the user can see how review focus appears.
