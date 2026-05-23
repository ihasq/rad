#!/bin/bash
RUST="$1"

# 鍵ペア生成
ALICE_KEYS=$($RUST keygen)
ALICE_PUB=$(echo "$ALICE_KEYS" | head -1 | awk '{print $2}')
ALICE_SEC=$(echo "$ALICE_KEYS" | sed -n '2p' | awk '{print $2}')
BOB_KEYS=$($RUST keygen)
BOB_PUB=$(echo "$BOB_KEYS" | head -1 | awk '{print $2}')
BOB_SEC=$(echo "$BOB_KEYS" | sed -n '2p' | awk '{print $2}')

# Rust テスト
R_OUT=$(cat <<EOF | "$RUST" pipeline --ephemeral 2>&1
write main.ts 5 10 alice $ALICE_SEC "const a = 1;"
write main.ts 5 10 bob $BOB_SEC "const a = 2;"
write utils.ts 1 5 alice $ALICE_SEC "export fn"
EOF
)

# T-W01: status visible
echo "$R_OUT" | head -1 | grep -q '"status":"visible"' || exit 1

# T-W02: id フィールド
echo "$R_OUT" | head -1 | grep -q '"id"' || exit 1

# T-W03: 署名検証
FIRST_OP=$(echo "$R_OUT" | head -1)
echo "$FIRST_OP" | "$RUST" verify --public-key "$ALICE_PUB" 2>&1 | grep -q 'valid' || exit 1

# T-W04: 未登録領域への write が Leader 登録
echo "$R_OUT" | sed -n '3p' | grep -q '"status":"visible"' || exit 1

# T-W07: 同一領域2回目成功
echo "$R_OUT" | sed -n '2p' | grep -q '"status":"visible"' || exit 1

