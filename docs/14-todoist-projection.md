# 14. Todoist Projection

## Purpose

Todoist is an execution surface for tasks. It is not the AdvisoryGraphen source of truth. AdvisoryGraphen should export task candidates as projection JSON, then a separate integration can create or update Todoist tasks.

## Projection audience

`todoist_task_export`

## Export policy

Default policy:

| Item | Export behavior |
| --- | --- |
| accepted action with owner and verification | exportable |
| accepted action without owner | blocked or exported with owner-needed label if policy allows |
| unreviewed completion candidate | not exportable by default |
| unreviewed completion candidate with draft policy | export as draft task candidate |
| high-severity obstruction resolution action | exportable only after review |
| raw evidence excerpt | never included by default |

## Task payload contract

```json
{
  "schema": "advisorygraphen.todoist.projection.v1",
  "projection_id": "projection:todoist:technical-smoke",
  "space_id": "space:advisory:technical-smoke",
  "tasks": [
    {
      "external_key": "advisorygraphen:action:replace-direct-db-read",
      "content": "Replace Order Service direct Billing DB read with Billing API call",
      "description": "AdvisoryGraphen ID: action:replace-direct-db-read\nRelated obstruction: obstruction:order-service-direct-billing-db-access\nReview status: accepted",
      "labels": ["AdvisoryGraphen", "technical-advisory", "reviewed"],
      "priority": 4,
      "due_string": null,
      "source_ids": ["action:replace-direct-db-read"],
      "review_status": "accepted",
      "export_status": "exportable"
    }
  ],
  "blocked_tasks": [],
  "projection_loss": []
}
```

## Mapping

| AdvisoryGraphen | Todoist projection field |
| --- | --- |
| action title | `content` |
| AdvisoryGraphen ID | `external_key`, description |
| owner | label or description; actual assignee only if integration supports it |
| severity | priority |
| due recommendation | `due_string` |
| verification method | description |
| review status | label and description |
| related obstruction | description |

## Priority mapping

| Severity | Todoist priority |
| --- | --- |
| `critical` | 4 |
| `high` | 4 |
| `medium` | 3 |
| `low` | 2 |
| `info` | 1 |

## Labels

Recommended labels:

- `AdvisoryGraphen`
- package label, e.g. `technical-advisory`
- `reviewed` or `draft`
- `owner-needed`
- `evidence-needed`
- `obstruction-resolution`

## Export statuses

| Status | Meaning |
| --- | --- |
| `exportable` | Safe to create/update external task |
| `draft_only` | Can be shown as draft but not auto-created by default |
| `blocked_missing_review` | Requires review |
| `blocked_missing_owner` | Requires owner |
| `blocked_policy` | Policy prevents export |

## Integration boundary

The projection crate emits JSON only. A future adapter may consume this JSON and call Todoist. That adapter should be isolated from core crates and should verify current Todoist API details at implementation time.

## Idempotency

Use `external_key` for idempotency. The Todoist integration should avoid duplicate task creation by storing or searching for the AdvisoryGraphen ID. The exact external metadata mechanism is adapter-specific and should not be embedded in the core model.

## Safety

Do not include:

- raw interview excerpts;
- private strategy details;
- credentials;
- unsupported recommendations;
- unreviewed candidate details unless draft policy allows.

Do include:

- concise action;
- AdvisoryGraphen ID;
- review status;
- owner-needed marker;
- internal link or reference when available;
- verification summary.
