#!/bin/bash
RUST="$1"

# T-K01: Rust exit 0
"$RUST" keygen > /tmp/rad-keygen-rust.txt 2>&1 || exit 1

# T-K03: Rust 1行目が 'public:  ' で始まる
head -1 /tmp/rad-keygen-rust.txt | grep -q '^public:  ' || exit 1

# T-K05: Rust 2行目が 'secret:  ' で始まる
sed -n '2p' /tmp/rad-keygen-rust.txt | grep -q '^secret:  ' || exit 1

rm -f /tmp/rad-keygen-rust.txt
