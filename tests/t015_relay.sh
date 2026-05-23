#!/bin/bash
RUST="$1"; TS="$2"

# TS Relay をバックグラウンドで起動
PORT=18923

# Cleanup function and trap
cleanup() { kill $RELAY_PID 2>/dev/null; wait $RELAY_PID 2>/dev/null; }
trap cleanup EXIT

"$TS" relay --port $PORT > /tmp/relay.log 2>&1 &
RELAY_PID=$!
sleep 2  # 起動待ち
BASE="http://localhost:$PORT"

# 鍵ペア生成
KEYS=$($RUST keygen)
PUB=$(echo "$KEYS" | head -1 | awk '{print $2}')
SEC=$(echo "$KEYS" | sed -n '2p' | awk '{print $2}')

KEYS2=$($RUST keygen)
PUB2=$(echo "$KEYS2" | head -1 | awk '{print $2}')
SEC2=$(echo "$KEYS2" | sed -n '2p' | awk '{print $2}')

# T-RL01: 参加登録 → 201 + participantId
R=$(curl -s -o /tmp/rl01.json -w '%{http_code}' -X POST $BASE/rad/participants \
  -H 'Content-Type: application/json' \
  -d '{"publicKey":"'$PUB'","displayName":"alice"}')
[ "$R" = "201" ] || { kill $RELAY_PID 2>/dev/null; exit 1; }
grep -q 'participantId' /tmp/rl01.json || { kill $RELAY_PID 2>/dev/null; exit 1; }
ALICE_ID=$(grep -o '"participantId":"[^"]*"' /tmp/rl01.json | cut -d'"' -f4)

# T-RL02: publicKey なし → 400
R=$(curl -s -o /dev/null -w '%{http_code}' -X POST $BASE/rad/participants \
  -H 'Content-Type: application/json' -d '{}')
[ "$R" = "400" ] || { kill $RELAY_PID 2>/dev/null; exit 1; }

# T-RL03: 参加者一覧
curl -s $BASE/rad/participants | grep -q 'alice' || { kill $RELAY_PID 2>/dev/null; exit 1; }

# 2人目の参加者登録
curl -s -o /tmp/rl_bob.json -X POST $BASE/rad/participants \
  -H 'Content-Type: application/json' \
  -d '{"publicKey":"'$PUB2'","displayName":"bob"}' > /dev/null 2>&1
BOB_ID=$(grep -o '"participantId":"[^"]*"' /tmp/rl_bob.json | cut -d'"' -f4)

# T-RL04: write → 201 + status: visible
# Operation JSON を構築し署名
OP='{"participantId":"'$ALICE_ID'","type":"write","regionId":"main.ts:1-10","content":"hello"}'
SIGNED=$(echo "$OP" | "$RUST" sign --secret-key "$SEC")
R=$(curl -s -o /tmp/rl04.json -w '%{http_code}' -X POST $BASE/rad/operations \
  -H 'Content-Type: application/json' -d "$SIGNED")
[ "$R" = "201" ] || { kill $RELAY_PID 2>/dev/null; exit 1; }
grep -qi 'visible' /tmp/rl04.json || { kill $RELAY_PID 2>/dev/null; exit 1; }
OP_ID=$(grep -o '"operationId":"[^"]*"' /tmp/rl04.json | cut -d'"' -f4)

# T-RL05: signature なし → 400
R=$(curl -s -o /dev/null -w '%{http_code}' -X POST $BASE/rad/operations \
  -H 'Content-Type: application/json' \
  -d '{"participantId":"'$ALICE_ID'","type":"write","regionId":"main.ts:11-20","content":"test"}')
[ "$R" = "400" ] || { kill $RELAY_PID 2>/dev/null; exit 1; }

# T-RL06: 不正署名 → 403
BAD_SIGNED='{"participantId":"'$ALICE_ID'","type":"write","regionId":"main.ts:21-30","content":"bad","signature":"invalidsignature"}'
R=$(curl -s -o /dev/null -w '%{http_code}' -X POST $BASE/rad/operations \
  -H 'Content-Type: application/json' -d "$BAD_SIGNED")
[ "$R" = "403" ] || { kill $RELAY_PID 2>/dev/null; exit 1; }

# T-RL07: GET /rad/operations/{id}/status → visible
curl -s $BASE/rad/operations/$OP_ID/status | grep -qi 'visible' || { kill $RELAY_PID 2>/dev/null; exit 1; }

# T-RL08: GET /rad/operations/{id} → 操作詳細
curl -s $BASE/rad/operations/$OP_ID | grep -q '"content":"hello"' || { kill $RELAY_PID 2>/dev/null; exit 1; }

# T-RL13: GET /rad/visible/{filePath} → visible な write 一覧
curl -s $BASE/rad/visible/main.ts | grep -q 'hello' || { kill $RELAY_PID 2>/dev/null; exit 1; }

