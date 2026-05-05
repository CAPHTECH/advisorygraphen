use advisorygraphen_core::{
    AdvisoryError, AdvisoryResult, AdvisorySpaceEnvelope, HigherGraphenAdvisorySpace,
};
use higher_graphen_structure::space::traversal::CycleSearchOptions;
use higher_graphen_structure::topology::SimpleCycleIndicator;
use serde_json::{json, Value};

use crate::higher::{violation_finding, FindingInput};

pub const CYCLE_INVARIANT: &str = "invariant:architecture_no_circular_dependencies";

/// Incidence relation_type values that participate in the directed dependency
/// graph used for cycle detection. `owns` and `verifies` are excluded because
/// they are not runtime/structural dependencies.
const DEPENDENCY_RELATIONS: &[&str] = &["accesses", "depends_on", "uses", "implements"];

pub fn evaluate_dependency_cycles(
    space: &AdvisorySpaceEnvelope,
    higher_space: &HigherGraphenAdvisorySpace,
    invariant_results: &mut Vec<Value>,
    obstructions: &mut Vec<Value>,
) -> AdvisoryResult<()> {
    let mut options = CycleSearchOptions::new();
    for relation in DEPENDENCY_RELATIONS {
        options = options.with_relation_type(*relation);
    }
    let cycles = higher_space
        .store
        .find_simple_cycles(&higher_space.space_id, &options)
        .map_err(|error| {
            AdvisoryError::Validation(format!("higher-graphen cycle search: {error}"))
        })?;
    for (index, cycle) in cycles.iter().enumerate() {
        invariant_results.push({
            let finding = build_finding(space, cycle, index)?;
            obstructions.push(finding.obstruction);
            finding.invariant_result
        });
    }
    Ok(())
}

struct CycleFinding {
    invariant_result: Value,
    obstruction: Value,
}

fn build_finding(
    space: &AdvisorySpaceEnvelope,
    cycle: &SimpleCycleIndicator,
    index: usize,
) -> AdvisoryResult<CycleFinding> {
    let cell_ids: Vec<String> = cycle
        .vertex_cell_ids
        .iter()
        .map(|id| id.as_str().to_string())
        .collect();
    let edge_ids: Vec<String> = cycle
        .edge_cell_ids
        .iter()
        .map(|id| id.as_str().to_string())
        .collect();
    let path_titles: Vec<String> = cell_ids
        .iter()
        .map(|cell_id| cell_title(space, cell_id))
        .collect();
    let obstruction_id = format!(
        "obstruction:{}-circular-dependency-{}",
        space
            .space_id
            .trim_start_matches("space:advisory:")
            .trim_start_matches("space:"),
        index + 1
    );
    let message = format!(
        "Circular dependency detected across {} components: {} -> {}",
        cell_ids.len(),
        path_titles.join(" -> "),
        path_titles.first().cloned().unwrap_or_default()
    );
    let mut witness_ids = cell_ids.clone();
    witness_ids.extend(edge_ids.clone());
    let blocked_ids: Vec<Value> = cell_ids
        .iter()
        .map(|id: &String| Value::String(id.clone()))
        .collect();
    let finding = violation_finding(FindingInput {
        space_id: &space.space_id,
        invariant_id: CYCLE_INVARIANT,
        obstruction_id: &obstruction_id,
        obstruction_type: "circular_dependency",
        severity: "medium",
        message,
        witness_ids,
        blocked_ids,
        evidence_ids: edge_ids
            .iter()
            .map(|id: &String| Value::String(id.clone()))
            .collect(),
        recommended_completion_types: vec!["proposed_dependency_break", "architecture_review"],
        resolution:
            "break the cycle by introducing an interface, inversion, or asynchronous boundary",
        metadata: json!({
            "rule_precision": "directed_dependency_cycle_in_typed_graph_complex",
            "evidence_strength": "topology_derived_dfs",
            "specificity": "topology_derived",
            "precision_note": "Cycle witnesses come from higher-graphen find_simple_cycles over depends_on / accesses / uses / implements incidences. Soft links such as owns or verifies are excluded.",
            "cycle_cell_ids": cell_ids,
            "cycle_edge_ids": edge_ids,
            "witness_edge_id": cycle.witness_edge_id.as_str()
        }),
    })?;
    Ok(CycleFinding {
        invariant_result: finding.invariant_result,
        obstruction: finding.obstruction,
    })
}

fn cell_title(space: &AdvisorySpaceEnvelope, cell_id: &str) -> String {
    space
        .cells
        .iter()
        .find(|cell| cell.get("id").and_then(Value::as_str) == Some(cell_id))
        .and_then(|cell| cell.get("title").and_then(Value::as_str))
        .unwrap_or(cell_id)
        .to_string()
}
