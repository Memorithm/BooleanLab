#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
git diff --quiet
git diff --cached --quiet
test -z "$(git ls-files --others --exclude-standard)"
OUT=target/bl14-matched-evidence
mkdir -p "$OUT"
git rev-parse HEAD > "$OUT/source-sha.txt"
rustc -Vv > "$OUT/rustc.txt"
cargo -V > "$OUT/cargo.txt"
uname -a > "$OUT/host.txt"
if [ -f Cargo.lock ]; then
  printf 'existing-lockfile\n' > "$OUT/lock-origin.txt"
else
  cargo generate-lockfile
  printf 'generated-for-this-run\n' > "$OUT/lock-origin.txt"
fi
cp Cargo.lock "$OUT/Cargo.lock"
cp crates/booleanlab-discovery/src/bin/bl14_structured_relu.rs "$OUT/fixture.rs"
cp crates/booleanlab-discovery/src/bin/support/bl14_matched_search.rs "$OUT/audit.rs"
cp experiments/BL-14.2.3-MATCHED-SEARCH-PROTOCOL.md "$OUT/protocol.md"
cargo test --locked -p booleanlab-discovery --bin bl14_structured_relu 2>&1 | tee "$OUT/tests.txt"
cargo build --locked -p booleanlab-discovery --bin bl14_structured_relu --release
./target/release/bl14_structured_relu --matched-search-v1 > "$OUT/report.tsv"
./target/release/bl14_structured_relu --matched-search-v1 > "$OUT/replay.tsv"
cmp "$OUT/report.tsv" "$OUT/replay.tsv"
./target/release/bl14_structured_relu > "$OUT/legacy.tsv"
printf '%s  %s\n' ee5670ee654692b40dfe9849ef7a6a019d45c41df27438985eca3f520e990588 "$OUT/legacy.tsv" | sha256sum -c -
if ./target/release/bl14_structured_relu --unknown > /dev/null 2>&1; then
  echo 'unexpected CLI argument was accepted' >&2; exit 1
fi
if ./target/release/bl14_structured_relu --matched-search-v1 extra > /dev/null 2>&1; then
  echo 'extra CLI argument was accepted' >&2; exit 1
fi
cmp Cargo.lock "$OUT/Cargo.lock"
(cd "$OUT" && sha256sum Cargo.lock fixture.rs audit.rs protocol.md report.tsv replay.tsv legacy.tsv > evidence.sha256)
printf '36 cells; matched budgets; exact direct encoding; replay; historical output preserved\n' > "$OUT/COMPLETE.txt"
cat "$OUT/COMPLETE.txt" "$OUT/evidence.sha256"
