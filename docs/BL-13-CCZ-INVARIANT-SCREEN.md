# BL-13 bounded CCZ necessary-invariant screen

Status: **exact bounded rejection screen; never an equivalence or novelty proof**.

For two declared vectorial Boolean maps `F,G : F_2^n -> F_2^m`, CCZ-equivalence preserves the differential spectrum and the extended absolute Walsh spectrum. BooleanLab therefore uses those already-implemented exact metrics only as necessary invariants:

```text
if differential_spectrum(F) != differential_spectrum(G):
    CCZ-equivalence is ruled out
else if absolute_extended_walsh_spectrum(F) != absolute_extended_walsh_spectrum(G):
    CCZ-equivalence is ruled out
else:
    INCONCLUSIVE
```

The Walsh implementation stores coefficients for every non-zero output mask. For fixed `n,m`, the omitted zero-output-mask slice is identical for every map: it contains `2^n` at input mask zero and zero for the other input masks. Equality of the stored absolute spectrum is therefore equivalent to equality of the complete extended absolute Walsh multiset for the purpose of comparing two maps with the same declared dimensions.

`vectorial_ccz_invariant_screen_with_work_limit` applies an explicit work limit to each individual exact spectrum calculation and stops at the first invariant mismatch. Any malformed truth table, out-of-range output, arithmetic/allocation failure or work-limit exhaustion is an error/non-result, never evidence for or against equivalence.

A matching result is intentionally named `Inconclusive`. The API has no positive-equivalence state. Equal spectra do not establish CCZ-equivalence, EA-equivalence, cryptographic suitability, common mechanism, prior-art status or novelty. Algebraic degree is not added as a CCZ rejection invariant because it is not preserved by CCZ-equivalence in general.

Regression fixtures cover:

- identity versus the PRESENT 4-bit permutation, rejected by differential-spectrum mismatch;
- PRESENT versus a fixed output translation, whose implemented necessary invariants match and therefore remains `Inconclusive` rather than being declared equivalent;
- a deterministic synthetic pair with the same differential spectrum but different absolute Walsh spectra, exercising the second rejection gate;
- malformed and deliberately under-budget inputs, which remain non-results.

This screen is suitable for eliminating impossible CCZ matches before any more expensive declared equivalence procedure. It must not be used to promote a BL-13 candidate to `KNOWN`, `NOVELTY_CANDIDATE` or `PROVED_NEW_UNDER_DECLARED_EQUIVALENCE` by itself.