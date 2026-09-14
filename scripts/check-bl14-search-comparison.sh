#!/usr/bin/env bash
# Exact SEARCH-only calibration; no model or HOLDOUT access.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
cargo test --locked -p booleanlab-discovery --bin bl14_search_comparison
cargo run --locked -p booleanlab-discovery --bin bl14_search_comparison --release
