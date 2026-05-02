use crate::AdvisorySpaceEnvelope;
use crate::{json_id, optional_string_array, slugify_id, AdvisoryError, AdvisoryResult};
use higher_graphen_core::{Confidence, Id, Provenance, ReviewStatus, SourceKind, SourceRef};
use higher_graphen_structure::context::Context;
use higher_graphen_structure::space::{
    Cell, InMemorySpaceStore, Incidence, IncidenceOrientation, Space,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::str::FromStr;

/// AdvisoryGraphen materialized as repository-owned HigherGraphen primitives.
#[derive(Clone, Debug)]
pub struct HigherGraphenAdvisorySpace {
    pub space_id: Id,
    pub store: InMemorySpaceStore,
    pub contexts: Vec<Context>,
}

impl HigherGraphenAdvisorySpace {
    pub fn space(&self) -> Option<&Space> {
        self.store.space(&self.space_id)
    }

    pub fn cell(&self, id: &str) -> Option<&Cell> {
        Id::new(id)
            .ok()
            .and_then(|cell_id| self.store.cell(&cell_id))
    }

    pub fn incidence(&self, id: &str) -> Option<&Incidence> {
        Id::new(id)
            .ok()
            .and_then(|incidence_id| self.store.incidence(&incidence_id))
    }

    pub fn incidence_records(&self) -> Vec<&Incidence> {
        self.space()
            .into_iter()
            .flat_map(|space| &space.incidence_ids)
            .filter_map(|incidence_id| self.store.incidence(incidence_id))
            .collect()
    }

    pub fn summary_json(&self) -> Value {
        let Some(space) = self.space() else {
            return json!({
                "engine": "higher-graphen",
                "space_id": self.space_id.as_str(),
                "materialized": false
            });
        };
        json!({
            "engine": "higher-graphen",
            "space_id": self.space_id.as_str(),
            "materialized": true,
            "cell_count": space.cell_ids.len(),
            "incidence_count": space.incidence_ids.len(),
            "context_count": self.contexts.len()
        })
    }
}

impl AdvisorySpaceEnvelope {
    pub fn to_higher_graphen(&self) -> AdvisoryResult<HigherGraphenAdvisorySpace> {
        let space_id = hg_id(&self.space_id)?;
        let context_ids = self
            .contexts
            .iter()
            .map(|context| hg_id(json_id(context)))
            .collect::<AdvisoryResult<Vec<_>>>()?;
        let mut store = InMemorySpaceStore::new();
        let mut space = Space::new(
            space_id.clone(),
            format!("AdvisoryGraphen {}", self.engagement_id),
        )
        .with_description("AdvisoryGraphen envelope materialized into HigherGraphen structure");
        space.context_ids = context_ids.clone();
        hg(store.insert_space(space))?;

        let mut inserted_cell_ids = BTreeSet::new();
        for cell in &self.cells {
            let inserted = insert_cell(&mut store, &space_id, cell)?;
            inserted_cell_ids.insert(inserted.id);
        }

        for context in &self.contexts {
            let context_id = hg_id(json_id(context))?;
            if inserted_cell_ids.contains(&context_id) {
                continue;
            }
            let inserted = insert_context_cell(&mut store, &space_id, context)?;
            inserted_cell_ids.insert(inserted.id);
        }

        for incidence in &self.incidences {
            insert_incidence(&mut store, &space_id, incidence)?;
        }

        let contexts = self
            .contexts
            .iter()
            .map(|context| build_context(context, &self.cells))
            .collect::<AdvisoryResult<Vec<_>>>()?;

        Ok(HigherGraphenAdvisorySpace {
            space_id,
            store,
            contexts,
        })
    }
}

fn insert_cell(
    store: &mut InMemorySpaceStore,
    space_id: &Id,
    cell: &Value,
) -> AdvisoryResult<Cell> {
    let mut higher_cell = Cell::new(
        hg_id(json_id(cell))?,
        space_id.clone(),
        dimension_for_cell(cell),
        required_string(cell, "cell_type")?,
    );
    if let Some(label) = cell.get("title").and_then(Value::as_str) {
        higher_cell = higher_cell.with_label(label);
    }
    for context_id in optional_string_array(cell, "context_ids") {
        higher_cell = higher_cell.with_context(hg_id(&context_id)?);
    }
    if let Some(provenance) = cell.get("provenance") {
        higher_cell = higher_cell.with_provenance(build_provenance(provenance)?);
    }
    hg(store.insert_cell(higher_cell))
}

fn insert_context_cell(
    store: &mut InMemorySpaceStore,
    space_id: &Id,
    context: &Value,
) -> AdvisoryResult<Cell> {
    let mut cell = Cell::new(hg_id(json_id(context))?, space_id.clone(), 0, "context");
    if let Some(label) = context.get("title").and_then(Value::as_str) {
        cell = cell.with_label(label);
    }
    if let Some(provenance) = context.get("provenance") {
        cell = cell.with_provenance(build_provenance(provenance)?);
    }
    hg(store.insert_cell(cell))
}

fn insert_incidence(
    store: &mut InMemorySpaceStore,
    space_id: &Id,
    incidence: &Value,
) -> AdvisoryResult<Incidence> {
    let mut higher_incidence = Incidence::new(
        hg_id(json_id(incidence))?,
        space_id.clone(),
        hg_id(required_string(incidence, "from_id")?)?,
        hg_id(required_string(incidence, "to_id")?)?,
        required_string(incidence, "relation_type")?,
        IncidenceOrientation::Directed,
    );
    if let Some(provenance) = incidence.get("provenance") {
        higher_incidence = higher_incidence.with_provenance(build_provenance(provenance)?);
    }
    hg(store.insert_incidence(higher_incidence))
}

fn build_context(context: &Value, cells: &[Value]) -> AdvisoryResult<Context> {
    let context_id = hg_id(json_id(context))?;
    let mut higher_context = Context::new(
        context_id.clone(),
        context
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_else(|| context_id.as_str()),
    )
    .map_err(higher_error)?;
    if let Some(summary) = context.get("summary").and_then(Value::as_str) {
        higher_context = higher_context.with_description(summary);
    }
    if let Some(provenance) = context.get("provenance") {
        higher_context = higher_context.with_provenance(build_provenance(provenance)?);
    }
    for cell in cells {
        if optional_string_array(cell, "context_ids")
            .iter()
            .any(|id| id == context_id.as_str())
        {
            higher_context = higher_context.with_element(hg_id(json_id(cell))?);
        }
    }
    Ok(higher_context)
}

fn build_provenance(value: &Value) -> AdvisoryResult<Provenance> {
    let origin = value
        .get("origin")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let source_kind = match origin {
        "source_backed" => SourceKind::Document,
        "inferred" => SourceKind::Ai,
        "review_promoted" => SourceKind::Human,
        _ => {
            SourceKind::custom(format!("advisory-{}", slugify_id(origin))).map_err(higher_error)?
        }
    };
    let mut source = SourceRef::new(source_kind);
    if let Some(actor) = value.get("actor").and_then(Value::as_str) {
        source = source.with_title(actor).map_err(higher_error)?;
    }
    let confidence = Confidence::new(
        value
            .get("confidence")
            .and_then(Value::as_f64)
            .unwrap_or(1.0),
    )
    .map_err(higher_error)?;
    let review_status = value
        .get("review_status")
        .and_then(Value::as_str)
        .map(ReviewStatus::from_str)
        .transpose()
        .map_err(higher_error)?
        .unwrap_or_default();
    Ok(Provenance::new(source, confidence).with_review_status(review_status))
}

fn dimension_for_cell(cell: &Value) -> u32 {
    match cell.get("cell_type").and_then(Value::as_str) {
        Some("evidence" | "owner" | "metric" | "data_store") => 0,
        Some("component" | "claim" | "requirement" | "test_or_verification") => 1,
        Some("action" | "decision") => 2,
        _ => 1,
    }
}

fn required_string<'a>(value: &'a Value, field: &str) -> AdvisoryResult<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| AdvisoryError::Validation(format!("missing non-string field `{field}`")))
}

