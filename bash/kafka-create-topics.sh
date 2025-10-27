#!/usr/bin/env bash
# 🏗️ Create all project topics if not exist

set -e
KAFKA_BIN="/srv/mistral/kafka/bin"
BOOTSTRAP="localhost:9092"

TOPICS=("messages" "assistant_responses")

for t in "${TOPICS[@]}"; do
  echo "📦 Creating topic: $t"
  "$KAFKA_BIN/kafka-topics.sh" --create \
    --topic "$t" \
    --bootstrap-server "$BOOTSTRAP" \
    --partitions 1 \
    --replication-factor 1 || echo "⚠️ Topic $t may already exist"
done
