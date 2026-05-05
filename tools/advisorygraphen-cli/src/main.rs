use advisorygraphen_core::{AdvisoryError, Severity, TOOL_VERSION};
use advisorygraphen_projection::OutputFormat;
use advisorygraphen_runtime::{
    case_close_check_workflow, case_import_workflow, case_reason_workflow, check_workflow,
    code_repo_snapshot_workflow, completions_apply_accepted_workflow, completions_propose_workflow,
    dogfood_repo_snapshot_workflow, hypothesis_accept_workflow,
    hypothesis_apply_proposals_workflow, hypothesis_falsify_workflow, hypothesis_propose_workflow,
    hypothesis_reject_workflow, hypothesis_support_workflow, lift_workflow, project_workflow,
    review_workflow, validate_workflow, CaseCloseCheckOptions, CaseImportOptions,
    CaseReasonOptions, CheckOptions, CodeRepoSnapshotOptions, CompletionApplyAcceptedOptions,
    CompletionProposeOptions, DogfoodRepoSnapshotOptions, HypothesisApplyProposalsOptions,
    HypothesisFalsifyOptions, HypothesisProposeOptions, LiftOptions, ProjectOptions, ReviewOptions,
    ValidateOptions,
};
use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "advisorygraphen",
    version,
    about = "Structured technical advisory CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Version,
    Validate(ValidateArgs),
    Lift(LiftArgs),
    Check(CheckArgs),
    Completions {
        #[command(subcommand)]
        command: CompletionsCommand,
    },
    Project(ProjectArgs),
    Dogfood {
        #[command(subcommand)]
        command: DogfoodCommand,
    },
    Code {
        #[command(subcommand)]
        command: CodeCommand,
    },
    Case {
        #[command(subcommand)]
        command: CaseCommand,
    },
    Hypothesis {
        #[command(subcommand)]
        command: HypothesisCommand,
    },
}

#[derive(Debug, Subcommand)]
enum HypothesisCommand {
    Propose(HypothesisProposeArgs),
    ApplyProposals(HypothesisApplyProposalsArgs),
    Falsify(HypothesisFalsifyArgs),
    Support(HypothesisFalsifyArgs),
    Accept(HypothesisFalsifyArgs),
    Reject(HypothesisFalsifyArgs),
}

#[derive(Debug, Args)]
struct HypothesisProposeArgs {
    #[arg(long)]
    space: PathBuf,
    #[arg(long = "from-report")]
    from_report: PathBuf,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long, default_value = "json")]
    format: String,
}

#[derive(Debug, Args)]
struct HypothesisApplyProposalsArgs {
    #[arg(long)]
    store: PathBuf,
    #[arg(long = "from-report")]
    from_report: PathBuf,
    #[arg(long)]
    policy: Option<PathBuf>,
    #[arg(long)]
    reviewer: String,
    #[arg(long)]
    reason: String,
    #[arg(long = "base-revision")]
    base_revision: Option<String>,
    #[arg(long)]
    dry_run: bool,
    #[arg(long, default_value = "json")]
    format: String,
}

#[derive(Debug, Args)]
struct HypothesisFalsifyArgs {
    #[arg(long)]
    store: PathBuf,
    #[arg(long = "from-report")]
    from_report: PathBuf,
    #[arg(long = "hypothesis-id")]
    hypothesis_id: String,
    #[arg(long = "evidence")]
    evidence: Vec<String>,
    #[arg(long)]
    reviewer: String,
    #[arg(long)]
    reason: String,
    #[arg(long = "base-revision")]
    base_revision: Option<String>,
    #[arg(long, default_value = "json")]
    format: String,
}

#[derive(Debug, Subcommand)]
enum CompletionsCommand {
    Propose(CompletionProposeArgs),
    Accept(ReviewArgs),
    Reject(ReviewArgs),
    ApplyAccepted(CompletionApplyAcceptedArgs),
}

#[derive(Debug, Subcommand)]
enum CaseCommand {
    Import(CaseImportArgs),
    Reason(CaseReasonArgs),
    CloseCheck(CaseCloseCheckArgs),
}

#[derive(Debug, Subcommand)]
enum DogfoodCommand {
    RepoSnapshot(DogfoodRepoSnapshotArgs),
}

#[derive(Debug, Subcommand)]
enum CodeCommand {
    RepoSnapshot(CodeRepoSnapshotArgs),
}

#[derive(Debug, Args)]
struct ValidateArgs {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    schema: Option<String>,
    #[arg(long, default_value = "json")]
    format: String,
}

#[derive(Debug, Args)]
struct LiftArgs {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    package: String,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long, default_value = "json")]
    format: String,
}

#[derive(Debug, Args)]
struct CheckArgs {
    #[arg(long)]
    space: PathBuf,
    #[arg(long)]
    ruleset: String,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long, default_value = "json")]
    format: String,
    #[arg(long)]
    fail_on: Option<String>,
}

