#!/bin/bash
RUST="$1"; TS="$2"

# 既存の relay プロセスをクリーンアップ
for pid in $(lsof -t -i:19000 -i:19001 2>/dev/null); do kill -9 $pid 2>/dev/null; done
killall -9 rad 2>/dev/null || true
sleep 1

# TS Relay と Rust Relay を別ポートで起動
TS_PORT=19000
RUST_PORT=19001

"$TS" relay --port $TS_PORT > /tmp/relay-ts-parity.log 2>&1 &
TS_PID=$!
"$RUST" relay --port $RUST_PORT > /tmp/relay-rust-parity.log 2>&1 &
RUST_PID=$!

sleep 3  # 起動待ち

TS_BASE="http://localhost:$TS_PORT"
RUST_BASE="http://localhost:$RUST_PORT"

# 鍵ペア生成
KEYS=$($RUST keygen)
PUB=$(echo "$KEYS" | head -1 | awk '{print $2}')
SEC=$(echo "$KEYS" | sed -n '2p' | awk '{print $2}')

# T-RP01: join リクエストのレスポンス構造一致
TS_JOIN=$(curl -s -X POST $TS_BASE/rad/participants \
  -H 'Content-Type: application/json' \
  -d '{"publicKey":"'$PUB'","displayName":"alice"}')
RUST_JOIN=$(curl -s -X POST $RUST_BASE/rad/participants \
  -H 'Content-Type: application/json' \
  -d '{"publicKey":"'$PUB'","displayName":"alice"}')

# キー名の存在を確認（値は異なるため構造のみチェック）
echo "$TS_JOIN" | grep -q 'participantId' || { kill $TS_PID $RUST_PID 2>/dev/null; exit 1; }
echo "$TS_JOIN" | grep -q 'isFounder' || { kill $TS_PID $RUST_PID 2>/dev/null; exit 1; }
echo "$RUST_JOIN" | grep -q 'participantId' || { kill $TS_PID $RUST_PID 2>/dev/null; exit 1; }
echo "$RUST_JOIN" | grep -q 'isFounder' || { kill $TS_PID $RUST_PID 2>/dev/null; exit 1; }

# participantId 取得
TS_PID_VAL=$(echo "$TS_JOIN" | grep -o '"participantId":"[^"]*"' | cut -d'"' -f4)
RUST_PID_VAL=$(echo "$RUST_JOIN" | grep -o '"participantId":"[^"]*"' | cut -d'"' -f4)

# T-RP02: write リクエストのレスポンス構造一致
OP='{"participantId":"'$TS_PID_VAL'","type":"write","regionId":"main.ts:1-10","content":"hello"}'
SIGNED=$(echo "$OP" | "$RUST" sign --secret-key "$SEC")

TS_WRITE=$(curl -s -X POST $TS_BASE/rad/operations \
  -H 'Content-Type: application/json' -d "$SIGNED")

OP_RUST='{"participantId":"'$RUST_PID_VAL'","type":"write","regionId":"main.ts:1-10","content":"hello"}'
SIGNED_RUST=$(echo "$OP_RUST" | "$RUST" sign --secret-key "$SEC")

RUST_WRITE=$(curl -s -X POST $RUST_BASE/rad/operations \
  -H 'Content-Type: application/json' -d "$SIGNED_RUST")

echo "$TS_WRITE" | grep -q 'operationId' || { kill $TS_PID $RUST_PID 2>/dev/null; exit 1; }
echo "$TS_WRITE" | grep -qi 'status' || { kill $TS_PID $RUST_PID 2>/dev/null; exit 1; }
echo "$RUST_WRITE" | grep -q 'operationId' || { kill $TS_PID $RUST_PID 2>/dev/null; exit 1; }
echo "$RUST_WRITE" | grep -qi 'status' || { kill $TS_PID $RUST_PID 2>/dev/null; exit 1; }

# operationId 取得
TS_OP_ID=$(echo "$TS_WRITE" | grep -o '"operationId":"[^"]*"' | cut -d'"' -f4)
RUST_OP_ID=$(echo "$RUST_WRITE" | grep -o '"operationId":"[^"]*"' | cut -d'"' -f4)

# T-RP03: accept リクエストのレスポンス構造一致
ACC='{"participantId":"'$TS_PID_VAL'","operationId":"'$TS_OP_ID'"}'
ACC_SIGNED=$(echo "$ACC" | "$RUST" sign --secret-key "$SEC")

TS_ACC=$(curl -s -X POST $TS_BASE/rad/accept \
  -H 'Content-Type: application/json' -d "$ACC_SIGNED")

ACC_RUST='{"participantId":"'$RUST_PID_VAL'","operationId":"'$RUST_OP_ID'"}'
ACC_SIGNED_RUST=$(echo "$ACC_RUST" | "$RUST" sign --secret-key "$SEC")

RUST_ACC=$(curl -s -X POST $RUST_BASE/rad/accept \
  -H 'Content-Type: application/json' -d "$ACC_SIGNED_RUST")

echo "$TS_ACC" | grep -q 'operationId' || { kill $TS_PID $RUST_PID 2>/dev/null; exit 1; }
echo "$TS_ACC" | grep -qi 'status' || { kill $TS_PID $RUST_PID 2>/dev/null; exit 1; }
echo "$RUST_ACC" | grep -q 'operationId' || { kill $TS_PID $RUST_PID 2>/dev/null; exit 1; }
echo "$RUST_ACC" | grep -qi 'status' || { kill $TS_PID $RUST_PID 2>/dev/null; exit 1; }

# T-RP04: log リクエストのレスポンス構造一致
TS_LOG=$(curl -s "$TS_BASE/rad/log")
RUST_LOG=$(curl -s "$RUST_BASE/rad/log")

# 配列形式であることを確認
echo "$TS_LOG" | grep -q '^\[' || { kill $TS_PID $RUST_PID 2>/dev/null; exit 1; }
echo "$RUST_LOG" | grep -q '^\[' || { kill $TS_PID $RUST_PID 2>/dev/null; exit 1; }
# 操作が含まれることを確認
echo "$TS_LOG" | grep -q '"write"' || { kill $TS_PID $RUST_PID 2>/dev/null; exit 1; }
echo "$RUST_LOG" | grep -q '"write"' || { kill $TS_PID $RUST_PID 2>/dev/null; exit 1; }

kill $TS_PID $RUST_PID 2>/dev/null
rm -f /tmp/relay-ts-parity.log /tmp/relay-rust-parity.log
