#!/usr/bin/env bash
# Preserve a complete BL-14.4.1 numerical-development run.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
if (( $# > 1 )); then
  echo "usage: bash scripts/run-bl14-relational-relu.sh [new-output-directory]" >&2
  exit 2
fi
if [[ -n "$(git status --porcelain --untracked-files=normal)" ]]; then
  echo "A clean source checkout is required for evidence capture." >&2
  exit 1
fi
OUT="${1:-target/bl14-relational-evidence}"
if [[ -e "$OUT" ]]; then
  echo "Refusing to overwrite existing evidence: $OUT" >&2
  exit 1
fi
mkdir -p "$OUT"
git rev-parse HEAD > "$OUT/source-sha.txt"
git rev-parse 'HEAD^{tree}' > "$OUT/source-tree.txt"
rustc --version --verbose > "$OUT/rustc.txt"
cargo --version > "$OUT/cargo.txt"
uname -a > "$OUT/host.txt"
if [[ ! -f Cargo.lock ]]; then
  cargo generate-lockfile
  printf 'generated-for-this-run\n' > "$OUT/lock-origin.txt"
else
  printf 'pre-existing-lockfile\n' > "$OUT/lock-origin.txt"
fi
cp Cargo.lock "$OUT/Cargo.lock"
sha256sum Cargo.toml crates/booleanlab-discovery/Cargo.toml \
  crates/booleanlab-core/Cargo.toml \
  crates/booleanlab-core/src/sparsity_relational.rs \
  crates/booleanlab-discovery/src/bin/bl14_structured_relu.rs \
  crates/booleanlab-discovery/src/bin/bl14_relational_relu.rs \
  crates/booleanlab-discovery/src/bin/support/bl14_relational.rs \
  experiments/BL-14.4.1-RELATIONAL-RELU-PROTOCOL.md \
  scripts/run-bl14-relational-relu.sh > "$OUT/source-files.sha256"
cargo test --locked -p booleanlab-discovery --bin bl14_relational_relu \
  2>&1 | tee "$OUT/tests.txt"
cargo build --locked --release -p booleanlab-discovery --bin bl14_relational_relu \
  2>&1 | tee "$OUT/build.txt"
target/release/bl14_relational_relu | tee "$OUT/report.tsv"
target/release/bl14_relational_relu > "$OUT/replay.tsv"
cmp "$OUT/report.tsv" "$OUT/replay.tsv"
cmp Cargo.lock "$OUT/Cargo.lock"
grep -Fxq '# COMPLETE_TRIALS=12' "$OUT/report.tsv"
(
  cd "$OUT"
  sha256sum Cargo.lock report.tsv replay.tsv > evidence.sha256
)
printf 'byte-identical replay; twelve trials complete\n' > "$OUT/COMPLETE.txt"
echo "Evidence directory: $OUT"
