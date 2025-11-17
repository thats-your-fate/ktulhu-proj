#!/usr/bin/env bash
# 🛰️ Consume messages from user_messages (from start)

set -e
KAFKA_BIN="/srv/mistral/kafka/bin"
BOOTSTRAP="localhost:9092"

"$KAFKA_BIN/kafka-console-consumer.sh" \
  --bootstrap-server "$BOOTSTRAP" \
  --topic conversation_state_delta \
  --from-beginning \
  --timeout-ms 10000 \
  --property print.key=true \
  --property print.value=true
