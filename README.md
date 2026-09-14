# BooleanLab

BooleanLab is the Memorithm research bench for **pure Boolean AI**, **Boolean-function discovery**, **Boolean sparsity control**, and **Boolean × mathematical-domain hybrid systems**.

The bench studies four connected programmes:

1. **Pure Boolean systems** — state, transitions, inference and execution represented in `{0,1}` and composed from discrete logical operators.
2. **Boolean × X systems** — a Boolean control/state plane interacting with a richer mathematical domain `X`: finite fields, real/complex systems, quaternions, octonions, sedenions, tensors, graphs, semirings, dynamical systems and other mathematical structures.
3. **Boolean-function discovery** — systematic generation, exact characterization, deduplication and equivalence screening of Boolean functions induced either by Boolean-only search or by Boolean × X constructions.
4. **Boolean sparsity control** — explicit Boolean equations used as a control plane for deciding which parts of an existing numerical model or computational graph remain active, without replacing the complete model with Boolean logic.

BooleanLab is a research bench, not a production inference runtime. Scientific claims require reproducible evidence and must remain narrower than the experiments that support them.

## Current research status

| Experiment / track | Status | Verified result or current question |
| --- | --- | --- |
| **BL-4 Boolean Attention Control Plane** | ACTIVE / PROPOSED | Determine whether early bitpacked Boolean routing can eliminate enough exact attention work and K/V traffic to improve real FLAT-ATTENTION execution while preserving declared quality. |
| **BL-14 Boolean Sparsity Control** | CALIBRATION IMPLEMENTED / MODEL STUDY PENDING | Exact mask primitives, exhaustive/CEGIS search comparison and an executable numerical linear-layer calibration are available. Trained-model quality and end-to-end hardware benefit remain unestablished. |
| BL-13.0.1 | VALIDATED | Exact Boolean screening reproduces frozen reference properties using SciRust ANF/Walsh metrics. |
| BL-13.1.1 | VALIDATED | Exhaustive scan of all 65,536 four-input functions: 12,870 balanced, 896 bent, 222 resilient under the declared criterion, 1,152 three-valued plateaued under the preregistered operational definition; maximum nonlinearity 6. |
| BL-13.1.2 | VALIDATED | From 4,096 deterministic eight-input circuits (4–24 gates), 1,685 exact unique functions were observed, including 473 balanced functions; best observed nonlinearity 96 and 18 Pareto-front members. |
| BL-13.2.0 | VALIDATED | A bounded SciRust sedenion control produced 16 distinct scalar functions `F_2^8 -> F_2`; observed algebraic degrees 6–8 and nonlinearities 76–100. No novelty claim. |
| BL-13.2.1 | EQUIVALENCE_SCREENED | Against the full 1,685-function BL-13.1.2 population, all 16 sedenion coordinates had 0 exact, 0 output-complement, 0 input-permutation and 0 input-permutation-plus-output-complement matches. Exact necessary invariants also excluded the declared affine relation `g(x)=f(Ax+b) XOR c` against every corpus member. EA/CCZ, matched construction cost and prior-art review remain pending. No novelty claim. |
| BL-13.3.1 | VALIDATED | Two `GF(2^8)` inversion constructions each produced eight distinct component functions with degree 7, nonlinearity 112 and balanced outputs. Pipeline validation only; no novelty claim. |

The machine-readable experiment registry is [`experiments/REGISTRY.tsv`](experiments/REGISTRY.tsv). Reproducible evidence is retained under [`experiments/results/`](experiments/results/), including the frozen [`BL-13.1 Boolean baseline`](experiments/results/BL-13.1-BOOLEAN-BASELINE.md) and the bounded [`BL-13.2.1 sedenion baseline screen`](experiments/results/BL-13.2.1-SEDENION-BASELINE-SCREEN.md). The BL-14 protocol is defined in [`experiments/BL-14-BOOLEAN-SPARSITY-CONTROL.md`](experiments/BL-14-BOOLEAN-SPARSITY-CONTROL.md).

## Priority programme: Boolean Attention Control Plane

The attention programme now treats Boolean logic as a potential **early control plane** rather than merely as a compressed representation.

The central systems hypothesis is:

```text
T_boolean_front_end + T_FLAT_survivors < T_FLAT_dense
```

The first target is not full 1-bit attention. The first target is to use compact Boolean decisions to decide which blocks or KV pages deserve exact numerical attention.

```text
Q/K state
  -> bitpacked signatures / Boolean predicates
  -> block or page admission
       reject -> avoid exact Q·K and, where architecture permits, avoid K/V staging/read
       accept -> execute exact FLAT attention
```

BooleanLab owns the scientific function/policy search and controlled evidence. **FLAT-ATTENTION** owns GPU consumption, routing placement and end-to-end timing. **KVLab** owns cache/page-specific retention, selection and tiering experiments. Reusable packed-bit/math primitives should be promoted to **SciRust** rather than duplicated.

Current BL-4 additions:

