use crate::write_json_if_requested;
use advisorygraphen_core::{validate_document, AdvisoryResult};
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CodeRepoSnapshotOptions {
    pub repo: PathBuf,
    pub output: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct CodeFile {
    relative_path: String,
    source_id: String,
    contents: String,
    kind: CodeFileKind,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum CodeFileKind {
    Manifest,
    Source,
    Test,
    ApiRoute,
}

#[derive(Debug, Default)]
struct Coverage {
    parsed_files: usize,
    skipped_files: usize,
    unsupported_extensions: BTreeMap<String, usize>,
    api_route_files: usize,
    test_files: usize,
    db_access_files: usize,
    env_usage_files: usize,
}

pub fn code_repo_snapshot_workflow(options: &CodeRepoSnapshotOptions) -> AdvisoryResult<Value> {
    let captured_at = Utc::now().to_rfc3339();
    let repo_name = options
        .repo
        .file_name()
        .and_then(|name| name.to_str())
        .map(slug)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "repository".to_string());
    let mut coverage = Coverage::default();
    let files = collect_code_files(&options.repo, &mut coverage)?;
    let sources = files
        .iter()
        .map(|file| code_source(file, &captured_at))
        .collect::<Vec<_>>();
    let records = code_records(&files, &mut coverage);
    let source_ids = sources
        .iter()
        .filter_map(|source| source.get("id").and_then(Value::as_str))
        .collect::<Vec<_>>();
    let snapshot = json!({
        "schema": "advisorygraphen.engagement.snapshot.v1",
        "snapshot_id": format!("snapshot:code-{repo_name}"),
        "engagement_id": format!("engagement:code-review-{repo_name}"),
        "captured_at": captured_at,
        "source_boundary": {
            "included_source_ids": source_ids,
            "excluded_summary": [
                "Generated, dependency, build output, hidden, and unsupported files were not parsed.",
                "This adapter currently extracts deterministic TypeScript/JavaScript/Next.js signals only."
            ],
            "extraction_loss": [
                "Code is represented as route, dependency, database, environment, and test records, not full source text.",
                "Detection is lexical and path-based; it does not resolve TypeScript types or runtime control flow."
            ],
            "trust_notes": [
                "AST-free deterministic scanner intended to seed AdvisoryGraphen review, not prove whole-program behavior.",
                "Use coverage_summary before treating findings as complete."
            ],
            "adapter_version": "code_repo_snapshot:0.1.0"
        },
        "sources": sources,
        "records": records,
        "metadata": {
            "adapter": "code_repo_snapshot",
            "repo": options.repo.display().to_string(),
            "coverage_summary": coverage_json(&coverage)
        }
    });
    validate_document(&snapshot, Some(advisorygraphen_core::SNAPSHOT_SCHEMA))?;
    write_json_if_requested(&options.output, &snapshot)?;
    Ok(snapshot)
}

fn collect_code_files(repo: &Path, coverage: &mut Coverage) -> AdvisoryResult<Vec<CodeFile>> {
    let mut paths = Vec::new();
    collect_paths(repo, repo, &mut paths, coverage)?;
    paths.sort();
    paths.dedup();
    let mut files = Vec::new();
    for relative_path in paths {
        let full_path = repo.join(&relative_path);
        let contents = fs::read_to_string(&full_path)?;
        let kind = classify_file(&relative_path);
        coverage.parsed_files += 1;
        if kind == CodeFileKind::ApiRoute {
            coverage.api_route_files += 1;
        }
        if kind == CodeFileKind::Test {
            coverage.test_files += 1;
        }
        files.push(CodeFile {
            source_id: format!("source:code-{}", path_slug(&relative_path)),
            relative_path,
            contents,
            kind,
        });
    }
    Ok(files)
}

fn collect_paths(
    repo: &Path,
    dir: &Path,
    paths: &mut Vec<String>,
    coverage: &mut Coverage,
) -> AdvisoryResult<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path
            .strip_prefix(repo)
            .unwrap_or(path.as_path())
            .to_string_lossy()
            .replace('\\', "/");
        if should_skip(&relative) {
            coverage.skipped_files += 1;
            continue;
        }
        if path.is_dir() {
            collect_paths(repo, &path, paths, coverage)?;
            continue;
        }
        if is_supported_file(&relative) {
            paths.push(relative);
        } else if let Some(extension) = Path::new(&relative)
            .extension()
            .and_then(|value| value.to_str())
        {
            *coverage
                .unsupported_extensions
                .entry(extension.to_string())
                .or_default() += 1;
        }
    }
    Ok(())
}

