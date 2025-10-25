#!/usr/bin/env bash
# 🧩 Describe a topic (usage: ./kafka-describe-topic.sh <topic>)

set -e
KAFKA_BIN="/srv/mistral/kafka/bin"
BOOTSTRAP="localhost:9092"
TOPIC="${1:-user_messages}"

"$KAFKA_BIN/kafka-configs.sh" \
  --bootstrap-server "$BOOTSTRAP" \
  --entity-type topics \
  --entity-name "$TOPIC" \
  --describe
