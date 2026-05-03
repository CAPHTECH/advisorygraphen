use crate::write_json_if_requested;
use advisorygraphen_core::{validate_document, AdvisoryResult};
use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DogfoodRepoSnapshotOptions {
    pub repo: PathBuf,
    pub output: Option<PathBuf>,
}

pub fn dogfood_repo_snapshot_workflow(
    options: &DogfoodRepoSnapshotOptions,
) -> AdvisoryResult<Value> {
    let captured_at = Utc::now().to_rfc3339();
    let snapshot = json!({
        "schema": "advisorygraphen.engagement.snapshot.v1",
        "snapshot_id": "snapshot:dogfood-higher-graphen-integration",
        "engagement_id": "engagement:advisorygraphen-self-review",
        "captured_at": captured_at,
        "source_boundary": {
            "included_source_ids": [
                "source:readme",
                "source:docs-manifest",
                "source:source-alignment",
                "source:testing-acceptance",
                "source:cli-contract",
                "source:completion-review-workflow",
                "source:rust-cli-first-adr",
                "source:reviewable-completions-adr",
                "source:workspace-manifest"
            ],
            "excluded_summary": [
                "Git history, issue tracker, pull request comments, CI run history, and the HigherGraphen workspace source body were not ingested.",
                "Rust source files are not parsed generically; selected product, CLI, testing, and ADR files are summarized as advisory records."
            ],
            "extraction_loss": [
                "Repository files are represented as structured claims, not full file contents.",
                "Dependency and CI coverage are summarized for deterministic dogfood coverage.",
                "The dogfood adapter is still curated and does not prove whole-repository architectural coverage."
            ],
            "trust_notes": [
                "Dogfood snapshot generated from local repository files.",
                "Generated records remain reviewable advisory structure, not proof of complete architecture coverage.",
                "Use audit_trace or ai_agent projection with close-check before treating the self-review as complete."
            ],
            "adapter_version": "repo_snapshot:0.1.0"
        },
        "sources": [
            repo_source(&options.repo, "README.md", "source:readme", "Project README", &captured_at)?,
            repo_source(&options.repo, "MANIFEST.md", "source:docs-manifest", "Repository manifest", &captured_at)?,
            repo_source(&options.repo, "docs/99-source-alignment.md", "source:source-alignment", "HigherGraphen source alignment", &captured_at)?,
            repo_source(&options.repo, "docs/13-testing-acceptance.md", "source:testing-acceptance", "Testing and acceptance strategy", &captured_at)?,
            repo_source(&options.repo, "docs/08-cli-contract.md", "source:cli-contract", "CLI contract", &captured_at)?,
            repo_source(&options.repo, "docs/06-completion-and-review-workflow.md", "source:completion-review-workflow", "Completion and review workflow", &captured_at)?,
            repo_source(&options.repo, "adrs/0001-rust-cli-first.md", "source:rust-cli-first-adr", "ADR 0001 Rust CLI first", &captured_at)?,
            repo_source(&options.repo, "adrs/0002-reviewable-completions.md", "source:reviewable-completions-adr", "ADR 0002 reviewable completions", &captured_at)?,
            repo_source(&options.repo, "Cargo.toml", "source:workspace-manifest", "Workspace manifest", &captured_at)?
        ],
        "records": dogfood_records(),
        "metadata": {
            "fixture": false,
            "dogfood": true,
            "repo": options.repo.display().to_string(),
            "generator": "advisorygraphen dogfood repo-snapshot"
        }
    });
    validate_document(&snapshot, Some(advisorygraphen_core::SNAPSHOT_SCHEMA))?;
    write_json_if_requested(&options.output, &snapshot)?;
    Ok(snapshot)
}

fn repo_source(
    repo: &Path,
    relative_path: &str,
    id: &str,
    title: &str,
    captured_at: &str,
) -> AdvisoryResult<Value> {
    let metadata = fs::metadata(repo.join(relative_path))?;
    Ok(json!({
        "id": id,
        "source_type": "repository_file",
        "title": title,
        "uri": relative_path,
        "captured_at": captured_at,
        "classification": "public",
        "metadata": {
            "relative_path": relative_path,
            "byte_len": metadata.len()
        }
    }))
}

