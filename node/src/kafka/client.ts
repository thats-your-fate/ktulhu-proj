import { Kafka, Producer } from "kafkajs";
import { log } from "../utils/logger";

let producer: Producer | null = null;

export async function ensureKafka(): Promise<Producer> {
  if (producer) return producer;


  const brokers =
    process.env.KAFKA_BROKERS?.split(",").map((b) => b.trim()) ||
    ["localhost:9092"];
  const clientId = process.env.KAFKA_CLIENT_ID || "ktulhu_backend";

  log.info(`🔌 Initializing Kafka producer`);
  log.info(JSON.stringify({ clientId, brokers }, null, 2));

  const kafka = new Kafka({ clientId, brokers });
  producer = kafka.producer();

  try {
    await producer.connect();
    log.ok("🪣 Connected to Kafka broker");
  } catch (err: any) {
    log.err(`❌ Kafka connection failed: ${err.message}`);
    throw err;
  }

  return producer;
}
