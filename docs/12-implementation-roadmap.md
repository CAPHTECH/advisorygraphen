# 12. Implementation Roadmap

## Phase 0: Repository and contracts

Deliverables:

- Rust workspace
- crate skeletons
- CLI skeleton
- JSON schemas copied into repository
- synthetic direct-db-access example
- CI: fmt, clippy, tests, schema validation
- `advisorygraphen version`
- `advisorygraphen validate`

Acceptance:

```sh
cargo test --workspace
cargo run -q -p advisorygraphen-cli -- version
cargo run -q -p advisorygraphen-cli -- validate --input examples/technical-advisory/direct-db-access/advisory.input.json --format json
```

## Phase 1: Lift MVP

Deliverables:

- `EngagementSnapshot` DTO
- `AdvisorySpaceEnvelope` DTO
- JSON source adapter
- `technical_advisory_mvp` lift
- provenance and review status mapping
- `advisorygraphen lift`

Acceptance:

- example input lifts into deterministic `advisory.space.v1`
- accepted observations and unreviewed inferences are separate
- source boundary appears in output

## Phase 2: Reasoning MVP

Deliverables:

- invariant registry
- MVP invariant evaluators
- obstruction emitter
- report envelope
- `advisorygraphen check`

Initial invariant evaluators:

- `architecture_no_cross_context_direct_database_access`
- `recommendation_requires_evidence`
- `action_requires_owner`
- `requirement_requires_verification`
- `projection_loss_declared`

Acceptance:

- direct DB access example emits boundary obstruction
- missing owner emits missing owner obstruction
- missing verification emits requirement obstruction
- findings are deterministic
- command exits `0`

## Phase 3: Completion MVP

Deliverables:

- completion rule registry
- candidate generator
- completion report
- `completions propose`
- candidate accept/reject review events without full case log

Acceptance:

- direct DB access obstruction proposes Billing API candidate
- candidate starts `unreviewed`
- acceptance requires reviewer and reason

## Phase 4: Projection MVP

Deliverables:

- executive projection
- developer action projection
- audit trace projection
- AI agent projection
- Todoist task export projection
- Markdown renderer derived from JSON
- `advisorygraphen project`

Acceptance:

- every projection includes represented IDs, omitted IDs, information loss
- executive view does not present unreviewed candidate as accepted action
- Todoist export excludes unreviewed action unless draft policy is enabled

## Phase 5: Case log MVP

Deliverables:

- local append-only JSONL log
- import command
- review morphisms
- replay materializer
- case reason projection
- close-check

Acceptance:

- accept/reject events survive replay
- derived blockers and close status match invariant results
- stale base revision fails

## Phase 6: Agent skill hardening

Deliverables:

- `skills/advisorygraphen/SKILL.md`
- no-network CLI examples
- schema validator script or cargo command
- agent safety tests

Acceptance:

- agent can run lift/check/project from examples
- agent output keeps candidates unreviewed
- audit projection exposes source boundary and projection loss

## Phase 7: Production adapters and private packages

Deliverables:

- optional GitHub/Jira/Slack source adapters
- private customer interpretation packages
- internal hosted storage candidate
- redaction and policy checks

Acceptance:

- adapters preserve source boundary
- customer-specific packages are outside public repository
- Todoist integration remains projection-driven

## Cut lines

Do not build before MVP contract stability:

- hosted SaaS
- full UI
- MCP server
- provider marketplace packaging
- real-time collaboration
- automatic network adapters
- LLM prompt chains embedded in core crates

## Implementation priority

1. Deterministic file-based CLI
2. Schema validation
3. Lift and check direct DB access scenario
4. Completion candidates
5. Projection loss
6. Review workflow
7. Case log
8. External integrations
