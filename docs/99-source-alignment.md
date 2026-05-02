# 99. Alignment with HigherGraphen Source Documents

This file records why AdvisoryGraphen is designed this way. It is a traceability document for implementers.

## Source-aligned design decisions

| AdvisoryGraphen decision | Source alignment |
| --- | --- |
| Treat consulting reports as projections, not model | HigherGraphen models reports as projections from richer structure. |
| Keep AI-inferred structure unreviewed | Product integration guide requires inferred structure to remain reviewable. |
| Preserve source boundary | Product integration guide begins with bounded source snapshot. |
| Emit obstructions as domain findings | HigherGraphen uses obstructions as structured reasons something cannot proceed safely. |
| Keep completion candidates separate from accepted change | HigherGraphen completion candidates remain reviewable proposals. |
| Start with CLI + JSON schema + skill | AI integration document recommends CLI and schema before MCP/plugin layers. |
| Use Architecture Product as MVP inspiration | HigherGraphen MVP roadmap names Architecture Product as first reference product. |
| Use append-only case log for engagement memory | Native CaseGraphen design uses `CaseSpace` plus append-only `MorphismLog`. |
| Keep private interpretation packages outside public repo | Commercial boundary document separates public core from production/customer packages. |

## References

- HigherGraphen repository: <https://github.com/CAPHTECH/higher-graphen>
- Product Integration for AI Agents: <https://raw.githubusercontent.com/CAPHTECH/higher-graphen/refs/heads/main/docs/guides/product-integration-for-ai-agents.md>
- AI Agent Integration: <https://raw.githubusercontent.com/CAPHTECH/higher-graphen/refs/heads/main/docs/specs/ai-agent-integration.md>
- Architecture Product: <https://raw.githubusercontent.com/CAPHTECH/higher-graphen/refs/heads/main/docs/product-packages/architecture-product.md>
- MVP Roadmap: <https://raw.githubusercontent.com/CAPHTECH/higher-graphen/refs/heads/main/docs/mvp-roadmap.md>
- Native CaseGraphen Case Management: <https://raw.githubusercontent.com/CAPHTECH/higher-graphen/refs/heads/main/docs/specs/intermediate-tools/casegraphen-native-case-management.md>
- Commercial Boundary: <https://raw.githubusercontent.com/CAPHTECH/higher-graphen/refs/heads/main/COMMERCIAL_BOUNDARY.md>

## Implementation caution

The current MVP verifies the selected local HigherGraphen APIs through path
dependencies on `higher-graphen-core`, `higher-graphen-structure`,
`higher-graphen-reasoning`, `higher-graphen-evidence`, and
`higher-graphen-projection`.
`AdvisorySpaceEnvelope::to_higher_graphen()` materializes advisory cells,
contexts, and incidences into HigherGraphen `InMemorySpaceStore`/`Context`
records. `advisorygraphen check` emits HigherGraphen `CheckResult`,
`Violation`, `Obstruction`, `Counterexample`, `RequiredResolution`, and
confidence evidence records, then projects them into the AdvisoryGraphen report
contract. `advisorygraphen project` builds a HigherGraphen `Projection` and
`ProjectionResult` before rendering the audience-specific AdvisoryGraphen
projection. The check report includes `result.higher_graphen` so callers can see
that the HigherGraphen store was built.

`higher-graphen-runtime` and `higher-graphen-interpretation` remain upstream
workflow/package crates. AdvisoryGraphen uses its own product-specific runtime
and interpretation package, but those layers now consume and emit HigherGraphen
core, structure, reasoning, evidence, and projection primitives.
