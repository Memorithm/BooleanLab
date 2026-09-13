# BL-4 Boolean Attention Control Plane

BL-4 tests whether a cheap Boolean control plane can reject attention work before exact numerical attention is executed. This document freezes the first portable protocol boundary. It does not claim an acceleration result.

## Systems hypothesis

The primary systems hypothesis is:

```text
T_boolean_front_end + T_exact_survivors < T_exact_dense
```

subject to a declared downstream quality/recall constraint. Operation-count reductions alone are not evidence of end-to-end acceleration.

## BL-4.4.1 — block admission

Question: under exactly matched retained density, can a Boolean admission rule preserve a frozen dense-attention target set better than structural and deterministic random masks?

Required controls:

1. dense oracle;
2. structural mask at matched retained cardinality;
3. deterministic random mask at matched retained cardinality;
4. Boolean admission mask;
5. identical query/key inputs and oracle criterion across all masks.

Record exact counts before any ratio is reported:

```text
total_pairs
admitted_pairs
reference_positive_pairs
true_positive_pairs
false_positive_pairs
false_negative_pairs
```

A lower retained density is not a positive result by itself.

## BL-4.4.2 — bit-packed Q/K signatures

The first portable family uses canonical bit-packed signatures and exact Hamming distance:

```text
d(q, k) = popcount(q XOR k)
admit(q, k; tau) = d(q, k) <= tau
```

The implementation must fail closed on:

- empty signatures;
- different signature widths;
- malformed packed storage or non-zero padding bits;
- thresholds larger than the declared signature width;
- candidate/reference masks with incompatible lengths.

The threshold sweep must be preregistered for each signature width. Do not tune `tau` on the final holdout.

For every threshold, retain the exact integer evidence needed to reconstruct:

- candidate density;
- oracle recall;
- false-negative rate;
- false-positive rate;
- candidate count presented to exact attention.

Floating-point ratios are reporting conveniences only; the integer counts are authoritative.

## Signature families

Each tested signature family must have an explicit construction identifier and immutable parameters. Initial admissible families are:

- sign bits from a frozen projection;
- threshold bits from frozen per-dimension thresholds;
- grouped parity/XOR predicates;
- Boolean functions promoted from BooleanLab only after their construction is frozen.

A signature family learned from data must state train/calibration/validation/holdout separation. No holdout-derived threshold or projection may leak into the candidate rule.

## BL-4.5.1 — KV-page admission

After pair/block admission is qualified, aggregate signatures at the declared KV page granularity. The Boolean plane may reject an entire numerical KV page only when the page rule is available before the corresponding numerical staging/read that the experiment claims to avoid.

Measure separately:

```text
Boolean metadata bytes read
numerical KV bytes avoided
pages admitted / rejected
first-token routing latency after prefill
steady-state decode routing latency
```

The ratio `numerical_KV_bytes_avoided / Boolean_metadata_bytes_read` is evidence to record, never an assumed benefit.

## BL-4.6.1 — systems qualification

FLAT-ATTENTION owns end-to-end execution evidence. Compare at minimum:

1. dense exact FLAT;
2. structural mask + exact FLAT;
3. deterministic random mask + exact FLAT at matched density;
4. Boolean prefilter + exact FLAT;
5. page/KV Boolean routing + exact FLAT when applicable.

Required measurements include Boolean front-end time, exact-survivor time, dispatch/synchronization overhead, retained density, K/V bytes avoided, metadata bytes, prefill latency, first-token latency, steady decode latency, tokens/s, and the frozen downstream quality/oracle metrics.

No speedup claim is accepted unless measured on declared hardware at an exact repository SHA.

## BL-4.7.1 — one-bit QK gate

One-bit QK is a later experiment, not a prerequisite. It starts only after Boolean-prefilter-plus-exact-QK has been qualified. Compare it against exact QK, an appropriate low-precision QK baseline, and Boolean-prefilter-plus-exact-QK under matched workloads.

## Current code boundary

`booleanlab-core::attention` owns portable, deterministic correctness primitives:

- canonical packed signatures;
- XOR + POPCOUNT Hamming distance;
- threshold admission;
- ordered candidate-mask construction;
- exact candidate/oracle confusion counts.

BooleanLab does not claim that these CPU-portable primitives are the eventual production kernel. Hardware-specific placement and timing belong to the consuming runtime after the scientific rule is qualified.
