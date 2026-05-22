#!/bin/bash
RUST="$(realpath "$1")"; TS="$(realpath "$2")"

# 鍵ペア生成
KEYS=$($RUST keygen)
SEC=$(echo "$KEYS" | sed -n '2p' | awk '{print $2}')

# Rust: compact テスト
R_DIR=$(mktemp -d)
(cd "$R_DIR" && "$RUST" init --participant alice --secret-key "$SEC" > /dev/null 2>&1)
(cd "$R_DIR" && echo "write src/main.ts 1 10 alice $SEC \"hello\"" | "$RUST" pipeline > /dev/null 2>&1)

# accept
OP_ID=$(grep -o '"id":"[^"]*"' "$R_DIR/.rad/oplog.json" | head -1 | cut -d'"' -f4)
(cd "$R_DIR" && echo "accept $OP_ID alice $SEC" | "$RUST" pipeline > /dev/null 2>&1)

# compact
(cd "$R_DIR" && "$RUST" compact > /dev/null 2>&1)

# T-P01: snapshots にファイル
ls "$R_DIR/.rad/snapshots/src/main.ts" 2>/dev/null || { rm -rf "$R_DIR"; exit 1; }

# T-P02: oplog から accepted 削除
! grep -q '"status":"accepted"' "$R_DIR/.rad/oplog.json" || { rm -rf "$R_DIR"; exit 1; }

# T-P03: snapshot 内容
cat "$R_DIR/.rad/snapshots/src/main.ts" | grep -q 'hello' || { rm -rf "$R_DIR"; exit 1; }

# T-P04: バッチ書き込み (10件の write)
B_DIR=$(mktemp -d)
(cd "$B_DIR" && "$RUST" init --participant alice --secret-key "$SEC" > /dev/null 2>&1)
for i in {1..10}; do
  (cd "$B_DIR" && echo "write file$i.ts 1 10 alice $SEC \"content$i\"" | "$RUST" pipeline > /dev/null 2>&1)
done
# 全件が永続化されているか確認
OPLOG_COUNT=$(grep -o '"id":"op-[^"]*"' "$B_DIR/.rad/oplog.json" | wc -l)
[ "$OPLOG_COUNT" -eq 10 ] || { rm -rf "$R_DIR" "$B_DIR"; exit 1; }

# T-P05: .rad/ なしでエラー
EMPTY=$(mktemp -d)
ERROR_OUT=$(cd "$EMPTY" && echo 'chain x 1 1' | "$RUST" pipeline 2>&1)
echo "$ERROR_OUT" | grep -qi 'error\|not initialized\|not a rad project' || { rm -rf "$R_DIR" "$B_DIR" "$EMPTY"; exit 1; }

# T-P06: 破損した oplog.json からのリカバリ
CORRUPT_DIR=$(mktemp -d)
(cd "$CORRUPT_DIR" && "$RUST" init --participant alice --secret-key "$SEC" > /dev/null 2>&1)
# 不正な JSON を書き込む
echo "{ invalid json" > "$CORRUPT_DIR/.rad/oplog.json"
CORRUPT_OUT=$(cd "$CORRUPT_DIR" && echo 'chain x 1 1' | "$RUST" pipeline 2>&1)
echo "$CORRUPT_OUT" | grep -qi 'error\|invalid\|corrupt' || { rm -rf "$R_DIR" "$B_DIR" "$EMPTY" "$CORRUPT_DIR"; exit 1; }

rm -rf "$R_DIR" "$B_DIR" "$EMPTY" "$CORRUPT_DIR"
