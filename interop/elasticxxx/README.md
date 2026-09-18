# BooleanLab → ElasticXxx exact Boolean interop vectors

`exact-vectors-v1.tsv` is a versioned **test fixture**, not a runtime API and not an experimental result. `exact-vectors-v1.manifest` is the adjacent provenance contract: it pins the exact BooleanLab generator commit, generator-module SHA-256, fixture SHA-256, row ordering and claim boundary. BooleanLab owns the exact small-domain Boolean semantics. The fixture is generated deterministically by:

```bash
cargo +1.89.0 run -q -p booleanlab-discovery --bin bl_elastic_interop_vectors
```

The repository test regenerates the file byte-for-byte. Assignment row `x` uses packed integer order with `p0` as the least-significant bit. The compact RPN grammar is limited to `pN`, `true`, `false`, `not`, `and`, `or`, `xor`, and `implies` under an explicit token bound.

Each row records the exact truth table plus SciRust-backed algebraic degree, nonlinearity, balancedness and correlation-immunity metrics, BooleanLab's non-cryptographic content fingerprint, and exact satisfiable/tautology/contradiction properties. The fingerprint is only provenance metadata and is not proof of equality; consumers must compare exact semantics.

Cross-repository consumers should snapshot **both** the TSV and its adjacent manifest, verify the recorded SHA-256 values, retain the pinned BooleanLab generator commit, and verify their own implementation against the exact truth table. A consumer must not depend on BooleanLab experimental runtime code, and a passing vector does not authorize validation, actuation, performance, novelty, or scientific claims.
