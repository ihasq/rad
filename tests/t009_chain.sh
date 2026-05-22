#!/bin/bash
RUST="$1"; TS="$2"
KEYS=$($RUST keygen)
SEC=$(echo "$KEYS" | sed -n '2p' | awk '{print $2}')
KEYS2=$($RUST keygen)
SEC2=$(echo "$KEYS2" | sed -n '2p' | awk '{print $2}')

R_OUT=$(cat <<EOF | "$RUST" pipeline --ephemeral 2>&1
write main.ts 5 10 alice $SEC "v1"
write main.ts 5 10 bob $SEC2 "v2"
chain main.ts 5 10
EOF
)

# T-C01: 時系列順（v1 が v2 より前）
CHAIN=$(echo "$R_OUT" | tail -n +4)
FIRST_LINE=$(echo "$CHAIN" | head -1)
SECOND_LINE=$(echo "$CHAIN" | sed -n '2p')
echo "$FIRST_LINE" | grep -q 'v1' || exit 1
echo "$SECOND_LINE" | grep -q 'v2' || exit 1

# T-C02: status 表示
echo "$CHAIN" | grep -q 'visible' || exit 1

# T-C03: 参加者名
echo "$CHAIN" | grep -q 'alice' || exit 1

# T-C04: content
echo "$CHAIN" | grep -q 'v1' || exit 1

# T-C05: Rust/TS 出力一致（id 除外）
T_OUT=$(cat <<EOF | "$TS" pipeline --ephemeral 2>&1
write main.ts 5 10 alice $SEC "v1"
write main.ts 5 10 bob $SEC2 "v2"
chain main.ts 5 10
EOF
)
R_NORM=$(echo "$R_OUT" | sed 's/op-[a-zA-Z0-9_-]*/op-ID/g' | sed 's/t=[0-9]*/t=T/g')
T_NORM=$(echo "$T_OUT" | sed 's/op-[a-zA-Z0-9_-]*/op-ID/g' | sed 's/t=[0-9]*/t=T/g')
[ "$R_NORM" = "$T_NORM" ] || exit 1
