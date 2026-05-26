use serde_json::{json, Value};

const ASSUMPTION_MARKERS: &[&str] = &[
    "assume",
    "assuming",
    "probably",
    "likely",
    "maybe",
    "seems",
    "推測",
    "仮定",
    "おそらく",
    "たぶん",
    "ように思",
    "と思う",
    "はず",
];

const EVIDENCE_MARKERS: &[&str] = &[
    "test",
    "tests",
    "passed",
    "failing",
    "log",
    "trace",
    "observed",
    "verified",
    "source:",
    "file:",
    ".rs:",
    ".ts:",
    ".tsx:",
    ".py:",
    ".ex:",
    "根拠",
    "確認",
    "観測",
    "検証",
    "ログ",
    "テスト",
];

const STRONG_CLAIM_MARKERS: &[&str] = &[
    "always",
    "never",
    "must",
    "safe",
    "secure",
    "done",
    "fixed",
    "works",
    "no issue",
    "問題ない",
    "安全",
    "完了",
    "修正済",
    "必ず",
    "絶対",
    "確実",
];

const RISK_MARKERS: &[&str] = &[
    "auth",
    "security",
    "permission",
    "delete",
    "migration",
    "database",
    "payment",
    "billing",
    "認証",
    "認可",
    "権限",
    "削除",
    "移行",
    "DB",
    "データベース",
    "支払い",
    "請求",
];

const CAUSE_MARKERS: &[&str] = &[
    "because",
    "caused by",
    "root cause",
    "原因",
    "ため",
    "なので",
    "によって",
];

