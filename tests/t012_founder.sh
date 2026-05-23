#!/bin/bash
RUST="$(realpath "$1")"

# 鍵ペア生成
A_KEYS=$($RUST keygen)
A_SEC=$(echo "$A_KEYS" | sed -n '2p' | awk '{print $2}')
B_KEYS=$($RUST keygen)
B_SEC=$(echo "$B_KEYS" | sed -n '2p' | awk '{print $2}')

# Rust: init + pipeline
R_DIR=$(mktemp -d)
(cd "$R_DIR" && "$RUST" init --participant alice --secret-key "$A_SEC" > /dev/null 2>&1)

R_OUT=$(cd "$R_DIR" && cat <<EOF | "$RUST" pipeline 2>&1
write src/main.ts 1 10 alice $A_SEC "hello"
write src/components/Button.tsx 1 5 bob $B_SEC "button"
founder .
founder src/
founder src/components/
EOF
)

# T-F02: root Founder = alice
echo "$R_OUT" | grep -E '^\.: founder: alice$' || { rm -rf "$R_DIR"; exit 1; }

# T-F03: src/components/ Founder = bob (最初に write したのが bob)
echo "$R_OUT" | grep -E '^src/components/: founder: bob$' || { rm -rf "$R_DIR"; exit 1; }

# T-F04: 上位 Founder (alice) が下位 Founder (bob) の write を reject (reason 不要)
R_REJECT_UP=$(cd "$R_DIR" && cat <<EOF | "$RUST" pipeline 2>&1
write src/components/Button.tsx 1 5 bob $B_SEC "v1"
reject @1 alice $A_SEC
EOF
)
echo "$R_REJECT_UP" | grep -q '"status":"rejected"' || { rm -rf "$R_DIR"; exit 1; }

# T-F05: 下位 Founder (bob) が上位 Founder (alice) の write を reject (reason 必須)
R_REJECT_DOWN=$(cd "$R_DIR" && cat <<EOF | "$RUST" pipeline 2>&1
write src/main.ts 1 10 alice $A_SEC "v1"
reject @1 bob $B_SEC
EOF
)
echo "$R_REJECT_DOWN" | grep -qiE 'error.*reason' || { rm -rf "$R_DIR"; exit 1; }