- `BL-4.4.1`: Boolean block admission versus dense, structural and density-matched random masks.
- `BL-4.4.2`: bitpacked Q/K signature families and XOR/XNOR-popcount selection frontiers.
- `BL-4.5.1`: query-aware Boolean KV-page admission available from the first decode token after prefill.
- `BL-4.6.1`: measured Boolean-control overhead versus exact attention work and K/V traffic eliminated.
- `BL-4.7.1`: 1-bit QK only after the router path is qualified, compared against exact FLAT, low-precision QK and Boolean-prefilter-plus-exact-QK.

No acceleration claim is accepted from theoretical operation counts alone. Hardware evidence must include Boolean front-end time, retained density, K/V bytes avoided, metadata cost, first-token latency, steady-state decode, prefill, tokens/s and quality/false-negative metrics.

## Priority programme: Boolean Sparsity Control

BL-14 studies Boolean equations as a **sparsity control plane inside an existing model**, not as a proposal to replace the whole model with Boolean computation.

For a declared group `g`—for example a weight group, channel, block, head, edge, expert or activation group—the minimal contract is:

```text
p_g = P_g(state, statistics, input, structure)
z_g = F_bool(p_g, context)
y_g = z_g * G_g(x)
```

with `z_g in {0,1}`. The Boolean equation therefore decides whether existing numerical work is retained or skipped.

BL-14 starts with six controlled experiments:

- `BL-14.0.1`: calibrate dense, random, magnitude and structured sparsity baselines at exactly matched retained densities.
- `BL-14.1.1`: static Boolean sparsity with a frozen final mask.
- `BL-14.2.1`: structured Boolean sparsity over hardware-relevant groups such as blocks, channels or heads.
- `BL-14.3.1`: dynamic input-conditioned Boolean sparsity with controller overhead measured separately.
- `BL-14.4.1`: relational Boolean sparsity based on explicit redundancy relations rather than independent scalar scores alone.
- `BL-14.5.1`: discrete synthesis/search of compact Boolean sparsity rules under a frozen multi-objective evaluation contract.

The evaluation is explicitly multi-objective: task quality, retained density, Boolean-rule complexity, controller cost, effective compute, memory traffic and measured latency/throughput remain separate evidence. More zeros alone are not a positive result.

### Executable BL-14 calibrations

The [search-method comparison](experiments/results/BL-14.5-SEARCH-COMPARISON.md) compares CEGIS with a matched linear first-exact search, using full exhaustive fitting as an independent all-optima oracle. It retains negative work-count controls, incomplete-coverage ties and inconsistent-label failures.

The [numerical linear-layer calibration](experiments/results/BL-14-LINEAR-CALIBRATION.md) executes the same fixed untrained integer operator with dense, magnitude, 2:4, random and Boolean masks. All sparse controls retain exactly 8/16 coefficients. It checks reconstruction error against an independent exact identity and reports multiplication and mask-test counts separately. This is not trained-model or hardware-speed evidence.

```bash
cargo run --locked -p booleanlab-discovery --bin bl14_search_comparison --release
cargo run --locked -p booleanlab-discovery --bin bl14_linear_calibration --release
```

The notation `y_g = z_g * G_g(x)` specifies the output, not an automatic execution saving: a runner must reject a group **before** evaluating its numerical work. Multiplying an already-computed result by zero does not skip that work. The numerical calibration tests this early-dispatch boundary. TDI-9.3 owns the corresponding action-policy calibration and its separate observation/final-evaluation contracts.

## Core hybrid contract

The generic research object is

```text
b(t+1) = F_bool(b(t), P_X(x(t)), u(t))
x(t+1) = G_{b(t+1)}(x(t), u(t))
```

where `b ∈ {0,1}^m`, `x ∈ X^n`, `P_X` extracts explicit predicates from the mathematical domain, and the Boolean state may select, mask, route, constrain or verify operations in `X`.

### Canonical extended contract

The two-equation contract above remains the stable minimal notation. Experiments that require explicit observation, outputs, transition-sensitive dynamics, parameters, noise, nondeterminism or multi-objective evaluation use the following canonical extension:

```text
p(t)   = P_X(x(t))
b(t+1) = F_bool(b(t), p(t), u(t); theta_B)
x(t+1) = G(x(t), u(t), b(t), b(t+1); theta_X)
y(t)   = H_bool(b(t), p(t), x(t); theta_H)
J      = E(b(0:T), x(0:T), y(0:T), u(0:T))
```

with the typed maps, suppressing fixed parameters in the signatures,

```text
P_X    : X^n -> {0,1}^p
F_bool : {0,1}^m × {0,1}^p × U -> {0,1}^m
G      : X^n × U × {0,1}^m × {0,1}^m -> X^n
H_bool : {0,1}^m × {0,1}^p × X^n -> {0,1}^q
E      : trajectories -> R^k
```

