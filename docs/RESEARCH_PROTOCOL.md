# BooleanLab Research Protocol

BooleanLab separates implementation from scientific qualification.

## Evidence classes

Every reported result must be labelled as one of:

- `EXACT` — exhaustively enumerated or algebraically exact under the declared finite semantics;
- `PROVED UNDER DECLARED ASSUMPTIONS` — supported by a proof whose assumptions are explicit;
- `NUMERICAL EVIDENCE` — reproducible finite-precision experiment;
- `HARDWARE EVIDENCE` — reproducible measurement tied to a recorded device/software environment;
- `CONJECTURE` — a testable hypothesis without qualifying evidence;
- `REFUTED` — a declared claim contradicted by a valid counterexample or failed frozen criterion;
- `INCONCLUSIVE` — the experiment does not distinguish the competing hypotheses.

## Baseline rule

A Boolean × X candidate must be compared against, where meaningful:

1. a Boolean-only baseline;
2. an X-only baseline;
3. the hybrid candidate;
4. matched or explicitly accounted resource budgets.

No advantage may be attributed to the hybrid coupling when it is explained by additional state, parameters, memory, compute or privileged information.

## Exact Boolean semantics

BL-0 exact enumeration is deliberately bounded. Larger spaces must use a declared proof/search/sampling method and may not be described as exhaustively verified.

Every Boolean predicate exposed by a `PredicateBridge<X>` must have a documented semantic meaning. Learned opaque bits are allowed only when the experiment explicitly studies latent Boolean state and does not mislabel those bits as human-interpretable predicates.

## Hybrid-domain rule

For

```text
b(t+1) = F_bool(b(t), P_X(x(t)), u(t))
x(t+1) = G_{b(t+1)}(x(t), u(t))
```

experiments must state:

- the exact domain `X`;
- the predicate map `P_X`;
- the Boolean transition/update function;
- which operation `G_b` is selected or modified by the Boolean state;
- which invariants of `X` are preserved;
- which invariants are intentionally not assumed.

## Hypercomplex caution

Quaternion, octonion and sedenion experiments must use the algebraic laws of the actual domain. In particular, sedenion experiments must not silently use associativity, alternativity or composition-algebra norm multiplicativity.

## Promotion rule

BooleanLab owns experiments, not ecosystem-wide production primitives. A mechanism moves to SciRust, TDI, ADA, KVLab, FLAT-ATTENTION, NNIS, ElasticXxx or another Memorithm project only after the receiving project defines its own integration contract and validation gates.

## Reproducibility

Development validation for the Rust workspace:

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p booleanlab-runner --release
```

Hardware claims require additional device-specific provenance and raw measurements.