pub fn analyze(input_text: &str) -> Value {
    let normalized_lines = input_text
        .lines()
        .map(clean_line)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let units = split_claim_units(&normalized_lines);
    let mut claims = Vec::new();
    let mut assumptions = Vec::new();
    let mut missing_checks = Vec::new();
    let mut alternative_hypotheses = Vec::new();
    let mut structure_error_risks = Vec::new();
    let mut high_risk_claim_count = 0usize;
    let mut unsupported_claim_count = 0usize;
    let mut high_structure_error_risk_count = 0usize;

    for (index, unit) in units.iter().enumerate() {
        let markers = matching_markers(unit, EVIDENCE_MARKERS);
        let assumption_markers = matching_markers(unit, ASSUMPTION_MARKERS);
        let strong_markers = matching_markers(unit, STRONG_CLAIM_MARKERS);
        let risk_markers = matching_markers(unit, RISK_MARKERS);
        let cause_markers = matching_markers(unit, CAUSE_MARKERS);
        let evidence_status = if !markers.is_empty()
            && markers
                .iter()
                .any(|marker| marker.contains("test") || marker.contains("テスト"))
        {
            "test_backed"
        } else if !markers.is_empty() {
            "source_backed"
        } else if !assumption_markers.is_empty() {
            "assumption_marked"
        } else if !strong_markers.is_empty() {
            "unsupported_strong_claim"
        } else {
            "unsupported"
        };
        if evidence_status == "unsupported" || evidence_status == "unsupported_strong_claim" {
            unsupported_claim_count += 1;
        }
        if !risk_markers.is_empty() {
            high_risk_claim_count += 1;
        }
        let claim_id = format!("claim:{:03}", index + 1);
        let risk_flags = risk_markers
            .iter()
            .map(|marker| {
                json!({
                    "marker": marker,
                    "risk": "small_scope_high_blast_radius"
                })
            })
            .collect::<Vec<_>>();
        claims.push(json!({
            "id": claim_id,
            "text": unit,
            "evidence_status": evidence_status,
            "evidence_markers": markers,
            "assumption_markers": assumption_markers,
            "strong_claim_markers": strong_markers,
            "risk_flags": risk_flags
        }));

        let structure_error_risk = structure_error_risk(
            &claim_id,
            unit,
            evidence_status,
            &markers,
            &assumption_markers,
            &strong_markers,
            &risk_markers,
            &cause_markers,
        );
        if structure_error_risk
            .get("error_risk")
            .and_then(Value::as_str)
            == Some("high")
        {
            high_structure_error_risk_count += 1;
        }
        structure_error_risks.push(structure_error_risk);

        if !assumption_markers.is_empty() {
            assumptions.push(json!({
                "claim_id": format!("claim:{:03}", index + 1),
                "text": unit,
                "markers": assumption_markers,
                "status": "requires_confirmation_or_downgrade"
            }));
        }

        if evidence_status == "unsupported_strong_claim" || !risk_markers.is_empty() {
            missing_checks.push(json!({
                "id": format!("check:{:03}", missing_checks.len() + 1),
                "claim_id": format!("claim:{:03}", index + 1),
                "reason": if !risk_markers.is_empty() {
                    "high-blast-radius claim lacks explicit bounded evidence"
                } else {
                    "strong completion or safety claim lacks explicit bounded evidence"
                },
                "suggested_observation": suggested_observation(unit, !risk_markers.is_empty())
            }));
        }

        if !cause_markers.is_empty() {
            alternative_hypotheses.push(json!({
                "id": format!("hypothesis:alternative-{:03}", alternative_hypotheses.len() + 1),
                "source_claim_id": format!("claim:{:03}", index + 1),
                "current_claim": unit,
                "alternative": "The observed outcome may be explained by configuration, dependency state, input data, or execution timing rather than the stated cause.",
                "falsifier": "A direct observation that isolates the stated cause from configuration, dependency, input, and timing variables."
            }));
        }
    }

    let line_count = normalized_lines.len();
    let word_count = count_words(input_text);
    let escalation_reasons = escalation_reasons(
        claims.len(),
        word_count,
        high_risk_claim_count,
        unsupported_claim_count,
    );
    let recommended_mode = if escalation_reasons.is_empty() {
        "micro_review"
    } else {
        "full_advisory_workflow_recommended"
    };
    let recommended_next_observation = missing_checks
        .first()
        .cloned()
        .unwrap_or_else(|| {
            json!({
                "id": "check:000",
                "reason": "no high-priority missing check detected",
                "suggested_observation": "Keep the current answer as a lightweight note; no full AdvisoryGraphen workflow is indicated."
            })
        });

    json!({
        "mode": {
            "recommended": recommended_mode,
            "escalate_to_full_workflow": !escalation_reasons.is_empty(),
            "escalation_reasons": escalation_reasons
        },
        "scale_signals": {
            "line_count": line_count,
            "word_count": word_count,
            "claim_count": claims.len(),
            "unsupported_claim_count": unsupported_claim_count,
            "high_risk_claim_count": high_risk_claim_count,
            "high_structure_error_risk_count": high_structure_error_risk_count
        },
        "small_scope_value": {
            "role": "ai_answer_self_review",
            "value_proposition": "Flag assumptions, unsupported strong claims, structure error risk, falsification checks, missing checks, and alternative hypotheses without running lift/check/completions/project."
        },
        "claims": claims,
        "structure_error_risk_summary": structure_error_risk_summary(&structure_error_risks),
        "structure_error_risks": structure_error_risks,
        "assumptions": assumptions,
        "missing_checks": missing_checks,
        "alternative_hypotheses": alternative_hypotheses,
        "recommended_next_observation": recommended_next_observation
    })
}

