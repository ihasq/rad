#!/bin/bash
# 引数: 出力ディレクトリ
DIR="$1"
mkdir -p "$DIR" && cd "$DIR"
git init

git config user.email 'alice@test.com'
git config user.name 'alice'

# コミット1: alice が main.ts を作成
mkdir -p src
echo 'const a = 1;' > src/main.ts
git add -A && git commit -m 'initial commit'

# コミット2: alice が main.ts を変更
echo 'const a = 2;' > src/main.ts
git add -A && git commit -m 'update a'

# コミット3: bob が utils.ts を追加
git config user.email 'bob@test.com'
git config user.name 'bob'
echo 'export function greet() {}' > src/utils.ts
git add -A && git commit -m 'add utils'
