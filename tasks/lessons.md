# Lessons

## Don't fix a brittle keyword classifier by adding more keywords (2026-05-29)

**Context:** `micro review` failed to flag an overconfident AI answer
("definitely... no downside... 10x"). My first fix was to expand the
`STRONG_CLAIM_MARKERS` substring list.

**Correction (user):** "パターンで判定している？問題では？" — the keyword/regex
approach is itself the defect, not the list contents. Expanding markers is the
same brittle path: it never generalizes, mis-handles negation, and is an
endless maintenance treadmill (a local optimum, which violates the global
elegance/minimalism rules).

**Rule:** Semantic judgement (is this overconfident? an assumption?
evidence-backed?) belongs to the LLM/agent. A deterministic tool should only
enforce *structural facts* it can verify (e.g. "a claim classified
evidence-backed must cite a witness"). When you catch yourself adding keywords
to capture meaning, stop — push the classification to the agent and have the
tool validate structure. This mirrors AdvisoryGraphen's core thesis
(`supported_hypothesis_missing_support`): AI infers, the tool enforces that
support can't be self-certified without structure.
