use advisorygraphen_core::Severity;
use advisorygraphen_projection::OutputFormat;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ValidateOptions {
    pub input: PathBuf,
    pub schema: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LiftOptions {
    pub input: PathBuf,
    pub package: String,
    pub output: Option<PathBuf>,
    pub command: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CheckOptions {
    pub space: PathBuf,
    pub ruleset: String,
    pub output: Option<PathBuf>,
    pub fail_on: Option<Severity>,
    pub command: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompletionProposeOptions {
    pub space: PathBuf,
    pub from_report: PathBuf,
    pub output: Option<PathBuf>,
    pub command: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HypothesisProposeOptions {
    pub space: PathBuf,
    pub from_report: PathBuf,
    pub output: Option<PathBuf>,
    pub command: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HypothesisApplyProposalsOptions {
    pub store: PathBuf,
    pub from_report: PathBuf,
    pub policy: Option<PathBuf>,
    pub reviewer: String,
    pub reason: String,
    pub base_revision: Option<String>,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct ReviewOptions {
    pub store: PathBuf,
    pub candidate_id: String,
    pub from_report: Option<PathBuf>,
    pub reviewer: String,
    pub reason: String,
    pub outcome: String,
    pub base_revision: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompletionApplyAcceptedOptions {
    pub store: PathBuf,
    pub space_id: String,
    pub reviewer: String,
    pub reason: String,
    pub base_revision: Option<String>,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct ProjectOptions {
    pub space: PathBuf,
    pub report: PathBuf,
    pub completions_report: Option<PathBuf>,
    pub audience: String,
    pub format: OutputFormat,
    pub output: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct CaseImportOptions {
    pub store: PathBuf,
    pub space: PathBuf,
    pub revision_id: String,
}

#[derive(Debug, Clone)]
pub struct CaseReasonOptions {
    pub store: PathBuf,
    pub space_id: String,
}

#[derive(Debug, Clone)]
pub struct CaseCloseCheckOptions {
    pub store: PathBuf,
    pub space_id: String,
    pub base_revision: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HypothesisFalsifyOptions {
    pub store: PathBuf,
    pub from_report: PathBuf,
    pub hypothesis_id: String,
    pub evidence_ids: Vec<String>,
    pub reviewer: String,
    pub reason: String,
    pub base_revision: Option<String>,
}
