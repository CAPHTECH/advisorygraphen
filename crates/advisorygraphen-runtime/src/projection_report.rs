use advisorygraphen_core::{AdvisoryError, AdvisoryResult};
use serde_json::{json, Value};
use std::{fs, path::Path};

pub fn read_projection_report(
    report: &Path,
    completions_report: Option<&Path>,
) -> AdvisoryResult<Value> {
    let mut report: Value = serde_json::from_slice(&fs::read(report)?)?;
    let Some(completions_report) = completions_report else {
        return Ok(report);
    };
    let completions: Value = serde_json::from_slice(&fs::read(completions_report)?)?;
    let report_object = report.as_object_mut().ok_or_else(|| {
        AdvisoryError::Validation("projection report must be a JSON object".to_string())
    })?;
    report_object
        .entry("related_reports")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| AdvisoryError::Validation("related_reports must be an object".to_string()))?
        .insert("completions".to_string(), completions);
    Ok(report)
}
