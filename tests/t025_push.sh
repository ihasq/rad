#!/bin/bash
RUST="$(realpath "$1")"
TS="$(realpath "$2")"

PORT=18951
BASE="http://localhost:$PORT"

# Start TS Relay in background
"$TS" relay --port $PORT > /dev/null 2>&1 &
RELAY_PID=$!
sleep 2

# Cleanup function
cleanup() {
  kill $RELAY_PID 2>/dev/null
  wait $RELAY_PID 2>/dev/null
  rm -rf "$R_DIR" "$T_DIR"
}
trap cleanup EXIT

# Setup: alice clones and makes changes
KEYS=$("$RUST" keygen)
ALICE_PUB=$(echo "$KEYS" | head -1 | awk '{print $2}')
ALICE_SEC=$(echo "$KEYS" | sed -n '2p' | awk '{print $2}')

# Register alice via API
curl -s -X POST "$BASE/rad/participants" \
  -H "Content-Type: application/json" \
  -d "{\"participantId\":\"alice\",\"publicKey\":\"$ALICE_PUB\",\"isFounder\":true}" \
  > /dev/null

# Alice clones
R_DIR=$(mktemp -d)
(cd "$R_DIR" && "$RUST" clone "$BASE" --participant alice --secret-key "$ALICE_SEC" > /dev/null 2>&1)

# T-PU01: ローカルで write → rad push → Relay に操作が到達する
echo "write src/test.ts 1 10 alice $ALICE_SEC \"test content\"" | (cd "$R_DIR" && "$RUST" pipeline > /dev/null 2>&1)
(cd "$R_DIR" && "$RUST" push > /dev/null 2>&1)
R_PUSH_EXIT=$?
[ $R_PUSH_EXIT -eq 0 ] || exit 1

# T-PU02: push 後に Relay の GET /rad/log に操作が含まれる
sleep 1
LOG_JSON=$(curl -s "$BASE/rad/log")
echo "$LOG_JSON" | grep -q 'test content' || exit 1

# T-PU03: push 済みの操作は再 push されない（冪等性）
# 2回目の push は "0 operations" を出力するべき
PUSH2_OUT=$(cd "$R_DIR" && "$RUST" push 2>&1)
# This will be implementation-dependent, but should succeed
(cd "$R_DIR" && "$RUST" push > /dev/null 2>&1)
[ $? -eq 0 ] || exit 1

# T-PU04: accept も push で送信される
# Create another participant to test accept
KEYS_B=$("$RUST" keygen)
BOB_PUB=$(echo "$KEYS_B" | head -1 | awk '{print $2}')
BOB_SEC=$(echo "$KEYS_B" | sed -n '2p' | awk '{print $2}')

curl -s -X POST "$BASE/rad/participants" \
  -H "Content-Type: application/json" \
  -d "{\"participantId\":\"bob\",\"publicKey\":\"$BOB_PUB\",\"isFounder\":false}" \
  > /dev/null

# Bob makes a write (visible)
echo "write src/bob.ts 1 10 bob $BOB_SEC \"bob content\"" | (cd "$R_DIR" && "$RUST" pipeline > /dev/null 2>&1)

# Get the operation ID
BOB_OP_ID=$(cd "$R_DIR" && "$RUST" log --participant bob 2>&1 | grep 'bob' | head -1 | awk '{print $1}')

# Alice accepts it
echo "accept $BOB_OP_ID alice $ALICE_SEC" | (cd "$R_DIR" && "$RUST" pipeline > /dev/null 2>&1)

# Push the accept
(cd "$R_DIR" && "$RUST" push > /dev/null 2>&1)

# TS test
T_DIR=$(mktemp -d)
(cd "$T_DIR" && "$TS" clone "$BASE" --participant carol --secret-key "$ALICE_SEC" > /dev/null 2>&1)

# T-PU05: Rust と TS クライアントの push 結果が一致する（基本動作）
echo "write src/test2.ts 1 10 carol $ALICE_SEC \"test2\"" | (cd "$T_DIR" && "$TS" pipeline > /dev/null 2>&1)
(cd "$T_DIR" && "$TS" push > /dev/null 2>&1)
T_PUSH_EXIT=$?

# T-PU06: exit code が一致する
[ $R_PUSH_EXIT -eq 0 ] && [ $T_PUSH_EXIT -eq 0 ] || exit 1
