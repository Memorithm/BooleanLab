# BL-BE3 bounded Strong-Kleene semantic classes

Status: **development exact oracle; bounded candidate-set canonicalization only**.

This slice advances the unfinished BL-BE3 canonical-form work without introducing a SAT/BDD dependency or copying ElasticXxx runtime semantics. BooleanLab already has a collision-free exhaustive `KleeneSemanticKey`; the new `canonicalize_kleene_programs` surface groups a caller-declared bounded set of postfix Strong-Kleene programs by that exact key.

For each observed semantic class, the oracle selects one deterministic representative by:

1. fewer postfix instructions;
2. a stable lexicographic encoding of instruction kind, input index and constant value.

The complete `3^n` domain is evaluated. `Unknown` remains distinct from `False` and `True`. The candidate count, row count and total worst-case instruction visits are bounded before semantic evaluation. Overflow, work exhaustion and malformed candidates are explicit non-results.

The returned representative is canonical only **within the supplied bounded candidate set**. This does not prove global formula minimality, BDD canonicity, SAT equivalence outside the enumerated domain, production reachability, dead-rule safety, performance benefit or novelty. The next BL-BE3 step remains an explicitly resource-bounded BDD/SAT-style experiment (if dependency/licence review qualifies it), differentially checked against the exact oracle; timeout/resource exhaustion must remain a non-result.
