# BL-14.1.2 — Trained linear development pilot

Status: IMPLEMENTATION IN PROGRESS. This is a small synthetic development pilot,
not a language-model benchmark, final confirmation or hardware-speed result.

## Fixed pilot design

Preserve one eight-input, one-output numerical linear regressor. Fit its eight
binary64 coefficients from zero by 256 full-batch gradient steps, with learning
rate 1/8 and mean-squared-error gradient. Fit on 128 TRAIN examples only. Then
freeze the learned coefficients: no sparse retraining or validation feedback.

Use 64 SEARCH examples to screen static Boolean masks. Construct 64 VALIDATION
examples only after the selection object exists. These are non-final development
partitions, not an untouched confirmatory holdout. All generator settings are
public, fixed and reproducible; no result-conditioned seed retry is permitted.

Input coordinates come from the existing SplitMix64-style deterministic key
stream with seed 0xB1140012, using disjoint index ranges for the three partitions.
Quantize each key modulo 17 to an integer in -8..=8, divide by 8, then apply fixed
feature scales `[1, 2, 1, 0.5, 2, 0.25, 1, 0.5]`. Regression targets use teacher
coefficients `[3, -1, 2, -4, 0.5, 1, -2, 1.5]`. There is no target noise in v1.
Teacher coefficients are available only to the generator/scorer, never to mask
predicate extraction. This easy realizable synthetic task is deliberately not
claimed representative of trained neural networks.

## Controls and Boolean predicates

Retain exactly 4/8 coefficients in every sparse control:

- global learned-weight magnitude, with ascending-index tie breaking;
- per-contiguous-group 2:4 learned-weight magnitude;
- activation-energy saliency `weight^2 * mean_TRAIN(input^2)`;
- four fixed deterministic random masks, with seeds 0, 1, 2 and 3;
- every three-predicate Boolean truth table meeting the same exact cardinality.

Dense remains a separate 8/8 reference. The Boolean/global controls are not
required to satisfy per-group 2:4: equal global density is not equal hardware
structure and no 2:4 hardware equivalence is claimed.

The coefficient-level predicates are frozen from TRAIN/model metadata:

`p0 = coefficient is in the top four absolute learned weights`

`p1 = learned coefficient is negative`

`p2 = input feature is in the top four TRAIN mean-square energies`

Both rankings use explicit ascending-index ties. No SEARCH or VALIDATION labels
or statistics enter these predicates. Enumerate all 256 three-input truth tables
with the existing exhaustive-rule generator and materialize masks with the
existing exact function-to-mask adapter. At least the `p0` function is eligible.

Choose every rule with the smallest SEARCH task MSE among exact-density members;
retain exact floating-point ties, even when different rules induce the same mask.
This is a declared, single-quality-objective selection at fixed cardinality, NOT
a complete Pareto frontier over unmeasured hardware costs. Learned weights,
resolved predicate rows, exact truth tables, masks and selected identities are
owned by an immutable selection object before VALIDATION. Validation does not
re-rank or retrain that object.

## Evidence

Report TRAIN task MSE before and after fitting, exact learned-weight bit patterns,
resolved predicate bits, all eligible SEARCH rows, selected truth-table codes,
and VALIDATION task MSE plus reconstruction MSE against the frozen dense model.
Report example count, retained cardinality, executed coefficient multiplications
and mask tests separately. Non-finite input, intermediate arithmetic or metrics,
shape mismatch and counter overflow fail explicitly. Rejected coefficients must
be skipped before multiplication.

Floating-point losses are numerical evidence, not exact mathematical identities.
Counts are reference-operation accounting, not elapsed time or ISA instructions.
Training, search, ranking, predicate construction, allocation, metadata and
memory traffic are not included in inference operation counters. No latency,
energy, memory-reduction, novelty or universal sparsity-superiority claim is
licensed by this pilot.

## Reproduce

```bash
cargo test --locked -p booleanlab-discovery --bin bl14_trained_linear
cargo run --locked -p booleanlab-discovery --bin bl14_trained_linear --release
```

## Next gates and reuse

Preserve the result, including an equal or worse Boolean result. Then expand to
multiple preregistered teacher/input regimes and seeds, a nonlinear trained
model, explicit controller overhead and declared hardware measurements before
any promotion. TDI-9.3 can reuse the partition/freeze/validation discipline, not
these synthetic regression labels or a claim of dynamic-inference improvement.
Mathematically general adapters may move to SciRust after separate qualification;
TDI-specific action semantics and final-evaluation gates remain in TDI.
