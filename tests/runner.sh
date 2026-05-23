#!/bin/bash
set -uo pipefail

RUST_BIN="$(readlink -f "${RUST_BIN:-rust/target/release/rad}")"
TS_BIN="$(readlink -f "${TS_BIN:-ts/dist/rad}")"
PASS=0; FAIL=0; SKIP=0; TOTAL=0

run_test() {
  local name="$1" script="$2"
  TOTAL=$((TOTAL+1))
  # Relay tests need TS_BIN (t015, t024-t029), CLI tests only need RUST_BIN
  if [[ "$name" == *relay* ]] || [[ "$name" == *s3* ]] || [[ "$name" == "t024_clone" ]] || [[ "$name" == "t025_push" ]] || [[ "$name" == "t026_pull" ]]; then
    local result; result=$(bash "$script" "$RUST_BIN" "$TS_BIN" 2>/dev/null)
    local code=$?
  else
    local result; result=$(bash "$script" "$RUST_BIN" 2>/dev/null)
    local code=$?
  fi

  if [ $code -eq 0 ]; then
    echo "  ✅ $name"
    PASS=$((PASS+1))
  elif [ $code -eq 77 ]; then
    echo "  ⏭  $name (SKIP)"
    SKIP=$((SKIP+1))
  else
    echo "  ❌ $name"
    FAIL=$((FAIL+1))
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
