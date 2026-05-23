#!/bin/bash
cd "$(dirname "$0")"

# WASM ビルド
(cd ../rust && cargo build -p rad-wasm --target wasm32-unknown-unknown --release)

# dist ディレクトリ作成
mkdir -p dist

# WASM コピー
cp ../rust/target/wasm32-unknown-unknown/release/rad_wasm.wasm dist/rad_wasm.wasm 2>/dev/null || true

# TS ビルド
bun build src/main.ts --compile --outfile dist/rad
