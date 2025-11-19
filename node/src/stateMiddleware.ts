// stateMiddleware.ts
import {
  StateHistoryResponse,
  StateHistoryEntry,
  StateDelta,
  MergedState
} from "./types";

const BASE_URL = "https://persistence.ktulhu.com";

// -----------------------------------------------
// 🔥 Properly typed fetchChatState()
// -----------------------------------------------
export async function fetchChatState(chatId: string): Promise<MergedState> {
  const url = `${BASE_URL}/state-delta/history/${chatId}`;

  const res = await fetch(url);
  if (!res.ok) {
    return { intents: [], facts: [], summary: null };
  }

  const json: StateHistoryResponse = await res.json();
  const history: StateHistoryEntry[] = json.history || [];

  const intents: string[] = [];
  const facts: string[] = [];
  let summary: string | null = null;

  // -----------------------------------------------
  // Merge deltas into single memory object
  // -----------------------------------------------
  for (const item of history) {
    const delta: StateDelta = item.state_delta || {};

    // intents
    if (typeof delta.user_intent === "string") {
      intents.push(delta.user_intent);
    }

    // facts
    if (Array.isArray(delta.new_facts)) {
      for (const f of delta.new_facts) {
        if (typeof f === "object") {
          facts.push(f.fact || f.value || JSON.stringify(f));
        }
      }
    }

    // summary (take latest)
    if (typeof delta.message_summary === "string") {
      summary = delta.message_summary;
    }
  }

  return { intents, facts, summary };
}
