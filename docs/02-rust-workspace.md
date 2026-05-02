# 02. Rust Workspace and Crate Boundaries

## Workspace layout

```text
advisorygraphen/
  Cargo.toml
  crates/
    advisorygraphen-core/
      src/lib.rs
    advisorygraphen-lift/
      src/lib.rs
    advisorygraphen-interpretation/
      src/lib.rs
    advisorygraphen-reasoning/
      src/lib.rs
    advisorygraphen-projection/
      src/lib.rs
    advisorygraphen-runtime/
      src/lib.rs
  tools/
    advisorygraphen-cli/
      src/main.rs
  schemas/
    advisorygraphen/
  examples/
  skills/
  docs/
```

## Dependency direction

```text
advisorygraphen-cli
  -> advisorygraphen-runtime
    -> advisorygraphen-lift
    -> advisorygraphen-interpretation
    -> advisorygraphen-reasoning
    -> advisorygraphen-projection
      -> advisorygraphen-core
        -> higher-graphen-core
        -> higher-graphen-structure
        -> higher-graphen-evidence
        -> higher-graphen-reasoning
        -> higher-graphen-projection
        -> higher-graphen-interpretation
```

Rules:

1. CLI depends on runtime; lower crates do not depend on CLI.
2. Core types do not depend on runtime, adapters, HTTP, provider SDKs, or Todoist.
3. Projection crate may define `todoist_task_export`, but actual Todoist API mutation belongs to a separate adapter.
4. Interpretation packages define vocabulary, invariants, completion rules, and projection templates.
5. Customer-specific interpretation packages live outside the public workspace unless intentionally open-sourced.

## Root `Cargo.toml` sketch

```toml
[workspace]
members = [
  "crates/advisorygraphen-core",
  "crates/advisorygraphen-lift",
  "crates/advisorygraphen-interpretation",
  "crates/advisorygraphen-reasoning",
  "crates/advisorygraphen-projection",
  "crates/advisorygraphen-runtime",
  "tools/advisorygraphen-cli",
]
resolver = "2"

[workspace.package]
edition = "2021"
license = "Apache-2.0"
repository = "https://github.com/CAPHTECH/advisorygraphen"

[workspace.dependencies]
anyhow = "1"
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }
clap = { version = "4", features = ["derive", "env"] }
indexmap = { version = "2", features = ["serde"] }
petgraph = "0.6"
schemars = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "fs"] }
tracing = "0.1"
uuid = { version = "1", features = ["serde", "v4"] }

# During local development, prefer path dependencies to the checked-out HigherGraphen workspace.
# After publication, pin crate versions and keep Cargo.lock committed for CLI reproducibility.
higher-graphen-core = { path = "../higher-graphen/crates/higher-graphen-core" }
higher-graphen-structure = { path = "../higher-graphen/crates/higher-graphen-structure" }
higher-graphen-evidence = { path = "../higher-graphen/crates/higher-graphen-evidence" }
higher-graphen-reasoning = { path = "../higher-graphen/crates/higher-graphen-reasoning" }
higher-graphen-projection = { path = "../higher-graphen/crates/higher-graphen-projection" }
higher-graphen-interpretation = { path = "../higher-graphen/crates/higher-graphen-interpretation" }
higher-graphen-runtime = { path = "../higher-graphen/crates/higher-graphen-runtime" }
```

## Crate responsibilities

### `advisorygraphen-core`

Owns AdvisoryGraphen domain records that are not already HigherGraphen primitives.

Contains:

- `AdvisorySpaceEnvelope`
- `EngagementSnapshot`
- `AdvisoryCellKind`
- `AdvisoryContextKind`
- `AdvisoryRelationKind`
- `AdvisoryReviewEvent`
- `AdvisoryReportEnvelope`
- schema constants
- ID helpers
- serde DTOs

Must not contain:

- LLM calls
- filesystem command logic
- Todoist network calls
- provider-specific agent configuration

### `advisorygraphen-lift`

Turns bounded input snapshots into `AdvisorySpaceEnvelope`.

Contains:

- JSON lift adapter
- Markdown excerpt lift adapter
- architecture input lift adapter
- interview note lift adapter
- source boundary validation
- provenance construction

### `advisorygraphen-interpretation`

Defines interpretation packages.

Initial packages:

- `technical_advisory_mvp`
- `architecture_boundary_review`
- `evidence_backed_report`

Package fields:

- vocabulary mappings
- invariant templates
- completion rule templates
- projection templates
- policy defaults
- source adapter expectations

### `advisorygraphen-reasoning`

Runs domain checks.

Contains:

- invariant evaluators
- obstruction emitters
- completion candidate generators
- evidence sufficiency checks
- review requirement checks
- close-check evaluator

### `advisorygraphen-projection`

Builds audience-specific views.

Contains:

- executive projection
- developer action projection
- audit trace projection
- ai agent projection
- Todoist task export projection
- Markdown renderer
- JSON renderer

### `advisorygraphen-runtime`

Coordinates workflows.

Contains:

- `lift_workflow`
- `check_workflow`
- `completion_proposal_workflow`
- `review_workflow`
- `project_workflow`
- file I/O boundaries
- report envelope creation

### `advisorygraphen-cli`

Command-line interface only.

Contains:

- Clap command parsing
- stdout / file output
- exit code policy
- no domain-specific reasoning that belongs in lower crates

## Error policy

Domain findings are not tool failures.

Exit `0`:

- obstruction found
- missing evidence found
- unreviewed completion candidate found
- projection loss found
- uncovered requirement found

Non-zero exit:

- unreadable input
- invalid JSON
- schema mismatch
- unsupported package
- invalid CLI options
- output write failure
- internal panic converted to structured error

## Feature flags

```toml
[features]
default = ["json", "markdown"]
json = []
markdown = []
todoist-projection = []
case-log = []
path-highergraphen = []
mcp = [] # future; must not be part of MVP default
```

## Rust module naming

| Cargo package | Rust crate name |
| --- | --- |
| `advisorygraphen-core` | `advisorygraphen_core` |
| `advisorygraphen-lift` | `advisorygraphen_lift` |
| `advisorygraphen-interpretation` | `advisorygraphen_interpretation` |
| `advisorygraphen-reasoning` | `advisorygraphen_reasoning` |
| `advisorygraphen-projection` | `advisorygraphen_projection` |
| `advisorygraphen-runtime` | `advisorygraphen_runtime` |

## Build commands

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -q -p advisorygraphen-cli -- version
cargo run -q -p advisorygraphen-cli -- validate --input examples/technical-advisory/direct-db-access/advisory.input.json --format json
```
