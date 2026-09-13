# BL-14 — Boolean Sparsity Control

Status: **PROPOSED / ACTIVE DESIGN**

## Scope

BL-14 studies whether explicit Boolean equations can improve sparsity decisions inside an existing numerical model or computational graph.

This series does **not** propose replacing a complete model with Boolean logic. The Boolean layer is a control mechanism that decides which existing weights, channels, blocks, heads, edges, experts, activations or other declared groups remain active.

For a controlled group `g`, the minimal contract is

```text
p_g      = P_g(state, statistics, input, structure)
z_g      = F_bool(p_g, context)
y_g      = z_g * G_g(x)
```

with `z_g in {0,1}`. `F_bool` may be hand-specified for calibration or synthesized/learned under a frozen evaluation contract.

The research question is not "can Boolean masks create zeros?". It is:

> Can a compact Boolean decision rule produce a better verified sparsity/quality/resource trade-off than matched non-Boolean pruning or routing baselines, after the Boolean-controller overhead is included?

## Required comparisons

Every BL-14 result must report matched comparisons where applicable:

- dense or fully active baseline;
- density-matched random mask;
- magnitude-based sparsification;
- a declared structured baseline such as block or N:M sparsity when the experiment uses comparable structure;
- Boolean controller with the same retained-density budget;
- optional learned continuous gate only when its semantics and training budget are matched.

A Boolean method is not considered better solely because it creates more zeros.

## Metrics

At minimum, experiments must record:

- retained density and exact sparsity;
- task-quality delta against the dense reference;
- controller equation size, literal count, depth and state footprint;
- mask computation time;
- effective compute skipped;
- bytes read/written or traffic avoided where measurable;
- end-to-end latency or throughput on the declared hardware when a performance claim is made;
- stability of the selected sparse structure across seeds/inputs where applicable.

Dynamic experiments must additionally report mask churn and decision overhead per input or token. Relational experiments must report the declared redundancy relation and how ties are resolved.

## Boolean observation boundary

Boolean inputs must be explicit predicates rather than hidden floating-point decisions. Example predicate families include:

```text
magnitude_high(g)
activity_recent(g)
sensitivity_high(g)
redundancy_low(g)
structure_required(g)
budget_available(g)
input_relevant(g, x)
```

A controller may then synthesize rules such as

```text
keep(g) =
    (magnitude_high(g) AND activity_recent(g))
    OR (sensitivity_high(g) AND NOT redundancy_high(g))
    OR structure_required(g)
```

The exact predicates, thresholds, quantizers or categorical boundaries that create Boolean inputs are part of the experimental definition and must be frozen before evaluation on a holdout set.

## Experiment campaign

### BL-14.0.1 — Sparsity calibration

Question: does the BL-14 harness reproduce declared dense, random, magnitude and structured-mask baselines at exactly matched retained densities?

Purpose: validate accounting before any Boolean superiority claim.

Acceptance gate:

- exact mask cardinality is reproducible;
- quality and resource metrics are collected through the same path for every baseline;
- no Boolean search is performed on holdout evidence.

### BL-14.1.1 — Static Boolean sparsity

Question: can a Boolean equation synthesize a frozen mask that preserves more task quality than density-matched random and magnitude baselines under the same retained-density budget?

The controller is evaluated during selection, then the resulting mask is frozen for final measurement.

### BL-14.2.1 — Structured Boolean sparsity

Question: can Boolean equations choose blocks, channels, heads or another declared hardware-relevant group while preserving quality better than matched structured baselines?

The experiment must distinguish nominal zero count from actually skippable structured work.

### BL-14.3.1 — Dynamic Boolean sparsity

Question: can an input-conditioned Boolean controller change the active sparse structure cheaply enough that avoided work exceeds decision overhead while preserving the declared quality threshold?

Minimal form:

```text
z_g(x) = F_bool(P_g(x), local_state, budget_state)
```

The result must report Boolean-controller time separately from downstream compute time.

### BL-14.4.1 — Relational Boolean sparsity

Question: can explicit Boolean relations identify functionally redundant groups more effectively than independent per-group scores at the same retained density?

Example form:

```text
keep(i) = important(i)
          AND NOT exists(j): redundant(i, j) AND preferred(j, i)
```

The redundancy predicate must be operationally defined and evaluated against independent evidence.

### BL-14.5.1 — Boolean sparsity-rule synthesis

Question: can SAT/MaxSAT, CEGIS, Forge-style search or another declared discrete search method find a compact Boolean controller on a Pareto frontier of quality, retained density, rule complexity and measured resource cost?

Search evidence and final holdout evidence must remain separated.

## Evaluation objective

BL-14 is explicitly multi-objective. A candidate should retain the vector rather than collapse it prematurely:

```text
J = (
    quality_loss,
    retained_density,
    controller_cost,
    effective_compute,
    memory_traffic,
    latency,
    rule_complexity
)
```

A scalar objective may be used by a search algorithm only if its weights are preregistered. Scientific reporting must preserve the underlying components.

## Promotion path

- **BooleanLab** owns Boolean predicates, controller synthesis, matched experimental evidence and rule provenance.
- **SciRust** is the promotion target for reusable Boolean analysis, mask accounting and generic sparse-control primitives.
- **Forge** may search Boolean equations once the BL-14 evaluation contract is frozen.
- **FLAT-ATTENTION** may consume BL-14 results specifically for attention-related structured or dynamic sparsity, while BL-4 remains the attention-control research series.
- **NNIS** may consume qualified masks/controllers for native GPU execution studies.
- Other Memorithm projects should consume BL-14 results only through an explicit integration contract.

## Claim discipline

BL-14 does not assume that Boolean control is faster than numerical scoring, that greater sparsity yields lower latency, or that a discovered rule generalizes outside its declared workload. These are empirical questions.

A positive BL-14 result requires a matched baseline, an explicit Boolean observation boundary, reproducible mask accounting, holdout evaluation and—when performance is claimed—measured end-to-end hardware evidence including the controller overhead.
