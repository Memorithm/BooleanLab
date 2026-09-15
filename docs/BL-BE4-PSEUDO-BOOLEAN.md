# BL-BE4 — exact bounded pseudo-Boolean constraints

Status: **experimental exact oracle; small-domain qualification only**.

BL-BE4 starts the pseudo-Boolean phase of the Boolean-elasticity programme with a deliberately small reference surface. BooleanLab remains the experimental/differential-oracle owner; this contract does not grant ElasticXxx runtime or actuation authority.

The core abstraction evaluates a declared weighted predicate

```text
sum_i w_i x_i  R  threshold
```

where `x_i` is Boolean, `w_i` and `threshold` are non-negative integers, and `R` is `>=`, `<=`, or `=`. Cardinality constraints are the special case `w_i = 1`.

The first gate provides:

- exact `u128` accumulation of selected `u64` weights so the mathematical sum cannot silently wrap at `u64`;
- checked positive integer rescaling of all coefficients and the threshold;
- explicit rejection when any rescaled coefficient or threshold is not representable;
- exhaustive truth-table equivalence for at most 63 variables, with an explicit assignment budget checked before enumeration;
- explicit non-results for work-bound exhaustion, arity mismatch, invalid assignment shape, zero scale, and scaling overflow.

Positive integer rescaling is expected to preserve the Boolean truth table only when every scaled integer remains representable. The exhaustive oracle is the reference check for small domains; an overflow or work-limit failure is never interpreted as equivalence or non-equivalence evidence.

This increment does **not** provide a SAT/PB optimizer, LP/MIP backend, production guard representation, learned sparsity policy, performance benchmark, novelty claim, or cross-repository promotion. Any later backend must be differentially checked against the exact oracle and retain explicit resource limits.
