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
        // Filter or transform before posting
        parentPort?.postMessage({ topic, payload });
      } catch (err) {
        console.error("Kafka bridge parse error:", err);
      }
    },
  });

  console.log("🪣 Kafka bridge running");
})();
