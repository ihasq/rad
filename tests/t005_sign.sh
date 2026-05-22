#!/bin/bash
RUST="$1"; TS="$2"

# 共通テスト用の鍵ペアと Operation を生成
RUST_KEYS=$($RUST keygen)
PUB=$(echo "$RUST_KEYS" | head -1 | awk '{print $2}')
SEC=$(echo "$RUST_KEYS" | sed -n '2p' | awk '{print $2}')

OP='{"content":"const a = 1;","id":"op-001","participantId":"alice","reason":null,"regionId":"main.ts:5-10","timestamp":1716000000,"type":"write"}'

# T-S01: Rust sign exit 0
RUST_SIGNED=$(echo "$OP" | "$RUST" sign --secret-key "$SEC" 2>&1) || exit 1

# T-S02: TS sign exit 0
TS_SIGNED=$(echo "$OP" | "$TS" sign --secret-key "$SEC" 2>&1) || exit 1

# T-S03: Rust 出力に signature あり
echo "$RUST_SIGNED" | grep -q '"signature"' || exit 1

# T-S04: TS 出力に signature あり
echo "$TS_SIGNED" | grep -q '"signature"' || exit 1

# T-S05: Rust sign → Rust verify
echo "$RUST_SIGNED" | "$RUST" verify --public-key "$PUB" 2>&1 | grep -q 'valid' || exit 1

# T-S06: TS sign → TS verify
echo "$TS_SIGNED" | "$TS" verify --public-key "$PUB" 2>&1 | grep -q 'valid' || exit 1

# T-S07: Rust sign → TS verify（相互検証）
echo "$RUST_SIGNED" | "$TS" verify --public-key "$PUB" 2>&1 | grep -q 'valid' || exit 1

# T-S08: TS sign → Rust verify（相互検証）
echo "$TS_SIGNED" | "$RUST" verify --public-key "$PUB" 2>&1 | grep -q 'valid' || exit 1
