#!/bin/bash

# T-WH01-T-WH08: TS から WASM をロードしてホスト関数を検証
# このテストは ts/src/wasm/test.ts を実行する

cd ts || exit 1

# WASM ファイルを ts/ にコピー
WASM_SRC="../rust/target/wasm32-unknown-unknown/release/rad_wasm.wasm"
WASM_DST="rad_wasm.wasm"

[ -f "$WASM_SRC" ] || {
  echo "WASM file not found: $WASM_SRC"
  exit 1
}

cp "$WASM_SRC" "$WASM_DST"

# TS test harness を実行
if [ -f "src/wasm/test.ts" ]; then
  bun run src/wasm/test.ts > /dev/null 2>&1
  TEST_EXIT=$?
  rm -f "$WASM_DST"
  [ $TEST_EXIT -eq 0 ] || exit 1
else
  # test.ts がまだない場合は SKIP
  rm -f "$WASM_DST"
  exit 77
fi

exit 0
