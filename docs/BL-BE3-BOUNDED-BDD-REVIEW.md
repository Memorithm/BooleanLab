# BL-BE3 — bounded BDD/SAT dependency review and experiment

Status: **candidate development evidence**. This records a narrow dependency decision and bounded experiment. It is not a production solver selection, runtime authorization, or actuation authority.

Review date: 2026-09-18.

## Required contract

A BL-BE3 backend may run only after BooleanLab has bounded the input domain and work. Resource exhaustion, timeout, node-limit exhaustion, allocation failure, backend interruption, or semantic mismatch are non-results. They must never be reinterpreted as SAT/UNSAT, equivalence, implication, reachability, or production authorization.

BooleanLab declares Rust 1.89 as its MSRV. The experiment remains optional and disabled by default.

## Candidate review

### `rustsat` 0.7.5

Registry metadata inspected with `cargo info rustsat@0.7.5`: MIT, Rust 1.76.0. RustSAT exposes `OutOfMemory` on several encoding surfaces. Its documentation also states that allocation failure is not converted to a recoverable error on every allocation path, so this alone is not a hard in-process memory ceiling for this experiment.

Sources:

- <https://docs.rs/rustsat/0.7.5/rustsat/enum.OutOfMemory.html>
- <https://docs.rs/crate/rustsat/0.7.5>

Decision: **not selected for this BL-BE3 bounded backend experiment**. This is not a quality judgment on RustSAT.
### `varisat` 0.2.2

Registry metadata inspected with `cargo info varisat@0.2.2`: MIT/Apache-2.0. The reviewed solver surface exposes a recoverable `Interrupted` outcome, but no caller-specified result-node/work ceiling comparable to the contract required here. A separately supervised process could impose OS resource limits, but that is a different experiment and is not introduced by this slice.

Sources:

- <https://docs.rs/crate/varisat/0.2.2>
- <https://github.com/jix/varisat>

Decision: **not selected for this BL-BE3 bounded backend experiment**.

### `biodivine-lib-bdd` 0.6.3

Registry metadata inspected with `cargo info biodivine-lib-bdd@0.6.3`: MIT, Rust 1.88.0, compatible with BooleanLab's Rust 1.89 MSRV. Version 0.6.3 exposes `Bdd::binary_op_with_limit`, which returns no BDD when the result node count exceeds the caller-provided ceiling.

Sources:

- <https://docs.rs/crate/biodivine-lib-bdd/0.6.3>
- <https://docs.rs/biodivine-lib-bdd/0.6.3/biodivine_lib_bdd/struct.Bdd.html>

Decision: **qualified only for a small optional BL-BE3 experiment**, with additional BooleanLab-side bounds. This does not qualify the crate for ElasticXxx production use.
## BooleanLab wrapper

The `bdd-experiments` feature is off by default. The wrapper in `bounded_bdd.rs` adds constraints stricter than the backend API alone:

- at most 8 Boolean inputs, hence at most 256 exact assignments;
- a caller-specified result-node ceiling for every BDD AND/OR;
- a caller-specified pre-dispatch ceiling on `left_nodes * right_nodes` for every BDD AND/OR;
- DNF construction streams one satisfying row at a time rather than retaining an unbounded term collection;
- every final assignment is replayed against BooleanLab's exact truth table;
- any limit hit or differential mismatch is an explicit error/non-result.

The operand-pair precheck is deliberately conservative. It is a bounded work proxy, not a claim about exact allocator bytes or wall-clock time. The experiment reports semantic agreement and explicit ceilings only; it makes no speed, memory, energy, novelty, or production-runtime claim.

## Exit interpretation

If the optional experiment passes its exhaustive differential tests, BL-BE3 has evidence for one bounded external BDD backend experiment. It does **not** imply that a SAT/BDD backend should be promoted into ElasticXxx. Cross-repository promotion remains a separate BL-BE5 / Elastic BE15 decision with pinned artifacts and consumer-side validation.