fn hg_id(value: &str) -> AdvisoryResult<Id> {
    Id::new(value).map_err(higher_error)
}

fn hg<T>(result: higher_graphen_core::Result<T>) -> AdvisoryResult<T> {
    result.map_err(higher_error)
}

fn higher_error(error: higher_graphen_core::CoreError) -> AdvisoryError {
    AdvisoryError::Validation(format!("higher-graphen bridge: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn materializes_advisory_space_into_higher_graphen_store() {
        let space = AdvisorySpaceEnvelope {
            schema: crate::SPACE_SCHEMA.to_string(),
            space_id: "space:test".to_string(),
            engagement_id: "engagement:test".to_string(),
            snapshot_id: "snapshot:test".to_string(),
            package_id: crate::PACKAGE_TECHNICAL_ADVISORY_MVP.to_string(),
            cells: vec![
                cell("cell:order-service", "component", ["context:orders"]),
                cell("cell:billing-db", "data_store", ["context:billing"]),
            ],
            contexts: vec![
                context("context:orders", "Orders"),
                context("context:billing", "Billing"),
            ],
            incidences: vec![json!({
                "id": "incidence:direct-read",
                "relation_type": "accesses",
                "from_id": "cell:order-service",
                "to_id": "cell:billing-db",
                "context_ids": ["context:orders", "context:billing"],
                "evidence_ids": [],
                "strength": "hard",
                "provenance": provenance(),
                "metadata": { "access_type": "direct_database_read" }
            })],
            morphisms: vec![],
            invariants: vec![],
            policies: vec![],
            metadata: crate::JsonMap::new(),
        };

        let higher = space.to_higher_graphen().expect("higher graphen space");

        assert_eq!(higher.space().expect("space").cell_ids.len(), 4);
        assert_eq!(higher.incidence_records().len(), 1);
        assert_eq!(higher.contexts.len(), 2);
        assert_eq!(
            higher
                .cell("cell:order-service")
                .expect("order service")
                .context_ids,
            vec![Id::new("context:orders").expect("id")]
        );
    }

    fn cell<const N: usize>(id: &str, cell_type: &str, context_ids: [&str; N]) -> Value {
        let context_ids = context_ids.to_vec();
        json!({
            "id": id,
            "cell_type": cell_type,
            "title": id,
            "summary": null,
            "context_ids": context_ids,
            "source_ids": [],
            "structure_refs": [],
            "provenance": provenance(),
            "metadata": {}
        })
    }

    fn context(id: &str, title: &str) -> Value {
        json!({
            "id": id,
            "context_type": "technical_boundary",
            "title": title,
            "summary": null,
            "provenance": provenance(),
            "metadata": {}
        })
    }

    fn provenance() -> Value {
        json!({
            "origin": "source_backed",
            "actor": "tester",
            "confidence": 1.0,
            "review_status": "accepted"
        })
    }
}
