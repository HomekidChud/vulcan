#!/usr/bin/env bash
# Vulcan vs Rust benchmark. Runs each workload 5x per side and reports the
# best (fastest) time, plus end-to-end compile times.
#   Vulcan: vulcan build (transpile + gcc -O3)
#   Rust:   rustc -O (single file, no cargo)
set -u
cd "$(dirname "$0")/.."
V=./target/release/vulcan
RUSTC="${RUSTC:-rustc}"
BENCHES="fib collatz primes physics"
RUNS=5

now() { date +%s.%N; }
elapsed() { echo "$2 - $1" | bc; }

timed() { # command... -> seconds
  local s e
  s=$(now); "$@" > /dev/null 2>&1; e=$(now)
  elapsed "$s" "$e"
}

best_of() { # binary
  local bin=$1 best=""
  for i in $(seq $RUNS); do
    t=$(timed "$bin")
    if [ -z "$best" ] || [ "$(echo "$t < $best" | bc)" = 1 ]; then best=$t; fi
  done
  echo "$best"
}

printf "%-10s %14s %14s %10s\n" "benchmark" "vulcan best(s)" "rust best(s)" "winner"
echo "------------------------------------------------------------"
for b in $BENCHES; do
  $V build bench/$b.vulcan > /dev/null 2>&1
  $RUSTC -O -o bench/$b.rs.bin bench/$b.rs > /dev/null 2>&1
  v=$(best_of ./bench/$b.vbin)
  r=$(best_of ./bench/$b.rs.bin)
  if [ "$(echo "$v <= $r" | bc)" = 1 ]; then w="VULCAN"; else w="rust"; fi
  printf "%-10s %14s %14s %10s\n" "$b" "$v" "$r" "$w"
done

echo
echo "vulcan C backend: ${VULCAN_CC:-auto (clang if available, else gcc)}"
echo "compile times (end to end, best of $RUNS):"
for b in $BENCHES; do
  vbest=""; rbest=""
  for i in $(seq $RUNS); do
    t=$(timed $V build bench/$b.vulcan)
    [ -z "$vbest" ] && vbest=$t || { [ "$(echo "$t < $vbest" | bc)" = 1 ] && vbest=$t; }
    t=$(timed $RUSTC -O -o bench/$b.rs.bin bench/$b.rs)
    [ -z "$rbest" ] && rbest=$t || { [ "$(echo "$t < $rbest" | bc)" = 1 ] && rbest=$t; }
  done
  if [ "$(echo "$vbest <= $rbest" | bc)" = 1 ]; then w="VULCAN"; else w="rust"; fi
  printf "  %-10s vulcan %ss   rustc -O %ss   -> %s\n" "$b" "$vbest" "$rbest" "$w"
done
