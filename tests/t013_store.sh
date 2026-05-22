#!/bin/bash
RUST="$(realpath "$1")"; TS="$(realpath "$2")"

# 鍵ペア生成
A_KEYS=$($RUST keygen)
A_SEC=$(echo "$A_KEYS" | sed -n '2p' | awk '{print $2}')
B_KEYS=$($RUST keygen)
B_SEC=$(echo "$B_KEYS" | sed -n '2p' | awk '{print $2}')

# Rust: init + write in session 1
R_DIR=$(mktemp -d)
(cd "$R_DIR" && "$RUST" init --participant alice --secret-key "$A_SEC" > /dev/null 2>&1)
(cd "$R_DIR" && echo "write src/main.ts 1 10 alice $A_SEC \"v1\"" | "$RUST" pipeline > /dev/null 2>&1)

# T-S01: oplog.json にエントリ
grep -q 'src/main.ts' "$R_DIR/.rad/oplog.json" || { rm -rf "$R_DIR"; exit 1; }

# T-S02: 別セッションで chain 表示
R_CHAIN=$(cd "$R_DIR" && echo 'chain src/main.ts 1 10' | "$RUST" pipeline 2>&1)
echo "$R_CHAIN" | grep -q 'v1' || { rm -rf "$R_DIR"; exit 1; }

# write の op-id を oplog.json から取得
OP_ID=$(grep -o '"id":"[^"]*"' "$R_DIR/.rad/oplog.json" | head -1 | cut -d'"' -f4)

# T-S05: セッション2 で accept
(cd "$R_DIR" && echo "accept $OP_ID alice $A_SEC" | "$RUST" pipeline > /dev/null 2>&1)

# T-S03: oplog の status が updated
grep -qi 'accepted' "$R_DIR/.rad/oplog.json" || { rm -rf "$R_DIR"; exit 1; }

# T-S04: region register 後に regions.json にエントリ
# (既に write で region が登録されている)
grep -q 'src/main.ts' "$R_DIR/.rad/regions.json" || { rm -rf "$R_DIR"; exit 1; }

# TS: 同一手順
T_DIR=$(mktemp -d)
(cd "$T_DIR" && "$TS" init --participant alice --secret-key "$A_SEC" > /dev/null 2>&1)
(cd "$T_DIR" && echo "write src/main.ts 1 10 alice $A_SEC \"v1\"" | "$TS" pipeline > /dev/null 2>&1)

# T-S06: セッション間で chain 表示
T_CHAIN=$(cd "$T_DIR" && echo 'chain src/main.ts 1 10' | "$TS" pipeline 2>&1)
echo "$T_CHAIN" | grep -q 'v1' || { rm -rf "$R_DIR" "$T_DIR"; exit 1; }

# T-S08: chain 出力一致
R_NORM=$(echo "$R_CHAIN" | sed 's/op-[a-zA-Z0-9_-]*/op-ID/g' | sed 's/t=[0-9]*/t=T/g')
T_NORM=$(echo "$T_CHAIN" | sed 's/op-[a-zA-Z0-9_-]*/op-ID/g' | sed 's/t=[0-9]*/t=T/g')
[ "$R_NORM" = "$T_NORM" ] || { rm -rf "$R_DIR" "$T_DIR"; exit 1; }

# T-S07: .rad/ ディレクトリ構造一致
R_STRUCTURE=$(cd "$R_DIR" && find .rad -type f | sort)
T_STRUCTURE=$(cd "$T_DIR" && find .rad -type f | sort)
[ "$R_STRUCTURE" = "$T_STRUCTURE" ] || { rm -rf "$R_DIR" "$T_DIR"; exit 1; }

rm -rf "$R_DIR" "$T_DIR"
