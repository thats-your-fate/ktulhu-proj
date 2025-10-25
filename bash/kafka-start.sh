#!/usr/bin/env bash
set -e
KAFKA_HOME="/srv/mistral/kafka"
LOG_DIR="$KAFKA_HOME/logs"
DATA_DIR="$KAFKA_HOME/data"
CONFIG_FILE="$KAFKA_HOME/config/kraft/server.properties"

mkdir -p "$DATA_DIR" "$LOG_DIR"

if ! grep -q "process.roles" "$CONFIG_FILE"; then
  echo "⚙️  Initializing Kafka KRaft storage..."
  CLUSTER_ID=$("$KAFKA_HOME/bin/kafka-storage.sh" random-uuid)
  "$KAFKA_HOME/bin/kafka-storage.sh" format \
    -t "$CLUSTER_ID" \
    -c "$CONFIG_FILE"
fi

echo "🚀 Starting Kafka (KRaft mode)..."
"$KAFKA_HOME/bin/kafka-server-start.sh" "$CONFIG_FILE" \
  > "$LOG_DIR/kafka.log" 2>&1 &

PID=$!
echo $PID > "$LOG_DIR/kafka.pid"
echo "✅ Kafka started with PID: $PID"
