# 16. Rust API Sketches

These sketches are implementation guidance, not final compiled code. Exact HigherGraphen crate APIs should be verified against the selected release or path dependency.

## Core types

```rust
use serde::{Deserialize, Serialize};
use indexmap::IndexMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvisorySpaceEnvelope {
    pub schema: String,
    pub space_id: String,
    pub engagement_id: String,
    pub snapshot_id: String,
    pub package_id: String,
    pub cells: Vec<AdvisoryCell>,
    pub contexts: Vec<AdvisoryContext>,
    pub incidences: Vec<AdvisoryIncidence>,
    pub morphisms: Vec<AdvisoryMorphism>,
    pub invariants: Vec<AdvisoryInvariant>,
    pub policies: Vec<AdvisoryPolicy>,
    pub metadata: IndexMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvisoryCell {
    pub id: String,
    pub cell_type: AdvisoryCellKind,
    pub title: String,
    pub summary: Option<String>,
    pub context_ids: Vec<String>,
    pub source_ids: Vec<String>,
    pub structure_refs: Vec<String>,
    pub provenance: AdvisoryProvenance,
    pub metadata: IndexMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryCellKind {
    Observation,
    Claim,
    Evidence,
    Component,
    Interface,
    DataStore,
    Requirement,
    TestOrVerification,
    Decision,
    Risk,
    Action,
    Owner,
    Metric,
    Policy,
    Obstruction,
    Completion,
    ProjectionRecord,
    Custom(String),
}
```

## Provenance

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvisoryProvenance {
    pub origin: EvidenceOrigin,
    pub actor: String,
    pub confidence: Option<f64>,
    pub review_status: ReviewStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceOrigin {
    SourceBacked,
    Inferred,
    ReviewPromoted,
    Rejected,
    Contradicting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    Unreviewed,
    NeedsReview,
    Accepted,
    Rejected,
    Waived,
    Superseded,
    Reopened,
}
```

## Validation

```rust
pub fn validate_space(space: &AdvisorySpaceEnvelope) -> AdvisoryResult<ValidationReport> {
    validate_schema(&space.schema, "advisorygraphen.space.v1")?;
    validate_unique_ids(space)?;
    validate_relation_endpoints(space)?;
    validate_source_refs(space)?;
    validate_provenance(space)?;
    validate_no_hidden_required_metadata(space)?;
    Ok(ValidationReport::valid())
}
```

## Interpretation package

```rust
pub trait AdvisoryInterpretationPackage {
    fn package_id(&self) -> &'static str;
    fn vocabulary(&self) -> VocabularyMap;
    fn invariant_evaluators(&self) -> Vec<Box<dyn AdvisoryInvariantEvaluator>>;
    fn completion_rules(&self) -> Vec<Box<dyn CompletionRule>>;
    fn projection_renderers(&self) -> Vec<Box<dyn ProjectionRenderer>>;
    fn default_policy(&self) -> AdvisoryPolicy;
}

pub fn technical_advisory_mvp_package() -> impl AdvisoryInterpretationPackage {
    TechnicalAdvisoryMvpPackage::default()
}
```

## Runtime workflows

```rust
pub async fn lift_workflow(args: LiftWorkflowArgs) -> AdvisoryResult<AdvisorySpaceEnvelope> {
    let snapshot = load_snapshot(&args.input).await?;
    validate_snapshot(&snapshot)?;
    let package = load_package(&args.package_id)?;
    let space = lift_snapshot(snapshot, package.as_ref())?;
    validate_space(&space)?;
    write_if_requested(&args.output, &space).await?;
    Ok(space)
}

pub async fn check_workflow(args: CheckWorkflowArgs) -> AdvisoryResult<AdvisoryReportEnvelope> {
    let space = load_space(&args.space).await?;
    validate_space(&space)?;
    let package = load_package_for_space(&space)?;
    let results = run_invariants(&space, package.as_ref(), &args.ruleset)?;
    let obstructions = emit_obstructions(&space, &results)?;
    Ok(AdvisoryReportEnvelope::check(space.space_id.clone(), results, obstructions))
}
```

## Projection renderer

```rust
pub struct ExecutiveProjectionRenderer;

impl ProjectionRenderer for ExecutiveProjectionRenderer {
    fn audience(&self) -> AdvisoryAudience {
        AdvisoryAudience::Executive
    }

    fn project(
        &self,
        space: &AdvisorySpaceEnvelope,
        report: &AdvisoryReportEnvelope,
        request: &ProjectionRequest,
    ) -> AdvisoryResult<ProjectionResult> {
        // Select only decision-relevant cells.
        // Preserve high-severity obstructions.
        // Do not promote unreviewed candidates.
        // Declare projection loss.
        todo!()
    }
}
```

## Error type

```rust
#[derive(thiserror::Error, Debug)]
pub enum AdvisoryError {
    #[error("schema mismatch: expected {expected}, got {actual}")]
    SchemaMismatch { expected: String, actual: String },

    #[error("duplicate id: {0}")]
    DuplicateId(String),

    #[error("unresolved reference: {0}")]
    UnresolvedReference(String),

    #[error("unsupported package: {0}")]
    UnsupportedPackage(String),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub type AdvisoryResult<T> = Result<T, AdvisoryError>;
```

## Report envelope builder

```rust
impl AdvisoryReportEnvelope {
    pub fn check(
        space_id: String,
        invariant_results: Vec<InvariantCheckResult>,
        obstructions: Vec<AdvisoryObstruction>,
    ) -> Self {
        Self {
            schema: "advisorygraphen.report.v1".to_string(),
            report_type: "check".to_string(),
            report_version: 1,
            tool: ToolMetadata::current(),
            input: serde_json::json!({ "space_id": space_id }),
            result: serde_json::json!({
                "invariant_results": invariant_results,
                "obstructions": obstructions,
            }),
            projection: serde_json::json!({}),
            warnings: vec![],
        }
    }
}
```

## Testing helper

```rust
pub fn assert_all_candidates_unreviewed(report: &AdvisoryReportEnvelope) {
    let candidates = report.result["completion_candidates"].as_array().unwrap();
    for candidate in candidates {
        assert_eq!(candidate["review_status"], "unreviewed");
    }
}
```
