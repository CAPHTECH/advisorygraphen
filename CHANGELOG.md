# Changelog

## 0.2.0 - 2026-05-29

- **Breaking:** `micro review` no longer pattern-matches prose. It now takes an
  `advisorygraphen.micro_review.request.v1` document of agent self-classified
  claims and enforces structural honesty deterministically: a claim marked
  evidence-backed without a cited witness becomes a
  `claim_marked_supported_without_evidence` obstruction, declared strong claims
  and unsupported high-blast-radius claims are flagged, and escalation is a
  deterministic rule over the classifications. The previous keyword/regex
  classification and heuristic risk scoring were removed.
- Add the `micro-review.request.v1` schema, an example request, and updated
  skills/docs/tests for the agent-classified contract.
- Add the task-oriented facade commands `propose`, `status`, `report`, and
  `review` with a case manifest, plus `status --brief` exposing a compact
  decision surface (`summary`, `top_blockers`, `next_best_action`).

## 0.1.3 - 2026-05-27

- Add micro review structure-risk triage for small AI answers, summaries, and compact PR notes.
- Add hypothesis-to-proposal evaluation coverage so unsupported hypotheses cannot become primary recommendations without observation work.
- Add medium/large PR review evaluation coverage proving broad review surfaces can be reduced to a bounded priority map.
- Update AdvisoryGraphen skills and acceptance docs for small, medium, and large review/proposal workflows.

## 0.1.2 - 2026-05-23

- Upgrade HigherGraphen workspace dependency requirements to `0.5.0`.
- Add review-gated completion dry-run gluing so accepted completions expose explicit policy overrides, blockers, and materialization requirements.
- Expose bounded HigherGraphen correspondence analysis in AI-agent projections, including emitted/omitted candidate counts, review focus candidates, and gluing summaries.
- Improve correspondence ranking so review focus prioritizes requirement, obstruction, and evidence participants over generic candidate-only similarities.
- Add an AdvisoryGraphen PR review skill that models PR diffs as bounded advisory snapshots and emphasizes AI authority, persistence, evidence-to-fact, public output/schema, and dependency/version boundaries.
- Update the dogfood example, CLI contract, workflow docs, schema artifact, and acceptance coverage for the HigherGraphen 0.5.0 review workflow.

## 0.1.1 - 2026-05-09

- Upgrade HigherGraphen workspace dependency requirements to `0.4.1`.
- Expose schema morphisms from lift output and all projections.
- Add projection loss metrics for omitted, collapsed, and source-trace-gap information.
- Add observation actions so agents can gather bounded evidence before promoting unsupported hypotheses.
- Update the AI agent operation contract, docs, and skill guidance to require these support objects during advisory workflows.

## 0.1.0 - 2026-05-09

- Initial crates.io release of the AdvisoryGraphen CLI and supporting crates.
