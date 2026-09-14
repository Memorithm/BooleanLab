# BL-14.3.1 evidence — GitHub Actions run 34904995201

This directory preserves the exact provenance bundle emitted by the non-final BL-14.3.1 dynamic-routing development run.

- Source head: `b4491ed7c9377ca70827494f5d3aa1bf2178390a`
- Pull-request checkout SHA: `b5f5a58a1b3d6d2d0b707b88a293766f46bac7d2`
- GitHub Actions run: `34904995201`, attempt `1`
- Artifact id: `10372007104`
- Artifact digest reported by GitHub: `sha256:e457c779a122283d11a29a97893b6ec14562a6b608bdd0bb1b493f30c4306286`
- Runner: Linux X64
- rustc: `1.98.1 (48a229cea 2026-09-01)`
- cargo: `1.98.1 (797e8a9bc 2026-08-05)`
- Generated resolved `Cargo.lock` SHA-256: `46f6d95f77baad13e8bd971540122cf2bbd85419f39dc2e71f04ba474d8f6cce`
- `report.tsv` SHA-256: `8d359dd9e09c23c04ef0dccd5bd78a7e2193604d524a555600ab6712d5143a96`
- `replay.tsv` SHA-256: `8d359dd9e09c23c04ef0dccd5bd78a7e2193604d524a555600ab6712d5143a96`
- Report and replay were verified byte-identical before publication.

`provenance.zip` is the unmodified GitHub Actions artifact and contains `report.tsv`, `replay.tsv`, the generated resolved `Cargo.lock`, toolchain identities, source/checkout SHAs, manifest and `SHA256SUMS`.

## Non-final development observation

This is the already-observed BL-14 development VALIDATION panel, not an untouched holdout. Across the 12 declared trials, lower task MSE for `boolean_dynamic_x0_sign` compared with each control was:

| Comparator | Better | Equal | Worse |
|---|---:|---:|---:|
| dense | 0 | 0 | 12 |
| magnitude 4/8 | 9 | 2 | 1 |
| unit-level 2:4 | 10 | 1 | 1 |
| activation-energy 4/8 | 8 | 1 | 3 |
| frozen static Boolean best | 7 | 4 | 1 |
| best of four frozen static-random 4/8 controls | 7 | 0 | 5 |
| matched dynamic-random pair | 12 | 0 | 0 |

Dense therefore remains better on task MSE in every trial. The Boolean dynamic controller is not uniformly superior to the matched static controls. These observations do not authorize a quality-preservation, generalization, novelty or performance claim.

For each 64-example VALIDATION batch, the deterministic reference work counters are 2,560 multiplications and 512 ReLUs for dense versus 1,280 multiplications, 256 ReLUs and 512 mask tests for the 4/8 dynamic Boolean path, plus 64 separately counted route-predicate tests. These are reference operation counts only; they are not elapsed time, memory traffic, HBM residency, bandwidth, energy or hardware-speed measurements.
