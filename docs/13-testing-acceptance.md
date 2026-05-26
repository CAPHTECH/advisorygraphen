# 13. Testing and Acceptance

## Test levels

| Level | Tests |
| --- | --- |
| Unit | ID parsing, provenance validation, invariant evaluators |
| Contract | JSON schema validation, report envelope shape |
| Snapshot | deterministic CLI output |
| Integration | lift → check → completions → project |
| Safety | candidate review, projection loss, AI inference boundaries |
| Regression | direct DB access smoke scenario |

## Golden scenario

`examples/technical-advisory/direct-db-access/advisory.input.json`

Expected findings:

1. `Order Service` is a component in `context:orders`.
2. `Billing DB` is a data store in `context:billing`.
3. `Billing Service` owns `Billing DB`.
4. `Order Service` accesses `Billing DB`.
5. `architecture_no_cross_context_direct_database_access` is violated.
6. An obstruction is emitted.
7. A `Billing status API` completion candidate is proposed.
8. The candidate remains `unreviewed`.
9. Executive projection discloses the boundary violation.
10. Audit projection exposes source boundary and projection loss.

## Hypothesis-to-proposal evaluation

`examples/evaluation/medium-hypothesis-proposal/advisory.input.json`

This medium-scale fixture checks the main advisory use case: controlling early
AI convergence and over-proposal before a recommendation becomes primary.

Expected findings:

1. The fixture includes an AI baseline proposal that converges on cache TTL as
   the root cause.
2. Competing hypotheses for direct Inventory DB coupling and upstream rate
   limiting remain visible.
3. The cache TTL proposal is derived from an unsupported hypothesis.
4. `check` emits `proposal_derived_from_unsupported_hypothesis`.
5. High-priority proposal promotion also requires hypothesis refinement.
6. `completions propose` keeps all generated recommendations as
   `follow_up_observation`.
7. The `ai_agent` projection reports `primary_count: 0`.
8. Ranked observation tasks describe the evidence needed before proposal
   promotion.

## Medium/large PR review evaluation

`examples/evaluation/medium-pr-review/advisory.input.json`

This medium-scale fixture checks whether AdvisoryGraphen is useful for review:
it must turn a broad PR surface into a bounded review priority map instead of
asking the reviewer to inspect every changed area equally.

Expected findings:

1. The fixture contains five review-area requirements across auth, migration,
   public API, docs, and UI copy changes.
2. Auth tenant isolation, billing migration rollback safety, and public API
   compatibility remain unresolved verification requirements.
3. `check` emits `requirement_unverified` obstructions for those three
   contract areas.
4. Docs changelog and UI copy requirements are verified by explicit test
   outputs, so they do not emit missing-verification obstructions.
5. `completions propose` creates verification candidates only for the
   unresolved review targets.
6. The `ai_agent` projection exposes ranked observation tasks,
   correspondence analysis, and projection-loss metrics for the remaining
   review work.

## CLI acceptance test

```sh
cargo run -q -p advisorygraphen-cli -- lift \
  --input examples/technical-advisory/direct-db-access/advisory.input.json \
  --package technical_advisory \
  --output target/tmp/advisory.space.json \
  --format json

cargo run -q -p advisorygraphen-cli -- check \
  --space target/tmp/advisory.space.json \
  --ruleset technical_advisory_mvp \
  --output target/tmp/advisory.check.report.json \
  --format json

cargo run -q -p advisorygraphen-cli -- completions propose \
  --space target/tmp/advisory.space.json \
  --from-report target/tmp/advisory.check.report.json \
  --output target/tmp/advisory.completions.report.json \
  --format json

cargo run -q -p advisorygraphen-cli -- project \
  --space target/tmp/advisory.space.json \
  --report target/tmp/advisory.check.report.json \
  --audience executive \
  --format markdown \
  --output target/tmp/executive-review.md
```

## Snapshot testing

Use `insta` or plain JSON fixtures. Output should be sorted by stable IDs.

Recommended snapshot files:

```text
tests/snapshots/
  lift_direct_db_access.snap.json
  check_direct_db_access.snap.json
  completions_direct_db_access.snap.json
  project_executive_direct_db_access.snap.md
  project_audit_direct_db_access.snap.json
```

## Safety tests

### AI inference cannot satisfy evidence

Given a recommendation supported only by inferred evidence, `recommendation_requires_evidence` must emit `insufficient_evidence` unless a review event promotes that evidence.

### Candidate cannot be accepted by confidence

Given candidate confidence `0.99` and no review event, candidate remains `unreviewed` and cannot appear as accepted action.

### Projection loss is required

Given a projection with omitted source IDs but no information loss entries, `projection_loss_declared` fails.

### Close-check blocks unresolved obstruction

Given medium-or-higher obstruction unresolved, `case close-check` must return closeable = false with obstruction witness IDs.

## Property tests

Useful property tests:

- duplicate IDs are always rejected;
- relation endpoint references must resolve;
- projection represented IDs and omitted IDs are disjoint;
- accept/reject review event cannot target unknown candidate;
- log replay is deterministic;
- materialized space checksum changes when review event changes.

## CI

Minimum CI:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --manifest-path tests/advisorygraphen-cli-acceptance/Cargo.toml
cargo package --workspace
cargo publish --dry-run --workspace
cargo run -q -p advisorygraphen-cli -- validate --input examples/technical-advisory/direct-db-access/advisory.input.json --format json
```

## Acceptance definition for v0.1.3

v0.1.3 can be tagged when:

1. workspace tests, clippy, and CLI acceptance tests pass;
2. crates.io publish dry-run passes for the workspace;
3. micro review reports relative structure-error risk, falsification checks,
   and escalation guidance for small AI answers or PR notes;
4. hypothesis-to-proposal evaluation keeps unsupported proposals out of
   primary recommendations and exposes ranked observation tasks;
5. medium/large PR review evaluation distinguishes Must Review and Can Skim
   areas with an executable priority-map fixture.

## Acceptance definition for v0.1.2

v0.1.2 can be tagged when:

1. workspace tests, clippy, and CLI acceptance tests pass;
2. crates.io publish dry-run passes for the workspace;
3. `ai_agent` projection exposes bounded HigherGraphen correspondence counts, review focus candidates, and gluing summaries;
4. completion dry-run gluing keeps blockers and policy overrides review-visible until explicit review;
5. PR review skill guidance emphasizes AI authority, persistence, evidence-to-fact, public output/schema, and dependency/version boundaries.

## Acceptance definition for v0.1.1

v0.1.1 can be tagged when:

1. workspace tests and CLI acceptance tests pass;
2. crates.io package and publish dry-run pass for the workspace;
3. `ai_agent` projection exposes `observation_actions`, `projection_loss_metrics`, and `schema_morphisms`;
4. the agent operation contract requires reading those support objects before promotion or summary;
5. the adversarial dogfood fixture keeps unsupported-hypothesis proposals as follow-up observations.

## Acceptance definition for v0.1.0

v0.1.0 can be tagged when:

1. direct-db-access example passes end to end;
2. all schemas are committed;
3. all commands in docs run;
4. JSON reports are deterministic;
5. AI inference, candidate review, and projection loss safety tests pass;
6. no customer data is present in examples or docs;
7. skill file matches CLI commands;
8. README includes public/private boundary warning.