fn should_skip(relative_path: &str) -> bool {
    relative_path.split('/').any(|part| {
        matches!(
            part,
            ".git"
                | ".next"
                | ".turbo"
                | "build"
                | "coverage"
                | "dist"
                | "node_modules"
                | "target"
                | "vendor"
        )
    })
}

fn is_supported_file(relative_path: &str) -> bool {
    matches!(relative_path, "package.json" | "tsconfig.json")
        || matches!(
            Path::new(relative_path)
                .extension()
                .and_then(|value| value.to_str()),
            Some("js" | "jsx" | "ts" | "tsx")
        )
}

fn classify_file(relative_path: &str) -> CodeFileKind {
    if matches!(relative_path, "package.json" | "tsconfig.json") {
        CodeFileKind::Manifest
    } else if is_api_route(relative_path) {
        CodeFileKind::ApiRoute
    } else if is_test_file(relative_path) {
        CodeFileKind::Test
    } else {
        CodeFileKind::Source
    }
}

fn is_api_route(relative_path: &str) -> bool {
    (relative_path.starts_with("app/api/") || relative_path.starts_with("src/app/api/"))
        && (relative_path.ends_with("/route.ts")
            || relative_path.ends_with("/route.tsx")
            || relative_path.ends_with("/route.js")
            || relative_path.ends_with("/route.jsx"))
}

fn is_test_file(relative_path: &str) -> bool {
    relative_path.contains(".test.")
        || relative_path.contains(".spec.")
        || relative_path.starts_with("__tests__/")
        || relative_path.contains("/__tests__/")
}

fn code_source(file: &CodeFile, captured_at: &str) -> Value {
    json!({
        "id": file.source_id,
        "source_type": match file.kind {
            CodeFileKind::Manifest => "code_manifest",
            CodeFileKind::ApiRoute => "api_route_file",
            CodeFileKind::Test => "test_file",
            CodeFileKind::Source => "code_file"
        },
        "title": file.relative_path,
        "uri": file.relative_path,
        "captured_at": captured_at,
        "classification": "public",
        "metadata": {
            "relative_path": file.relative_path,
            "byte_len": file.contents.len(),
            "language": language_for_path(&file.relative_path)
        }
    })
}

fn code_records(files: &[CodeFile], coverage: &mut Coverage) -> Vec<Value> {
    let mut records = Vec::new();
    if let Some(package_file) = files
        .iter()
        .find(|file| file.relative_path == "package.json")
    {
        records.push(record_owned(OwnedRecordSpec {
            id: "record:package-manifest".to_string(),
            record_type: "component".to_string(),
            title: "Node package manifest".to_string(),
            summary: "package.json declares the JavaScript/TypeScript application boundary."
                .to_string(),
            source_ids: vec![package_file.source_id.clone()],
            context_hints: vec!["code".to_string(), "manifest".to_string()],
            relation: None,
            metadata: json!({"component_type": "manifest"}),
        }));
    }

    let has_any_db_access = files.iter().any(has_db_access);
    if has_any_db_access {
        records.push(record(RecordSpec {
            id: "record:application-database",
            record_type: "data_store",
            title: "Application database",
            summary:
                "Detected database access through Prisma, SQL, database service, or query helpers.",
            source_ids: &[],
            context_hints: &["code", "data"],
            relation: None,
            metadata: json!({"store_type": "database", "confidence": "medium"}),
        }));
    }

    for file in files {
        if has_db_access(file) {
            coverage.db_access_files += 1;
        }
        if !env_var_names(&file.contents).is_empty() {
            coverage.env_usage_files += 1;
        }
        match file.kind {
            CodeFileKind::ApiRoute => records.extend(api_route_records(file, has_any_db_access)),
            CodeFileKind::Test => records.push(test_record(file)),
            CodeFileKind::Manifest | CodeFileKind::Source => {}
        }
        records.extend(env_records(file));
    }

    let mut seen = BTreeSet::new();
    records.retain(|record| {
        record
            .get("id")
            .and_then(Value::as_str)
            .map(|id| seen.insert(id.to_string()))
            .unwrap_or(false)
    });
    records
}

