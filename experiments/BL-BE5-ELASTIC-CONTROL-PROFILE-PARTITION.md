# BL-BE5 Elastic control-profile partition

Status: bounded exact differential oracle. No runtime promotion.

## Source contract

The compared structural formulas are pinned to SLHAv2 merge
`0bf49558eef14519ae1ce4b246b67347941c8237`, which retains three read-only
control-plane representations:

- sparse W512: 512 payload bits per present slot;
- dense W128: 128 payload bits per physical slot position;
- hybrid W64 + Boolean planes: 64 generation bits per physical slot position
  plus three packed Boolean bitplanes over the full slot domain.

BooleanLab does not import SLHAv2 runtime code. The formulas are represented as
a pinned exact oracle so downstream production code has no live BooleanLab
dependency.

## Bounded question

For every

```text
slot_count    in 1..=4096
present_slots in 0..=slot_count
```

does a compact set of comparison predicates recover exactly the argmin set of
the three declared payload formulas, including ties?

The oracle preserves an argmin bitmask rather than imposing a tie-break policy.

## Reduced predicates

The direct comparisons reduce exactly to:

```text
sparse <= dense
512 * present <= 128 * slots
4 * present <= slots

sparse <= hybrid
512 * present <= 64 * slots + 192 * ceil(slots / 64)
8 * present <= slots + 3 * ceil(slots / 64)

hybrid <= dense
64 * slots + 192 * ceil(slots / 64) <= 128 * slots
3 * ceil(slots / 64) <= slots
```

The implementation checks both the direct payload comparison and these reduced
forms on every bounded case.

## Evidence boundary

This experiment qualifies only structural backing-payload comparisons.

It does not include:

- Rust object or allocator overhead;
- transition/re-encoding cost;
- cache-line or branch behavior;
- DRAM/HBM traffic;
- GPU execution;
- TTFT, TPOT, tokens/s;
- model quality;
- a production profile-selection policy.

A structural argmin is therefore not a runtime winner. Promotion, if any,
belongs to ElasticXxx after destination-side evidence and stability/transaction
gates.
