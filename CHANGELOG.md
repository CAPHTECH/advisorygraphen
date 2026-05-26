# Changelog

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