fn api_route_records(file: &CodeFile, has_db_store: bool) -> Vec<Value> {
    let route_id = format!("record:api-route-{}", path_slug(&file.relative_path));
    let methods = http_methods(&file.contents);
    let mut records = vec![record_owned(OwnedRecordSpec {
        id: route_id.clone(),
        record_type: "component".to_string(),
        title: format!("API route {}", route_path(&file.relative_path)),
        summary: format!(
            "Next.js API route exposing {}.",
            if methods.is_empty() {
                "an unknown HTTP method".to_string()
            } else {
                methods.join(", ")
            }
        ),
        source_ids: vec![file.source_id.clone()],
        context_hints: vec!["code".to_string(), "api".to_string()],
        relation: None,
        metadata: json!({
            "component_type": "api_endpoint",
            "route_path": route_path(&file.relative_path),
            "http_methods": methods,
            "auth_detected": has_auth_check(&file.contents),
            "db_access_detected": has_db_access(file),
            "env_var_names": env_var_names(&file.contents),
            "confidence": "medium"
        }),
    })];
    if has_db_store && has_db_access(file) {
        records.push(record_owned(OwnedRecordSpec {
            id: format!(
                "record:{}-accesses-application-database",
                route_id.trim_start_matches("record:")
            ),
            record_type: "access_relation".to_string(),
            title: format!(
                "{} accesses application database",
                route_path(&file.relative_path)
            ),
            summary: "Route file contains database access signals.".to_string(),
            source_ids: vec![file.source_id.clone()],
            context_hints: vec!["code".to_string(), "api".to_string(), "data".to_string()],
            relation: Some(json!({
                "relation_type": "accesses",
                "from_record_id": route_id,
                "to_record_id": "record:application-database"
            })),
            metadata: json!({
                "access_type": "database_access",
                "detectors": db_detectors(&file.contents),
                "confidence": "medium"
            }),
        }));
    }
    records
}

fn test_record(file: &CodeFile) -> Value {
    record_owned(OwnedRecordSpec {
        id: format!("record:test-{}", path_slug(&file.relative_path)),
        record_type: "test_or_verification".to_string(),
        title: format!("Test {}", file.relative_path),
        summary: "Detected test/spec file in the repository.".to_string(),
        source_ids: vec![file.source_id.clone()],
        context_hints: vec!["code".to_string(), "testing".to_string()],
        relation: None,
        metadata: json!({
            "test_type": "code_test",
            "confidence": "high"
        }),
    })
}

fn env_records(file: &CodeFile) -> Vec<Value> {
    env_var_names(&file.contents)
        .into_iter()
        .map(|name| {
            record_owned(OwnedRecordSpec {
                id: format!(
                    "record:env-{}-{}",
                    slug(&name),
                    path_slug(&file.relative_path)
                ),
                record_type: "requirement".to_string(),
                title: format!("Environment variable {name} is used"),
                summary: format!("{name} is referenced from {}.", file.relative_path),
                source_ids: vec![file.source_id.clone()],
                context_hints: vec!["code".to_string(), "configuration".to_string()],
                relation: None,
                metadata: json!({
                    "require_verification": true,
                    "verification_required": true,
                    "criticality": "medium",
                    "requirement_type": "configuration_validation",
                    "env_var_name": name,
                    "confidence": "high"
                }),
            })
        })
        .collect()
}

