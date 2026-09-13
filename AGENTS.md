# BooleanLab Agent Bootstrap Contract

Before autonomous coding, experiment design, Boolean search/calibration changes, cross-repository promotion, PR creation, or merge decisions, read the persistent integration overlay:

```bash
git fetch origin agent/ecosystem-roadmap && \
git show origin/agent/ecosystem-roadmap:.agent/BOOLEAN_ELASTICITY_INTEGRATION.yaml
```

The canonical production roadmap is maintained by `Memorithm/ElasticXxx` on `agent/ecosystem-roadmap:.agent/BOOLEAN_ELASTICITY_ROADMAP.yaml`. BooleanLab is the experimental/differential-oracle bench, not the production runtime owner.

If the local overlay cannot be fetched or read, fail closed for cross-repository promotion or merge decisions. Read-only diagnosis is allowed.

## Boolean elasticity programme

Agents must execute the local sequence in order unless an earlier phase is explicitly blocked:

1. **BL-BE0 — bootstrap and inventory:** keep this bootstrap current; inventory BooleanLab expression, mask, exhaustive-search, fitting, calibration, and complexity primitives; map them to ElasticXxx needs without copying production semantics.
2. **BL-BE1 — exact differential oracles:** exhaustively test strong-Kleene `True/False/Unknown`, generic expressions versus compiled conjunction masks, contradiction/tautology/redundancy detection, and small-domain canonicalization.
3. **BL-BE2 — measured representation performance:** benchmark scalar expression evaluation, `u64` masks, multiword masks, and batch filtering; report only reproducible measured results and never extrapolate automatically to full ElasticXxx runtime speedups.
4. **BL-BE3 — bounded symbolic analysis:** experiment with canonical forms, BDDs, bounded SAT, implication, reachability, dead-rule detection, and complexity explosion. Record negative results and resource limits.
5. **BL-BE4 — pseudo-Boolean constraints:** experiment with weighted/cardinality constraints, exact integer scaling, overflow boundaries, and exhaustive small-domain verification.
6. **BL-BE5 — promotion:** publish only versioned, evidence-backed generic contracts or test vectors; pin the source commit; promote production runtime primitives to ElasticXxx and mathematically general primitives to SciRust only when ownership and evidence justify it.

## Cross-repository boundaries

- ElasticXxx owns production `TruthValue`, predicate identity, guard/EIR semantics, runtime fact derivation, pruning, evidence, CLI, macros, and actuation integration.
- BooleanLab owns experimental Boolean-function search, exact small-domain oracles, candidate simplification, complexity studies, mask fitting, and backend comparison.
- TDI owns its preregistered scientific Boolean-policy lineage and holdout/final-evaluation boundaries. Do not access, reproduce, infer, or move forbidden TDI final material.
- Prefer adapters and versioned artifacts over copied implementations.
- Production tests in another repository must not require live BooleanLab access.

## Scientific and engineering constraints

- no fabricated novelty or performance claim;
- exhaustive exact evidence dominates heuristic interpretation on enumerable domains;
- distinguish `Unknown` from `False` in all three-valued experiments;
- preserve deterministic seeds/configuration/provenance for every experiment;
- bound expression size, solver work, memory, and runtime for adversarial cases;
- record solver timeout/resource exhaustion as an explicit non-result, never as satisfiable/unsatisfiable evidence;
- preserve negative and null results;
- required CI must be green on the exact PR head before merge.

Reread this bootstrap and the overlay at every session start, before selecting a new BL-BE phase, after any user change to the Boolean research direction, before cross-repository promotion, and before PR/merge decisions.

Do not merge the off-main roadmap overlay itself into `main` unless the user explicitly requests it.