#[derive(Debug, Args)]
struct CompletionProposeArgs {
    #[arg(long)]
    space: PathBuf,
    #[arg(long = "from-report")]
    from_report: PathBuf,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long, default_value = "json")]
    format: String,
}

#[derive(Debug, Args)]
struct CompletionApplyAcceptedArgs {
    #[arg(long)]
    store: PathBuf,
    #[arg(long = "space-id")]
    space_id: String,
    #[arg(long)]
    reviewer: String,
    #[arg(long)]
    reason: String,
    #[arg(long = "base-revision")]
    base_revision: Option<String>,
    #[arg(long)]
    dry_run: bool,
    #[arg(long, default_value = "json")]
    format: String,
}

#[derive(Debug, Args)]
struct ReviewArgs {
    #[arg(long)]
    store: PathBuf,
    #[arg(long = "candidate-id")]
    candidate_id: String,
    #[arg(long = "from-report")]
    from_report: Option<PathBuf>,
    #[arg(long)]
    reviewer: String,
    #[arg(long)]
    reason: String,
    #[arg(long = "base-revision")]
    base_revision: Option<String>,
    #[arg(long, default_value = "json")]
    format: String,
}

#[derive(Debug, Args)]
struct ProjectArgs {
    #[arg(long)]
    space: PathBuf,
    #[arg(long)]
    report: PathBuf,
    #[arg(long = "completions-report")]
    completions_report: Option<PathBuf>,
    #[arg(long)]
    audience: String,
    #[arg(long, default_value = "json")]
    format: String,
    #[arg(long)]
    output: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct DogfoodRepoSnapshotArgs {
    #[arg(long, default_value = ".")]
    repo: PathBuf,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long, default_value = "json")]
    format: String,
}

#[derive(Debug, Args)]
struct CodeRepoSnapshotArgs {
    #[arg(long, default_value = ".")]
    repo: PathBuf,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long, default_value = "json")]
    format: String,
}

#[derive(Debug, Args)]
struct CaseImportArgs {
    #[arg(long)]
    store: PathBuf,
    #[arg(long)]
    space: PathBuf,
    #[arg(long = "revision-id")]
    revision_id: String,
    #[arg(long, default_value = "json")]
    format: String,
}

#[derive(Debug, Args)]
struct CaseReasonArgs {
    #[arg(long)]
    store: PathBuf,
    #[arg(long = "space-id")]
    space_id: String,
    #[arg(long, default_value = "json")]
    format: String,
}

#[derive(Debug, Args)]
struct CaseCloseCheckArgs {
    #[arg(long)]
    store: PathBuf,
    #[arg(long = "space-id")]
    space_id: String,
    #[arg(long = "base-revision")]
    base_revision: Option<String>,
    #[arg(long, default_value = "json")]
    format: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(error.exit_code());
    }
}

