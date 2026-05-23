#!/bin/bash
RUST="$(realpath "$1")"

# Setup: create git repo with initial state
R_DIR=$(mktemp -d)
bash "$SCRIPT_DIR/helpers/create_git_repo.sh" "$R_DIR" > /dev/null 2>&1

# Generate keys
KEYS=$("$RUST" keygen)
ALICE_SEC=$(echo "$KEYS" | sed -n '2p' | awk '{print $2}')

KEYS_B=$("$RUST" keygen)
BOB_PUB=$(echo "$KEYS_B" | head -1 | awk '{print $2}')
BOB_SEC=$(echo "$KEYS_B" | sed -n '2p' | awk '{print $2}')

KEYS_C=$("$RUST" keygen)
CAROL_PUB=$(echo "$KEYS_C" | head -1 | awk '{print $2}')
CAROL_SEC=$(echo "$KEYS_C" | sed -n '2p' | awk '{print $2}')

# Init and import
(cd "$R_DIR" && "$RUST" init --participant importer --secret-key "$ALICE_SEC" > /dev/null 2>&1)
(cd "$R_DIR" && "$RUST" import > /dev/null 2>&1)

# Create some visible writes from bob (bob is already a participant from git import)
echo "write src/utils.ts 1 10 bob $BOB_SEC \"export function helper() {}\"" | (cd "$R_DIR" && "$RUST" pipeline > /dev/null 2>&1)
echo "write src/main.ts 5 10 bob $BOB_SEC \"const updated = true;\"" | (cd "$R_DIR" && "$RUST" pipeline > /dev/null 2>&1)

# Create a visible write to a new file
echo "write src/lib.ts 1 5 bob $BOB_SEC \"new file content\"" | (cd "$R_DIR" && "$RUST" pipeline > /dev/null 2>&1)

# T-ST01: rad status が exit 0 で終了する（Rust）
R_OUT=$(cd "$R_DIR" && "$RUST" status 2>&1)
R_EXIT=$?
[ $R_EXIT -eq 0 ] || { rm -rf "$R_DIR"; exit 1; }

# T-ST02: 出力に "founder:" が含まれる
echo "$R_OUT" | grep -q 'founder:' || { rm -rf "$R_DIR"; exit 1; }

# T-ST03: 出力に "participants:" と件数が含まれる
echo "$R_OUT" | grep -q 'participants:' || { rm -rf "$R_DIR"; exit 1; }

# T-ST04: 出力に "operations:" と件数・内訳が含まれる
echo "$R_OUT" | grep -q 'operations:' || { rm -rf "$R_DIR"; exit 1; }

# T-ST05: visible writes の一覧が表示される
echo "$R_OUT" | grep -q 'visible' || { rm -rf "$R_DIR"; exit 1; }

bash "$SCRIPT_DIR/helpers/create_git_repo.sh" "$T_DIR" > /dev/null 2>&1



T_EXIT=$?

R_NORM=$(echo "$R_OUT" | sed -E 's/op-[0-9]+-[0-9]+/OP-ID/g')
T_NORM=$(echo "$T_OUT" | sed -E 's/op-[0-9]+-[0-9]+/OP-ID/g')

