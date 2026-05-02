# 04. Source Adapters

## Purpose

Source adapters create bounded source snapshots. They do not create accepted consulting conclusions. Their job is to preserve input boundaries, source IDs, extraction loss, and provenance.

## Adapter interface

```rust
#[async_trait::async_trait]
pub trait SourceAdapter {
    type Config;
    type Error;

    async fn capture(&self, config: Self::Config) -> Result<EngagementSnapshot, Self::Error>;
}
```

## MVP adapters

| Adapter | Input | Output records |
| --- | --- | --- |
| `json_snapshot` | hand-authored JSON | direct records |
| `markdown_notes` | Markdown advisory notes | observations, claims, evidence excerpts |
| `architecture_json` | system/component/DB relation JSON | components, data stores, access relations |
| `interview_excerpt` | interview summary JSON | claims, evidence, unknowns |
| `issue_summary` | issue/PR summary JSON | actions, decisions, risks, evidence |

## Non-MVP adapters

- GitHub API adapter
- Jira adapter
- Slack adapter
- Google Drive adapter
- Notion adapter
- Code parser
- OpenAPI parser
- Database schema parser
- Metrics/log adapter

Do not implement network adapters until the file-based contract is stable.

## Source boundary contract

Every snapshot must specify:

| Field | Description |
| --- | --- |
| `included_source_ids` | What was included |
| `excluded_summary` | What was known but intentionally not included |
| `extraction_loss` | What was lost during summarization, parsing, or redaction |
| `trust_notes` | Source-specific reliability notes |
| `adapter_version` | Adapter version used to produce snapshot |

## Record kinds

| Record kind | Example |
| --- | --- |
| `component` | Order Service |
| `data_store` | Billing DB |
| `interface` | Billing API |
| `access_relation` | Order Service reads Billing DB |
| `requirement` | Billing status required at order confirmation |
| `test_or_verification` | Integration test for billing status |
| `claim` | “This direct access is risky.” |
| `evidence_excerpt` | ADR paragraph, interview quote summary |
| `risk` | DB schema change can break order flow |
| `action` | Replace direct DB read with API call |
| `unknown` | Ownership unclear |

## Accepted observations

A record can start as `accepted` only when:

1. it came from bounded source input;
2. the adapter marks it as source-backed;
3. the product policy allows adapter-supplied observations to be accepted;
4. it is not a derived recommendation, equivalence claim, or candidate.

AI-generated or heuristic-created records start as `unreviewed`.

## Extraction loss examples

- Full transcript omitted; only summary retained.
- Source document contains ambiguous ownership; adapter selected no owner.
- Code parser ignored dynamic runtime dependencies.
- PR summary omitted line-level diff.
- Customer-sensitive names redacted.

## Adapter output requirements

Adapters must not:

- infer absence from missing input;
- convert proposed actions into approved actions;
- merge context-specific terms without explicit mapping;
- hide extraction loss;
- place secrets into metadata;
- create Todoist tasks directly.

Adapters should:

- preserve source IDs;
- assign stable record IDs when possible;
- emit unknowns instead of guessing;
- keep confidence separate from review status;
- include adapter version and timestamp.
