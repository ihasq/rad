#!/bin/bash
RUST="$1"

# 鍵ペア生成
ALICE_KEYS=$($RUST keygen)
ALICE_SEC=$(echo "$ALICE_KEYS" | sed -n '2p' | awk '{print $2}')
BOB_KEYS=$($RUST keygen)
BOB_SEC=$(echo "$BOB_KEYS" | sed -n '2p' | awk '{print $2}')
CAROL_KEYS=$($RUST keygen)
CAROL_SEC=$(echo "$CAROL_KEYS" | sed -n '2p' | awk '{print $2}')

# Group A: accept 基本

# T-A01: Leader が Follower の write を accept → status "accepted"
R_OUT=$(cat <<EOF | "$RUST" pipeline --ephemeral 2>&1
write main.ts 5 10 alice $ALICE_SEC "v1"
write main.ts 5 10 bob $BOB_SEC "v2"
accept @2 alice $ALICE_SEC
EOF
)
echo "$R_OUT" | sed -n '3p' | grep -q '"status":"accepted"' || exit 1

# T-A02: accept 後に chain 表示で [accepted] が表示される
R_CHAIN=$(cat <<EOF | "$RUST" pipeline --ephemeral 2>&1
write main.ts 5 10 alice $ALICE_SEC "v1"
write main.ts 5 10 bob $BOB_SEC "v2"
accept @2 alice $ALICE_SEC
chain main.ts 5 10
EOF
)
echo "$R_CHAIN" | grep -q '\[accepted\]' || exit 1

# T-A03: 非 Leader が accept を試みるとエラー
R_FAIL=$(cat <<EOF | "$RUST" pipeline --ephemeral 2>&1
write main.ts 5 10 alice $ALICE_SEC "v1"
write main.ts 5 10 bob $BOB_SEC "v2"
accept @2 bob $BOB_SEC
EOF
)
echo "$R_FAIL" | grep -qiE 'error.*leader|only.*leader' || exit 1

# Group B: 階段飛ばし

# T-A05, T-A06: 3番目を accept → 2番目が discarded
R_SKIP=$(cat <<EOF | "$RUST" pipeline --ephemeral 2>&1
write main.ts 5 10 alice $ALICE_SEC "v1"
write main.ts 5 10 bob $BOB_SEC "v2"
write main.ts 5 10 carol $CAROL_SEC "v3"
accept @3 alice $ALICE_SEC
chain main.ts 5 10
EOF
)
echo "$R_SKIP" | grep -q '\[discarded\]' || exit 1

# T-A07: discarded な操作への accept はエラー
R_DISCARD=$(cat <<EOF | "$RUST" pipeline --ephemeral 2>&1
write main.ts 5 10 alice $ALICE_SEC "v1"
write main.ts 5 10 bob $BOB_SEC "v2"
write main.ts 5 10 carol $CAROL_SEC "v3"
accept @3 alice $ALICE_SEC
accept @2 alice $ALICE_SEC
EOF
)
echo "$R_DISCARD" | grep -qiE 'error.*cannot accept|error.*status' || exit 1

# T-A08: 1番目(Leader自身の write)は discarded にならない
echo "$R_SKIP" | grep 'alice.*v1' | grep -q '\[visible\]' || exit 1
