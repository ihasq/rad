#!/bin/bash
RUST="$(realpath "$1")"

# Setup: create git repo with initial state
R_DIR=$(mktemp -d)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
bash "$SCRIPT_DIR/helpers/create_git_repo.sh" "$R_DIR" > /dev/null 2>&1

# Generate keys
KEYS=$("$RUST" keygen)
ALICE_SEC=$(echo "$KEYS" | sed -n '2p' | awk '{print $2}')

KEYS_B=$("$RUST" keygen)
BOB_PUB=$(echo "$KEYS_B" | head -1 | awk '{print $2}')
BOB_SEC=$(echo "$KEYS_B" | sed -n '2p' | awk '{print $2}')

# Init and import
(cd "$R_DIR" && "$RUST" init --participant importer --secret-key "$ALICE_SEC" > /dev/null 2>&1)
(cd "$R_DIR" && "$RUST" import > /dev/null 2>&1)

# Create some operations with bob (already a participant from git import)
echo "write src/utils.ts 1 10 bob $BOB_SEC \"export function helper() {}\"" | (cd "$R_DIR" && "$RUST" pipeline > /dev/null 2>&1)
echo "write src/main.ts 15 20 bob $BOB_SEC \"const updated = true;\"" | (cd "$R_DIR" && "$RUST" pipeline > /dev/null 2>&1)

# T-LG01: rad log が全操作を時系列で表示する（Rust）
R_OUT=$(cd "$R_DIR" && "$RUST" log 2>&1)
R_EXIT=$?
[ $R_EXIT -eq 0 ] || { rm -rf "$R_DIR"; exit 1; }

# T-LG02: 各行に [status] participant file content が含まれる
echo "$R_OUT" | grep -q '\[' || { rm -rf "$R_DIR"; exit 1; }
echo "$R_OUT" | grep -q 'alice\|bob' || { rm -rf "$R_DIR"; exit 1; }
echo "$R_OUT" | grep -q 'src/' || { rm -rf "$R_DIR"; exit 1; }

# T-LG03: --participant フィルタが動作する
R_BOB=$(cd "$R_DIR" && "$RUST" log --participant bob 2>&1)
echo "$R_BOB" | grep -q 'bob' || { rm -rf "$R_DIR"; exit 1; }
# Alice should not appear in bob filter
echo "$R_BOB" | grep -q 'alice' && { rm -rf "$R_DIR"; exit 1; }

# T-LG04: --file フィルタが動作する
R_UTILS=$(cd "$R_DIR" && "$RUST" log --file src/utils.ts 2>&1)
echo "$R_UTILS" | grep -q 'src/utils.ts' || { rm -rf "$R_DIR"; exit 1; }

# T-LG05: --status visible フィルタが動作する
R_VISIBLE=$(cd "$R_DIR" && "$RUST" log --status visible 2>&1)
echo "$R_VISIBLE" | grep -q 'visible' || { rm -rf "$R_DIR"; exit 1; }

# T-LG06: フィルタなしの出力に全操作が含まれる
# Should have multiple operations (at least 3 from import + 2 writes)
LINE_COUNT=$(echo "$R_OUT" | wc -l)
[ $LINE_COUNT -ge 5 ] || { rm -rf "$R_DIR"; exit 1; }

rm -rf "$R_DIR"
