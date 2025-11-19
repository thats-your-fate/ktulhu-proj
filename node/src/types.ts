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


// types/state.ts

export interface StateDelta {
  user_intent?: string;
  message_summary?: string;
  new_facts?: Array<{ fact?: string; value?: string }>;
}

export interface StateHistoryEntry {
  chat_id: string;
  state_delta: StateDelta;
  ts: number;
}

export interface StateHistoryResponse {
  chat_id: string;
  count: number;
  history: StateHistoryEntry[];
}

export interface MergedState {
  intents: string[];
  facts: string[];
  summary: string | null;
}
