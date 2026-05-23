#!/bin/bash
RUST="$1"
# T05: Rust の rad --help が 'Usage:' を含む
"$RUST" --help 2>&1 | grep -q 'Usage:' || exit 1
