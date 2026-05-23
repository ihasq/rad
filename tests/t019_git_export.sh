#!/bin/bash
RUST="$(realpath "$1")"

# Rust export test
R_DIR=$(mktemp -d)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
bash "$SCRIPT_DIR/helpers/create_git_repo.sh" "$R_DIR" > /dev/null 2>&1

KEYS=$("$RUST" keygen)
SEC=$(echo "$KEYS" | sed -n '2p' | awk '{print $2}')

(cd "$R_DIR" && "$RUST" init --participant exporter --secret-key "$SEC" > /dev/null 2>&1)
(cd "$R_DIR" && "$RUST" import > /dev/null 2>&1)

# 新しい write を追加
(cd "$R_DIR" && echo 'const b = 3;' > src/new.ts)
OP='{"participantId":"exporter","type":"write","regionId":"src/new.ts:1-1","content":"const b = 3;"}'
SIGNED=$(echo "$OP" | "$RUST" sign --secret-key "$SEC")
echo "$SIGNED" | (cd "$R_DIR" && "$RUST" pipeline --ephemeral > /dev/null 2>&1) || true

# Accept the operation
OPLOG=$(cat "$R_DIR/.rad/oplog.json")
LAST_OP_ID=$(echo "$OPLOG" | grep -o '"id":"[^"]*"' | tail -1 | cut -d'"' -f4)
if [ -n "$LAST_OP_ID" ]; then
    ACC='{"participantId":"exporter","operationId":"'$LAST_OP_ID'"}'
    ACC_SIGNED=$(echo "$ACC" | "$RUST" sign --secret-key "$SEC")
    echo "accept $LAST_OP_ID exporter dummy" | (cd "$R_DIR" && "$RUST" pipeline > /dev/null 2>&1) || true
fi

# Get commit count before export
BEFORE_COMMITS=$(git -C "$R_DIR" log --oneline | wc -l)

# T-GE01: rad export が exit 0 で終了する
(cd "$R_DIR" && "$RUST" export > /dev/null 2>&1) || { rm -rf "$R_DIR"; exit 1; }

# T-GE02: export 後に git log に新しいコミットが追加されている
AFTER_COMMITS=$(git -C "$R_DIR" log --oneline | wc -l)
[ "$AFTER_COMMITS" -gt "$BEFORE_COMMITS" ] || { rm -rf "$R_DIR"; exit 1; }

# T-GE03: export のコミットメッセージに "rad:" プレフィクスが含まれる
git -C "$R_DIR" log -1 --pretty=%B | grep -q 'rad:' || { rm -rf "$R_DIR"; exit 1; }

# T-GE04: export 後のファイル内容確認（基本チェック）
[ -f "$R_DIR/src/main.ts" ] || { rm -rf "$R_DIR"; exit 1; }

rm -rf "$R_DIR"
