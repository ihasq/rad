#!/bin/bash
RUST="$(realpath "$1")"
TS="$(realpath "$2")"

PORT=18950
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

# Setup: alice creates initial state via Relay API
KEYS=$("$RUST" keygen)
ALICE_PUB=$(echo "$KEYS" | head -1 | awk '{print $2}')
ALICE_SEC=$(echo "$KEYS" | sed -n '2p' | awk '{print $2}')

# Register alice
curl -s -X POST "$BASE/rad/participants" \
  -H "Content-Type: application/json" \
  -d "{\"participantId\":\"alice\",\"publicKey\":\"$ALICE_PUB\",\"isFounder\":true}" \
  > /dev/null

# Alice creates a write operation via API
OP_JSON=$(cat <<EOF
{
  "id": "op-1000000000-0",
  "participantId": "alice",
  "regionId": "src/main.ts:1-10",
  "type": "write",
  "content": "const x = 1;",
  "signature": "test-sig",
  "timestamp": 1000000000,
  "status": "accepted"
}
EOF
)

curl -s -X POST "$BASE/rad/operations" \
  -H "Content-Type: application/json" \
  -d "$OP_JSON" \
  > /dev/null

# Bob keys
KEYS_B=$("$RUST" keygen)
BOB_SEC=$(echo "$KEYS_B" | sed -n '2p' | awk '{print $2}')

# T-CL01: rad clone が exit 0 で終了する（Rust）
R_DIR=$(mktemp -d)
(cd "$R_DIR" && "$RUST" clone "$BASE" --participant bob --secret-key "$BOB_SEC" > /dev/null 2>&1)
R_EXIT=$?
[ $R_EXIT -eq 0 ] || exit 1

# T-CL02: clone 後に .rad/ ディレクトリが作成される
[ -d "$R_DIR/.rad" ] || exit 1

# T-CL03: clone 後に .rad/remote.json に Relay URL が保存される
[ -f "$R_DIR/.rad/remote.json" ] || exit 1
grep -q "$BASE" "$R_DIR/.rad/remote.json" || exit 1

# T-CL04: clone 後に rad status が正しい状態を表示する
R_STATUS=$(cd "$R_DIR" && "$RUST" status 2>&1)
echo "$R_STATUS" | grep -q 'participants:' || exit 1

# TS test
T_DIR=$(mktemp -d)
(cd "$T_DIR" && "$TS" clone "$BASE" --participant bob2 --secret-key "$BOB_SEC" > /dev/null 2>&1)
T_EXIT=$?

# T-CL05: exit code が一致する
[ $R_EXIT -eq $T_EXIT ] || exit 1

# T-CL06: clone 後の .rad/ ディレクトリ構造が一致する（基本チェック）
[ -d "$T_DIR/.rad" ] || exit 1
[ -f "$T_DIR/.rad/remote.json" ] || exit 1
