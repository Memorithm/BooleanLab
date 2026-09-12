# BooleanLab

BooleanLab is the Memorithm research bench for **pure Boolean AI**, **Boolean-function discovery**, and **Boolean × mathematical-domain hybrid systems**.

The bench studies three connected programmes:

1. **Pure Boolean systems** — state, transitions, inference and execution represented in `{0,1}` and composed from discrete logical operators.
2. **Boolean × X systems** — a Boolean control/state plane interacting with a richer mathematical domain `X`: finite fields, real/complex systems, quaternions, octonions, sedenions, tensors, graphs, semirings, dynamical systems and other mathematical structures.
3. **Boolean-function discovery** — systematic generation, exact characterization, deduplication and equivalence screening of Boolean functions induced either by Boolean-only search or by Boolean × X constructions.

BooleanLab is a research bench, not a production inference runtime. Scientific claims require reproducible evidence and must remain narrower than the experiments that support them.

## Current research status

| Experiment | Status | Verified result |
| --- | --- | --- |
| BL-13.0.1 | VALIDATED | Exact Boolean screening reproduces frozen reference properties using SciRust ANF/Walsh metrics. |
| BL-13.1.1 | VALIDATED | Exhaustive scan of all 65,536 four-input functions: 12,870 balanced, 896 bent, 222 resilient under the declared criterion, 1,152 three-valued plateaued under the preregistered operational definition; maximum nonlinearity 6. |
| BL-13.1.2 | VALIDATED | From 4,096 deterministic eight-input circuits (4–24 gates), 1,685 exact unique functions were observed, including 473 balanced functions; best observed nonlinearity 96 and 18 Pareto-front members. |
| BL-13.2.0 | VALIDATED | A bounded SciRust sedenion control produced 16 distinct scalar functions `F_2^8 -> F_2`; observed algebraic degrees 6–8 and nonlinearities 76–100. No novelty claim. |
| BL-13.2.1 | PROPOSED | Screen the sedenion-induced functions against the validated bounded Boolean-only population, then strengthen equivalence and construction-cost matching. |
| BL-13.3.1 | VALIDATED | Two `GF(2^8)` inversion constructions each produced eight distinct component functions with degree 7, nonlinearity 112 and balanced outputs. Pipeline validation only; no novelty claim. |

The machine-readable experiment registry is [`experiments/REGISTRY.tsv`](experiments/REGISTRY.tsv). Reproducible evidence is retained under [`experiments/results/`](experiments/results/), including the frozen [`BL-13.1 Boolean baseline`](experiments/results/BL-13.1-BOOLEAN-BASELINE.md).

## Core hybrid contract

The generic research object is

```text
b(t+1) = F_bool(b(t), P_X(x(t)), u(t))
x(t+1) = G_{b(t+1)}(x(t), u(t))
```

where `b ∈ {0,1}^m`, `x ∈ X^n`, `P_X` extracts explicit predicates from the mathematical domain, and the Boolean state may select, mask, route, constrain or verify operations in `X`.

The central question is not whether Boolean × X is automatically superior. It is:

> For which domains X, tasks and resource envelopes does coupling an explicit Boolean state/control plane to X provide a measurable capability, efficiency, verifiability or interpretability advantage over matched Boolean-only and X-only baselines?

For Boolean-function discovery, a second question is equally important:

> Can a Boolean × X construction induce a new construction rule, equivalence class, or non-dominated property trade-off that is not recovered by a matched Boolean-only search?

## Research series

