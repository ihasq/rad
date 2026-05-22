#!/bin/bash
RUST="$1"; TS="$2"
# T01: Rust バイナリが存在し実行可能
[ -x "$RUST" ] || exit 1
# T02: TS バイナリが存在し実行可能
[ -x "$TS" ] || exit 1
# T03: Rust の rad --version が 'rad 0.0.1' を含む
"$RUST" --version 2>&1 | grep -q 'rad 0.0.1' || exit 1
# T04: TS の rad --version が 'rad 0.0.1' を含む
"$TS" --version 2>&1 | grep -q 'rad 0.0.1' || exit 1
# T07: 出力が完全一致
RUST_OUT=$("$RUST" --version 2>&1)
TS_OUT=$("$TS" --version 2>&1)
[ "$RUST_OUT" = "$TS_OUT" ] || exit 1
