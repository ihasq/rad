#!/bin/bash
RUST="$(realpath "$1")"
TS="$(realpath "$2")"

# Rust import test
R_DIR=$(mktemp -d)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
bash "$SCRIPT_DIR/helpers/create_git_repo.sh" "$R_DIR" > /dev/null 2>&1

KEYS=$("$RUST" keygen)
SEC=$(echo "$KEYS" | sed -n '2p' | awk '{print $2}')

# T-GI01: rad import が exit 0 で終了する（Rust）
(cd "$R_DIR" && "$RUST" init --participant importer --secret-key "$SEC" > /dev/null 2>&1)
(cd "$R_DIR" && "$RUST" import > /dev/null 2>&1) || { rm -rf "$R_DIR"; exit 1; }

# T-GI03: import 後に .rad/oplog.json にコミット数と同じ数の操作がある
OP_COUNT=$(cat "$R_DIR/.rad/oplog.json" | jq 'length' 2>/dev/null || echo 0)
[ "$OP_COUNT" = "3" ] || { rm -rf "$R_DIR"; exit 1; }

# T-GI04: import 後に .rad/participants.json にコミット作者が登録されている
grep -q 'alice' "$R_DIR/.rad/participants.json" || { rm -rf "$R_DIR"; exit 1; }
grep -q 'bob' "$R_DIR/.rad/participants.json" || { rm -rf "$R_DIR"; exit 1; }

# T-GI05: 全操作の status が accepted である
ACCEPTED_COUNT=$(cat "$R_DIR/.rad/oplog.json" | jq '[.[] | select(.status == "accepted")] | length' || echo 0)
[ "$ACCEPTED_COUNT" = "3" ] || { rm -rf "$R_DIR"; exit 1; }

# T-GI06: 全操作の signature が "git-imported" である
GIT_SIG_COUNT=$(cat "$R_DIR/.rad/oplog.json" | jq '[.[] | select(.signature == "git-imported")] | length' || echo 0)
[ "$GIT_SIG_COUNT" = "3" ] || { rm -rf "$R_DIR"; exit 1; }

# T-GI07: CodeRegion が生成されている
[ -f "$R_DIR/.rad/regions.json" ] || { rm -rf "$R_DIR"; exit 1; }
grep -q 'src/main.ts' "$R_DIR/.rad/regions.json" || { rm -rf "$R_DIR"; exit 1; }

rm -rf "$R_DIR"

# TS import test
T_DIR=$(mktemp -d)
bash "$SCRIPT_DIR/helpers/create_git_repo.sh" "$T_DIR" > /dev/null 2>&1

KEYS=$("$RUST" keygen)
SEC=$(echo "$KEYS" | sed -n '2p' | awk '{print $2}')

# T-GI02: rad import が exit 0 で終了する（TS）
(cd "$T_DIR" && "$TS" init --participant importer --secret-key "$SEC" > /dev/null 2>&1)
(cd "$T_DIR" && "$TS" import > /dev/null 2>&1) || { rm -rf "$T_DIR"; exit 1; }

# T-GI08: Rust と TS の import 結果が一致する（基本構造）
T_OP_COUNT=$(cat "$T_DIR/.rad/oplog.json" | jq 'length' 2>/dev/null || echo 0)
[ "$T_OP_COUNT" = "3" ] || { rm -rf "$T_DIR"; exit 1; }

rm -rf "$T_DIR"
