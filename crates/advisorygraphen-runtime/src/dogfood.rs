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
                "source:source-alignment",
                "source:testing-acceptance",
                "source:workspace-manifest"
            ],
            "excluded_summary": [
                "Git history, issue tracker, pull request comments, and the HigherGraphen workspace source body were not ingested.",
                "This bounded ingestion reads selected repository files and summarizes them as advisory records."
            ],
            "extraction_loss": [
                "Repository files are represented as structured claims, not full file contents.",
                "Dependency and CI coverage are summarized for deterministic dogfood coverage."
            ],
            "trust_notes": [
                "Dogfood snapshot generated from local repository files.",
                "Generated records remain reviewable advisory structure, not proof of complete architecture coverage."
            ],
            "adapter_version": "repo_snapshot:0.1.0"
        },
        "sources": [
            repo_source(&options.repo, "docs/99-source-alignment.md", "source:source-alignment", "HigherGraphen source alignment", &captured_at)?,
            repo_source(&options.repo, "docs/13-testing-acceptance.md", "source:testing-acceptance", "Testing and acceptance strategy", &captured_at)?,
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
        record(RecordSpec { id: "record:advisorygraphen-runtime", record_type: "component", title: "AdvisoryGraphen Runtime", summary: "Product-specific CLI workflow orchestration for lift, check, completions, project, dogfood, and case commands.", source_ids: &["source:source-alignment"], context_hints: &["advisorygraphen", "runtime"], relation: None, metadata: json!({"component_type": "runtime"}) }),
        record(RecordSpec { id: "record:higher-graphen-primitives", record_type: "component", title: "HigherGraphen Primitives", summary: "HigherGraphen core, structure, interpretation, reasoning, evidence, and projection crates used at AdvisoryGraphen domain boundaries.", source_ids: &["source:source-alignment", "source:workspace-manifest"], context_hints: &["higher-graphen", "architecture"], relation: None, metadata: json!({"component_type": "library_boundary"}) }),
        record(RecordSpec { id: "record:higher-graphen-runtime", record_type: "component", title: "HigherGraphen Runtime", summary: "Upstream workflow orchestration crate that remains outside the AdvisoryGraphen MVP runtime path.", source_ids: &["source:source-alignment"], context_hints: &["higher-graphen", "runtime"], relation: None, metadata: json!({"component_type": "runtime"}) }),
        record(RecordSpec { id: "record:cli-acceptance-suite", record_type: "test", title: "CLI Acceptance Suite", summary: "End-to-end CLI coverage for lift, check, completions, projection, dogfood generation, and case close-check flows.", source_ids: &["source:testing-acceptance"], context_hints: &["advisorygraphen", "testing"], relation: None, metadata: json!({"test_type": "acceptance"}) }),
        record(RecordSpec { id: "record:hg-boundary-requirement", record_type: "requirement", title: "HG boundary outputs must be observable", summary: "Lift, check, completion, review, and projection outputs should expose HigherGraphen-derived metadata so integration is visible to reviewers.", source_ids: &["source:source-alignment", "source:testing-acceptance"], context_hints: &["higher-graphen", "testing"], relation: None, metadata: json!({"criticality": "high", "require_verification": true}) }),
        record(RecordSpec { id: "record:runtime-adoption-requirement", record_type: "requirement", title: "HG runtime adoption decision needs explicit verification", summary: "The decision to keep AdvisoryGraphen runtime orchestration product-specific should have a post-MVP verification path before replacing it with higher-graphen-runtime.", source_ids: &["source:source-alignment"], context_hints: &["higher-graphen", "runtime"], relation: None, metadata: json!({"criticality": "medium", "require_verification": true}) }),
        record(RecordSpec { id: "record:runtime-adoption-action", record_type: "action", title: "Evaluate higher-graphen-runtime adoption", summary: "Decide whether post-MVP case workflows should call higher-graphen-runtime directly or keep the current AdvisoryGraphen runtime facade.", source_ids: &["source:source-alignment"], context_hints: &["higher-graphen", "runtime"], relation: None, metadata: json!({"priority": "post_mvp"}) }),
        record(RecordSpec { id: "record:hg-boundary-requirement-verified-by-cli-acceptance", record_type: "verification_relation", title: "CLI acceptance verifies HG boundary outputs", summary: "CLI acceptance checks that core dogfood outputs include HigherGraphen-derived fields.", source_ids: &["source:testing-acceptance"], context_hints: &["higher-graphen", "testing"], relation: Some(json!({"relation_type": "verifies", "from_record_id": "record:cli-acceptance-suite", "to_record_id": "record:hg-boundary-requirement"})), metadata: json!({}) }),
        record(RecordSpec { id: "record:advisorygraphen-runtime-uses-hg-primitives", record_type: "dependency_relation", title: "AdvisoryGraphen runtime uses HigherGraphen primitives", summary: "AdvisoryGraphen runtime keeps workflow orchestration local while lower-level crates consume HigherGraphen primitives.", source_ids: &["source:source-alignment", "source:workspace-manifest"], context_hints: &["advisorygraphen", "higher-graphen"], relation: Some(json!({"relation_type": "uses", "from_record_id": "record:advisorygraphen-runtime", "to_record_id": "record:higher-graphen-primitives"})), metadata: json!({}) }),
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
