#!/usr/bin/env bash
# 🔁 Reset offsets for consumer group (usage: ./kafka-reset-offsets.sh <group> <topic>)

set -e
KAFKA_BIN="/srv/mistral/kafka/bin"
BOOTSTRAP="localhost:9092"
GROUP="${1:-chat-summary-consumer}"
TOPIC="${2:-user_messages}"

"$KAFKA_BIN/kafka-consumer-groups.sh" \
  --bootstrap-server "$BOOTSTRAP" \
  --group "$GROUP" \
  --topic "$TOPIC" \
  --reset-offsets --to-earliest --execute

echo "✅ Offsets reset for group $GROUP on topic $TOPIC"
