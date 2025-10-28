export interface ChatMessage {
  role: "user" | "assistant" | "summary";
  chat_id: string;
  session_id?: string;
  device_hash?: string | null;
  text?: string;
  summary?: string;
  ts: number;
}
