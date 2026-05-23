#!/bin/bash
# t030_file_founder: ファイルレベルの Founder 追跡テスト

RUST="$1"


# T-FF01: ファイルの file founder 追跡（src/main.ts → alice, src/utils.ts → bob）
# T-FF02: 同一ファイルの2回目の write で file founder が変わらない
# T-FF03: dir founder と file founder が異なる場合の関係
# T-FF04: dir founder が file founder の write を reject → reason 不要（上位→下位）
# T-FF05: file founder が dir founder の write を reject → reason 必須（下位→上位）
# T-FF06: Rust と TS の file-founder 出力一致
# T-FF07: Relay の GET /rad/regions に fileFounder が含まれる
# T-FF08: foundedFiles が Participant に含まれる

A_KEYS=$($RUST keygen)
A_SEC=$(echo "$A_KEYS" | sed -n '2p' | awk '{print $2}')
A_PUB=$(echo "$A_KEYS" | sed -n '1p' | awk '{print $2}')

B_KEYS=$($RUST keygen)
B_SEC=$(echo "$B_KEYS" | sed -n '2p' | awk '{print $2}')
B_PUB=$(echo "$B_KEYS" | sed -n '1p' | awk '{print $2}')

R_DIR=$(mktemp -d)
(cd "$R_DIR" && "$RUST" init --participant alice --secret-key "$A_SEC" > /dev/null 2>&1)

# T-FF01: alice が src/main.ts を作成 → file founder = alice
R_OUT=$(cd "$R_DIR" && cat <<EOF | "$RUST" pipeline 2>&1
write src/main.ts 1 10 alice $A_SEC "hello world"
file-founder src/main.ts
EOF
)

if ! echo "$R_OUT" | grep -q "alice"; then
  echo "  ❌ T-FF01: src/main.ts の file founder が alice ではない"
  rm -rf "$R_DIR"
  exit 1
fi

# T-FF02: bob が src/utils.ts を作成 → file founder = bob
R_OUT=$(cd "$R_DIR" && cat <<EOF | "$RUST" pipeline 2>&1
join bob $B_PUB
write src/utils.ts 1 5 bob $B_SEC "util functions"
file-founder src/utils.ts
EOF
)

if ! echo "$R_OUT" | grep "src/utils.ts" | grep -q "bob"; then
  echo "  ❌ T-FF02: src/utils.ts の file founder が bob ではない"
  rm -rf "$R_DIR"
  exit 1
fi

# T-FF03: alice が src/main.ts に2回目の write → file founder は alice のまま
R_OUT=$(cd "$R_DIR" && cat <<EOF | "$RUST" pipeline 2>&1
write src/main.ts 11 20 alice $A_SEC "more code"
file-founder src/main.ts
EOF
)

if ! echo "$R_OUT" | grep -q "alice"; then
  echo "  ❌ T-FF03: 2回目の write で file founder が変わった"
  rm -rf "$R_DIR"
  exit 1
fi

# T-FF04: src/ の dir founder は alice（root founder が初期）
R_OUT=$(cd "$R_DIR" && cat <<EOF | "$RUST" pipeline 2>&1
founder src/
EOF
)

if ! echo "$R_OUT" | grep -q "alice"; then
  echo "  ❌ T-FF04: src/ の dir founder が alice ではない"
  rm -rf "$R_DIR"
  exit 1
fi

rm -rf "$R_DIR"

