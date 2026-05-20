# Requirements definition reference

Use this when the task starts from existing documents (interviews, workflow
analysis, competitive research, stakeholder requirements) rather than from code
or architecture. The goal is to derive a validated `advisory.input.json` from
unstructured sources before running the lift → check pipeline.

## 0. Incremental reading strategy

**Only add what the source explicitly states.** Do not add records for actions,
owners, or decisions unless the source document names them. Do not promote
`origin: inferred` to `origin: source_backed`. If a concept feels implied but
is not written, leave it out — the lift → check pipeline will surface the gap
as an obstruction or missing candidate.

Read sources one at a time. After each source, write extracted structure into
`advisory.input.json` before reading the next source. Use the accumulated
structure as working memory when reading subsequent sources.

```
read source-1 → write hypotheses/requirements/claims to advisory.input.json
read source-2 + current advisory.input.json
  → does source-2 support, contradict, or refine existing hypotheses?
  → add supports/competes_with/refines relations; add new hypotheses if needed
read source-3 + current advisory.input.json
  → repeat
...
run lift → check only after all sources are processed
```

This way the agent never needs all source content in context simultaneously.
The JSON file serves as inter-source working memory, and contradictions become
visible by comparing new content against already-formalised structure.

Before reading each subsequent source, extract only the working-memory skeleton
from the accumulated file rather than reading the full JSON:

```sh
jq '[.records[] | {
  id,
  type: .record_type,
  label: .metadata.label,
  status: .metadata.hypothesis_status,
  rel: (if .relation then "\(.relation.relation_type):\(.relation.from_record_id)→\(.relation.to_record_id)" else null end)
}]' advisory.input.json
```

This reduces token cost by ~75% while preserving all information needed to
detect support, contradiction, or redundancy against the next source.

## 1. Hypothesis extraction from source documents

Read each source document with these questions:

- What causal claims does this document make? ("X is caused by Y")
- What assumptions does the proposed solution depend on?
- What would have to be true for this recommendation to fail?

For each load-bearing claim, write a falsifiable hypothesis with
`record_type: "hypothesis_seed"`, including `expected_observations` and
`falsifiers`. Do not record conclusions as hypotheses — a conclusion has no
falsifier.

Prioritise hypotheses that are directly load-bearing for a proposed action, are
disputed between sources, or have never been measured.

## 2. Contradiction detection as competing hypothesis seed

When two sources disagree on a claim, treat the disagreement as a competing
hypothesis pair rather than resolving it editorially.

Example: management document says "mobile is low priority"; interview data says
"2/5 users want mobile"; competitive benchmark says "mobile adoption is low for
office workers." Do not pick one. Generate the hypothesis and its competitor,
link them with `relation_type: "competes_with"`, and let hypothesis classification
decide.

Do not resolve source contradictions by choosing one source. Both sides belong in
the snapshot.

## 3. Requirements as verification contracts

Record each in-scope feature as `record_type: "requirement"` with
`require_verification: true`. AdvisoryGraphen emits `requirement_unverified` for
any requirement without a `verifies` or `implements` incidence. In requirements
definition, the "test" is the measurement plan or KPI — record it as
`record_type: "test_or_verification"` and link it with a `verifies` relation.

Treat `proposal_derived_from_unsupported_hypothesis` as a scope risk signal: the
proposed feature has no validated need. Do not suppress it; surface it to the
stakeholder.

## 4. Decision recording

When requirements definition produces a go/no-go decision, record it as
`record_type: "claim"` with `metadata.decision_type`. Decisions are human
judgments, not tool outputs — `claim` cells are intentionally excluded from
obstruction checking and projection views.

```json
{
  "id": "record:decision-...",
  "record_type": "claim",
  "metadata": {
    "decision_type": "go | conditional_go | deferred | rejected",
    "phase": "phase-1",
    "rationale": "...",
    "acceptance_criterion": "..."
  }
}
```

Do not record a `go` decision until the relevant hypothesis is classified as
`supported` or stronger. A `go` on a `candidate` hypothesis is a premature
decision, not a structured conclusion.
