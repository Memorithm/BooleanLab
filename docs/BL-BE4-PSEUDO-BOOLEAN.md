# BL-BE4 — exact bounded pseudo-Boolean constraints

Status: **experimental exact oracle; small-domain qualification only**.

BL-BE4 starts the pseudo-Boolean phase of the Boolean-elasticity programme with a deliberately small reference surface. BooleanLab remains the experimental/differential-oracle owner; this contract does not grant ElasticXxx runtime or actuation authority.

The core abstraction evaluates a declared weighted predicate

```text
sum_i w_i x_i  R  threshold
```

where `x_i` is Boolean, `w_i` and `threshold` are non-negative integers, and `R` is `>=`, `<=`, or `=`. Cardinality constraints are the special case `w_i = 1`.

The exact-oracle gate provides:

- exact `u128` accumulation of selected `u64` weights so the mathematical sum cannot silently wrap at `u64`;
- checked positive integer rescaling of all coefficients and the threshold;
- explicit rejection when any rescaled coefficient or threshold is not representable;
- exhaustive truth-table equivalence for at most 63 variables, with an explicit assignment budget checked before enumeration;
- exact primitive normalization by the greatest common divisor shared by every coefficient and the threshold;
- a returned normalization factor that can reconstruct the original representation by checked exact scaling;
- exhaustive logical implication over a common declared Boolean domain, with the complete work bound checked before evaluation;
- the first implication counterexample in canonical ascending assignment-mask order when entailment fails;
- explicit non-results for work-bound exhaustion, arity mismatch, invalid assignment shape, zero scale, and scaling overflow.

Positive integer rescaling is expected to preserve the Boolean truth table only when every scaled integer remains representable. Primitive normalization performs only the inverse case in which every integer is exactly divisible by a shared positive factor; it does not round or tighten thresholds. Fully zero constraints and already coprime forms remain unchanged with normalization factor `1`.

The exhaustive oracle is the reference check for small domains; an overflow or work-limit failure is never interpreted as equivalence, non-equivalence, entailment, or non-entailment evidence. Common-factor normalization is a representation canonicalization aid, not by itself a complete pseudo-Boolean equivalence classifier: distinct primitive representations can still encode the same Boolean predicate. Exact implication is likewise only a bounded reference relation: it does not authorize rule removal in ElasticXxx and it is not a substitute for a resource-bounded SAT/PB backend.

This programme does **not** yet provide a SAT/PB optimizer, LP/MIP backend, production guard representation, learned sparsity policy, performance benchmark, novelty claim, or cross-repository promotion. Any later backend must be differentially checked against the exact oracle and retain explicit resource limits.
