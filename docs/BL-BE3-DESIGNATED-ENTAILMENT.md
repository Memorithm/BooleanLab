# BL-BE3 designated-value entailment

Status: development infrastructure, exact bounded semantics only.

This slice begins BL-BE3 with a relation that can be checked directly from the exact `KleeneSemanticKey` artifacts introduced in BL-BE1.

For two exhaustively evaluated Strong-Kleene programs `A` and `B` on the same declared `3^n` domain, BooleanLab defines the experimental relation

```text
A |=_T B  iff  for every row r, A(r) = True implies B(r) = True.
```

`True` is the designated antecedent value. `Unknown` is not rewritten as `False`: when `A(r) = True` and `B(r) = Unknown`, the result is a concrete non-entailment witness whose consequent remains `Unknown`.

This relation is intentionally different from evaluating the connective `!A OR B`. In Strong-Kleene logic that connective is not reflexively `True` on `Unknown`, whereas designated-value entailment is reflexive and is useful for exact small-domain guard-subsumption experiments.

## Fail-closed key validation

Before comparing rows, both public key structures are revalidated:

- schema version must match `KLEENE_SEMANTIC_KEY_SCHEMA_VERSION`;
- `rows` must equal `3^input_arity`;
- packed payload length must be exactly `ceil(rows / 4)`;
- the reserved two-bit code `0b11` is rejected;
- unused high bits in the final byte must be zero;
- antecedent and consequent must declare the same input arity.

A malformed or differently scoped artifact therefore never becomes an entailment result.

## Scientific boundary

This is an exact enumerable-domain relation, not a production authorization primitive. It does not establish that a rule is useful, reachable in a real runtime, safe to remove, faster, or novel. BL-BE3 follow-up work may use this exact relation as an oracle for bounded BDD/SAT/canonicalization studies, but solver timeout or resource exhaustion must remain an explicit non-result.

No TDI holdout/final material is consumed by this slice, and no ElasticXxx runtime semantics are copied into BooleanLab.
