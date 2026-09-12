# BL-13 — Boolean Function Discovery Protocol

BooleanLab may discover Boolean functions while exploring pure Boolean systems and Boolean × X hybrids. This document defines what counts as a candidate and what evidence is required before using the word `new`.

## 1. Object classes

Scalar track:

`f : F_2^n -> F_2`

Vectorial track:

`F : F_2^n -> F_2^m`

The tracks must not silently share metrics or novelty claims. A scalar-coordinate novelty result does not establish novelty of a vectorial mapping, and vice versa.

## 2. Sources of candidates

Candidates may arise from:

- direct Boolean circuit search;
- SAT / MaxSAT / CEGIS synthesis;
- evolutionary or Forge-style search;
- learned discrete logic;
- exact projections of Boolean × X hybrid systems;
- threshold/predicate surfaces derived from an X-domain state;
- recurrent transitions or attractor maps;
- cross-domain composition;
- human-derived constructions used as explicit seeds.

For a hybrid candidate, the projection from X-domain state to bits must be specified before confirmatory evaluation. Post-hoc threshold tuning is exploratory evidence only.

## 3. A truth table is not enough for novelty

For fixed `n`, all `2^(2^n)` scalar Boolean functions exist mathematically. Finding a truth table not previously encountered by BooleanLab is therefore not a scientific novelty result.

A candidate may become scientifically interesting if evidence supports at least one of:

1. a new explicit construction rule or parametric family;
2. a candidate outside the declared equivalence classes represented in the comparison corpus;
3. a previously unreported combination or Pareto trade-off of relevant properties;
4. a new compact implementation or circuit construction under exact functional semantics;
5. a cross-domain mechanism that repeatedly generates the same Boolean structural family and admits a mathematical explanation.

Prior-art search remains mandatory before any public novelty claim.

## 4. Exact scalar fingerprint

Where width permits exhaustive evaluation, every scalar candidate records:

- ordered truth table and cryptographic digest;
- Hamming weight and balancedness;
- algebraic normal form (ANF);
- algebraic degree;
- Walsh-Hadamard spectrum;
- nonlinearity;
- correlation-immunity order and resiliency when balanced;
- autocorrelation / propagation metrics when implemented;
- algebraic-immunity and fast-algebraic-immunity metrics when implemented;
- symmetry descriptors;
- circuit gate count, depth and declared primitive basis;
- provenance: generator, seed, X domain, projection, search budget and code revision.

Missing metrics must be marked `NOT_IMPLEMENTED`; they must never be inferred.

## 5. Vectorial fingerprint

Vectorial candidates additionally require metrics appropriate to `F_2^n -> F_2^m`, including when implemented:

- coordinate and component-function spectra;
- differential distribution information;
- linear approximation information;
- differential uniformity;
- algebraic degree by coordinate/component;
- permutation/bijection status where `n = m`;
- relevant EA/CCZ or other declared equivalence screening;
- implementation cost.

The exact equivalence relation used in a result must always be named.

## 6. Equivalence and canonicalization ladder

Candidate deduplication is staged. BooleanLab must distinguish cheap fingerprints from mathematical equivalence proofs.

Suggested scalar ladder:

1. exact truth-table identity;
2. complement identity;
3. input-variable permutation canonicalization;
4. declared affine-input/output transformations;
5. stronger equivalence analysis when scientifically required.

Suggested vectorial ladder:

1. exact mapping identity;
2. affine / EA screening;
3. CCZ screening when applicable and implemented;
4. dedicated proof or exhaustive canonicalization at bounded widths.

A collision-free digest proves identity only for the serialized representation used by the bench; it does not prove inequivalence under a stronger relation.

## 7. Search comparison

Every hybrid discovery campaign must run a matched Boolean-only control. The comparison must freeze:

- input/output widths;
- primitive grammar;
- evaluation budget;
- wall-clock or candidate-count accounting;
- objective vector;
- seeds/split discipline;
- stopping rules.

The hybrid mechanism earns scientific attention only if it changes the reachable family or the property/cost frontier rather than merely spending more search resources.

## 8. Objective vector

Do not collapse Boolean-function quality into one score unless a preregistered downstream task requires it. Maintain a typed multi-objective record such as:

`(balancedness, nonlinearity, degree, correlation_immunity, algebraic_immunity, gate_count, depth, task_metric, hardware_metric)`

Metrics that are incompatible or undefined for a candidate remain separate rather than being coerced into a scalar.

## 9. Discovery statuses

- `GENERATED` — candidate exists and is reproducible.
- `FINGERPRINTED` — required current metrics recorded.
- `DEDUPLICATED` — no identical candidate in the frozen corpus.
- `EQUIVALENCE_SCREENED` — passed the declared implemented equivalence screens.
- `PARETO_INTERESTING` — non-dominated under the preregistered metric vector.
- `MECHANISM_FOUND` — recurring construction mechanism has been isolated.
- `PRIOR_ART_PENDING` — internal candidate only; no novelty language allowed.
- `KNOWN` — matched prior construction/equivalence class.
- `NOVELTY_CANDIDATE` — survived implemented screens and prior-art search, but is not a proof of novelty.
- `PROVED_NEW_UNDER_DECLARED_EQUIVALENCE` — only after an explicit mathematical proof within the stated universe/equivalence relation.

`NEW` must never be emitted as an automatic label from a search run.

## 10. Initial BL-13 programme

- `BL-13.1` — calibrate search against known Boolean classes and build the Boolean-only Pareto baseline.
- `BL-13.2` — Boolean × sedenion induced functions.
- `BL-13.3` — Boolean × finite-field induced functions.
- `BL-13.4` — Boolean × tropical induced functions.
- `BL-13.5` — Boolean × dynamical-system induced functions.
- `BL-13.6` — cross-domain convergence: identify the same Boolean structure arising independently from different X domains.
- `BL-13.7` — vectorial Boolean-function discovery.

## 11. Promotion to SciRust / Forge / downstream systems

BooleanLab owns experiments and evidence. Reusable exact metrics and canonicalization primitives belong in SciRust after they are stable. Search machinery that is domain-independent may be promoted to Forge. A function useful to attention, KV memory, control, cryptography or another downstream project must be integrated through a separate engineering contract; an experimental BooleanLab result never silently becomes production behavior.
