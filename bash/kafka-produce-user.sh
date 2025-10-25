#!/usr/bin/env bash
# 💬 Produce test message to user_messages

set -e
KAFKA_BIN="/srv/mistral/kafka/bin"
BOOTSTRAP="localhost:9092"

echo "Type a message and press Enter. Ctrl+D to exit."
"$KAFKA_BIN/kafka-console-producer.sh" \
  --bootstrap-server "$BOOTSTRAP" \
  --topic user_messages
