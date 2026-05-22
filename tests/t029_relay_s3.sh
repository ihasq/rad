#!/bin/bash
RUST="$(realpath "$1")"
TS="$(realpath "$2")"

# MinIO health check - skip if not available
curl -s http://localhost:19000/minio/health/live > /dev/null 2>&1 || {
  echo "MinIO not available, skipping S3 Relay tests"
  exit 77
}

PORT_TS=18960
PORT_RUST=18961
S3_OPTS="--storage s3 --s3-endpoint http://localhost:19000 --s3-bucket rad-relay-test --s3-access-key radtest --s3-secret-key radtest123 --s3-region us-east-1"

# Generate keys
KEYS=$("$RUST" keygen)
PUB=$(echo "$KEYS" | head -1 | awk '{print $2}')
SEC=$(echo "$KEYS" | sed -n '2p' | awk '{print $2}')

# Clean S3 bucket before tests
# (MinIO CLI not available, will be cleaned by test operations)

# --- TS Relay S3 Test ---
# T-RS01: Start TS Relay with S3
"$TS" relay --port $PORT_TS $S3_OPTS > /tmp/ts-relay-s3.log 2>&1 &
TS_PID=$!
sleep 3

# Check if relay is responding
curl -s http://localhost:$PORT_TS/rad/participants > /dev/null || {
  echo "TS Relay failed to start with S3"
  kill $TS_PID 2>/dev/null
  exit 1
}

# Join
curl -s -X POST http://localhost:$PORT_TS/rad/participants \
  -H 'Content-Type: application/json' \
  -d '{"publicKey":"'"$PUB"'","displayName":"alice"}' > /dev/null

# Get participant ID
ALICE_ID=$(curl -s http://localhost:$PORT_TS/rad/participants | grep -o '"participantId":"[^"]*"' | head -1 | cut -d'"' -f4)
if [ -z "$ALICE_ID" ]; then
  ALICE_ID="alice"
fi

# Write operation (signed)
OP='{"participantId":"'$ALICE_ID'","type":"write","regionId":"main.ts:1-10","content":"hello s3"}'
SIGNED=$(echo "$OP" | "$RUST" sign --secret-key "$SEC")
curl -s -X POST http://localhost:$PORT_TS/rad/operations \
  -H 'Content-Type: application/json' -d "$SIGNED" > /dev/null

# T-RS03: Restart TS Relay and verify log persists
kill $TS_PID 2>/dev/null
sleep 2

"$TS" relay --port $PORT_TS $S3_OPTS > /tmp/ts-relay-s3-2.log 2>&1 &
TS_PID=$!
sleep 3

curl -s http://localhost:$PORT_TS/rad/log | grep -q "hello s3" || {
  echo "TS Relay: operation not persisted after restart"
  kill $TS_PID 2>/dev/null
  exit 1
}

# T-RS06: Verify participants persist
curl -s http://localhost:$PORT_TS/rad/participants | grep -q "alice" || {
  echo "TS Relay: participants not persisted after restart"
  kill $TS_PID 2>/dev/null
  exit 1
}

kill $TS_PID 2>/dev/null
sleep 1

# T-RS09: Start without --storage flag (in-memory mode)
"$TS" relay --port $PORT_TS > /tmp/ts-relay-memory.log 2>&1 &
TS_PID=$!
sleep 2

# Should start with empty state (no S3 data visible)
LOG=$(curl -s http://localhost:$PORT_TS/rad/log)
if echo "$LOG" | grep -q "hello s3"; then
  echo "TS Relay: in-memory mode showing S3 data (should be isolated)"
  kill $TS_PID 2>/dev/null
  exit 1
fi

kill $TS_PID 2>/dev/null
sleep 1

# --- Rust Relay S3 Test ---
# T-RS02: Start Rust Relay with S3
"$RUST" relay --port $PORT_RUST $S3_OPTS > /tmp/rust-relay-s3.log 2>&1 &
RUST_PID=$!
sleep 3

curl -s http://localhost:$PORT_RUST/rad/participants > /dev/null || {
  echo "Rust Relay failed to start with S3"
  kill $RUST_PID 2>/dev/null
  exit 1
}

# Verify data from TS Relay is visible (shared S3 bucket)
curl -s http://localhost:$PORT_RUST/rad/log | grep -q "hello s3" || {
  echo "Rust Relay: cannot read TS Relay data from S3"
  kill $RUST_PID 2>/dev/null
  exit 1
}

kill $RUST_PID 2>/dev/null

# All tests passed
exit 0
