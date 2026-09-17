# BL-14 search implementation status

This status concerns the Boolean sparsity controller inside existing numerical models. It does not propose replacing a complete model.

## Executable research infrastructure

- Exact static, structured, dynamic and relational mask primitives are present in `booleanlab-core`.
- Full scalar-function enumeration is bounded to 1..=4 predicates.
- Exhaustive mask fitting preserves minimum-error ties.
- Finite-domain CEGIS searches the declared candidate order and counts synthesis and verification work separately.
- SEARCH-to-HOLDOUT binding preserves exact rule semantics, predicate schema and resolved predicate parameters.
- Bounded conjunction synthesis now has a versioned `bl14.conjunctive-freeze.v1` adapter into the exact SEARCH→FREEZE→HOLDOUT semantic boundary. The adapter evaluates the original `booleanlab-core` rule over the complete canonical predicate table rather than copying its evaluator, preserves caller-owned predicate schema/resolved parameters, and fails closed where the exact scalar Boolean-function contract cannot represent the rule.
- [`bl14_search_comparison`](results/BL-14.5-SEARCH-COMPARISON.md) adds a matched linear first-exact baseline and checks the search outputs against full fitting. It includes positive, negative, ambiguous and inconsistent-label controls.
- Resource-aware Pareto screening has an explicit provenance guard: reference-model accounting and hardware measurements cannot be pooled, and candidates must share one frozen resource protocol and provenance identifier before their resource objectives are compared.
- Resource-aware Pareto **screening is SEARCH-only**. Frozen HOLDOUT candidates are rejected explicitly so final evidence cannot feed back into candidate selection.
- `bl14.matched-baseline-set.v1` now materializes magnitude-like, deterministic-random, and N:M structured controls at the exact width and retained cardinality of an already-materialized Boolean candidate. It binds score/seed provenance, rejects duplicate random seeds, and refuses structured density rounding when the candidate cardinality cannot be distributed uniformly across complete groups. This is construction infrastructure only; it does not score a model or create a performance result.
- `bl14.matched-evaluation-arms.v1` freezes the model-evaluation arm identity/order from that immutable baseline set: dense reference first, then the exact Boolean, magnitude, structured, and preregistered random masks. Dense is explicitly full while every sparse arm must preserve exact Boolean width/cardinality. The manifest still performs no model execution, HOLDOUT inspection, ranking, traffic measurement, or performance claim.
- Post-freeze HOLDOUT reporting now has a separate executable surface. It revalidates the frozen candidate identity, matched domain size, exact Boolean truth table, predicate schema and resolved predicate parameters, requires one resource-evidence kind/protocol/provenance contract, and emits observations in the immutable SEARCH-selected order. HOLDOUT objectives are not used to rank, Pareto-screen, retune or reorder candidates.

## Still required for sparsity-benefit claims

The search calibrations do not establish improved model sparsity or end-to-end performance. The next model-level slice needs an existing numerical workload, explicit magnitude/random/structured baselines at matched density, frozen predicate extraction, independently scored task quality, controller overhead, and actual work/traffic accounting. Real-device timing and subsequent runtime promotion are separate gates.

TDI-9.3 owns the corresponding action-policy calibration and its own scientific lineage. Only versioned, validated generic contracts should be shared; neither bench may borrow the other's final material or reinterpret representation calibration as task performance.