| Series | Programme |
| --- | --- |
| BL-0 | Boolean foundations, exact semantics, packed state and circuit IR |
| BL-1 | Pure Boolean feed-forward learning/inference |
| BL-2 | Recurrent Boolean state, memory and cellular dynamics |
| BL-3 | Discrete learning/search without floating relaxation: SAT/MaxSAT/CEGIS/evolutionary search |
| BL-4 | Boolean attention, associative memory and KV policies |
| BL-5 | Boolean × real/complex/quaternion systems |
| BL-6 | Boolean × octonion/sedenion/Cayley-Dickson systems |
| BL-7 | Boolean × finite-field and coding-theory systems |
| BL-8 | Boolean × graph/semiring/tropical systems |
| BL-9 | Boolean × field/dynamical-system control |
| BL-10 | Noisy/stochastic Boolean systems and perturbation response |
| BL-11 | Automatic synthesis of cognitive/logical circuits |
| BL-12 | CPU SIMD / GPU / FPGA execution and measured resource comparison |
| **BL-13** | **Boolean-function discovery: exact metrics, Boolean-only baselines, Boolean × X generators, equivalence screening and novelty candidates** |

## BL-13 discovery pipeline

```text
GENERATE
  -> EXACT TRUTH TABLE
  -> ANF / WALSH METRICS
  -> EXACT DEDUPLICATION
  -> DECLARED EQUIVALENCE SCREENING
  -> PARETO FILTERING
  -> MATCHED BOOLEAN-ONLY COMPARISON
  -> PRIOR-ART REVIEW
  -> NOVELTY CANDIDATE
```

BooleanLab never treats an unseen truth table as a discovery by itself. For fixed `n`, all scalar Boolean functions already exist as mathematical objects. Scientifically meaningful novelty requires at least one declared target such as a new construction, a new parametric family, a new equivalence class under a specified relation, or a new verified trade-off among properties.

## Evidence rules

- No novelty claim without prior-art review.
- No performance or energy claim without measured hardware evidence.
- Boolean-only, X-only and Boolean × X baselines must be matched as closely as the hypothesis permits.
- Exact results, numerical evidence, conjectures and engineering observations are labelled separately.
- Stable fingerprints are indexes only; equality is confirmed from exact truth tables.
- Sedenion experiments account explicitly for non-associativity and zero divisors; algebraic identities are never imported from associative algebras without proof.
- Failed, equivalent, inconclusive and negative experiments are retained.
- `NEW` is not an automatic experiment status. A candidate must pass explicit equivalence and prior-art gates first.

## Architecture and ecosystem rule

BooleanLab reuses Memorithm foundations rather than silently copying them:

- **SciRust** supplies canonical mathematical primitives and exact Boolean analysis.
- **BooleanLab** owns experiments, candidate provenance, controlled generators and evidence.
- **Forge** may later search Boolean topology and Boolean × X operator choices under frozen evaluation contracts.
- **ProofLab / SciRust-Verify** are natural promotion targets for equivalence and invariant proofs.
- **TDI, ADA, FLAT-ATTENTION, KVLab, NNIS, ElasticXxx and other Memorithm projects** may consume only promoted results with an explicit integration contract.

The stable BooleanLab core remains on stable Rust. Experiments requiring SciRust `portable-simd`, including the current sedenion control, are isolated behind an optional feature and a separately pinned nightly CI job.

## Immediate research direction

The Boolean-only reference stage is now validated. The active critical path is:

```text
BL-13.1.1 exact four-input reference-space calibration      [VALIDATED]
        -> BL-13.1.2 bounded eight-input Boolean population [VALIDATED]
        -> BL-13.2.1 sedenion-vs-Boolean bounded screening
        -> stronger declared equivalence screening
        -> construction-cost matching / synthesis
        -> cross-domain Boolean-function discovery
```

The first bounded comparison already demonstrates why the next gates matter: the Boolean-only population reached nonlinearity 96, while some sedenion-induced control functions reached 98–100 and the known `GF(2^8)` inversion components reached 112. These observations do **not** establish novelty or superiority. The search population is bounded, equivalence screening is still limited, and construction costs are not yet matched.

## License

PolyForm Noncommercial License 1.0.0. See [`LICENSE.md`](LICENSE.md).

Copyright 2026 Tarek Zekriti.
