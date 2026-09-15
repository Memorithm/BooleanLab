# BL-13 vectorial Boolean metrics

Status: **exact bounded reference metrics; no novelty or security claim**.

BL-13 treats scalar maps `F_2^n -> F_2` and vectorial maps `F_2^n -> F_2^m` as distinct objects. The vectorial path now has deterministic small-domain references for differential uniformity, component-Walsh nonlinearity, component algebraic degree and the complete differential spectrum over non-zero input differences.

## Differential spectrum contract

For a declared truth table `F`, BooleanLab builds the difference-distribution row for every non-zero input difference `a`:

```text
DDT[a,b] = |{ x : F(x) xor F(x xor a) = b }|.
```

The exact differential spectrum is the histogram

```text
S[c] = |{ (a,b) : a != 0 and DDT[a,b] = c }|.
```

The implementation checks a conservative work bound before scratch allocation. It then performs deterministic histogram resets, exact difference evaluations and complete scans. Work-limit exhaustion, malformed dimensions, out-of-range outputs, arithmetic overflow and allocation failure are explicit non-results.

Useful invariants are checked in regression tests: `sum_c S[c] = (2^n - 1) 2^m` and `sum_c c S[c] = (2^n - 1) 2^n`. The largest `c` with a non-zero bucket agrees with differential uniformity.

The PRESENT 4-bit S-box is used only as a regression reference. Under the declared table, the exact spectrum has 144 zero-count cells, 72 cells of count 2 and 24 cells of count 4; this is a test vector, not a BooleanLab discovery.

## Boundaries

These metrics do not establish EA or CCZ equivalence, cryptographic suitability, prior-art status, novelty, mechanism, hardware cost or runtime performance. A truth table not previously seen by this repository is not a discovery. Any later EA/CCZ screening must state the exact declared equivalence relation, retain witnesses/provenance, respect explicit resource bounds and preserve `UNKNOWN`/non-result states on exhaustion.

Promotion to SciRust should happen only after the vectorial API and semantics stabilize. Forge may later consume the metrics for search/synthesis, but candidate ranking must not be presented as novelty evidence; ProofLab/SciRust-Verify remains the place for formal equivalence/proof obligations where applicable.