`P_X` is the declared Boolean observation boundary between the mathematical domain and the Boolean plane. The update order is intentionally causal: predicates are extracted from `x(t)`, the Boolean decision `b(t+1)` is formed, and that newly formed decision may gate, mask, route, constrain, verify or otherwise select the update applied to `x`. Dependence of `G` on both `b(t)` and `b(t+1)` permits transition-sensitive behavior; the minimal `G_{b(t+1)}` notation is the special case in which the previous Boolean state is irrelevant to the `X` update.

`H_bool` makes the observable Boolean output explicit. This is the canonical boundary for truth-table extraction and Boolean-function characterization in BL-13. `E` is an evaluation map, not an assumption of scalar optimality: `J ∈ R^k` may retain several declared objectives such as correctness, nonlinearity, algebraic degree, latency, memory, traffic, energy or robustness without silently collapsing them into one score.

For noisy or stochastic experiments, disturbances must be explicit rather than hidden inside the deterministic notation:

```text
b(t+1) = F_bool(b(t), p(t), u(t), eta(t); theta_B)
x(t+1) = G(x(t), u(t), b(t), b(t+1), xi(t); theta_X)
```

where the experiment declares the domains and sampling law, adversarial rule or perturbation schedule for `eta(t)` and `xi(t)`. If an experiment genuinely admits several legal successors rather than a sampled deterministic successor, it may use set-valued transitions:

```text
b(t+1) in F_set(b(t), p(t), u(t))
x(t+1) in G_set(x(t), u(t), b(t), b(t+1))
```

The pure-Boolean specialization is obtained by removing `X` and its predicate boundary:

```text
b(t+1) = F_bool(b(t), u(t))
y(t)   = H_bool(b(t))
```

Every experiment must state which specialization of this contract it implements. Deterministic, stochastic, set-valued and learned/parameterized variants are not interchangeable, and claims must be limited to the declared semantics and measured evidence.

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
| **BL-4** | **Boolean attention control plane, associative memory and KV policies** |
| BL-5 | Boolean × real/complex/quaternion systems |
| BL-6 | Boolean × octonion/sedenion/Cayley-Dickson systems |
| BL-7 | Boolean × finite-field and coding-theory systems |
| BL-8 | Boolean × graph/semiring/tropical systems |
| BL-9 | Boolean × field/dynamical-system control |
| BL-10 | Noisy/stochastic Boolean systems and perturbation response |
| BL-11 | Automatic synthesis of cognitive/logical circuits |
| BL-12 | CPU SIMD / GPU / FPGA execution and measured resource comparison |
| **BL-13** | **Boolean-function discovery: exact metrics, Boolean-only baselines, Boolean × X generators, equivalence screening and novelty candidates** |
| **BL-14** | **Boolean sparsity control: static, structured, dynamic and relational sparsity plus discrete rule synthesis** |

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
- **FLAT-ATTENTION** consumes qualified Boolean attention policies and measures real attention-system behavior.
- **KVLab** owns cache/page-specific experiments and cache resource accounting.
- **TDI, ADA, NNIS, ElasticXxx and other Memorithm projects** may consume only promoted results with an explicit integration contract.

The stable BooleanLab core remains on stable Rust. Experiments requiring SciRust `portable-simd`, including the current sedenion control, are isolated behind an optional feature and a separately pinned nightly CI job.

## Immediate research direction

Three active tracks now run in parallel:

```text
BL-13 function discovery:
BL-13.1 validated Boolean baselines
  -> BL-13.2.1 exact/output-complement/input-permutation and affine-invariant exclusion completed
  -> EA/CCZ or another stronger declared equivalence + construction-cost matching
  -> cross-domain discovery

BL-4 Boolean Attention Control Plane:
BL-4.4 block-admission + bitpacked signature experiments
  -> BL-4.5 first-token KV-page routing
  -> BL-4.6 measured FLAT systems comparison
  -> BL-4.7 gated 1-bit QK research

BL-14 Boolean Sparsity Control:
BL-14.0 matched-baseline calibration
  -> BL-14.1 static Boolean masks
  -> BL-14.2 structured hardware-relevant sparsity
  -> BL-14.3 dynamic input-conditioned sparsity
  -> BL-14.4 relational redundancy-aware sparsity
  -> BL-14.5 discrete Boolean rule synthesis
```

The bounded BL-13 comparison already demonstrates why stronger gates matter: the Boolean-only population reached nonlinearity 96, while some sedenion-induced control functions reached 98–100 and the known `GF(2^8)` inversion components reached 112. The expanded declared screen found no match for the 16 tested sedenion coordinates in the bounded 1,685-function Boolean-only population under exact equality, output complement, any permutation of the eight input variables, or input permutation followed by output complement. The subsequent exact necessary-invariant screen excluded the declared affine relation `g(x)=f(Ax+b) XOR c` against every member of that bounded corpus for all 16 coordinates. These observations do **not** establish novelty or superiority. The search population is bounded, EA/CCZ and prior-art screening remain pending, and construction costs are not yet matched.

## License

PolyForm Noncommercial License 1.0.0. See [`LICENSE.md`](LICENSE.md).

Copyright 2026 Tarek Zekriti.
