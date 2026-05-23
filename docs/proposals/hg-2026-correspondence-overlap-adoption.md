# HigherGraphen 0.5 Correspondence / Overlap adoption assessment

Status: implemented for AI-agent projection diagnostics and completion review
policy evidence.
Decision: adopt the dependency update and expose HigherGraphen correspondence
and gluing as first-class AI-agent projection and completion-review structure.

## Summary

HigherGraphen `0.5.0` adds a first-class correspondence surface:

- `CorrespondenceCell`
- `OverlapWitness`
- `DifferenceWitness`
- `GluingAttempt`
- deterministic correspondence candidate generation
- deterministic gluing checks
- human, AI-agent, and audit projections for correspondence review
- bounded semantic correspondence signals that remain reviewable candidates

This overlaps strongly with AdvisoryGraphen's existing model for reviewable
completion candidates, hypothesis support/falsification, proposal content,
schema morphisms, and projection loss. AdvisoryGraphen now exposes this overlap
through `ai_agent.correspondence_analysis` and
`higher_graphen_gluing_review`, while keeping generated correspondences
reviewable and non-promoted.

## Immediate compatibility

AdvisoryGraphen currently depends on HigherGraphen through sibling path
dependencies. Once the sibling checkout is at `v0.5.0`, the old `^0.4.1`
requirements no longer select a compatible local package.

The immediate update is therefore required for local development against the
current HigherGraphen checkout:

- `higher-graphen-core = "0.5.0"`
- `higher-graphen-evidence = "0.5.0"`
- `higher-graphen-interpretation = "0.5.0"`
- `higher-graphen-projection = "0.5.0"`
- `higher-graphen-reasoning = "0.5.0"`
- `higher-graphen-structure = "0.5.0"`

`cargo check --workspace` passes after this update, so no public API break
currently blocks AdvisoryGraphen.

## Fit for AdvisoryGraphen

### Strong fit: hypothesis and evidence overlap

AdvisoryGraphen often has multiple hypotheses, evidence records, falsifiers,
and structure proposals that refer to the same underlying source material or
claim. HigherGraphen correspondence can make those overlaps explicit instead
of encoding them only in ad hoc metadata arrays.

Useful mappings:

| AdvisoryGraphen concept | HigherGraphen 0.5 concept |
| --- | --- |
| Competing hypotheses that share evidence or claims | `CorrespondenceCell` with `OverlapWitness` |
| A falsifier contradicting a hypothesis claim | `DifferenceWitness` with blocking or major severity |
| Evidence reused across proposals | Evidence overlap candidate |
| Hypothesis refinement lineage | `Refinement` correspondence |
| Projection-loss explanation | Correspondence projection with retained witnesses and omitted fields |

### Strong fit: completion dry-run and application review

Completion candidates already carry `proposal_content.scenario`,
`proposal_content.morphism`, `invariant_checks`, witnesses, valuation, policy,
and an application plan. A `GluingAttempt` is a good fit for the final review
question: "can the candidate structure be joined with the current advisory
space without silent loss?"

The first useful slice is not to replace completion candidates. It is to add an
optional correspondence/gluing diagnostic in dry-run output when a candidate
would add, replace, or relate existing cells/incidences.

### Medium fit: duplicate or equivalent source records

Deterministic overlap on normalized labels, shared evidence, invariants, and
typed relation triples could help `lift` report possible duplicate records or
duplicate claims. This is useful, but should start as a warning/projection
surface because source records can intentionally repeat similar language.

### Not a fit yet: automatic semantic merge

HigherGraphen 0.5 explicitly keeps LLM and embedding signals as reviewable
semantic candidates. AdvisoryGraphen should preserve that boundary. Semantic
overlap can rank review work, but it must not accept hypotheses, merge records,
or promote completion candidates by confidence alone.

## Implemented adoption slice: AI-agent projection

1. Keep the dependency bump to `0.5.0`.
2. Add an internal adapter that converts selected AdvisoryGraphen cells into
   `CorrespondenceSubject` values:
   - hypotheses
   - evidence
   - obstructions
   - completion candidates
   - invariants
3. Run `derive_correspondence_candidates` in `ai_agent` projection diagnostics.
4. Run `attempt_gluing` for each correspondence candidate and attach the
   resulting `GluingAttempt` to the candidate.
5. Project each correspondence through HigherGraphen's AI-agent correspondence
   projection.
6. Expose the result under `correspondence_analysis`:
   - `candidate_count` as the total generated count
   - `emitted_candidate_count`, `omitted_candidate_count`, and
     `max_emitted_candidates`
   - `gluing_summary`
   - ranked `review_focus_candidates`
   - bounded HG `candidates`
   - bounded HG `ai_agent_projections`
7. Keep every generated correspondence at reviewable candidate status.

The AI-agent projection intentionally does not expand every success-only
correspondence. It ranks gluing failures, gluing review candidates, blocking
differences, major differences, and structural or constraint witnesses first,
then emits only the bounded review-focus set. Omitted candidates remain counted
for audit and can be regenerated from the source graph if full trace expansion
is needed.

## Implemented adoption slice: completion review

Completion candidate dry-runs now attach `higher_graphen_gluing_review` to
each applied entry. The review is derived from the candidate, the pre-existing
obstructions it claims to resolve, the materialized cells/incidences, and any
incidence removals. It exposes:

- correspondence count and gluing summary
- preserved structures and invariants
- blocking difference IDs
- raw correspondence/gluing diagnostics
- policy blockers

`completions accept` records the same review in review-event metadata under
`higher_graphen_gluing_policy`. Accepting a candidate with blockers is still
allowed, but the event records
`policy_override: "explicit_completion_review"` so the override is explicit
rather than silent.

`completions apply-accepted` carries the review result, blockers, and explicit
override forward into applied-structure output. The result is therefore visible
at proposal, review, and materialization time.

## Risks

- Schema expansion risk: adding correspondence arrays to stable reports may
  require schema and fixture updates.
- Terminology overlap: AdvisoryGraphen already uses "candidate",
  "proposal_content", and "morphism"; projections must make clear when a
  correspondence candidate is only diagnostic.
- Overfitting risk: deterministic overlap on labels can create noisy matches.
  Start with evidence, invariant, and typed relation overlap before enabling
  label-only recommendations.
- Review-boundary risk: semantic signals must stay candidates until explicit
  review, matching the existing AG rule that confidence never replaces review.

## Current decision

Use `0.5.0` immediately for dependency compatibility. Treat correspondence and
gluing as first-class AI-agent diagnostics and completion-review evidence, not
accepted advisory facts. A HigherGraphen gluing blocker cannot silently promote,
reject, or merge advisory structures; it must be resolved by revision or by an
explicit completion review record.