fn structure_error_risk(
    claim_id: &str,
    text: &str,
    evidence_status: &str,
    evidence_markers: &[String],
    assumption_markers: &[String],
    strong_markers: &[String],
    risk_markers: &[String],
    cause_markers: &[String],
) -> Value {
    let mut risk_score = 0.05_f64;
    let mut risk_factors = Vec::new();
    let mut falsification_checks = Vec::new();

    if evidence_markers.is_empty() {
        add_factor(
            &mut risk_factors,
            &mut falsification_checks,
            "missing_source_witness",
            0.25,
            "claim has no explicit source, file, log, test, or observation marker",
            "Find a bounded source witness, file path, command output, log, or reviewed note that directly supports this structure.",
            &mut risk_score,
        );
    }

    if evidence_status == "unsupported_strong_claim"
        || (!strong_markers.is_empty() && evidence_markers.is_empty())
    {
        add_factor(
            &mut risk_factors,
            &mut falsification_checks,
            "unsupported_strong_claim",
            0.25,
            "completion, safety, or certainty wording appears without direct evidence",
            "Try to falsify the completion/safety claim with a focused negative test or counterexample.",
            &mut risk_score,
        );
    }

    if !assumption_markers.is_empty() {
        add_factor(
            &mut risk_factors,
            &mut falsification_checks,
            "assumption_promoted_to_structure",
            0.15,
            "likely/probably/assumed language may have been promoted into structure",
            "Downgrade the structure to a hypothesis unless an observation confirms the assumed relation.",
            &mut risk_score,
        );
    }

    if !risk_markers.is_empty() {
        add_factor(
            &mut risk_factors,
            &mut falsification_checks,
            "high_blast_radius_structure",
            0.20,
            "security, authorization, database, deletion, migration, payment, or billing marker detected",
            &suggested_observation(text, true),
            &mut risk_score,
        );
    }

    if !cause_markers.is_empty() && !matches!(evidence_status, "source_backed" | "test_backed") {
        add_factor(
            &mut risk_factors,
            &mut falsification_checks,
            "unsupported_causal_link",
            0.15,
            "causal wording appears without direct evidence isolating the cause",
            "Check whether configuration, dependency state, input data, or execution timing can explain the same outcome.",
            &mut risk_score,
        );
    }

    if has_ambiguous_structure_terms(text) && evidence_markers.is_empty() {
        add_factor(
            &mut risk_factors,
            &mut falsification_checks,
            "ambiguous_term_mapping",
            0.10,
            "structure term appears without a concrete mapping to source material",
            "Map each named owner, service, route, middleware, API, requirement, or component to a concrete source reference.",
            &mut risk_score,
        );
    }

    let risk_score = risk_score.min(0.95);
    let error_risk = if risk_score >= 0.70 {
        "high"
    } else if risk_score >= 0.40 {
        "medium"
    } else {
        "low"
    };
    if falsification_checks.is_empty() {
        falsification_checks.push(
            "No immediate falsification check was generated; keep the structure low-risk unless later evidence contradicts it."
                .to_string(),
        );
    }

    json!({
        "structure_id": claim_id,
        "source_claim_id": claim_id,
        "claim_text": text,
        "error_risk": error_risk,
        "risk_score": round_score(risk_score),
        "calibration_status": "uncalibrated",
        "interpretation": "relative_error_risk_not_probability",
        "risk_factors": risk_factors,
        "falsification_checks": falsification_checks
    })
}

fn add_factor(
    risk_factors: &mut Vec<Value>,
    falsification_checks: &mut Vec<String>,
    factor: &str,
    weight: f64,
    reason: &str,
    falsification_check: &str,
    risk_score: &mut f64,
) {
    *risk_score += weight;
    risk_factors.push(json!({
        "factor": factor,
        "weight": weight,
        "reason": reason
    }));
    falsification_checks.push(falsification_check.to_string());
}

fn structure_error_risk_summary(risks: &[Value]) -> Value {
    let mut low = 0usize;
    let mut medium = 0usize;
    let mut high = 0usize;
    let mut highest_risk_structure_ids = Vec::new();

    for risk in risks {
        match risk.get("error_risk").and_then(Value::as_str) {
            Some("high") => {
                high += 1;
                if let Some(id) = risk.get("structure_id").and_then(Value::as_str) {
                    highest_risk_structure_ids.push(id.to_string());
                }
            }
            Some("medium") => medium += 1,
            _ => low += 1,
        }
    }

    json!({
        "calibration_status": "uncalibrated",
        "interpretation": "risk_score is a relative heuristic for review prioritization, not a calibrated error probability",
        "low": low,
        "medium": medium,
        "high": high,
        "highest_risk_structure_ids": highest_risk_structure_ids
    })
}

