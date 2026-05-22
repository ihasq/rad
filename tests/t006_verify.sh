#!/bin/bash
RUST="$1"; TS="$2"

KEYS=$($RUST keygen)
PUB=$(echo "$KEYS" | head -1 | awk '{print $2}')
SEC=$(echo "$KEYS" | sed -n '2p' | awk '{print $2}')
OTHER_KEYS=$($RUST keygen)
OTHER_PUB=$(echo "$OTHER_KEYS" | head -1 | awk '{print $2}')

OP='{"content":"x","id":"op-002","participantId":"bob","reason":null,"regionId":"r1","timestamp":1716000001,"type":"write"}'
SIGNED=$(echo "$OP" | "$RUST" sign --secret-key "$SEC")

# T-V01: 正しい署名 → valid + exit 0
echo "$SIGNED" | "$RUST" verify --public-key "$PUB" 2>&1 | grep -q '^valid$' || exit 1

# T-V02: 改竄 → invalid + exit 1
TAMPERED=$(echo "$SIGNED" | sed 's/"content":"x"/"content":"y"/')
echo "$TAMPERED" | "$RUST" verify --public-key "$PUB" 2>&1 | grep -q '^invalid$' || exit 1

# T-V03: 公開鍵不一致 → invalid + exit 1
echo "$SIGNED" | "$RUST" verify --public-key "$OTHER_PUB" 2>&1 | grep -q '^invalid$' || exit 1

# T-V04: Rust と TS の verify 結果一致
R_RESULT=$(echo "$SIGNED" | "$RUST" verify --public-key "$PUB" 2>&1)
T_RESULT=$(echo "$SIGNED" | "$TS" verify --public-key "$PUB" 2>&1)
[ "$R_RESULT" = "$T_RESULT" ] || exit 1
