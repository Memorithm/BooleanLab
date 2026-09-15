# BL-14 search implementation status

This status concerns the Boolean sparsity controller inside existing numerical models. It does not propose replacing a complete model.

## Executable research infrastructure

- Exact static, structured, dynamic and relational mask primitives are present in `booleanlab-core`.
- Full scalar-function enumeration is bounded to 1..=4 predicates.
- Exhaustive mask fitting preserves minimum-error ties.
- Finite-domain CEGIS searches the declared candidate order and counts synthesis and verification work separately.
- SEARCH-to-HOLDOUT binding preserves exact rule semantics, predicate schema and resolved predicate parameters.
- [`bl14_search_comparison`](results/BL-14.5-SEARCH-COMPARISON.md) adds a matched linear first-exact baseline and checks the search outputs against full fitting. It includes positive, negative, ambiguous and inconsistent-label controls.
- Resource-aware Pareto screening has an explicit provenance guard: reference-model accounting and hardware measurements cannot be pooled, and candidates must share one frozen resource protocol and provenance identifier before their resource objectives are compared.

## Still required for sparsity-benefit claims

The search calibrations do not establish improved model sparsity or end-to-end performance. The next model-level slice needs an existing numerical workload, explicit magnitude/random/structured baselines at matched density, frozen predicate extraction, independently scored task quality, controller overhead, and actual work/traffic accounting. Real-device timing and subsequent runtime promotion are separate gates.

TDI-9.3 owns the corresponding action-policy calibration and its own scientific lineage. Only versioned, validated generic contracts should be shared; neither bench may borrow the other's final material or reinterpret representation calibration as task performance.