fn run() -> Result<(), AdvisoryError> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Version) {
        Command::Version => {
            println!("advisorygraphen {TOOL_VERSION}");
            Ok(())
        }
        Command::Validate(args) => {
            require_json_format(&args.format)?;
            print_json(&validate_workflow(&ValidateOptions {
                input: args.input,
                schema: args.schema,
            })?)
        }
        Command::Lift(args) => {
            require_json_format(&args.format)?;
            let space = lift_workflow(&LiftOptions {
                input: args.input,
                package: args.package,
                output: args.output,
                command: Some(command_string()),
            })?;
            print_json(&space)
        }
        Command::Check(args) => {
            require_json_format(&args.format)?;
            let fail_on = parse_fail_on(args.fail_on.as_deref())?;
            let report = check_workflow(&CheckOptions {
                space: args.space,
                ruleset: args.ruleset,
                output: args.output,
                fail_on,
                command: Some(command_string()),
            })?;
            print_json(&report)
        }
        Command::Completions { command } => match command {
            CompletionsCommand::Propose(args) => {
                require_json_format(&args.format)?;
                let report = completions_propose_workflow(&CompletionProposeOptions {
                    space: args.space,
                    from_report: args.from_report,
                    output: args.output,
                    command: Some(command_string()),
                })?;
                print_json(&report)
            }
            CompletionsCommand::Accept(args) => run_review(args, "accepted"),
            CompletionsCommand::Reject(args) => run_review(args, "rejected"),
            CompletionsCommand::ApplyAccepted(args) => {
                require_json_format(&args.format)?;
                print_json(&completions_apply_accepted_workflow(
                    &CompletionApplyAcceptedOptions {
                        store: args.store,
                        space_id: args.space_id,
                        reviewer: args.reviewer,
                        reason: args.reason,
                        base_revision: args.base_revision,
                        dry_run: args.dry_run,
                    },
                )?)
            }
        },
        Command::Project(args) => {
            let format = OutputFormat::parse(&args.format)?;
            let rendered = project_workflow(&ProjectOptions {
                space: args.space,
                report: args.report,
                completions_report: args.completions_report,
                audience: args.audience,
                format,
                output: args.output,
            })?;
            println!("{rendered}");
            Ok(())
        }
        Command::Dogfood { command } => match command {
            DogfoodCommand::RepoSnapshot(args) => {
                require_json_format(&args.format)?;
                print_json(&dogfood_repo_snapshot_workflow(
                    &DogfoodRepoSnapshotOptions {
                        repo: args.repo,
                        output: args.output,
                    },
                )?)
            }
        },
        Command::Code { command } => match command {
            CodeCommand::RepoSnapshot(args) => {
                require_json_format(&args.format)?;
                print_json(&code_repo_snapshot_workflow(&CodeRepoSnapshotOptions {
                    repo: args.repo,
                    output: args.output,
                })?)
            }
        },
        Command::Hypothesis { command } => match command {
            HypothesisCommand::Propose(args) => {
                require_json_format(&args.format)?;
                print_json(&hypothesis_propose_workflow(&HypothesisProposeOptions {
                    space: args.space,
                    from_report: args.from_report,
                    output: args.output,
                    command: Some(command_string()),
                })?)
            }
            HypothesisCommand::ApplyProposals(args) => {
                require_json_format(&args.format)?;
                print_json(&hypothesis_apply_proposals_workflow(
                    &HypothesisApplyProposalsOptions {
                        store: args.store,
                        from_report: args.from_report,
                        policy: args.policy,
                        reviewer: args.reviewer,
                        reason: args.reason,
                        base_revision: args.base_revision,
                        dry_run: args.dry_run,
                    },
                )?)
            }
            HypothesisCommand::Falsify(args) => run_hypothesis_event(args, "falsify"),
            HypothesisCommand::Support(args) => run_hypothesis_event(args, "support"),
            HypothesisCommand::Accept(args) => run_hypothesis_event(args, "accept"),
            HypothesisCommand::Reject(args) => run_hypothesis_event(args, "reject"),
        },
        Command::Case { command } => match command {
            CaseCommand::Import(args) => {
                require_json_format(&args.format)?;
                print_json(&case_import_workflow(&CaseImportOptions {
                    store: args.store,
                    space: args.space,
                    revision_id: args.revision_id,
                })?)
            }
            CaseCommand::Reason(args) => {
                require_json_format(&args.format)?;
                print_json(&case_reason_workflow(&CaseReasonOptions {
                    store: args.store,
                    space_id: args.space_id,
                })?)
            }
            CaseCommand::CloseCheck(args) => {
                require_json_format(&args.format)?;
                print_json(&case_close_check_workflow(&CaseCloseCheckOptions {
                    store: args.store,
                    space_id: args.space_id,
                    base_revision: args.base_revision,
                })?)
            }
        },
    }
}

fn run_hypothesis_event(args: HypothesisFalsifyArgs, action: &str) -> Result<(), AdvisoryError> {
    require_json_format(&args.format)?;
    let options = HypothesisFalsifyOptions {
        store: args.store,
        from_report: args.from_report,
        hypothesis_id: args.hypothesis_id,
        evidence_ids: args.evidence,
        reviewer: args.reviewer,
        reason: args.reason,
        base_revision: args.base_revision,
    };
    let event = match action {
        "falsify" => hypothesis_falsify_workflow(&options)?,
        "support" => hypothesis_support_workflow(&options)?,
        "accept" => hypothesis_accept_workflow(&options)?,
        "reject" => hypothesis_reject_workflow(&options)?,
        other => {
            return Err(AdvisoryError::Validation(format!(
                "unsupported hypothesis action: {other}"
            )))
        }
    };
    print_json(&event)
}

fn run_review(args: ReviewArgs, outcome: &str) -> Result<(), AdvisoryError> {
    require_json_format(&args.format)?;
    print_json(&review_workflow(&ReviewOptions {
        store: args.store,
        candidate_id: args.candidate_id,
        from_report: args.from_report,
        reviewer: args.reviewer,
        reason: args.reason,
        outcome: outcome.to_string(),
        base_revision: args.base_revision,
    })?)
}

fn parse_fail_on(value: Option<&str>) -> Result<Option<Severity>, AdvisoryError> {
    value
        .map(|value| {
            Severity::parse(value)
                .ok_or_else(|| AdvisoryError::Validation(format!("invalid severity: {value}")))
        })
        .transpose()
}

fn require_json_format(format: &str) -> Result<(), AdvisoryError> {
    if format == "json" {
        Ok(())
    } else {
        Err(AdvisoryError::Validation(format!(
            "only json format is supported for this command: {format}"
        )))
    }
}

fn print_json<T: Serialize>(value: &T) -> Result<(), AdvisoryError> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn command_string() -> String {
    std::env::args().collect::<Vec<_>>().join(" ")
}