fn dogfood_records() -> Vec<Value> {
    vec![
        record(RecordSpec { id: "record:advisorygraphen-runtime", record_type: "component", title: "AdvisoryGraphen Runtime", summary: "Product-specific CLI workflow orchestration for lift, check, completions, project, dogfood, and case commands.", source_ids: &["source:readme", "source:cli-contract", "source:source-alignment"], context_hints: &["advisorygraphen", "runtime"], relation: None, metadata: json!({"component_type": "runtime"}) }),
        record(RecordSpec { id: "record:higher-graphen-primitives", record_type: "component", title: "HigherGraphen Primitives", summary: "HigherGraphen core, structure, interpretation, reasoning, evidence, and projection crates used at AdvisoryGraphen domain boundaries.", source_ids: &["source:source-alignment", "source:workspace-manifest", "source:docs-manifest"], context_hints: &["higher-graphen", "architecture"], relation: None, metadata: json!({"component_type": "library_boundary"}) }),
        record(RecordSpec { id: "record:higher-graphen-runtime", record_type: "component", title: "HigherGraphen Runtime", summary: "Upstream workflow orchestration crate that remains outside the AdvisoryGraphen MVP runtime path.", source_ids: &["source:source-alignment"], context_hints: &["higher-graphen", "runtime"], relation: None, metadata: json!({"component_type": "runtime"}) }),
        record(RecordSpec { id: "record:cli-acceptance-suite", record_type: "test", title: "CLI Acceptance Suite", summary: "End-to-end CLI coverage for lift, check, completions, projection, dogfood generation, and case close-check flows.", source_ids: &["source:testing-acceptance", "source:cli-contract"], context_hints: &["advisorygraphen", "testing"], relation: None, metadata: json!({"test_type": "acceptance"}) }),
        record(RecordSpec { id: "record:hg-boundary-requirement", record_type: "requirement", title: "HG boundary outputs must be observable", summary: "Lift, check, completion, review, and projection outputs should expose HigherGraphen-derived metadata so integration is visible to reviewers.", source_ids: &["source:source-alignment", "source:testing-acceptance", "source:cli-contract"], context_hints: &["higher-graphen", "testing"], relation: None, metadata: json!({"criticality": "high", "require_verification": true}) }),
        record(RecordSpec { id: "record:runtime-adoption-requirement", record_type: "requirement", title: "HG runtime adoption decision needs explicit verification", summary: "The decision to keep AdvisoryGraphen runtime orchestration product-specific should have a post-MVP verification path before replacing it with higher-graphen-runtime.", source_ids: &["source:source-alignment"], context_hints: &["higher-graphen", "runtime"], relation: None, metadata: json!({"criticality": "medium", "require_verification": true}) }),
        record(RecordSpec { id: "record:runtime-adoption-action", record_type: "action", title: "Evaluate higher-graphen-runtime adoption", summary: "Decide whether post-MVP case workflows should call higher-graphen-runtime directly or keep the current AdvisoryGraphen runtime facade.", source_ids: &["source:source-alignment"], context_hints: &["higher-graphen", "runtime"], relation: None, metadata: json!({"priority": "post_mvp"}) }),
        record(RecordSpec { id: "record:hg-boundary-requirement-verified-by-cli-acceptance", record_type: "verification_relation", title: "CLI acceptance verifies HG boundary outputs", summary: "CLI acceptance checks that core dogfood outputs include HigherGraphen-derived fields.", source_ids: &["source:testing-acceptance", "source:cli-contract"], context_hints: &["higher-graphen", "testing"], relation: Some(json!({"relation_type": "verifies", "from_record_id": "record:cli-acceptance-suite", "to_record_id": "record:hg-boundary-requirement"})), metadata: json!({}) }),
        record(RecordSpec { id: "record:advisorygraphen-runtime-uses-hg-primitives", record_type: "dependency_relation", title: "AdvisoryGraphen runtime uses HigherGraphen primitives", summary: "AdvisoryGraphen runtime keeps workflow orchestration local while lower-level crates consume HigherGraphen primitives.", source_ids: &["source:source-alignment", "source:workspace-manifest", "source:docs-manifest"], context_hints: &["advisorygraphen", "higher-graphen"], relation: Some(json!({"relation_type": "uses", "from_record_id": "record:advisorygraphen-runtime", "to_record_id": "record:higher-graphen-primitives"})), metadata: json!({}) }),
        record(RecordSpec { id: "record:reviewable-completion-requirement", record_type: "requirement", title: "Completion candidates must remain review-gated", summary: "Completion candidates are proposals and must not be treated as accepted changes until explicit review events are applied.", source_ids: &["source:completion-review-workflow", "source:reviewable-completions-adr", "source:readme"], context_hints: &["advisorygraphen", "review"], relation: None, metadata: json!({"criticality": "high", "require_verification": true}) }),
        record(RecordSpec { id: "record:reviewable-completion-verified-by-acceptance", record_type: "verification_relation", title: "Acceptance verifies review-gated completions", summary: "Acceptance coverage checks that completion candidates remain unreviewed until explicit accept or reject commands record a review event.", source_ids: &["source:testing-acceptance", "source:completion-review-workflow"], context_hints: &["advisorygraphen", "review"], relation: Some(json!({"relation_type": "verifies", "from_record_id": "record:cli-acceptance-suite", "to_record_id": "record:reviewable-completion-requirement"})), metadata: json!({}) }),
        record(RecordSpec { id: "record:cli-first-decision", record_type: "claim", title: "CLI-first implementation boundary", summary: "The MVP intentionally starts with a Rust CLI, JSON schemas, and file-based workflows before hosted UI or MCP layers.", source_ids: &["source:readme", "source:rust-cli-first-adr", "source:cli-contract"], context_hints: &["advisorygraphen", "architecture"], relation: None, metadata: json!({"decision_status": "accepted"}) }),
    ]
}

struct RecordSpec<'a> {
    id: &'a str,
    record_type: &'a str,
    title: &'a str,
    summary: &'a str,
    source_ids: &'a [&'a str],
    context_hints: &'a [&'a str],
    relation: Option<Value>,
    metadata: Value,
}

fn record(spec: RecordSpec<'_>) -> Value {
    json!({
        "id": spec.id,
        "record_type": spec.record_type,
        "title": spec.title,
        "summary": spec.summary,
        "source_ids": spec.source_ids,
        "context_hints": spec.context_hints,
        "relation": spec.relation,
        "provenance": {
            "origin": "source_backed",
            "actor": "source-adapter:repo-snapshot",
            "confidence": 1.0,
            "review_status": "accepted"
        },
        "metadata": spec.metadata
    })
}
