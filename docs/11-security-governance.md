# 11. Security and Governance

## Security posture

AdvisoryGraphen handles customer advisory material. The default posture must assume source data can include confidential product, architecture, business, personnel, and security information.

## Data classification

| Classification | Examples | Default handling |
| --- | --- | --- |
| `public` | synthetic examples, public docs | may be committed |
| `internal` | generic advisory templates | private or controlled repo |
| `customer_confidential` | customer architecture, strategy, incidents | never public repo |
| `regulated` | legal, financial, medical, personal data | strict policy, redaction, explicit approval |
| `secret` | tokens, private keys, credentials | must not be ingested; fail validation if detected |

## Public repository rule

Do not commit:

- customer-specific documents
- real engagement reports
- private case logs
- support notes
- deployment details
- proprietary interpretation packages
- private evaluation data
- secrets or credentials
- pricing or negotiation notes

## Source adapter governance

Adapters must:

- record included/excluded sources;
- record extraction loss;
- redact secrets when configured;
- fail closed for known secret patterns;
- avoid hidden network calls in MVP;
- preserve source boundaries.

## AI governance

AI-generated content must be marked as:

```json
{
  "origin": "inferred",
  "review_status": "unreviewed"
}
```

It cannot satisfy hard evidence requirements unless review-promoted.

## Projection governance

Each projection has audience and policy.

| Audience | Data risk |
| --- | --- |
| `executive` | strategic simplification may hide caveats |
| `developer_action` | task export may leak sensitive context |
| `audit_trace` | includes extensive provenance; high sensitivity |
| `ai_agent` | operational details and allowed commands; high sensitivity |

## Policy checks

Minimum policies:

- `customer_data_public_repo_prohibited`
- `secret_ingestion_prohibited`
- `audit_projection_requires_authorized_audience`
- `ai_projection_requires_internal_policy`
- `external_projection_must_disclose_loss`

## Logging

Runtime logs should include command metadata and IDs, but not raw source text unless debug mode is explicitly enabled and safe for the environment.

## Threats

| Threat | Mitigation |
| --- | --- |
| AI promotes unsupported recommendation | review status and invariant checks |
| Customer data committed to public repo | classification policy and pre-commit validation |
| Projection hides critical caveat | projection loss invariant |
| Stale review accepts obsolete candidate | base revision check |
| Source adapter over-infers | adapter governance and unknown records |
