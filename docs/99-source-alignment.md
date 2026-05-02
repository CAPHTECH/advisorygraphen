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
dependencies on `higher-graphen-core` and `higher-graphen-structure`.
`AdvisorySpaceEnvelope::to_higher_graphen()` materializes advisory cells,
contexts, and incidences into HigherGraphen `InMemorySpaceStore`/`Context`
records, and `advisorygraphen check` runs structural relationship checks against
that materialized store. The check report includes `result.higher_graphen` so
callers can see that the HigherGraphen store was built.

APIs from the other HigherGraphen crates remain integration candidates, not
compiled dependencies, until AdvisoryGraphen needs their concrete engines.
