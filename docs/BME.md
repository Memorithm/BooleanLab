# Boolean Matrix Equations (BME)

Status: canonical baseline infrastructure. No speedup or novelty claim.

BooleanLab treats Boolean Matrix Equations as the shared abstraction

```text
C_ij = R_k(Phi(A_ik, B_kj))
```

for experiments later consumed by KVLab, FLAT-ATTENTION, SciRust and hardware qualification layers.

## Canonical baselines

The four required references are implemented in `booleanlab-core` before any searched equation is evaluated:

1. OR reduction over pairwise AND (Boolean semiring product);
2. XOR reduction over pairwise AND (GF(2) product);
3. XNOR followed by exact popcount;
4. XNOR followed by exact popcount and a declared threshold.

`CanonicalBmeEquation` is the typed dispatch surface. Malformed dimensions and invalid thresholds fail closed.

## Logical cost accounting

`BmeLogicalCost` records exact abstract operation counts implied by matrix shape for the canonical semantics. For an `m × k` left matrix and `k × n` right matrix, each baseline evaluates `m*n*k` pair operations. The reference reduction count records `m*n*(k-1)` abstract combines when `k > 1`; it does not assume a hardware reduction topology. Thresholded popcount adds one declared comparison per output cell.

These are semantic operation counts only. They are not CPU instructions, SIMD instructions, GPU operations, memory transactions, bandwidth, latency, energy, throughput or end-to-end performance measurements.

## Promotion rule

A future searched `Phi/R` pair must be compared against these baselines for exact semantics and declared logical cost before hardware work. Any performance statement requires a reproducible backend benchmark on an exact commit and must report relevant systems costs rather than infer speed from logical gate count.

Reusable stabilized BME primitives should be promoted to SciRust only after BooleanLab has qualified their semantics. Forge may synthesize candidates under a frozen evaluation contract; FLAT-ATTENTION and KVLab own attention/cache usage evidence; NNIS may provide hardware-specific qualification without making CUDA a dependency of the portable core.
