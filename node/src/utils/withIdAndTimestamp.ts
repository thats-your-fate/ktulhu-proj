import crypto from "crypto";
import { ChatMessage } from "../types";


export function withIdAndTimestamp(msg: Partial<ChatMessage>): ChatMessage {
  return {
    id: crypto.randomUUID(),
    ts: Date.now(),
    ...msg,
  } as ChatMessage;
}
