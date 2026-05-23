#!/bin/bash
RUST="$(realpath "$1")"

# 鍵ペア生成
KEYS=$($RUST keygen)
PUB=$(echo "$KEYS" | head -1 | awk '{print $2}')
SEC=$(echo "$KEYS" | sed -n '2p' | awk '{print $2}')

# Rust init
R_DIR=$(mktemp -d)
R_OUT=$(cd "$R_DIR" && "$RUST" init --participant alice --secret-key "$SEC" 2>&1)
R_EXIT=$?

# T-I01: exit 0
[ $R_EXIT -eq 0 ] || { rm -rf "$R_DIR"; exit 1; }

# T-I03: .rad/config.json に founder
grep -q 'founder' "$R_DIR/.rad/config.json" || { rm -rf "$R_DIR"; exit 1; }

# T-I04: participants に1件
grep -q 'alice' "$R_DIR/.rad/participants.json" || { rm -rf "$R_DIR"; exit 1; }

# T-I05: 出力形式
echo "$R_OUT" | grep -q 'initialized:' || { rm -rf "$R_DIR"; exit 1; }
echo "$R_OUT" | grep -q 'founder:' || { rm -rf "$R_DIR"; exit 1; }

rm -rf "$R_DIR"
