# BooleanLab experiment programme

BooleanLab is expected to run many experiments. The repository therefore treats experiments as versioned scientific artefacts rather than ad-hoc scripts.

## Stable identifiers

An experiment identifier has the form `BL-SERIES.FAMILY.EXPERIMENT`.

Examples:

- `BL-1.1.1` — first pure-Boolean feed-forward experiment;
- `BL-4.2.3` — third Boolean-memory experiment in family 4.2;
- `BL-6.1.1` — first Boolean × sedenion experiment.

Identifiers are never reused, even after rejection.

## Lifecycle

Every experiment moves through the following explicit states:

`PROPOSED -> PREREGISTERED -> IMPLEMENTED -> VALIDATED -> RUNNABLE -> RUN -> CLASSIFIED`

`CLASSIFIED` is refined as one of:

- `SUPPORTED` — the preregistered criterion was met;
- `REFUTED` — the preregistered criterion failed in a way that falsifies the tested claim;
- `EQUIVALENT` — no material difference under the declared equivalence margin;
- `INCONCLUSIVE` — evidence is insufficient to classify;
- `INVALID` — protocol, implementation, provenance, or resource-accounting failure invalidated the run.

A result may also be marked `REPLICATION_PENDING` or `REPLICATED`, without changing the original classification.

## Required experimental tuple

Every experiment must freeze, before confirmatory execution:

`(question, H0, H1, X, Boolean coupling, baselines, workload, split, seed policy, resource budget, metrics, decision rule, provenance)`.

For hybrid experiments, the default comparison ladder is:

1. Boolean-only;
2. X-only;
3. Boolean × X;
4. capacity-matched Boolean-only when applicable;
5. capacity-matched X-only when applicable.

The purpose is to separate a coupling effect from extra compute, storage, parameters, or search budget.

## Campaign hierarchy

A **series** names a scientific line. A **family** holds experiments that share a mechanism and comparable baselines. An **experiment** changes one primary scientific variable. A **run** is a concrete execution of one frozen experiment manifest.

Run IDs are derived from the experiment ID plus a content hash of the frozen manifest. Re-running identical content must therefore preserve scientific identity while producing a new provenance record.

## Promotion rule

A Boolean × X mechanism is not promoted into SciRust, TDI, KVLab, FLAT-ATTENTION, NNIS, ElasticXxx, Forge, or another Memorithm project because it is interesting. Promotion requires:

- a classified BooleanLab result;
- an explicit target-project contract;
- resource accounting relevant to that target;
- target-specific correctness tests;
- no unsupported novelty or performance claim.

## Parallelism

Exploratory experiments may run in parallel when they do not consume a reserved confirmatory holdout. Confirmatory experiments must preserve their preregistered split/seed/holdout rules. CI is for validation and small deterministic smoke tests; it must not silently execute expensive or final confirmatory campaigns.

## Registry

`experiments/REGISTRY.tsv` is the machine-readable top-level catalogue. Detailed manifests and result artefacts should later live under stable experiment-specific paths, for example:

```text
experiments/BL-6/BL-6.1/BL-6.1.1/
  preregistration.md
  manifest.toml
  src-or-adapter/
  results/
```

The registry is an index, not evidence. Evidence remains in frozen manifests, executable code, logs, hashes, and result artefacts.
