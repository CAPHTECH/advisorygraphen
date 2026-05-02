use advisorygraphen_core::{
    canonical_package_id, AdvisoryError, AdvisoryResult, PACKAGE_TECHNICAL_ADVISORY_MVP,
};
use higher_graphen_core::Id;
use higher_graphen_interpretation::{
    InterpretationPackage as HigherInterpretationPackage, InterpretationTargetKind,
    InvariantTemplate, LiftAdapterDefinition, ProjectionTemplate, TypeMapping,
};
use serde_json::{json, Value};

pub const TECHNICAL_ADVISORY_RULESET: &str = "technical_advisory_mvp";

#[derive(Debug, Clone)]
pub struct InterpretationPackage {
    pub package_id: String,
    pub ruleset: &'static str,
    pub invariant_ids: Vec<&'static str>,
    pub higher_graphen: HigherInterpretationPackage,
}

impl InterpretationPackage {
    pub fn load(package: &str) -> AdvisoryResult<Self> {
        let package_id = canonical_package_id(package)?;
        Ok(Self {
            higher_graphen: build_higher_graphen_package(&package_id)?,
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

    pub fn higher_graphen_package_value(&self) -> AdvisoryResult<Value> {
        serde_json::to_value(&self.higher_graphen).map_err(AdvisoryError::from)
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

fn build_higher_graphen_package(package_id: &str) -> AdvisoryResult<HigherInterpretationPackage> {
    let mut package = HigherInterpretationPackage::new(
        id(package_id)?,
        "Technical Advisory MVP",
        env!("CARGO_PKG_VERSION"),
    )
    .map_err(hg_err)?
    .with_description("AdvisoryGraphen interpretation package backed by HigherGraphen templates")
    .map_err(hg_err)?;

    for cell_type in [
        "component",
        "data_store",
        "requirement",
        "test_or_verification",
        "owner",
        "metric",
        "action",
        "claim",
        "evidence",
    ] {
        package
            .register_type_mapping(
                TypeMapping::new(
                    id(&format!("type-mapping:{cell_type}"))?,
                    cell_type,
                    InterpretationTargetKind::Cell,
                    cell_type,
                )
                .map_err(hg_err)?,
            )
            .map_err(hg_err)?;
    }

    for invariant_id in [
        "architecture_no_cross_context_direct_database_access",
        "recommendation_requires_evidence",
        "action_requires_owner",
        "requirement_requires_verification",
        "projection_loss_declared",
    ] {
        package
            .register_invariant_template(
                InvariantTemplate::new(
                    id(&format!("invariant-template:{invariant_id}"))?,
                    invariant_id.replace('_', " "),
                    format!("technical advisory invariant `{invariant_id}` must hold"),
                )
                .map_err(hg_err)?,
            )
            .map_err(hg_err)?;
    }

    for audience in ["executive", "developer_action", "audit_trace", "ai_agent"] {
        package
            .register_projection_template(
                ProjectionTemplate::new(
                    id(&format!("projection-template:{audience}"))?,
                    format!("{audience} projection"),
                    audience,
                    "report",
                    "advisorygraphen.projection.v1",
                )
                .map_err(hg_err)?,
            )
            .map_err(hg_err)?;
    }

    package
        .register_lift_adapter(
            LiftAdapterDefinition::new(
                id("lift-adapter:json-snapshot")?,
                "JSON snapshot lift adapter",
                advisorygraphen_core::SNAPSHOT_SCHEMA,
            )
            .map_err(hg_err)?
            .with_output_kind(advisorygraphen_core::SPACE_SCHEMA)
            .map_err(hg_err)?,
        )
        .map_err(hg_err)?;

    Ok(package)
}

fn id(value: &str) -> AdvisoryResult<Id> {
    Id::new(value).map_err(hg_err)
}

fn hg_err(error: higher_graphen_core::CoreError) -> AdvisoryError {
    AdvisoryError::Validation(format!("higher-graphen interpretation: {error}"))
}
