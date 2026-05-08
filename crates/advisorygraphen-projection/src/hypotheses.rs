use serde_json::{json, Value};

pub fn hypotheses(report: &Value) -> Vec<Value> {
    array_at(report, "/result/hypotheses")
}

pub fn falsifiers(report: &Value) -> Vec<Value> {
    array_at(report, "/result/falsifiers")
}

pub fn argumentation_incidences(report: &Value) -> Vec<Value> {
    array_at(report, "/result/argumentation_incidences")
}

pub fn hypothesis_summary(hypotheses: &[Value]) -> Value {
    let mut candidate = 0_u64;
    let mut supported = 0_u64;
    let mut strongly_supported = 0_u64;
    let mut supported_needs_followup = 0_u64;
    let mut plausible_secondary = 0_u64;
    let mut accepted = 0_u64;
    let mut rejected = 0_u64;
    let mut falsified = 0_u64;
    let mut other = 0_u64;
    for hypothesis in hypotheses {
        match hypothesis
            .get("lifecycle_status")
            .and_then(Value::as_str)
            .or_else(|| hypothesis.get("status").and_then(Value::as_str))
            .unwrap_or("")
        {
            "candidate" => candidate += 1,
            "supported" => supported += 1,
            "strongly_supported" => strongly_supported += 1,
            "supported_needs_followup" => supported_needs_followup += 1,
            "plausible_secondary" => plausible_secondary += 1,
            "accepted" => accepted += 1,
            "rejected" => rejected += 1,
            "falsified" => falsified += 1,
            _ => other += 1,
        }
    }
    json!({
        "total": hypotheses.len(),
        "candidate": candidate,
        "supported": supported,
        "strongly_supported": strongly_supported,
        "supported_needs_followup": supported_needs_followup,
        "plausible_secondary": plausible_secondary,
        "accepted": accepted,
        "rejected": rejected,
        "falsified": falsified,
        "other": other
    })
}

fn array_at(report: &Value, pointer: &str) -> Vec<Value> {
    report
        .pointer(pointer)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}
