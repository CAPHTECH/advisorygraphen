use advisorygraphen_core::{AdvisoryError, Severity, TOOL_VERSION};
use advisorygraphen_projection::OutputFormat;
use advisorygraphen_runtime::{
    case_close_check_workflow, case_import_workflow, case_reason_workflow, check_workflow,
    completions_propose_workflow, dogfood_repo_snapshot_workflow, lift_workflow, project_workflow,
    review_workflow, validate_workflow, CaseCloseCheckOptions, CaseImportOptions,
    CaseReasonOptions, CheckOptions, CompletionProposeOptions, DogfoodRepoSnapshotOptions,
    LiftOptions, ProjectOptions, ReviewOptions, ValidateOptions,
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
    Case {
        #[command(subcommand)]
        command: CaseCommand,
    },
}

#[derive(Debug, Subcommand)]
enum CompletionsCommand {
    Propose(CompletionProposeArgs),
    Accept(ReviewArgs),
    Reject(ReviewArgs),
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
