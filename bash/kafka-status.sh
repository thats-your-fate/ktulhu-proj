#!/usr/bin/env bash
# 🩺 Quick check of Kafka service

set -e

if systemctl is-active --quiet kafka; then
  echo "✅ Kafka is running (systemd)"
else
  echo "⚠️ Kafka not managed by systemd, checking process list..."
  ps aux | grep kafka | grep -v grep || echo "❌ Kafka process not found"
fi

echo
echo "Active topics:"
/srv/mistral/kafka/bin/kafka-topics.sh --list --bootstrap-server localhost:9092
