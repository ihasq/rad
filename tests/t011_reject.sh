#!/bin/bash
RUST="$1"

# 鍵ペア生成
ALICE_KEYS=$($RUST keygen)
ALICE_SEC=$(echo "$ALICE_KEYS" | sed -n '2p' | awk '{print $2}')
BOB_KEYS=$($RUST keygen)
BOB_SEC=$(echo "$BOB_KEYS" | sed -n '2p' | awk '{print $2}')

# Group D: reject 基本

# T-R01: Leader → Follower の reject に reason あり → status "rejected"
R_REJECT=$(cat <<EOF | "$RUST" pipeline --ephemeral 2>&1
write main.ts 5 10 alice $ALICE_SEC "v1"
write main.ts 5 10 bob $BOB_SEC "v2"
reject @2 alice $ALICE_SEC "not good"
EOF
)
echo "$R_REJECT" | sed -n '3p' | grep -q '"status":"rejected"' || exit 1
echo "$R_REJECT" | sed -n '3p' | grep -q '"reason":"not good"' || exit 1

# T-R02: Leader → Follower の reject に reason なし → エラー
R_NO_REASON=$(cat <<EOF | "$RUST" pipeline --ephemeral 2>&1
write main.ts 5 10 alice $ALICE_SEC "v1"
write main.ts 5 10 bob $BOB_SEC "v2"
reject @2 alice $ALICE_SEC
EOF
)
echo "$R_NO_REASON" | grep -qiE 'error.*reason|must provide' || exit 1

# T-R03: Follower → Leader の reject（reason なし）→ 成功
R_F_TO_L=$(cat <<EOF | "$RUST" pipeline --ephemeral 2>&1
write main.ts 5 10 alice $ALICE_SEC "v1"
write main.ts 5 10 bob $BOB_SEC "v2"
reject @1 bob $BOB_SEC
EOF
)
echo "$R_F_TO_L" | sed -n '3p' | grep -q '"status":"rejected"' || exit 1

# T-R04: reject 後の chain で [rejected] が表示される
R_CHAIN=$(cat <<EOF | "$RUST" pipeline --ephemeral 2>&1
write main.ts 5 10 alice $ALICE_SEC "v1"
write main.ts 5 10 bob $BOB_SEC "v2"
reject @2 alice $ALICE_SEC "bad code"
chain main.ts 5 10
EOF
)
echo "$R_CHAIN" | grep -q '\[rejected\]' || exit 1

# Group E: reject エッジケース

# T-R05: 既に rejected な操作への reject はエラー
R_DOUBLE=$(cat <<EOF | "$RUST" pipeline --ephemeral 2>&1
write main.ts 5 10 alice $ALICE_SEC "v1"
write main.ts 5 10 bob $BOB_SEC "v2"
reject @2 alice $ALICE_SEC "reason1"
reject @2 alice $ALICE_SEC "reason2"
EOF
)
echo "$R_DOUBLE" | grep -qiE 'error.*cannot reject|error.*not visible' || exit 1

# T-R06: accepted な操作への reject はエラー
R_ACCEPT_REJECT=$(cat <<EOF | "$RUST" pipeline --ephemeral 2>&1
write main.ts 5 10 alice $ALICE_SEC "v1"
write main.ts 5 10 bob $BOB_SEC "v2"
accept @2 alice $ALICE_SEC
reject @2 alice $ALICE_SEC "too late"
EOF
)
echo "$R_ACCEPT_REJECT" | grep -qiE 'error.*cannot reject|error.*not visible' || exit 1

