#!/bin/bash
RUST="$1"
# T01: Rust バイナリが存在し実行可能
[ -x "$RUST" ] || exit 1
# T03: Rust の rad --version が 'rad 0.0.1' を含む
"$RUST" --version 2>&1 | grep -q 'rad 0.0.1' || exit 1
