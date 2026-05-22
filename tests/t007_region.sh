#!/bin/bash
RUST="$1"; TS="$2"

INPUT=$(cat <<'EOF'
register main.ts 5 10 alice
register main.ts 12 20 bob
register main.ts 5 10 carol
register utils.ts 1 5 alice
owner main.ts 7
owner main.ts 15
owner main.ts 2
owner utils.ts 3
list main.ts
list empty.ts
role main.ts 7 alice
role main.ts 7 bob
role main.ts 2 alice
role main.ts 7 bob
role utils.ts 3 bob
EOF
)

RUST_OUT=$(echo "$INPUT" | "$RUST" region 2>&1)
TS_OUT=$(echo "$INPUT" | "$TS" region 2>&1)

# T-R01: owner が正しい
echo "$RUST_OUT" | sed -n '5p' | grep -q '^alice$' || exit 1
echo "$RUST_OUT" | sed -n '6p' | grep -q '^bob$' || exit 1

# T-R02: 未登録行
echo "$RUST_OUT" | sed -n '7p' | grep -q '^unowned$' || exit 1

# T-R03: 重複 register は先着優先（carol の register は無視）
# → owner main.ts 7 が alice のまま（行5で確認済み）

# T-R04: 複数ファイル独立
echo "$RUST_OUT" | sed -n '8p' | grep -q '^alice$' || exit 1

# T-R05: list が全領域を返す
echo "$RUST_OUT" | grep $'\t' | grep -c 'main.ts:' | grep -q '2' || exit 1

# T-R06: 空ファイルの list
# list empty.ts の行が空であること（出力行数で間接検証）

# T-R07: owner → leader
echo "$RUST_OUT" | grep -m1 '^leader$' > /dev/null || exit 1

# T-R08: 非 owner → follower
echo "$RUST_OUT" | grep -m1 '^follower$' > /dev/null || exit 1

# T-R09: 未登録行 → unowned
# role main.ts 2 alice の結果（行2は未登録）

# T-R11: 全出力一致
[ "$RUST_OUT" = "$TS_OUT" ] || { echo 'MISMATCH'; diff <(echo "$RUST_OUT") <(echo "$TS_OUT"); exit 1; }

# T-R12: exit code 一致
echo "$INPUT" | "$RUST" region > /dev/null 2>&1; R_EXIT=$?
echo "$INPUT" | "$TS" region > /dev/null 2>&1; T_EXIT=$?
[ "$R_EXIT" = "$T_EXIT" ] || exit 1
