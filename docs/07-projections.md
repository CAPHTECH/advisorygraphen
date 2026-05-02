# 07. Projections

## Principle

Projection is a lossy audience-specific view. It is not the source of truth. Every projection must declare represented IDs, omitted IDs, and information loss.

HigherGraphen is operated primarily by AI agents. Human-facing views are projections over the structure, not the primary editing surface. The `ai_agent` projection is therefore an operational contract for the next agent step, not a decorative report.

## Projection audiences

| Audience | Purpose | Format |
| --- | --- | --- |
| `executive` | 意思決定者向け論点、リスク、選択肢、未解決障害 | Markdown / JSON |
| `developer_action` | 実装担当者向けタスク、依存関係、完了条件 | Markdown / JSON |
| `audit_trace` | 根拠、レビュー、source boundary、projection loss | JSON |
| `ai_agent` | AIエージェントがHGを継続操作するための操作契約、禁止操作、candidate 状態 | JSON |
| `client_review` | 顧客とのレビュー画面に出す候補 | JSON / Markdown |
| `cli` | deterministic command output | JSON |

## Projection request

```json
{
  "schema": "advisorygraphen.projection.request.v1",
  "projection_id": "projection:executive-review",
  "space_id": "space:advisory:technical-smoke",
  "audience": "executive",
  "purpose": "monthly_advisory_review",
  "include_ids": [],
  "exclude_ids": [],
  "policy_ids": ["policy:executive-summary-default"],
  "metadata": {}
}
```

## Projection result shape

```json
{
  "projection_id": "projection:executive-review",
  "audience": "executive",
  "represented_ids": [],
  "omitted_ids": [],
  "information_loss": [],
  "allowed_operations": [],
  "view": {}
}
```

## Executive projection

Must include:

- executive summary
- decisions required
- high-severity obstructions
- top completion candidates
- evidence confidence and review status summary
- items not safe to decide yet
- projection loss

Must not include:

- raw confidential excerpts unless policy allows
- unreviewed AI inference as accepted conclusion
- low-level task details beyond decision impact

## Developer action projection

Must include:

- actions
- owner or owner-needed marker
- dependencies
- verification method
- related obstruction
- related candidate
- review status
- definition of done

Task example:

```json
{
  "id": "action:replace-direct-db-read",
  "title": "Replace Order Service direct Billing DB read with Billing API call",
  "owner_id": "owner:platform-team",
  "depends_on": ["candidate:billing-status-api"],
  "verification_ids": ["cell:test-billing-status-flow"],
  "review_status": "accepted",
  "related_obstruction_ids": ["obstruction:order-service-direct-billing-db-access"]
}
```

## Audit projection

Must include:

- source boundary
- all source IDs represented
- all source IDs omitted
- extraction loss
- evidence origin
- review events
- candidate accept/reject history
- invariant check results
- obstructions and witnesses
- projection loss
- schema versions

Audit output should be machine-readable JSON first.

## AI agent projection

Must include:

- HG operation model
- allowed commands
- review-gated commands
- forbidden operations
- resume protocol
- unreviewed candidates
- missing evidence
- frontier work
- hard blockers
- policy requirements
- IDs required for follow-up commands

The AI projection should make it hard for an agent to accidentally promote candidate structure.

Minimum operation model:

```json
{
  "hg_operation_model": {
    "primary_operator": "ai_agent",
    "human_role": "sets goals, reviews candidates, and accepts or rejects promotions",
    "human_ui_role": "projection_consumer",
    "source_of_truth": "advisory_space_case_log_and_review_events"
  },
  "agent_operation_contract": {
    "allowed_commands": [
      "validate",
      "lift",
      "check",
      "completions propose",
      "project ai_agent",
      "project audit_trace",
      "case import",
      "case reason"
    ],
    "review_gated_commands": [
      "completions accept",
      "completions reject"
    ],
    "forbidden_operations": [
      "promote unreviewed candidate structure",
      "hide projection_loss",
      "treat inferred evidence as accepted fact"
    ],
    "resume_protocol": [
      "read close_status",
      "inspect open_obstructions",
      "propose missing owner or verification structure",
      "generate audit_trace before reporting final state"
    ]
  }
}
```

## Projection loss taxonomy

| Loss type | Meaning |
| --- | --- |
| `omitted_source_text` | raw source text excluded |
| `summarized_evidence` | evidence compressed into summary |
| `collapsed_contexts` | multiple contexts shown as one |
| `hidden_policy` | internal policy omitted from audience |
| `omitted_low_severity_items` | low-severity findings omitted |
| `redacted_customer_data` | sensitive data removed |
| `unsupported_claim_hidden` | unsupported claim intentionally hidden |
| `unreviewed_candidate_hidden` | candidate hidden from audience |

## Projection renderer API sketch

```rust
pub trait ProjectionRenderer {
    fn audience(&self) -> AdvisoryAudience;

    fn project(
        &self,
        space: &AdvisorySpaceEnvelope,
        report: &AdvisoryReportEnvelope,
        request: &ProjectionRequest,
    ) -> AdvisoryResult<ProjectionResult>;
}
```

## Markdown rendering rule

Markdown output must be generated from JSON projection data. Do not create a separate Markdown-only path that bypasses projection loss, review status, or evidence disclosure.
