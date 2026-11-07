#!/usr/bin/env bash
# Self-hosting test suite: every case in self/test/ is compiled by BOTH the
# Rust compiler and the self-hosted compiler (vulc); outputs must match.
set -euo pipefail
cd "$(dirname "$0")/.."

VULCAN=${VULCAN:-./target/release/vulcan}
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

if [ ! -x "$VULCAN" ]; then
  echo "building $VULCAN first..."
  (cd . && cargo build --release)
fi

pass=0; fail=0
for f in self/test/*.vulcan; do
  name=$(basename "$f" .vulcan)
  # reference output: Rust compiler
  "$VULCAN" run "$f" > "$TMP/$name.rust.out" 2>/dev/null
  # self-hosted compiler: stage-1 vulc compiles the case
  "$VULCAN" self "$f" > "$TMP/$name.self.out" 2>/dev/null
  if cmp -s "$TMP/$name.rust.out" "$TMP/$name.self.out"; then
    echo "  ok   $name"
    pass=$((pass+1))
  else
    echo "  FAIL $name"
    diff "$TMP/$name.rust.out" "$TMP/$name.self.out" | head -5
    fail=$((fail+1))
  fi
done

# the fixpoint: vulc must reproduce itself exactly
"$VULCAN" bootstrap > "$TMP/bootstrap.log" 2>&1 && echo "  ok   bootstrap fixpoint" && pass=$((pass+1)) \
  || { echo "  FAIL bootstrap fixpoint"; cat "$TMP/bootstrap.log"; fail=$((fail+1)); }

echo "self-test: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
