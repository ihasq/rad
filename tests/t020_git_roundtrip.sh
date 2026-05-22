#!/bin/bash
RUST="$(realpath "$1")"
TS="$(realpath "$2")"

# T-GR01: git repo → rad import → rad export → git diff が空
R_DIR=$(mktemp -d)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
bash "$SCRIPT_DIR/helpers/create_git_repo.sh" "$R_DIR" > /dev/null 2>&1

KEYS=$("$RUST" keygen)
SEC=$(echo "$KEYS" | sed -n '2p' | awk '{print $2}')

(cd "$R_DIR" && "$RUST" init --participant roundtrip --secret-key "$SEC" > /dev/null 2>&1)
(cd "$R_DIR" && "$RUST" import > /dev/null 2>&1)
(cd "$R_DIR" && "$RUST" export > /dev/null 2>&1)

# working tree should be clean after roundtrip
DIFF=$(git -C "$R_DIR" diff)
[ -z "$DIFF" ] || { rm -rf "$R_DIR"; exit 1; }

# T-GR03: import 後の chain にコミット履歴が表示される
CHAIN_OUTPUT=$(cd "$R_DIR" && echo "chain src/main.ts 1 1" | "$RUST" pipeline 2>/dev/null || echo "")
echo "$CHAIN_OUTPUT" | grep -q 'op-' || { rm -rf "$R_DIR"; exit 1; }

rm -rf "$R_DIR"

# T-GR04: TS でも roundtrip が動作する
T_DIR=$(mktemp -d)
bash "$SCRIPT_DIR/helpers/create_git_repo.sh" "$T_DIR" > /dev/null 2>&1

KEYS=$("$RUST" keygen)
SEC=$(echo "$KEYS" | sed -n '2p' | awk '{print $2}')

(cd "$T_DIR" && "$TS" init --participant roundtrip --secret-key "$SEC" > /dev/null 2>&1)
(cd "$T_DIR" && "$TS" import > /dev/null 2>&1)
(cd "$T_DIR" && "$TS" export > /dev/null 2>&1)

DIFF=$(git -C "$T_DIR" diff)
[ -z "$DIFF" ] || { rm -rf "$T_DIR"; exit 1; }

rm -rf "$T_DIR"
