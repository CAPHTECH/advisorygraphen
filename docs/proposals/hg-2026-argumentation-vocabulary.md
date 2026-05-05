# Proposal to HigherGraphen: argumentation relation_type vocabulary

Status: draft, not yet filed.
Filing target: `CAPHTECH/higher-graphen` Issues, after AdvisoryGraphen has
operational evidence from at least one customer engagement.

## Summary

AdvisoryGraphen has shipped a hypothesis-first reasoning layer (see
`docs/19-hypothesis-dogfood-findings.md`, `docs/01-domain-model.md`,
`crates/advisorygraphen-reasoning/src/hypotheses/`). It uses four argumentation
relation types — `explains`, `supported_by`, `falsified_by`, `competes_with` —
and two new cell types — `hypothesis`, `falsifier`. Both are encoded in the
existing HG primitives (cells, incidences, lifecycle metadata) without HG
changes.

This proposal asks HigherGraphen to elevate the four relation types into the
official HG `relation_type` vocabulary so that other HG-based tools
(CaseGraphen, future advisory packages, third-party consumers) can interoperate
without each re-deriving the same conventions.

## Why now

`HypothesisCell` is **not** at the same abstraction level as HG's existing
primitives. It is a domain shape (in the inquiry / abductive reasoning domain)
expressible from HG primitives. So `hypothesis` and `falsifier` belong in
*interpretation packages* (AdvisoryGraphen has them), not in HG core.

But the **relation types** that connect those domain cells *are* generic. They
describe how any structure participates in argumentation, not how it
participates in advisory work specifically. They have the same level of
generality as `supports`, `contradicts`, `depends_on`, `verifies`, `owns`, all
of which are HG-level vocabulary today. Keeping them in AdvisoryGraphen alone
forces every other HG consumer to either invent the same names or invent
slightly different names.

We waited until AdvisoryGraphen had a working closed loop (falsify →
propagation → reframe) before proposing because:

1. We needed evidence that the four relations are distinct and useful, not
   theoretical.
2. We needed to learn whether more or fewer relations would be needed (none
   were missing in the dogfood run; none were redundant).
3. We needed to validate the lifecycle vocabulary
   (`candidate / supported / accepted / rejected / falsified`) under a real
   `case_log` replay before suggesting any of it migrate to HG.

The lifecycle vocabulary is **not** part of this proposal — it interacts with
HG's `review_status` enum and probably belongs in HG only after additional
discussion. This proposal is about **relation types only**.

## Proposed relation types

| Relation | Direction | Meaning | Strength default |
| --- | --- | --- | --- |
| `explains` | hypothesis → obstruction (or any structure) | The source structure offers a candidate explanation for the target structure | soft |
| `supported_by` | hypothesis → evidence | The source structure is reinforced by the target evidence; promotion to acceptance still requires explicit review | soft until review-promoted |
| `falsified_by` | hypothesis → falsifier | The source structure is reinforced or refuted by observing the target falsifier condition | soft; becomes hard when the falsifier is observed |
| `competes_with` | hypothesis ↔ hypothesis | Two structures offer competing explanations for the same target; mutually exclusive after acceptance | diagnostic |

These four relation types are sufficient to model:

- multiple explanations of one obstruction
- evidence accumulating in favour of an explanation
- explicit refutation conditions that an external observer could check
- bookkeeping of which explanations rule each other out

They are **not** sufficient to model lifecycle (handled by `review_status` and
`lifecycle_status` separately) or final acceptance (handled by review events).

## What HG would need to do

1. Document `explains`, `supported_by`, `falsified_by`, `competes_with` in the
   relation_type catalogue alongside existing entries.
2. Optionally: add a `argumentation_relation` boolean field to incidence
   metadata or a per-relation classification flag, so consumers can filter
   "argumentation incidences" from "structural incidences" without grepping the
   relation_type string.
3. Optionally: a structural reasoner helper `find_competing_explanations(obstruction_id)`
   that walks `explains` incidences in reverse and clusters by `competes_with`
   edges. AdvisoryGraphen does this today in
   `crates/advisorygraphen-reasoning/src/hypotheses/` and would migrate to the
   HG helper when available.

What HG would **not** need to do:

- Define `HypothesisCell` or `FalsifierCell` as built-in cell types; those stay
  in interpretation packages.
- Define hypothesis lifecycle states; those stay in interpretation packages
  until cross-tool experience justifies promotion.
- Provide a falsify/support/accept/reject API; those are workflow concerns and
  belong in tools.

## Migration path for AdvisoryGraphen

If HG accepts this:

1. AdvisoryGraphen continues to emit incidences with these relation_types
   exactly as now; no schema change.
2. AdvisoryGraphen drops its local "argumentation_relation: true" metadata flag
   in favour of HG's classification, if one exists.
3. AdvisoryGraphen optionally rewrites
   `crates/advisorygraphen-reasoning/src/hypotheses/` to use the HG reasoner
   helper if HG ships one.

No version bump in the AG schema is required, because the relation_types are
already string-typed and unprefixed.

## Risk if rejected

AdvisoryGraphen continues to function. CaseGraphen and other future HG
consumers will either invent parallel vocabulary or import AG's interpretation
package conventions ad-hoc. The cost is interoperability, not correctness.

## Reference

- AG implementation: `crates/advisorygraphen-reasoning/src/hypotheses/`
- AG dogfood findings (motivation): `docs/19-hypothesis-dogfood-findings.md`
- AG domain model: `docs/01-domain-model.md`
- ADR-0002 (confidence is not review): the lifecycle promotion model that this
  vocabulary is designed to fit alongside.
