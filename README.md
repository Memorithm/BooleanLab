# BooleanLab

BooleanLab is the Memorithm research bench for **pure Boolean AI** and **Boolean × mathematical-domain hybrid systems**.

The bench studies two distinct programmes:

1. **Pure Boolean systems** — useful state, transitions, inference and execution are represented in `{0,1}` and composed from discrete logical operators.
2. **Boolean × X systems** — a Boolean control/state plane interacts with a richer mathematical domain `X` such as real or complex numbers, finite fields, quaternions, octonions, sedenions, tensors, graphs, semirings or dynamical systems.

BooleanLab is a research bench, not a production inference runtime. Scientific claims require reproducible evidence and must remain narrower than the experiments that support them.

## Core hybrid contract

The generic research object is

```text
b(t+1) = F_bool(b(t), P_X(x(t)), u(t))
x(t+1) = G_{b(t+1)}(x(t), u(t))
```

where `b ∈ {0,1}^m`, `x ∈ X^n`, `P_X` extracts explicit predicates from the mathematical domain, and the Boolean state may select, mask, route, constrain or verify operations in `X`.

The research question is not whether Boolean × X is automatically superior. The question is:

> For which domains X, tasks and resource envelopes does coupling an explicit Boolean state/control plane to X provide a measurable capability, efficiency, verifiability or interpretability advantage over matched Boolean-only and X-only baselines?

## Research series

| Series | Programme |
| --- | --- |
| BL-0 | Boolean foundations, exact semantics, bit-packing, ANF/Walsh bridges and circuit IR |
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

## Ecosystem rule

BooleanLab must reuse and extend Memorithm foundations instead of copying them silently. In particular, SciRust is the canonical mathematical/tooling layer; BooleanLab is the experimental layer. Promotion of a result into SciRust, TDI, ADA, FLAT-ATTENTION, KVLab, NNIS, ElasticXxx or another project requires an explicit evidence and integration contract.

## Scientific discipline

- No novelty claim without prior-art review.
- No performance claim without measured hardware evidence.
- Boolean-only, X-only and Boolean × X baselines must be matched as closely as the hypothesis permits.
- Exact results, numerical evidence, conjectures and engineering observations must be labelled separately.
- Sedenion experiments must account explicitly for non-associativity and zero divisors; algebraic identities must never be imported from associative algebras without proof.
- Failed and negative experiments are retained.

## Initial engineering target

BL-0 begins with a small reusable Rust core:

- packed Boolean state;
- typed Boolean circuit IR;
- deterministic evaluator;
- exact truth-table validation for bounded widths;
- `PredicateBridge<X>` and `HybridOperator<X>` contracts;
- a first Boolean × sedenion masked-component experiment backed by SciRust semantics rather than a duplicate hypercomplex implementation.

## License

PolyForm Noncommercial License 1.0.0. See `LICENSE.md` once the bootstrap PR lands.

Copyright 2026 Tarek Zekriti.
