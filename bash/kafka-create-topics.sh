#!/usr/bin/env bash
# 🏗️ Create all Kafka topics for the Ktulhu backend

set -e
KAFKA_BIN="/srv/mistral/kafka/bin"
BOOTSTRAP="localhost:9092"

# Consistent naming — all snake_case, one word per concept
TOPICS=(
  "messages"
  "conversation_state_delta"
)

echo "🚀 Creating Kafka topics..."
echo "🔌 Bootstrap server: $BOOTSTRAP"
echo

for t in "${TOPICS[@]}"; do
  echo "📦 Creating topic: $t"
  "$KAFKA_BIN/kafka-topics.sh" --create \
    --topic "$t" \
    --bootstrap-server "$BOOTSTRAP" \
    --partitions 3 \
    --replication-factor 1 \
    2>/dev/null || echo "⚠️ Topic $t already exists (skipping)"
done

echo
echo "✅ Done."
