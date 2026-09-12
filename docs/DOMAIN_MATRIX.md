# Boolean × X Research Domain Matrix

BooleanLab treats `X` as an open mathematical-domain parameter rather than a fixed shortlist.

The generic object is

```text
B × X
```

with a Boolean state/control plane `B = {0,1}^m` and a mathematical domain `X` carrying the non-Boolean structure.

The purpose of the matrix is not to assume that every coupling is useful. It is to make the search systematic, falsifiable and comparable.

## Coupling modes

Every experiment must declare at least one coupling mode:

| Mode | Meaning |
| --- | --- |
| `MASK` | Boolean state enables/disables coordinates, terms, edges, basis elements or operators in `X`. |
| `SELECT` | Boolean state selects one operator, chart, algebra, kernel, solver or representation from a finite family. |
| `ROUTE` | Boolean state routes information between components or memories. |
| `PREDICATE` | Properties of `X` are projected into Boolean predicates through `P_X`. |
| `CONSTRAIN` | Boolean clauses restrict admissible states or transitions in `X`. |
| `VERIFY` | Boolean/formal checks accept, reject or rollback an operation in `X`. |
| `MEMORY` | Boolean state stores persistent discrete information alongside an `X` state. |
| `TRANSITION` | Boolean dynamics choose or modify the transition law acting on `X`. |
| `ENCODE` | Values or structures in `X` are represented through Boolean codes. |
| `SYNTHESIZE` | A search procedure discovers Boolean structure controlling an `X` computation. |

## Domain families

### Algebraic domains

- `X = R`, `Q`, integers and fixed-point arithmetic
- complex numbers
- quaternions
- octonions
- sedenions and higher Cayley-Dickson constructions
- finite fields `GF(p^k)`
- Boolean rings and other finite rings
- semirings
- tropical semirings
- polynomial rings
- matrix algebras
- Clifford / geometric algebras

### Discrete and combinatorial domains

- graphs and hypergraphs
- finite-state machines
- automata and cellular automata
- SAT / CSP state spaces
- matroids
- codes and coding-theory objects
- permutations and groups
- simplicial / combinatorial complexes

### Geometric and topological domains

- Euclidean vector spaces
- manifolds
- Riemannian state spaces
- Lie groups and Lie algebras
- projective spaces
- metric spaces
- topological complexes

### Spectral and operator domains

- linear operators
- sparse operators
- graph Laplacians
- resolvents / Green-function representations
- Fourier domains
- Walsh-Hadamard domains
- wavelet and multiresolution representations

### Dynamical domains

- deterministic discrete-time systems
- continuous-time ODE systems
- hybrid dynamical systems
- recurrent state-space models
- control systems
- attractor systems
- field dynamics

### Probabilistic and information domains

- probability distributions
- Markov chains
- factor graphs
- Bayesian-network state spaces
- entropy / information representations
- stochastic processes
- noisy Boolean networks

### Optimization domains

- continuous optimization
- combinatorial optimization
- convex and non-convex problems
- Pareto / multi-objective search
- bandit state
- evolutionary search spaces

### Representation and AI domains

- dense tensors
- sparse tensors
- quantized tensors
- embeddings
- associative memory
- KV-cache representations
- attention state
- recurrent latent state
- mixture-of-experts routing
- world-model state

## Canonical experiment template

For each candidate domain `X`, define:

```text
H0: matched X-only and/or Boolean-only baselines are sufficient.
H1: an explicit Boolean × X coupling provides a measurable advantage under the declared metric and resource budget.
```

Then freeze:

1. `X` and its exact semantics;
2. coupling mode(s);
3. `P_X` predicate extraction, if present;
4. Boolean state width and update rule;
5. the `X` operator family controlled by Boolean state;
6. baselines;
7. parameter / memory / compute accounting;
8. metrics;
9. falsification criteria;
10. evidence class.

A hybrid candidate is not promoted merely because it performs better. The experiment must identify whether the gain comes from the coupling itself rather than extra capacity or information.

## Initial priority grid

| Priority | Domain | First coupling |
| --- | --- | --- |
| P0 | Boolean × Boolean circuit | exact control baseline |
| P0 | Boolean × finite state | recurrent transition / memory |
| P0 | Boolean × real tensor | component mask / routing |
| P0 | Boolean × sedenion | basis-component mask and operator selection |
| P0 | Boolean × graph | edge/node mask and reachability control |
| P0 | Boolean × associative memory | Hamming/XNOR retrieval and retention policy |
| P1 | Boolean × quaternion/octonion | matched hypercomplex comparison |
| P1 | Boolean × finite field | exact algebraic control / coding tasks |
| P1 | Boolean × tropical semiring | path / routing / dynamic-programming control |
| P1 | Boolean × dynamical system | regime switching and intervention |
| P1 | Boolean × probability distribution | discrete gating under explicit uncertainty |
| P2 | Boolean × manifold / Lie group | chart/operator selection under geometric constraints |
| P2 | Boolean × Clifford algebra | basis/operator gating |
| P2 | Boolean × spectral operator | mode selection / sparse control |

The priorities are engineering order only, not claims of scientific importance.
