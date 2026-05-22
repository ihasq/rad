#!/bin/bash
set -euo pipefail

RUST_BIN="${RUST_BIN:-rust/target/release/rad}"
TS_BIN="${TS_BIN:-ts/dist/rad}"
PASS=0; FAIL=0; SKIP=0; TOTAL=0

run_test() {
  local name="$1" script="$2"
  TOTAL=$((TOTAL+1))
  if bash "$script" "$RUST_BIN" "$TS_BIN" 2>/dev/null; then
    echo "  ✅ $name"
    PASS=$((PASS+1))
  else
    local code=$?
    if [ $code -eq 77 ]; then
      echo "  ⏭  $name (SKIP)"
      SKIP=$((SKIP+1))
    else
      echo "  ❌ $name"
      FAIL=$((FAIL+1))
    fi
  fi
}

echo "=== Rad Test Suite ==="
for t in tests/t*.sh; do
  run_test "$(basename $t .sh)" "$t"
done

echo ""
echo "=== SUMMARY ==="
echo "  total: $TOTAL"
echo "  pass:  $PASS"
echo "  fail:  $FAIL"
echo "  skip:  $SKIP"

[ $FAIL -eq 0 ]
