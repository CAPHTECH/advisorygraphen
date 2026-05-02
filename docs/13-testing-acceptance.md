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

### Todoist export policy

Given unreviewed action and no draft export policy, Todoist task export must omit the action or mark export as blocked.

### Close-check blocks unresolved obstruction

Given high-severity obstruction unresolved, `case close-check` must return closeable = false with obstruction witness IDs.

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
cargo run -q -p advisorygraphen-cli -- validate --input examples/technical-advisory/direct-db-access/advisory.input.json --format json
```

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
