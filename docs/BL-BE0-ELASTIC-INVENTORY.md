# BL-BE0 — BooleanLab inventory for ElasticXxx differential work

Status: engineering inventory, not a performance or novelty result.

Source repository: `Memorithm/BooleanLab`
Source base commit: `c762ca2a343e86624cc39b719b66e741d78b59db`
Canonical production owner for Boolean resource-control semantics: `Memorithm/ElasticXxx`

## Existing BooleanLab primitives

The current `booleanlab-core` public surface already contains:

- exact Boolean circuits and truth rows (`circuit`);
- exact mask/cardinality and static truth-table masks (`sparsity`, `sparsity_static`);
- dynamic predicate masks (`sparsity_dynamic`);
- deterministic ranking and structured N:M masks (`sparsity_ranking`);
- relational/redundancy masks (`sparsity_relational`);
- grouped structured masks (`sparsity_structured`);
- Boolean matrix-equation reference cells, packed variants, logical-cost accounting, and canonical equation identities (`bme*`);
- bit-signature/Hamming attention admission and page lower-bound admission (`attention`, `attention_page`);
- systems-evidence schemas that distinguish work, timing, and traffic evidence (`attention_evidence`);
- explicit Boolean×X bridge types (`hybrid`).

## Gap closed by the accompanying BL-BE1 slice

The base commit did not expose an independent Strong-Kleene three-valued oracle. The accompanying `kleene` module adds a dependency-free exact reference over `False`, `Unknown`, and `True`, with exhaustive binary truth-table generation and tests.

This oracle is deliberately not an ElasticXxx runtime implementation. It exists so production implementations can be compared against a small independent enumerable reference without creating a runtime dependency from ElasticXxx to BooleanLab.

## Mapping to the production roadmap

- Elastic BE1 / BE3: use the independent Strong-Kleene truth-table oracle and existing BooleanLab exact circuit machinery for differential tests.
- Elastic BE11: BooleanLab can experiment with contradiction, tautology, implication, BDD/SAT bounds, and failure/resource limits; production solver authority remains in ElasticXxx.
- Elastic BE13: existing exact masks and BME packed/reference pairs provide candidate forms for measured representation/evaluation comparisons; no speedup is implied until benchmarked on declared hardware.
- Elastic BE14 / BE15: attention, page routing, sparsity, and BME mechanisms remain experimental here until a versioned artifact and destination-owned requalification exist.

## Explicit non-claims

This inventory does not establish that any Boolean representation is faster than a numerical path, that a Boolean front-end improves attention, that any candidate is novel, or that any BooleanLab result is safe to actuate resources. Those conclusions require destination-owned tests and executed evidence.
