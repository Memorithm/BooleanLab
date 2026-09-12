# BooleanLab Conjecture Registry

This registry stores falsifiable Boolean × X research questions. Entries are hypotheses, not claims.

## Entry schema

Each conjecture must record:

- `ID`
- mathematical domain `X`
- coupling mode(s)
- precise hypothesis
- null hypothesis
- matched baselines
- resource accounting
- primary metric
- falsification criterion
- evidence status
- implementation owner / downstream beneficiary

Allowed evidence status values:

`UNTESTED`, `PREREGISTERED`, `RUNNING`, `EXACT`, `PROVED`, `SUPPORTED_NUMERICALLY`, `SUPPORTED_ON_HARDWARE`, `REFUTED`, `INCONCLUSIVE`.

## Seed conjectures

### BL-C001 — Boolean structural sparsity

**X:** real-valued tensors  
**Coupling:** `MASK`, `SYNTHESIZE`  
**Hypothesis:** for a declared task and matched active-operation budget, an explicitly learned Boolean structural mask can match or exceed a matched dense/sparse control while yielding a simpler executable structure.  
**Falsification:** no task-quality/resource advantage after controlling for active operations, memory and training information.

### BL-C002 — Boolean associative retention

**X:** associative / KV-like memory  
**Coupling:** `MEMORY`, `PREDICATE`, `ROUTE`  
**Hypothesis:** a learned Boolean retain/promote/evict policy preserves retrieval quality more efficiently than a matched static policy on at least one frozen workload family.  
**Falsification:** matched static or continuous-score policies dominate under the same memory/compute budget.

### BL-C003 — Boolean recurrent state sufficiency

**X:** bounded recurrent systems  
**Coupling:** `MEMORY`, `TRANSITION`  
**Hypothesis:** finite learned Boolean state is sufficient for a non-trivial subset of sequence tasks currently represented by continuous recurrent state under a matched memory budget.  
**Falsification:** Boolean state consistently loses required predictive information on the frozen task family at comparable memory cost.

### BL-C004 — Boolean × sedenion component gating

**X:** sedenions  
**Coupling:** `MASK`  
**Hypothesis:** explicit Boolean activation of sedenion basis components can produce a useful adaptive representation distinct from an equally parameterized static component subset.  
**Falsification:** adaptive masks provide no reproducible gain over matched static masks or real-vector controls.

No associativity, alternativity or norm-composition property is assumed for sedenions.

### BL-C005 — Boolean algebra-family selection

**X:** `{R, C, H, O, S}` representation family  
**Coupling:** `SELECT`  
**Hypothesis:** task-conditioned Boolean selection among representation algebras can outperform the best single fixed algebra under a matched end-to-end resource envelope.  
**Falsification:** a fixed representation matches or dominates after accounting for selector cost and capacity.

### BL-C006 — Boolean × finite-field exact memory

**X:** finite fields `GF(2^k)`  
**Coupling:** `ENCODE`, `VERIFY`, `MEMORY`  
**Hypothesis:** coupling Boolean control with finite-field state yields exact error-detecting or error-correcting memory behavior useful to an AI subsystem under controlled corruption.  
**Falsification:** equivalent Boolean-only coding or standard finite-field coding dominates without the hybrid controller.

### BL-C007 — Boolean × tropical routing

**X:** tropical semiring / path computation  
**Coupling:** `MASK`, `ROUTE`, `SELECT`  
**Hypothesis:** Boolean structural gating of tropical path computation improves adaptive routing efficiency under changing constraints without degrading exact path semantics for enabled edges.  
**Falsification:** standard dynamic graph updates dominate in cost and correctness.

### BL-C008 — Boolean attention visibility

**X:** attention / token-relation graphs  
**Coupling:** `MASK`, `PREDICATE`  
**Hypothesis:** learned Boolean visibility can replace a useful portion of continuous attention scoring on selected workloads while retaining task quality under lower declared memory/compute cost.  
**Falsification:** matched continuous or sparse-attention baselines dominate across the frozen workload.

### BL-C009 — Boolean × field regime switching

**X:** continuous field or dynamical-system state  
**Coupling:** `TRANSITION`, `SELECT`, `VERIFY`  
**Hypothesis:** an explicit Boolean regime controller can discover useful switching surfaces between continuous dynamical laws.  
**Falsification:** a single continuous law or conventional hybrid-system baseline matches the result with equal or lower complexity.

### BL-C010 — Boolean noise as information carrier

**X:** noisy dynamical systems  
**Coupling:** `MEMORY`, `TRANSITION`, `ENCODE`  
**Hypothesis:** structured Boolean perturbation patterns contain predictive information not captured by scalar noise amplitude alone on at least one preregistered system family.  
**Falsification:** stronger conventional noise descriptors fully absorb the apparent signal.

### BL-C011 — Boolean circuit synthesis for cognitive subroutines

**X:** bounded algorithmic tasks  
**Coupling:** `SYNTHESIZE`  
**Hypothesis:** bounded search can discover compact Boolean circuits implementing useful learned subroutines with exact post-training semantics.  
**Falsification:** discovered circuits fail held-out semantics or are not competitive with established synthesis baselines at matched search cost.

### BL-C012 — Boolean verification shell around continuous inference

**X:** continuous AI inference  
**Coupling:** `VERIFY`, `CONSTRAIN`  
**Hypothesis:** a Boolean verification layer can reduce violations of explicitly encoded invariants without unacceptable task-quality or compute cost.  
**Falsification:** violations are not reduced, or equivalent constrained decoding/control achieves the same result more efficiently.

## Growth rule

The registry is intentionally unbounded. New `X` domains may be added whenever there is:

1. a precise coupling definition;
2. a meaningful matched baseline;
3. a measurable outcome;
4. a falsification condition;
5. a plausible downstream beneficiary.

A conjecture must not be promoted to a project claim merely because an exploratory run is positive.
