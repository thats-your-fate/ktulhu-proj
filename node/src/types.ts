export interface ChatMessage {
  id: string;   // NEW: REQUIRED
  role: "user" | "assistant" | "summary" | "system";
  chat_id: string;
  user_id?: string | null;  
  session_id?: string;
  device_hash?: string | null;
  text?: string;
  summary?: string;
  ts: number;
}
