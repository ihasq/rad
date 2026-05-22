#!/bin/bash
RUST="$(realpath "$1")"
TS="$(realpath "$2")"

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

# T-DF04: visible write がない場合 "no pending changes" が表示される
R_EMPTY=$(cd "$R_DIR" && "$RUST" diff 2>&1)
echo "$R_EMPTY" | grep -q 'no pending changes' || { rm -rf "$R_DIR"; exit 1; }

# Create some visible writes from bob (already a participant from git import)
echo "write src/utils.ts 1 10 bob $BOB_SEC \"export function helper() {}\"" | (cd "$R_DIR" && "$RUST" pipeline > /dev/null 2>&1)
echo "write src/newfile.ts 1 5 bob $BOB_SEC \"const x = 1;\"" | (cd "$R_DIR" && "$RUST" pipeline > /dev/null 2>&1)

# T-DF01: accepted と visible の差分が表示される（Rust）
R_OUT=$(cd "$R_DIR" && "$RUST" diff 2>&1)
R_EXIT=$?
[ $R_EXIT -eq 0 ] || { rm -rf "$R_DIR"; exit 1; }

# T-DF02: 差分に --- / +++ ヘッダが含まれる
echo "$R_OUT" | grep -q -- '---' || { rm -rf "$R_DIR"; exit 1; }
echo "$R_OUT" | grep -q -- '+++' || { rm -rf "$R_DIR"; exit 1; }

# T-DF03: 新規ファイルの visible write が "(new file)" として表示される
echo "$R_OUT" | grep -q 'new file' || { rm -rf "$R_DIR"; exit 1; }

# TS test
T_DIR=$(mktemp -d)
bash "$SCRIPT_DIR/helpers/create_git_repo.sh" "$T_DIR" > /dev/null 2>&1

KEYS=$("$RUST" keygen)
ALICE_SEC=$(echo "$KEYS" | sed -n '2p' | awk '{print $2}')

KEYS_B=$("$RUST" keygen)
BOB_PUB=$(echo "$KEYS_B" | head -1 | awk '{print $2}')
BOB_SEC=$(echo "$KEYS_B" | sed -n '2p' | awk '{print $2}')

(cd "$T_DIR" && "$TS" init --participant importer --secret-key "$ALICE_SEC" > /dev/null 2>&1)
(cd "$T_DIR" && "$TS" import > /dev/null 2>&1)

echo "write src/utils.ts 1 10 bob $BOB_SEC \"export function helper() {}\"" | (cd "$T_DIR" && "$TS" pipeline > /dev/null 2>&1)
echo "write src/newfile.ts 1 5 bob $BOB_SEC \"const x = 1;\"" | (cd "$T_DIR" && "$TS" pipeline > /dev/null 2>&1)

T_OUT=$(cd "$T_DIR" && "$TS" diff 2>&1)
T_EXIT=$?

# T-DF05: Rust と TS の diff 出力が一致する（op-id 正規化後）
R_NORM=$(echo "$R_OUT" | sed -E 's/op-[0-9]+-[0-9]+/OP-ID/g')
T_NORM=$(echo "$T_OUT" | sed -E 's/op-[0-9]+-[0-9]+/OP-ID/g')
[ "$R_NORM" = "$T_NORM" ] || { rm -rf "$R_DIR" "$T_DIR"; exit 1; }

# T-DF06: exit code が両実装で一致する
[ $R_EXIT -eq $T_EXIT ] || { rm -rf "$R_DIR" "$T_DIR"; exit 1; }

rm -rf "$R_DIR" "$T_DIR"
