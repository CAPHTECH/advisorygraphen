use crate::{read_json, ReviewOptions};
use advisorygraphen_core::{AdvisoryError, AdvisoryResult};
use higher_graphen_core::Id as HigherId;
use higher_graphen_reasoning::completion::{
    review_completion, CompletionCandidate, CompletionReviewRequest,
};
use serde_json::Value;
use std::path::Path;

pub fn higher_graphen_completion_review(
    options: &ReviewOptions,
    from_report: &Path,
    reviewed_at: &str,
) -> AdvisoryResult<Value> {
    let report = read_json(from_report)?;
    let candidate = find_higher_graphen_candidate(&report, &options.candidate_id)?;
    let reviewer_id = HigherId::new(&options.reviewer).map_err(hg_runtime_err)?;
    let candidate_id = HigherId::new(&options.candidate_id).map_err(hg_runtime_err)?;
    let request = match options.outcome.as_str() {
        "accepted" => {
            CompletionReviewRequest::accepted(candidate_id, reviewer_id, options.reason.clone())
        }
        "rejected" => {
            CompletionReviewRequest::rejected(candidate_id, reviewer_id, options.reason.clone())
        }
        other => {
            return Err(AdvisoryError::Validation(format!(
                "unsupported completion review outcome: {other}"
            )))
        }
    }
    .and_then(|request| request.with_reviewed_at(reviewed_at))
    .map_err(hg_runtime_err)?;
    let record = review_completion(&candidate, request).map_err(hg_runtime_err)?;
    Ok(serde_json::to_value(record)?)
}

fn find_higher_graphen_candidate(
    report: &Value,
    candidate_id: &str,
) -> AdvisoryResult<CompletionCandidate> {
    let candidates = report
        .pointer("/result/completion_candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AdvisoryError::Validation(
                "from-report must contain result.completion_candidates".to_string(),
            )
        })?;
    let mut matches = candidates
        .iter()
        .filter(|candidate| candidate.get("id").and_then(Value::as_str) == Some(candidate_id));
    let candidate = matches.next().ok_or_else(|| {
        AdvisoryError::Validation(format!("candidate {candidate_id} not found in from-report"))
    })?;
    if matches.next().is_some() {
        return Err(AdvisoryError::Validation(format!(
            "candidate {candidate_id} appears more than once in from-report"
        )));
    }
    let higher_candidate = candidate.get("higher_graphen").cloned().ok_or_else(|| {
        AdvisoryError::Validation(format!(
            "candidate {candidate_id} has no higher_graphen snapshot"
        ))
    })?;
    Ok(serde_json::from_value(higher_candidate)?)
}

fn hg_runtime_err(error: higher_graphen_core::CoreError) -> AdvisoryError {
    AdvisoryError::Validation(format!("higher-graphen runtime: {error}"))
}
