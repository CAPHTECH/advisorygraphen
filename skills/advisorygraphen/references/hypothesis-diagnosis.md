# Hypothesis diagnosis reference

Use this when the user asks for diagnosis, investigation, quality assessment,
root-cause analysis, or evidence-backed proposal generation.

1. State the problem as a single falsifiable question. Example: "Is the default
   unit-test lane a trustworthy quality gate?"
2. Generate multiple hypotheses that could explain the same problem. Avoid
   making the first plausible explanation the default conclusion.
3. For each hypothesis, record:
   - expected observations if true;
   - observations that would weaken or falsify it;
   - source IDs or commands needed to check it;
   - initial confidence as unreviewed or inferred unless source-backed.
   In JSON snapshots, prefer `record_type: "hypothesis_seed"` for these
   records. AdvisoryGraphen lifts them to `cell_type: "hypothesis"` and
   preserves `metadata.expected_observations`, `metadata.falsifiers`, and
   `metadata.candidate_structure_types`.
4. Run the cheapest discriminating observations first. A useful observation is
   one that separates at least two hypotheses, not merely one that adds detail.
   When an observation narrows or revises a hypothesis, record the next version
   as `record_type: "hypothesis_refinement"` and connect it to the earlier
   hypothesis with `relation_type: "refines"`. Prefer deriving proposals from
   the refined hypothesis, not the initial seed.
5. Classify hypotheses explicitly:
   - `strongly_supported`: direct observation supports it and major competing
     explanations are weakened.
   - `supported`: source-backed evidence supports it, but another plausible
     explanation remains.
   - `supported_needs_followup`: evidence supports it, but a blocker prevents a
     decisive measurement.
   - `plausible_secondary`: evidence suggests it may matter, but it is not the
     direct observed cause.
   - `falsified`: observed evidence contradicts the hypothesis.
   - `insufficient_evidence`: no discriminating observation was collected.
6. Build proposals from supported hypotheses only. Proposal priority should
   follow causal order: unblock observation first, fix false-positive/failure
   semantics next, then improve policy, ownership, or performance.
   In JSON snapshots, record these as `record_type: "structure_proposal"` and
   connect them to their source hypotheses with a `derives_from` relation.
   AdvisoryGraphen lifts them to proposal actions and checks whether the
   underlying hypothesis is supported before the action can be treated as a
   primary recommendation.
   For P0/P1 proposals, ensure the source hypothesis has refinement lineage.
   Otherwise AdvisoryGraphen emits
   `high_priority_proposal_missing_hypothesis_refinement`.
7. Report proposal trace in this shape:
   `problem -> hypothesis -> evidence -> classification -> proposal -> required
   verification/owner`.
8. Preserve non-winning hypotheses in the projection. They are useful because
   they show why the chosen proposal is not just a single-agent guess.
