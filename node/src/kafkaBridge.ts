import { parentPort } from "worker_threads";
import { Kafka } from "kafkajs";

const kafka = new Kafka({
  clientId: "ws-proxy-bridge",
  brokers: ["localhost:9092"],
});

const consumer = kafka.consumer({ groupId: "chat-stream-bridge" });

(async () => {
  await consumer.connect();
  await consumer.subscribe({ topic: "assistant_responses", fromBeginning: false });

  // Optional: if you really need user_messages too, just uncomment next line
  // await consumer.subscribe({ topic: "user_messages", fromBeginning: false });

  await consumer.run({
    autoCommitInterval: 5000,
eachMessage: async ({ topic, message }) => {
  try {
    const val = message.value?.toString();
    if (!val) return;
    const payload = JSON.parse(val);

    // 🧠 Normalize message shape for frontend
    let normalized = payload;

    // Case 1: Python summary from assistant_responses
    if (payload.id && payload.summary) {
      normalized = {
        chat_id: payload.id,     // ✅ rename for React
        summary: payload.summary,
        ts: payload.ts || Date.now(),
      };
    }

    // Case 2: user_messages or other
    if (payload.chat_id && !payload.summary && payload.message?.text) {
      normalized = {
        chat_id: payload.chat_id,
        summary: payload.message.text.slice(0, 60), // short preview
        ts: payload.ts || Date.now(),
      };
    }

    // ✅ Forward normalized message to parent WS layer
    parentPort?.postMessage({ topic, payload: normalized });
  } catch (err) {
    console.error("Kafka bridge parse error:", err);
  }
}

  });

  console.log("🪣 Kafka bridge running");
})();
