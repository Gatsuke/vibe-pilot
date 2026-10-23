#!/usr/bin/env bash
# Reproduce the README benchmark numbers.
#   make bench   (or: bash bench/run.sh)
set -euo pipefail
cd "$(dirname "$0")/.."

HERE="$(pwd)"
BIN="$HERE/target/release/vp"
[ -x "$BIN" ] || BIN="$HERE/target/debug/vp"
[ -x "$BIN" ] || { echo "build first: cargo build --release"; exit 1; }

bash bench/gen.sh

echo
echo "== vp report (the whole story) =="
"$BIN" report /tmp/vp-bench-app

echo
echo "== task-focused: vp context 'fix the session refresh token bug' =="
time "$BIN" context /tmp/vp-bench-app fix the session refresh token bug -o /tmp/vp-bench-app/.context

echo
echo "== .context stats =="
head -30 /tmp/vp-bench-app/.context
