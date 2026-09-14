# BL-14 — Exact numerical sparse-layer integration calibration

Evidence class: **EXACT NUMERICAL CALIBRATION**, not a trained-model, HOLDOUT or hardware result.

## Reproduce

```bash
cargo test --locked -p booleanlab-discovery --bin bl14_linear_calibration
cargo run --locked -p booleanlab-discovery --bin bl14_linear_calibration --release
```

The binary accepts no arguments, reads no external dataset, and emits commented TSV (`bl14.linear-calibration.v1`). CI runs its tests and the report. Source code owns the complete fixed fixture.

## Preserved numerical operator

The experiment evaluates the same fixed, untrained integer linear layer `y = W x`, with row-major coefficients:

```text
 8 -1  2 -5
 3 -7  6 -4
-6  2 -8  1
-3  5  4 -7
```

The input batch contains all 16 vectors in `{-1,+1}^4`. A candidate produces `y_masked = (W * mask) x`; operationally, the mask is tested **before** a multiplication executes. This does not replace the layer with Boolean computation.

Inputs/weights use i64, products and accumulation use i128, and squared-error sums use checked u128. Individual i64 products fit in i128, but accumulation and squared-error overflow are explicitly rejected. Shape errors and unmatched reference batches also fail closed.

## Matched controls and search

Dense is the separate 16/16 reference. Every sparse baseline and eligible Boolean proposal retains exactly 8/16 coefficients:

- global absolute-magnitude pruning, with exact unsigned magnitude keys and original-index tie breaking;
- contiguous per-row 2:4 absolute-magnitude pruning;
- four deterministic random masks, with seeds 0, 1, 2 and 3;
- all admissible functions from exhaustive three-predicate Boolean search.

The fixed predicate schema is, in least-significant-bit order: `abs(weight) >= 5`, `weight < 0`, and `row_index is odd`. No predicate receives reference outputs, targets, reconstruction errors or future input. The threshold is a fixed fixture constant, not a fitted parameter.

Search examines all 256 truth tables, materializes each through the existing `BooleanFunction -> ExactMask` adapter, and evaluates the 34 tables meeting exact retained cardinality. All minimum-error ties are retained. This is calibration/SEARCH-only; no claim of generalization is made.

## Independent oracle and null control

The reported error is the exact squared reconstruction-error **sum**, with output count reported separately (64 scalar outputs). It is not language-model or downstream task quality.

For this complete sign-vector batch, the input columns are orthogonal. Consequently, for any fixed scalar mask:

```text
sum_x || W x - (W * mask) x ||^2
    = 16 * sum_(dropped coefficients w) w^2.
```

Tests compare executed sparse outputs with this independent identity for every eligible Boolean rule. Therefore magnitude selection is an optimal fixed-cardinality scalar mask for THIS fixture; no Boolean superiority is expected or asserted.

The unique best truth table in the declared three-predicate population is code 170 (`0xaa`), namely the first predicate `abs(weight) >= 5`. It obtains error sum 960, the same mask and error as both magnitude controls. This equality is a useful integration check, not a new pruning result.

The sparse kernel executes 128 coefficient multiplications over the batch instead of the dense reference's 256. It also performs 256 mask tests. These are exact reference-operation counts, not a twofold speedup. Static mask construction, Boolean search, predicate extraction, allocations, memory traffic and physical timing are excluded and disclosed in the report. No GPU N:M acceleration is implied by a 2:4 mask in this scalar Rust reference.

## Regression scope

Tests cover all 16 masks of an independent 2x2 layer against a masked-weight dense oracle, all-keep/all-drop controls, exact multiplication/mask-test accounting, rejected-overflow accumulation, malformed shapes and batch errors, all exact-density Boolean proposals, magnitude equality, and the sign-cube error identity.

## Remaining evidence

A real trained-model workload, leakage-safe train/validation/test protocol, learned or searched predicate families, controller and memory overhead, matched structured execution and measured device latency remain separate work. TDI-9.3 can reuse the testing discipline and later qualified generic contracts, not these reconstruction scores as adaptive-inference evidence.
