#!/usr/bin/env bash
KAFKA_HOME="/srv/mistral/kafka"
PID_FILE="$KAFKA_HOME/logs/kafka.pid"

if [ -f "$PID_FILE" ]; then
  PID=$(cat "$PID_FILE")
  echo "🛑 Stopping Kafka (PID: $PID)..."
  kill "$PID" && rm -f "$PID_FILE"
  echo "✅ Kafka stopped."
else
  echo "⚠️  No Kafka PID file found."
fi