# T-RL09: POST /rad/accept → 200 + status: accepted
ACC='{"participantId":"'$ALICE_ID'","operationId":"'$OP_ID'"}'
ACC_SIGNED=$(echo "$ACC" | "$RUST" sign --secret-key "$SEC")
R=$(curl -s -o /tmp/rl09.json -w '%{http_code}' -X POST $BASE/rad/accept \
  -H 'Content-Type: application/json' -d "$ACC_SIGNED")
[ "$R" = "200" ] || { kill $RELAY_PID 2>/dev/null; exit 1; }
grep -qi 'accepted' /tmp/rl09.json || { kill $RELAY_PID 2>/dev/null; exit 1; }

# T-RL10: POST /rad/accept を非 Leader が実行 → 403
# Alice が utils.ts:1-10 に write（Alice が Leader になる）
OP2='{"participantId":"'$ALICE_ID'","type":"write","regionId":"utils.ts:1-10","content":"util1"}'
SIGNED2=$(echo "$OP2" | "$RUST" sign --secret-key "$SEC")
curl -s -o /tmp/rl_op2a.json -X POST $BASE/rad/operations \
  -H 'Content-Type: application/json' -d "$SIGNED2" > /dev/null 2>&1

# Bob が同じ領域に write（Bob は Follower）
OP3='{"participantId":"'$BOB_ID'","type":"write","regionId":"utils.ts:1-10","content":"util2"}'
SIGNED3=$(echo "$OP3" | "$RUST" sign --secret-key "$SEC2")
curl -s -o /tmp/rl_op2b.json -X POST $BASE/rad/operations \
  -H 'Content-Type: application/json' -d "$SIGNED3" > /dev/null 2>&1
OP_ID2=$(grep -o '"operationId":"[^"]*"' /tmp/rl_op2b.json | cut -d'"' -f4)

# Bob が accept しようとする（Bob は utils.ts:1-10 の Leader ではない）
ACC2='{"participantId":"'$BOB_ID'","operationId":"'$OP_ID2'"}'
ACC2_SIGNED=$(echo "$ACC2" | "$RUST" sign --secret-key "$SEC2")
R=$(curl -s -o /dev/null -w '%{http_code}' -X POST $BASE/rad/accept \
  -H 'Content-Type: application/json' -d "$ACC2_SIGNED")
[ "$R" = "403" ] || { kill $RELAY_PID 2>/dev/null; exit 1; }

# T-RL11: POST /rad/operations (reject, L→F, reason あり) → 201
# Alice が Bob の utils.ts write を reject
REJ='{"participantId":"'$ALICE_ID'","type":"reject","targetOperationId":"'$OP_ID2'","reason":"not good"}'
REJ_SIGNED=$(echo "$REJ" | "$RUST" sign --secret-key "$SEC")
R=$(curl -s -o /tmp/rl11.json -w '%{http_code}' -X POST $BASE/rad/operations \
  -H 'Content-Type: application/json' -d "$REJ_SIGNED")
[ "$R" = "201" ] || { kill $RELAY_PID 2>/dev/null; exit 1; }

# T-RL12: POST /rad/operations (reject, reason なし) → 400
# 新しい write を作成（test.ts に Bob が書く）
OP4='{"participantId":"'$BOB_ID'","type":"write","regionId":"test.ts:1-5","content":"test3"}'
SIGNED4=$(echo "$OP4" | "$RUST" sign --secret-key "$SEC2")
curl -s -o /tmp/rl_op4.json -X POST $BASE/rad/operations \
  -H 'Content-Type: application/json' -d "$SIGNED4" > /dev/null 2>&1
OP_ID4=$(grep -o '"operationId":"[^"]*"' /tmp/rl_op4.json | cut -d'"' -f4)

# Bob が reason なしで reject（test.ts の Leader は Bob なので可能だが reason なしは NG）
REJ2='{"participantId":"'$BOB_ID'","type":"reject","targetOperationId":"'$OP_ID4'"}'
REJ2_SIGNED=$(echo "$REJ2" | "$RUST" sign --secret-key "$SEC2")
R=$(curl -s -o /dev/null -w '%{http_code}' -X POST $BASE/rad/operations \
  -H 'Content-Type: application/json' -d "$REJ2_SIGNED")
[ "$R" = "400" ] || { kill $RELAY_PID 2>/dev/null; exit 1; }

# T-RL14: GET /rad/files/{filePath} → accepted なファイル内容
curl -s $BASE/rad/files/main.ts | grep -q 'hello' || { kill $RELAY_PID 2>/dev/null; exit 1; }

# T-RL15: GET /rad/regions/{filePath} → コード領域一覧
curl -s $BASE/rad/regions/main.ts | grep -q 'main.ts' || { kill $RELAY_PID 2>/dev/null; exit 1; }

# T-RL16: GET /rad/log → 操作ログ
curl -s "$BASE/rad/log" | grep -q '"write"' || { kill $RELAY_PID 2>/dev/null; exit 1; }

kill $RELAY_PID 2>/dev/null
rm -f /tmp/rl*.json /tmp/relay.log
