# 03. Data Contracts and Schemas

## Contract stability principle

AdvisoryGraphen の CLI と agent skill は JSON schema を安定契約として扱う。Markdown や人間向けレポートは projection であり、機械処理の契約ではない。

## Schema files

| Schema | File |
| --- | --- |
| Engagement source snapshot | `schemas/advisorygraphen/engagement.snapshot.schema.json` |
| Advisory space | `schemas/advisorygraphen/advisory.space.schema.json` |
| Report envelope | `schemas/advisorygraphen/advisory.report.schema.json` |
| Projection request | `schemas/advisorygraphen/projection.request.schema.json` |
| Review event | `schemas/advisorygraphen/review.event.schema.json` |

## Engagement snapshot

The input snapshot is the bounded source material accepted by the workflow.

Required top-level shape:

```json
{
  "schema": "advisorygraphen.engagement.snapshot.v1",
  "snapshot_id": "snapshot:technical-advisory-smoke",
  "engagement_id": "engagement:acme-technical-advisory",
  "captured_at": "2026-05-02T00:00:00Z",
  "source_boundary": {
    "included_source_ids": [],
    "excluded_summary": [],
    "extraction_loss": []
  },
  "sources": [],
  "records": [],
  "metadata": {}
}
```

## Advisory space

The advisory space is the structural representation produced by lift.

Required top-level shape:

```json
{
  "schema": "advisorygraphen.space.v1",
  "space_id": "space:advisory:technical-smoke",
  "engagement_id": "engagement:acme-technical-advisory",
  "snapshot_id": "snapshot:technical-advisory-smoke",
  "package_id": "package:technical_advisory_mvp",
  "cells": [],
  "contexts": [],
  "incidences": [],
  "morphisms": [],
  "invariants": [],
  "policies": [],
  "metadata": {}
}
```

## Cell contract

```json
{
  "id": "cell:order-service",
  "cell_type": "component",
  "title": "Order Service",
  "summary": "Order management service",
  "context_ids": ["context:orders"],
  "source_ids": ["source:architecture-note"],
  "structure_refs": [],
  "provenance": {
    "origin": "source_backed",
    "actor": "source-adapter:json",
    "confidence": 1.0,
    "review_status": "accepted"
  },
  "metadata": {}
}
```

## Incidence contract

```json
{
  "id": "incidence:order-service-accesses-billing-db",
  "relation_type": "accesses",
  "from_id": "cell:order-service",
  "to_id": "cell:billing-db",
  "context_ids": ["context:orders", "context:billing"],
  "evidence_ids": ["cell:evidence-architecture-note-1"],
  "strength": "hard",
  "provenance": {
    "origin": "source_backed",
    "actor": "source-adapter:json",
    "confidence": 1.0,
    "review_status": "accepted"
  },
  "metadata": {}
}
```

## Invariant result contract

```json
{
  "invariant_id": "invariant:no_cross_context_direct_database_access",
  "status": "violated",
  "severity": "high",
  "witness_ids": [
    "cell:order-service",
    "cell:billing-db",
    "incidence:order-service-accesses-billing-db"
  ],
  "obstruction_ids": ["obstruction:order-service-direct-billing-db-access"],
  "message": "Order Service accesses Billing DB owned by Billing context."
}
```

## Obstruction contract

```json
{
  "id": "obstruction:order-service-direct-billing-db-access",
  "obstruction_type": "boundary_violation",
  "severity": "high",
  "blocked_ids": ["decision:approve-current-architecture"],
  "witness_ids": ["incidence:order-service-accesses-billing-db"],
  "evidence_ids": ["cell:evidence-architecture-note-1"],
  "recommended_completion_types": ["proposed_interface", "proposed_refactor_action"],
  "review_status": "unreviewed",
  "message": "Order Service directly reads Billing DB across ownership boundary."
}
```

## Completion candidate contract

```json
{
  "id": "candidate:billing-status-api",
  "candidate_type": "proposed_interface",
  "title": "Add Billing status query API",
  "rationale": "Remove cross-context direct database access while preserving billing status check.",
  "resolves_obstruction_ids": ["obstruction:order-service-direct-billing-db-access"],
  "proposed_cell_ids": ["cell:billing-status-api"],
  "source_ids": ["source:architecture-note"],
  "confidence": 0.82,
  "review_status": "unreviewed",
  "metadata": {}
}
```

## Report envelope

All report-producing commands must return a stable envelope.

```json
{
  "schema": "advisorygraphen.report.v1",
  "report_type": "check",
  "report_version": 1,
  "tool": {
    "name": "advisorygraphen",
    "version": "0.1.0"
  },
  "input": {},
  "result": {},
  "projection": {},
  "warnings": []
}
```

## Projection loss contract

Every projection must contain:

```json
{
  "projection_loss": [
    {
      "loss_type": "omitted_source_text",
      "description": "Interview transcript excerpts were summarized for executive audience.",
      "omitted_ids": ["source:interview-cto-full"],
      "severity": "medium"
    }
  ]
}
```

## Versioning

- Schema IDs are exact strings.
- Breaking changes increment schema suffix from `.v1` to `.v2`.
- Migrations are represented as schema morphisms.
- CLI commands must reject unsupported schema versions unless a migration is explicitly requested.

## Required validation behavior

`advisorygraphen validate` must check:

1. exact schema ID
2. stable IDs
3. duplicate IDs
4. relation endpoints exist
5. source IDs referenced by records exist
6. review status values are valid
7. provenance exists on all cells and incidences
8. projection loss exists for projection outputs
9. metadata does not carry required semantics
10. AI-inferred records are not accepted unless a review event exists
