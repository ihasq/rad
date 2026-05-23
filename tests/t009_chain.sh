#!/bin/bash
RUST="$1"
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

