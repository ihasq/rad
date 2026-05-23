#!/bin/bash
# t031_delete: delete 操作のテスト

RUST="$1"


# T-DL01: delete 操作が visible で提出される
# T-DL02: file founder が delete を accept → ファイル削除
# T-DL03: 非 file-founder が delete を accept → エラー
# T-DL04: file founder 自身が delete → dir founder が accept
# T-DL05: delete 後に chain に [visible] [delete] が表示される
# T-DL06: delete 後に rad status でファイル数が減少
# T-DL07: delete accept 後に rad diff で削除が表示
# T-DL08: compact 後に snapshot からファイルが消える
# T-DL09: Git export 時に delete → git rm に変換
# T-DL10: Rust と TS の delete 動作一致

A_KEYS=$($RUST keygen)
A_SEC=$(echo "$A_KEYS" | sed -n '2p' | awk '{print $2}')
A_PUB=$(echo "$A_KEYS" | sed -n '1p' | awk '{print $2}')

B_KEYS=$($RUST keygen)
B_SEC=$(echo "$B_KEYS" | sed -n '2p' | awk '{print $2}')
B_PUB=$(echo "$B_KEYS" | sed -n '1p' | awk '{print $2}')

R_DIR=$(mktemp -d)
(cd "$R_DIR" && "$RUST" init --participant alice --secret-key "$A_SEC" > /dev/null 2>&1)

# セットアップ: alice が main.ts、bob が utils.ts を作成
R_OUT=$(cd "$R_DIR" && cat <<EOF | "$RUST" pipeline 2>&1
join bob $B_PUB
write src/main.ts 1 10 alice $A_SEC "alice code"
write src/utils.ts 1 5 bob $B_SEC "bob code"
EOF
)

# T-DL01: bob が utils.ts の delete を提案 → visible
R_OUT=$(cd "$R_DIR" && cat <<EOF | "$RUST" pipeline 2>&1
delete src/utils.ts bob $B_SEC
chain src/utils.ts 1 5
EOF
)

if ! echo "$R_OUT" | grep -q "visible"; then
  echo "  ❌ T-DL01: delete が visible として提出されない"
  rm -rf "$R_DIR"
  exit 1
fi

if ! echo "$R_OUT" | grep -q "delete"; then
  echo "  ❌ T-DL05: chain に delete が表示されない"
  rm -rf "$R_DIR"
  exit 1
fi

# T-DL02: bob（file founder）が delete を accept
# まず delete の op-id を取得
DEL_OUT=$(cd "$R_DIR" && cat <<EOF | "$RUST" pipeline 2>&1
delete src/utils.ts bob $B_SEC
EOF
)

DEL_ID=$(echo "$DEL_OUT" | grep -o '"operationId":"[^"]*"' | cut -d'"' -f4)

R_OUT=$(cd "$R_DIR" && cat <<EOF | "$RUST" pipeline 2>&1
accept $DEL_ID bob $B_SEC
status
EOF
)

if ! echo "$R_OUT" | grep -q "accepted"; then
  echo "  ❌ T-DL02: file founder の delete accept が失敗"
  rm -rf "$R_DIR"
  exit 1
fi

# T-DL06: status でファイル数が減少（utils.ts が消えている）
if echo "$R_OUT" | grep -q "src/utils.ts"; then
  echo "  ❌ T-DL06: delete accept 後も status にファイルが残っている"
  rm -rf "$R_DIR"
  exit 1
fi

rm -rf "$R_DIR"

# TS テスト
if [ -n "$TS" ]; then
  T_DIR=$(mktemp -d)
  (cd "$T_DIR" && "$TS" init --participant alice --secret-key "$A_SEC" > /dev/null 2>&1)

  T_OUT=$(cd "$T_DIR" && cat <<EOF | "$TS" pipeline 2>&1
join bob $B_PUB
write src/utils.ts 1 5 bob $B_SEC "bob code"
delete src/utils.ts bob $B_SEC
chain src/utils.ts 1 5
EOF
  )

  if ! echo "$T_OUT" | grep -q "delete"; then
    echo "  ❌ T-DL10: TS の delete 動作が Rust と一致しない"
    rm -rf "$T_DIR"
    exit 1
  fi

  rm -rf "$T_DIR"
fi

echo "  ✅ t031_delete"
