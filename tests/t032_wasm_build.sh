#!/bin/bash

# T-WB01: wasm32-unknown-unknown ターゲットでビルド成功
cd rust || exit 1
cargo build -p rad-wasm --target wasm32-unknown-unknown --release > /dev/null 2>&1
BUILD_EXIT=$?
[ $BUILD_EXIT -eq 0 ] || { echo "T-WB01: WASM build failed"; exit 1; }

# T-WB02: .wasm ファイルが生成される
WASM_PATH="target/wasm32-unknown-unknown/release/rad_wasm.wasm"
[ -f "$WASM_PATH" ] || { echo "T-WB02: .wasm file not found"; exit 1; }

# T-WB03: wasm ファイルサイズが 5MB 以下
WASM_SIZE=$(stat -c%s "$WASM_PATH" 2>/dev/null || stat -f%z "$WASM_PATH" 2>/dev/null)
MAX_SIZE=$((5 * 1024 * 1024))  # 5MB
[ "$WASM_SIZE" -le "$MAX_SIZE" ] || {
  echo "T-WB03: WASM size $WASM_SIZE exceeds 5MB limit"
  exit 1
}

# T-WB04: wasm-opt で最適化後のサイズが元より小さい（wasm-opt 存在時のみ）
if command -v wasm-opt > /dev/null 2>&1; then
  OPT_PATH="target/wasm32-unknown-unknown/release/rad_wasm_opt.wasm"
  wasm-opt -Oz -o "$OPT_PATH" "$WASM_PATH" > /dev/null 2>&1
  if [ -f "$OPT_PATH" ]; then
    OPT_SIZE=$(stat -c%s "$OPT_PATH" 2>/dev/null || stat -f%z "$OPT_PATH" 2>/dev/null)
    [ "$OPT_SIZE" -lt "$WASM_SIZE" ] || {
      echo "T-WB04: Optimized WASM not smaller than original"
      exit 1
    }
  fi
fi

exit 0
