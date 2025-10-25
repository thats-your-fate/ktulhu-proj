#!/usr/bin/env bash
# ♻️ Set retention for user_messages to 7 days

set -e
KAFKA_BIN="/srv/mistral/kafka/bin"
BOOTSTRAP="localhost:9092"
TOPIC="${1:-user_messages}"

"$KAFKA_BIN/kafka-configs.sh" \
  --bootstrap-server "$BOOTSTRAP" \
  --alter \
  --entity-type topics \
  --entity-name "$TOPIC" \
  --add-config retention.ms=604800000,cleanup.policy=delete

echo "✅ Retention set to 7 days for topic $TOPIC"
