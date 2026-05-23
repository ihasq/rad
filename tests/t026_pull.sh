#!/bin/bash
RUST="$(realpath "$1")"
TS="$(realpath "$2")"

PORT=18952
BASE="http://localhost:$PORT"

# Start TS Relay in background
"$TS" relay --port $PORT > /dev/null 2>&1 &
RELAY_PID=$!
sleep 2

# Cleanup function
cleanup() {
  kill $RELAY_PID 2>/dev/null
  wait $RELAY_PID 2>/dev/null
  rm -rf "$R_DIR" "$T_DIR" "$R2_DIR"
}
trap cleanup EXIT

# Setup: alice and bob clone
KEYS=$("$RUST" keygen)
ALICE_PUB=$(echo "$KEYS" | head -1 | awk '{print $2}')
ALICE_SEC=$(echo "$KEYS" | sed -n '2p' | awk '{print $2}')

KEYS_B=$("$RUST" keygen)
BOB_PUB=$(echo "$KEYS_B" | head -1 | awk '{print $2}')
BOB_SEC=$(echo "$KEYS_B" | sed -n '2p' | awk '{print $2}')

# Register alice
curl -s -X POST "$BASE/rad/participants" \
  -H "Content-Type: application/json" \
  -d "{\"participantId\":\"alice\",\"publicKey\":\"$ALICE_PUB\",\"isFounder\":true}" \
  > /dev/null

# Alice clones
R_DIR=$(mktemp -d)
(cd "$R_DIR" && "$RUST" clone "$BASE" --participant alice --secret-key "$ALICE_SEC" > /dev/null 2>&1)

# Bob clones
R2_DIR=$(mktemp -d)
(cd "$R2_DIR" && "$RUST" clone "$BASE" --participant bob --secret-key "$BOB_SEC" > /dev/null 2>&1)

# T-PL01: Relay に他の参加者が write → rad pull でローカルに取得される
# Alice makes a write and pushes
echo "write src/alice.ts 1 10 alice $ALICE_SEC \"alice content\"" | (cd "$R_DIR" && "$RUST" pipeline > /dev/null 2>&1)
(cd "$R_DIR" && "$RUST" push > /dev/null 2>&1)

# Bob pulls
(cd "$R2_DIR" && "$RUST" pull > /dev/null 2>&1)
R_PULL_EXIT=$?
[ $R_PULL_EXIT -eq 0 ] || exit 1

# T-PL02: pull 後に rad log で取得した操作が表示される
BOB_LOG=$(cd "$R2_DIR" && "$RUST" log 2>&1)
echo "$BOB_LOG" | grep -q 'alice content' || exit 1

# T-PL03: pull 後に rad diff で取得した visible write が表示される
BOB_DIFF=$(cd "$R2_DIR" && "$RUST" diff 2>&1)
echo "$BOB_DIFF" | grep -q 'alice' || exit 1

# T-PL04: 二重 pull しても操作が重複しない（冪等性）
# Count operations before second pull
OPS_BEFORE=$(cd "$R2_DIR" && "$RUST" log 2>&1 | wc -l)

# Second pull
(cd "$R2_DIR" && "$RUST" pull > /dev/null 2>&1)

# Count operations after second pull
OPS_AFTER=$(cd "$R2_DIR" && "$RUST" log 2>&1 | wc -l)

# Should be the same
[ "$OPS_BEFORE" -eq "$OPS_AFTER" ] || exit 1

# TS test
T_DIR=$(mktemp -d)
(cd "$T_DIR" && "$TS" clone "$BASE" --participant carol --secret-key "$ALICE_SEC" > /dev/null 2>&1)

# Alice makes another write
echo "write src/alice2.ts 1 10 alice $ALICE_SEC \"alice2\"" | (cd "$R_DIR" && "$RUST" pipeline > /dev/null 2>&1)
(cd "$R_DIR" && "$RUST" push > /dev/null 2>&1)

(cd "$T_DIR" && "$TS" pull > /dev/null 2>&1)
T_PULL_EXIT=$?

# T-PL06: exit code が一致する
[ $R_PULL_EXIT -eq 0 ] && [ $T_PULL_EXIT -eq 0 ] || exit 1

# Verify TS pulled the new operation
T_LOG=$(cd "$T_DIR" && "$TS" log 2>&1)
echo "$T_LOG" | grep -q 'alice2' || exit 1
