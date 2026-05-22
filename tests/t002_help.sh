#!/bin/bash
RUST="$1"; TS="$2"
# T05: Rust の rad --help が 'Usage:' を含む
"$RUST" --help 2>&1 | grep -q 'Usage:' || exit 1
# T06: TS の rad --help が 'Usage:' を含む
"$TS" --help 2>&1 | grep -q 'Usage:' || exit 1
# T08: 出力が完全一致
RUST_OUT=$("$RUST" --help 2>&1)
TS_OUT=$("$TS" --help 2>&1)
[ "$RUST_OUT" = "$TS_OUT" ] || exit 1