fn has_ambiguous_structure_terms(text: &str) -> bool {
    matching_markers(
        text,
        &[
            "owner",
            "service",
            "route",
            "middleware",
            "api",
            "requirement",
            "component",
            "team",
            "責任者",
            "サービス",
            "ルート",
            "ミドルウェア",
            "要求",
            "要件",
            "コンポーネント",
            "チーム",
        ],
    )
    .is_empty()
        == false
}

fn round_score(score: f64) -> f64 {
    (score * 100.0).round() / 100.0
}

fn split_claim_units(lines: &[String]) -> Vec<String> {
    let mut units = Vec::new();
    for line in lines {
        let mut start = 0usize;
        for (index, ch) in line.char_indices() {
            if matches!(ch, '.' | '!' | '?' | '。' | '！' | '？') {
                push_unit(&mut units, &line[start..=index]);
                start = index + ch.len_utf8();
            }
        }
        if start < line.len() {
            push_unit(&mut units, &line[start..]);
        }
    }
    units
}

fn push_unit(units: &mut Vec<String>, raw: &str) {
    let unit = clean_line(raw);
    if unit.len() >= 8 {
        units.push(unit);
    }
}

fn clean_line(line: &str) -> String {
    line.trim()
        .trim_start_matches(|ch: char| {
            ch == '-' || ch == '*' || ch == '•' || ch.is_ascii_digit() || ch == ')' || ch == '.'
        })
        .trim()
        .to_string()
}

fn matching_markers(text: &str, markers: &[&str]) -> Vec<String> {
    let lower = text.to_lowercase();
    markers
        .iter()
        .filter(|marker| lower.contains(&marker.to_lowercase()))
        .map(|marker| (*marker).to_string())
        .collect()
}

fn count_words(text: &str) -> usize {
    let ascii_words = text.split_whitespace().count();
    if ascii_words > 1 {
        ascii_words
    } else {
        text.chars().filter(|ch| !ch.is_whitespace()).count()
    }
}

fn suggested_observation(text: &str, high_risk: bool) -> String {
    let lower = text.to_lowercase();
    if lower.contains("test") || lower.contains("テスト") {
        "Record the exact test command, result, and failure boundary referenced by this claim."
            .to_string()
    } else if lower.contains("auth") || lower.contains("認証") || lower.contains("認可") {
        "Run or cite an auth/permission check that proves the route or action is guarded."
            .to_string()
    } else if lower.contains("database") || lower.contains("db") || lower.contains("データベース")
    {
        "Inspect the direct database access path and cite the file, query, or migration evidence."
            .to_string()
    } else if high_risk {
        "Collect one bounded source-backed observation before treating this claim as accepted."
            .to_string()
    } else {
        "Attach a concrete witness such as a file path, command output, test result, log, or reviewed source note."
            .to_string()
    }
}

fn escalation_reasons(
    claim_count: usize,
    word_count: usize,
    high_risk_claim_count: usize,
    unsupported_claim_count: usize,
) -> Vec<Value> {
    let mut reasons = Vec::new();
    if claim_count > 8 {
        reasons.push(json!({
            "reason": "many_claims",
            "detail": "More than eight claim units were detected; full hypothesis and completion review may be cheaper than ad hoc checking."
        }));
    }
    if word_count > 400 {
        reasons.push(json!({
            "reason": "large_input",
            "detail": "Input exceeds the intended micro-review size."
        }));
    }
    if high_risk_claim_count > 0 {
        reasons.push(json!({
            "reason": "high_blast_radius_claims",
            "detail": "Security, authorization, database, deletion, migration, payment, or billing markers were detected."
        }));
    }
    if unsupported_claim_count > 5 {
        reasons.push(json!({
            "reason": "many_unsupported_claims",
            "detail": "Several claims lack explicit evidence markers."
        }));
    }
    reasons
}
