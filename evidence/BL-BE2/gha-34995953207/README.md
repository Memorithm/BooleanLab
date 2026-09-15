# BL-BE2 representation evidence — GitHub Actions run 34995953207

This directory preserves the exact text evidence emitted by the BL-BE2 representation benchmark on pull request #93. It is a hosted-runner development measurement, not a production or hardware qualification.

- Source head: `beb4e3b84a49f3ea5ea70de8b29aeed941830b3f`
- Pull-request checkout SHA: `d3bfd33060245833c988222ed29dcf66c435492b`
- GitHub Actions run: `34995953207`, attempt `1`
- Artifact id: `10407328413`
- Artifact digest reported by GitHub: `sha256:80e2fa022e03d3c65d3ef98d597df8854632fc89f2c44a8536499e65affd02d6`
- Runner: Linux X64
- rustc: `1.98.1 (48a229cea 2026-09-01)`
- cargo: `1.98.1 (797e8a9bc 2026-08-05)`
- Resolved `Cargo.lock` SHA-256: `46f6d95f77baad13e8bd971540122cf2bbd85419f39dc2e71f04ba474d8f6cce`
- `report.tsv` SHA-256: `3c1350d5ba2bc9fd9f176f4f4145b64699364353aa42980657f93b1bca9c9119`

The original artifact is retained by GitHub Actions for 90 days. `report.tsv`, the resolved `Cargo.lock`, the emitted manifest, and the artifact checksum inventory are copied here so the observation does not disappear when the Actions artifact expires.

## Observed development measurements

The run used seed `42`, 257 deterministic corpus rows and 20,000 timed evaluations per backend. Every backend produced the same semantic checksum (`234`) after the preflight differential checks.

| Panel | Backend | Arity | Packed words / polarity | Elapsed ns | ns / evaluation |
| --- | --- | ---: | ---: | ---: | ---: |
| u64-comparable | generic postfix | 32 | 0 | 3,290,346 | 164.517 |
| u64-comparable | u64 conjunction | 32 | 1 | 179,907 | 8.995 |
| u64-comparable | multiword conjunction | 32 | 1 | 175,770 | 8.788 |
| multiword | generic postfix | 256 | 0 | 24,516,508 | 1,225.825 |
| multiword | multiword conjunction | 256 | 4 | 350,219 | 17.510 |

These are raw wall-clock observations on one GitHub-hosted X64 Linux runner. They do not establish a portable ratio, an ElasticXxx runtime speedup, an attention/KV-cache speedup, an energy benefit, or a model-level quality result. The generic postfix evaluator and compiled conjunctions have different implementation structures, so assigning a mechanism requires separate profiling and hardware qualification.

No acceptance threshold is derived from these timings. Semantic mismatch, resource exhaustion, or a later failure to reproduce the observation remains a negative/non-result and must be retained.
