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

The exact APIs of HigherGraphen crates should be verified against the selected release or path dependency before implementing code. AdvisoryGraphen docs define the product and integration contract, not a guarantee that every API sketch compiles without adjustment.
