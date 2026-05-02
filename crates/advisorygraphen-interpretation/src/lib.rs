use advisorygraphen_core::{canonical_package_id, AdvisoryResult, PACKAGE_TECHNICAL_ADVISORY_MVP};
use serde_json::{json, Value};

pub const TECHNICAL_ADVISORY_RULESET: &str = "technical_advisory_mvp";

#[derive(Debug, Clone)]
pub struct InterpretationPackage {
    pub package_id: String,
    pub ruleset: &'static str,
    pub invariant_ids: Vec<&'static str>,
}

impl InterpretationPackage {
    pub fn load(package: &str) -> AdvisoryResult<Self> {
        let package_id = canonical_package_id(package)?;
        Ok(Self {
            package_id,
            ruleset: TECHNICAL_ADVISORY_RULESET,
            invariant_ids: vec![
                "invariant:architecture_no_cross_context_direct_database_access",
                "invariant:recommendation_requires_evidence",
                "invariant:action_requires_owner",
                "invariant:requirement_requires_verification",
                "invariant:projection_loss_declared",
            ],
        })
    }

    pub fn invariant_records(&self) -> Vec<Value> {
        self.invariant_ids
            .iter()
            .map(|id| json!({ "id": id, "ruleset": self.ruleset }))
            .collect()
    }

    pub fn policy_records(&self) -> Vec<Value> {
        vec![json!({
            "id": "policy:technical-advisory-mvp-defaults",
            "package_id": self.package_id,
            "projection_loss_required": true
        })]
    }
}

pub fn load_ruleset(ruleset: &str) -> AdvisoryResult<InterpretationPackage> {
    match ruleset {
        TECHNICAL_ADVISORY_RULESET => InterpretationPackage::load(PACKAGE_TECHNICAL_ADVISORY_MVP),
        other => Err(advisorygraphen_core::AdvisoryError::UnsupportedRuleset(
            other.to_string(),
        )),
    }
}
