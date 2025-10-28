import { Kafka, Producer } from "kafkajs";
import { log } from "../utils/logger";
import { CONFIG } from "../config";

let producer: Producer | null = null;

export async function ensureKafka(): Promise<Producer> {
  if (!producer) {
    const kafka = new Kafka({
      clientId: CONFIG.clientId,
      brokers: CONFIG.kafkaBrokers,
    });
    producer = kafka.producer();
    try {
      await producer.connect();
      log.ok("🪣 Connected to Kafka broker");
    } catch (err: any) {
      log.err(`Kafka connection failed: ${err.message}`);
      throw err;
    }
  }
  return producer;
}
