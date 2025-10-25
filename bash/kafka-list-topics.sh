#!/usr/bin/env bash
# 📜 List all Kafka topics on localhost

set -e
KAFKA_BIN="/srv/mistral/kafka/bin"
BOOTSTRAP="localhost:9092"

echo "🔍 Listing Kafka topics..."
"$KAFKA_BIN/kafka-topics.sh" --list --bootstrap-server "$BOOTSTRAP"
