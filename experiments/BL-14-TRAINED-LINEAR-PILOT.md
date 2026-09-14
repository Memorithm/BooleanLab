# BL-14.1.2 — Trained linear development pilot

Status: IMPLEMENTED; qualification is tracked in PR #70. This is a small synthetic
development pilot, not a language-model benchmark, final confirmation or
hardware-speed result.

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

## Evidence and regression tests

Report TRAIN task MSE before and after fitting, exact learned-weight bit patterns,
resolved predicate bits, all eligible SEARCH rows, selected truth-table codes,
and VALIDATION task MSE plus reconstruction MSE against the frozen dense model.
Report example count, retained cardinality, executed coefficient multiplications
and mask tests separately. Non-finite input, intermediate arithmetic or metrics,
shape mismatch and counter overflow fail explicitly. Rejected coefficients must
be skipped before multiplication.

Eight tests cover actual learning, partition ID/input separation, complete
partition membership, exact mask cardinality and all best-fit ties, immutable
selection under validation-label perturbation, early skip, finite magnitude
ranking and malformed/non-finite inputs. Labels cannot hide duplicate, missing
or out-of-partition instance identities.

Floating-point losses are numerical evidence, not exact mathematical identities.
Counts are reference-operation accounting, not elapsed time or ISA instructions.
Training, search, ranking, predicate construction, allocation and memory traffic
are not included in inference operation counters. No latency, energy,
memory-reduction, novelty or universal sparsity-superiority claim is licensed
by this pilot.

## Reproduce from a fresh checkout

The workspace currently does not commit `Cargo.lock`. Resolve dependencies once
before using `--locked`; do not silently overwrite an existing lockfile. Preserve
the resolved lockfile, toolchain and commit together with the report:

```bash
set -euo pipefail
if [ ! -f Cargo.lock ]; then cargo generate-lockfile; fi
mkdir -p target/bl14-pilot-evidence
cp Cargo.lock target/bl14-pilot-evidence/Cargo.lock
git rev-parse HEAD > target/bl14-pilot-evidence/commit.txt
rustc --version --verbose > target/bl14-pilot-evidence/rustc.txt
sha256sum Cargo.lock > target/bl14-pilot-evidence/lock.sha256
cargo test --locked -p booleanlab-discovery --bin bl14_trained_linear
cargo run --locked -p booleanlab-discovery --bin bl14_trained_linear --release \
  | tee target/bl14-pilot-evidence/report.tsv
```

The initial resolution needs network access unless all inputs are cached. A
newly generated lockfile may differ at another date: `--locked` prevents changes
to the local resolved file; it does not make an uncommitted dependency graph a
repository-wide immutable pin. Reproducing a prior run requires its preserved
lockfile and recorded toolchain, not a fresh resolution assumed equivalent.
The existing CI's preceding unlocked workspace test resolves dependencies before
its locked pilot invocation; that is not a substitute for this fresh-checkout
bootstrap. A committed workspace lockfile is a separate remaining reproducibility
improvement.

## Next gates and reuse

Preserve the result, including an equal or worse Boolean result. Then expand to
multiple preregistered teacher/input regimes and seeds, a nonlinear trained
model, explicit controller overhead and declared hardware measurements before
any promotion. TDI-9.3 can reuse the partition/freeze/validation discipline, not
these synthetic regression labels or a claim of dynamic-inference improvement.
Mathematically general adapters may move to SciRust after separate qualification;
TDI-specific action semantics and final-evaluation gates remain in TDI.