fn http_methods(contents: &str) -> Vec<String> {
    ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
        .into_iter()
        .filter(|method| {
            contents.contains(&format!("function {method}"))
                || contents.contains(&format!("const {method}"))
                || contents.contains(&format!("export async function {method}"))
                || contents.contains(&format!("export function {method}"))
        })
        .map(str::to_string)
        .collect()
}

fn route_path(relative_path: &str) -> String {
    let route = relative_path
        .trim_start_matches("src/")
        .trim_start_matches("app/api/")
        .trim_end_matches("/route.ts")
        .trim_end_matches("/route.tsx")
        .trim_end_matches("/route.js")
        .trim_end_matches("/route.jsx");
    format!("/api/{route}")
}

fn has_auth_check(contents: &str) -> bool {
    [
        "auth(",
        "getServerSession",
        "requireAuth",
        "currentUser",
        "organization_memberships",
        "verifyToken",
        "withAuth",
    ]
    .iter()
    .any(|needle| contents.contains(needle))
}

fn has_db_access(file: &CodeFile) -> bool {
    !db_detectors(&file.contents).is_empty()
        || file.relative_path.contains("/database/")
        || file.relative_path.contains("/db/")
}

fn db_detectors(contents: &str) -> Vec<String> {
    [
        ("prisma", "prisma."),
        ("sql_tag", "sql`"),
        ("query_call", ".query("),
        ("execute_call", ".execute("),
        ("supabase", "supabase."),
        ("database_client", "database"),
        ("db_client", "db."),
    ]
    .into_iter()
    .filter(|(_, needle)| contents.contains(needle))
    .map(|(name, _)| name.to_string())
    .collect()
}

fn env_var_names(contents: &str) -> Vec<String> {
    let mut names = BTreeSet::new();
    for segment in contents.split("process.env.").skip(1) {
        let name = segment
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect::<String>();
        if !name.is_empty() {
            names.insert(name);
        }
    }
    names.into_iter().collect()
}

fn language_for_path(relative_path: &str) -> &str {
    match Path::new(relative_path)
        .extension()
        .and_then(|value| value.to_str())
    {
        Some("ts") | Some("tsx") => "typescript",
        Some("js" | "jsx") => "javascript",
        Some("json") => "json",
        _ => "unknown",
    }
}

fn coverage_json(coverage: &Coverage) -> Value {
    json!({
        "parsed_files": coverage.parsed_files,
        "skipped_files": coverage.skipped_files,
        "unsupported_extensions": coverage.unsupported_extensions,
        "api_route_files": coverage.api_route_files,
        "test_files": coverage.test_files,
        "db_access_files": coverage.db_access_files,
        "env_usage_files": coverage.env_usage_files,
        "confidence_model": {
            "file_detection": "high",
            "api_route_detection": "high_for_nextjs_app_router_paths",
            "db_access_detection": "medium_lexical",
            "auth_detection": "medium_lexical",
            "env_usage_detection": "high_for_process_env_dot_access"
        }
    })
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

struct OwnedRecordSpec {
    id: String,
    record_type: String,
    title: String,
    summary: String,
    source_ids: Vec<String>,
    context_hints: Vec<String>,
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
        "provenance": provenance(),
        "metadata": spec.metadata
    })
}

fn record_owned(spec: OwnedRecordSpec) -> Value {
    json!({
        "id": spec.id,
        "record_type": spec.record_type,
        "title": spec.title,
        "summary": spec.summary,
        "source_ids": spec.source_ids,
        "context_hints": spec.context_hints,
        "relation": spec.relation,
        "provenance": provenance(),
        "metadata": spec.metadata
    })
}

fn provenance() -> Value {
    json!({
        "origin": "source_backed",
        "actor": "source-adapter:code-repo-snapshot",
        "confidence": 1.0,
        "review_status": "accepted"
    })
}

fn slug(value: &str) -> String {
    let mut slug = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    slug.trim_matches('-').to_string()
}

fn path_slug(value: &str) -> String {
    format!("{}-{:08x}", slug(value), stable_hash(value))
}

fn stable_hash(value: &str) -> u32 {
    let mut hash = 0x811c9dc5_u32;
    for byte in value.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}
